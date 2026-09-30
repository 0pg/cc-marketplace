//! Bounded candidate reranking with one unverified source-context feedback pass.
//! All source identity/range decisions remain here, outside the model process.
use super::{PreparedSearch, SemanticChunkMatch, SemanticError, projection};
use crate::model::{Record, TextRange};
use crate::security::{RedactionPolicy, hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

const METHOD: &str = "qwen-local-context-feedback-v1";
const FEEDBACK_LABEL: &str = "\n\nRetrieved candidate context (unverified):\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RerankConfig {
    pub command: Vec<String>,
    pub model_id: String,
    pub model_revision: String,
    pub pool_limit: usize,
    pub context_chars: usize,
    pub feedback_chars: usize,
    pub seed_min_score: f64,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            model_id: String::new(),
            model_revision: String::new(),
            pool_limit: 50,
            context_chars: 768,
            feedback_chars: 768,
            seed_min_score: 0.013634464528877288,
        }
    }
}

impl RerankConfig {
    pub(super) fn validate(&self) -> Result<(), SemanticError> {
        if self.model_id.is_empty()
            || self.model_revision.is_empty()
            || !(1..=50).contains(&self.pool_limit)
            || !(16..=768).contains(&self.context_chars)
            || !(1..=768).contains(&self.feedback_chars)
            || !self.seed_min_score.is_finite()
            || !(0.0..=1.0).contains(&self.seed_min_score)
        {
            return Err(SemanticError::InvalidConfig(
                "reranker requires a pinned model and bounded pool/context/seed limits".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RerankRequest {
    pub protocol: u32,
    pub model_id: String,
    pub model_revision: String,
    pub query: String,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RerankResponse {
    pub protocol: u32,
    pub model_id: String,
    pub model_revision: String,
    pub scores: Vec<f32>,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
}

pub trait RerankProvider {
    fn score(
        &self,
        request: &RerankRequest,
        timeout_ms: u64,
    ) -> Result<RerankResponse, SemanticError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankMatch {
    pub initial_score: f32,
    /// Relevance score after optional feedback; never factual confidence.
    pub score: f32,
    /// Expanded source contexts; their scores belong to the final reranking.
    pub chunks: Vec<SemanticChunkMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankSeed {
    pub project_id: String,
    pub source_id: String,
    pub record_id: String,
    pub revision: String,
    pub projection_revision: String,
    /// The exact retained range used in the feedback query.
    pub context: SemanticChunkMatch,
    /// The potentially larger context used to select this seed initially.
    pub scored_context: SemanticChunkMatch,
    pub initial_score: f32,
    pub text_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankInfo {
    pub model_id: String,
    pub model_revision: String,
    pub method: String,
    pub stages: u8,
    pub seed: Option<RerankSeed>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RerankMetrics {
    pub unique_contexts: usize,
    pub initial_ms: u64,
    pub feedback_ms: u64,
    pub backend: BTreeMap<String, f64>,
}

struct Context {
    input: usize,
    location: SemanticChunkMatch,
    prefix: String,
    body: String,
}

#[derive(Default)]
struct Plan {
    documents: Vec<String>,
    contexts: Vec<Vec<Context>>,
}

pub(super) fn apply(
    prepared: &mut PreparedSearch,
    records: &[Record],
    query: &str,
    config: &RerankConfig,
    policy: &RedactionPolicy,
    provider: &impl RerankProvider,
    deadline: Instant,
) -> Result<(), SemanticError> {
    if prepared.candidates.len() > config.pool_limit {
        prepared.omitted.push(format!(
            "embedding candidates beyond top {} omitted before reranking",
            config.pool_limit
        ));
        prepared.candidates.truncate(config.pool_limit);
    }
    let plan = build_plan(prepared, records, config, policy, deadline)?;
    let mut request = RerankRequest {
        protocol: 1,
        model_id: config.model_id.clone(),
        model_revision: config.model_revision.clone(),
        query: query.to_owned(),
        documents: plan.documents,
    };
    let mut info = RerankInfo {
        model_id: config.model_id.clone(),
        model_revision: config.model_revision.clone(),
        method: METHOD.into(),
        stages: 0,
        seed: None,
    };
    let mut metrics = RerankMetrics {
        unique_contexts: request.documents.len(),
        ..Default::default()
    };
    if request.documents.is_empty() {
        prepared.reranking = Some(info);
        prepared.metrics.reranking = Some(metrics);
        return Ok(());
    }
    let started = Instant::now();
    let initial = score(provider, &request, deadline)?;
    metrics.initial_ms = super::millis(started);
    add_metrics(&mut metrics, "initial", &initial.metrics);
    info.stages = 1;
    let mut selected: Option<(usize, usize, f32)> = None;
    // Candidate identity, then field/range, breaks ties independently of scores.
    let mut order: Vec<_> = prepared.candidates.iter().enumerate().collect();
    order.sort_by_key(|(_, c)| (&c.project_id, &c.source_id, &c.record_id));
    for (candidate_index, _) in order {
        let contexts = plan
            .contexts
            .get(candidate_index)
            .ok_or(SemanticError::Stale)?;
        let mut context_order: Vec<_> = contexts.iter().enumerate().collect();
        context_order.sort_by_key(|(_, c)| (&c.location.field, c.location.byte_start));
        for (context_index, context) in context_order {
            let value = value(&initial.scores, context.input)?;
            if selected.is_none_or(|(_, _, best)| value > best) {
                selected = Some((candidate_index, context_index, value));
            }
        }
    }
    let final_scores = if let Some((candidate_index, context_index, seed_score)) = selected
        && f64::from(seed_score) >= config.seed_min_score
    {
        let candidate = prepared
            .candidates
            .get(candidate_index)
            .ok_or(SemanticError::Stale)?;
        let context = plan
            .contexts
            .get(candidate_index)
            .and_then(|contexts| contexts.get(context_index))
            .ok_or(SemanticError::Stale)?;
        let original_input = request
            .documents
            .get(context.input)
            .ok_or(SemanticError::Stale)?;
        let text: String = original_input.chars().take(config.feedback_chars).collect();
        let mut scored_context = context.location.clone();
        scored_context.score = seed_score;
        info.seed = Some(RerankSeed {
            project_id: candidate.project_id.clone(),
            source_id: candidate.source_id.clone(),
            record_id: candidate.record_id.clone(),
            revision: candidate.revision.clone(),
            projection_revision: candidate.projection_revision.clone(),
            context: seed_location(context, &text, seed_score),
            scored_context,
            initial_score: seed_score,
            text_sha256: hash(text.as_bytes()),
        });
        request.query.push_str(FEEDBACK_LABEL);
        request.query.push_str(&text);
        let started = Instant::now();
        let response = score(provider, &request, deadline)?;
        metrics.feedback_ms = super::millis(started);
        add_metrics(&mut metrics, "feedback", &response.metrics);
        info.stages = 2;
        response.scores
    } else {
        initial.scores.clone()
    };
    for (index, candidate) in prepared.candidates.iter_mut().enumerate() {
        let contexts = plan.contexts.get(index).ok_or(SemanticError::Stale)?;
        let mut chunks = Vec::new();
        let mut initial_score = 0.0f32;
        for context in contexts {
            initial_score = initial_score.max(value(&initial.scores, context.input)?);
            let mut chunk = context.location.clone();
            chunk.score = value(&final_scores, context.input)?;
            chunks.push(chunk);
        }
        chunks.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.field.cmp(&b.field))
                .then(a.byte_start.cmp(&b.byte_start))
        });
        let score = chunks.first().ok_or(SemanticError::Stale)?.score;
        candidate.rerank = Some(RerankMatch {
            initial_score,
            score,
            chunks,
        });
    }
    prepared.candidates.sort_by(|a, b| {
        let a_score = a.rerank.as_ref().map_or(0.0, |r| r.score);
        let b_score = b.rerank.as_ref().map_or(0.0, |r| r.score);
        b_score
            .total_cmp(&a_score)
            .then((&a.project_id, &a.source_id, &a.record_id).cmp(&(
                &b.project_id,
                &b.source_id,
                &b.record_id,
            )))
    });
    remaining(deadline)?;
    prepared.reranking = Some(info);
    prepared.metrics.reranking = Some(metrics);
    Ok(())
}

fn build_plan(
    prepared: &PreparedSearch,
    records: &[Record],
    config: &RerankConfig,
    policy: &RedactionPolicy,
    deadline: Instant,
) -> Result<Plan, SemanticError> {
    let mut plan = Plan::default();
    let mut unique = BTreeMap::new();
    for candidate in &prepared.candidates {
        let record = records
            .iter()
            .find(|record| {
                record.project_id == candidate.project_id
                    && record.source_id == candidate.source_id
                    && record.id == candidate.record_id
            })
            .ok_or(SemanticError::Stale)?;
        let fields = projection(record, policy)?;
        let title = fields
            .iter()
            .find(|(name, _)| name == "title")
            .map_or("", |(_, text)| text.as_str());
        let mut contexts = Vec::new();
        let mut locations = BTreeSet::new();
        for chunk in &candidate.chunks {
            let text = fields
                .iter()
                .find(|(name, _)| name == &chunk.field)
                .map(|(_, text)| text.as_str())
                .ok_or(SemanticError::Stale)?;
            let prefix_chars = text
                .get(..chunk.byte_start)
                .ok_or(SemanticError::Stale)?
                .chars()
                .count();
            let original_chars = text
                .get(chunk.byte_start..chunk.byte_end)
                .ok_or(SemanticError::Stale)?
                .chars()
                .count();
            let total = text.chars().count();
            // Do not shrink an existing larger retrieval chunk without saying so.
            if original_chars > config.context_chars {
                return Err(SemanticError::InvalidConfig(
                    "reranker context_chars must cover the selected embedding chunk".into(),
                ));
            }
            let width = config.context_chars.min(total);
            let mut start = prefix_chars.saturating_sub(width.saturating_sub(original_chars) / 2);
            let end = start.saturating_add(width).min(total);
            if end == total {
                start = end.saturating_sub(width);
            }
            let start_byte = char_byte(text, start)?;
            let end_byte = char_byte(text, end)?;
            if !locations.insert((chunk.field.clone(), start_byte, end_byte)) {
                continue;
            }
            let body = text
                .get(start_byte..end_byte)
                .ok_or(SemanticError::Stale)?
                .to_owned();
            let prefix =
                if chunk.field != "title" && !title.is_empty() && title.chars().count() <= 96 {
                    format!("{title}\n")
                } else {
                    String::new()
                };
            let input_text = format!("{prefix}{body}");
            let input = if let Some(index) = unique.get(&input_text) {
                *index
            } else {
                let index = plan.documents.len();
                unique.insert(input_text.clone(), index);
                plan.documents.push(input_text);
                index
            };
            let start_line = text
                .get(..start_byte)
                .ok_or(SemanticError::Stale)?
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                .saturating_add(1);
            contexts.push(Context {
                input,
                location: SemanticChunkMatch {
                    field: chunk.field.clone(),
                    range: lines(start_line, &body),
                    byte_start: start_byte,
                    byte_end: end_byte,
                    score: 0.0,
                    context_fields: if prefix.is_empty() {
                        Vec::new()
                    } else {
                        vec!["title".into()]
                    },
                },
                prefix,
                body,
            });
        }
        plan.contexts.push(contexts);
        remaining(deadline)?;
    }
    Ok(plan)
}

fn seed_location(context: &Context, text: &str, score: f32) -> SemanticChunkMatch {
    let used = text.chars().count();
    let prefix_chars = context.prefix.chars().count();
    if !context.prefix.is_empty() && used <= prefix_chars {
        let title: String = context
            .prefix
            .chars()
            .take(used.min(prefix_chars.saturating_sub(1)))
            .collect();
        return SemanticChunkMatch {
            field: "title".into(),
            range: lines(1, &title),
            byte_start: 0,
            byte_end: title.len(),
            score,
            context_fields: Vec::new(),
        };
    }
    let body: String = context
        .body
        .chars()
        .take(used.saturating_sub(prefix_chars))
        .collect();
    let mut location = context.location.clone();
    location.byte_end = location.byte_start.saturating_add(body.len());
    location.range = lines(location.range.start_line, &body);
    location.score = score;
    location
}

fn lines(start: usize, text: &str) -> TextRange {
    let extra = text
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .saturating_sub(usize::from(text.ends_with('\n')));
    TextRange {
        start_line: start,
        end_line: start.saturating_add(extra),
    }
}

fn char_byte(text: &str, position: usize) -> Result<usize, SemanticError> {
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .nth(position)
        .ok_or(SemanticError::Stale)
}

fn value(scores: &[f32], index: usize) -> Result<f32, SemanticError> {
    scores
        .get(index)
        .copied()
        .ok_or_else(|| SemanticError::InvalidResponse("missing reranker score".into()))
}

fn remaining(deadline: Instant) -> Result<u64, SemanticError> {
    deadline
        .checked_duration_since(Instant::now())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .filter(|millis| *millis > 0)
        .ok_or_else(|| SemanticError::BudgetExceeded("reranking deadline".into()))
}

fn score(
    provider: &impl RerankProvider,
    request: &RerankRequest,
    deadline: Instant,
) -> Result<RerankResponse, SemanticError> {
    let response = provider.score(request, remaining(deadline)?)?;
    remaining(deadline)?;
    if response.protocol != request.protocol
        || response.model_id != request.model_id
        || response.model_revision != request.model_revision
        || response.scores.len() != request.documents.len()
        || response
            .scores
            .iter()
            .any(|score| !score.is_finite() || !(0.0..=1.0).contains(score))
        || response.metrics.values().any(|value| !value.is_finite())
    {
        return Err(SemanticError::InvalidResponse(
            "reranker model, score count or values differ from the request".into(),
        ));
    }
    Ok(response)
}

fn add_metrics(metrics: &mut RerankMetrics, prefix: &str, values: &BTreeMap<String, f64>) {
    metrics.backend.extend(
        values
            .iter()
            .map(|(key, value)| (format!("{prefix}.{key}"), *value)),
    );
}
