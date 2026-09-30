//! Explicit local embedding boundary. No vectors or plaintext cache are persisted.
//! Callers must authorize and structurally filter records before preparation, then
//! revalidate the same scope against fresh storage before rendering candidates.
mod process;
mod rerank;

pub use rerank::{
    RerankConfig, RerankInfo, RerankMatch, RerankMetrics, RerankProvider, RerankRequest,
    RerankResponse, RerankSeed,
};

use crate::model::{Availability, Record, TextRange};
use crate::security::{RedactionPolicy, hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub const CHUNK_VERSION: &str = "field-title-context-characters-v2";

#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    #[error("invalid semantic configuration: {0}")]
    InvalidConfig(String),
    #[error(
        "semantic backend unavailable: {0}; use literal/tokens search or configure a local model"
    )]
    Unavailable(String),
    #[error("semantic preparation exceeded its processing budget: {0}")]
    BudgetExceeded(String),
    #[error("invalid semantic backend response: {0}")]
    InvalidResponse(String),
    #[error("semantic preparation is stale; repeat search")]
    Stale,
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("masking: {0}")]
    Masking(#[from] regex::Error),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SemanticConfig {
    /// Executable plus fixed arguments. Never passed to a shell.
    pub command: Vec<String>,
    pub model_id: String,
    pub model_revision: String,
    pub chunk_chars: usize,
    pub overlap_chars: usize,
    pub max_records: usize,
    pub max_input_bytes: usize,
    pub max_chunks: usize,
    pub max_unique_chunks: usize,
    pub max_query_bytes: usize,
    pub timeout_ms: u64,
    pub min_score: f32,
    pub max_candidates: usize,
    pub rerank: Option<RerankConfig>,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            model_id: String::new(),
            model_revision: String::new(),
            chunk_chars: 256,
            overlap_chars: 32,
            max_records: 10_000,
            max_input_bytes: 32 * 1024 * 1024,
            max_chunks: 200_000,
            max_unique_chunks: 10_000,
            max_query_bytes: 4096,
            timeout_ms: 120_000,
            min_score: 0.0,
            max_candidates: 100,
            rerank: None,
        }
    }
}

impl SemanticConfig {
    fn validate(&self) -> Result<(), SemanticError> {
        if let Some(rerank) = &self.rerank {
            rerank.validate()?;
            if self.min_score != 0.0 {
                return Err(SemanticError::InvalidConfig(
                    "reranking uses a candidate pool without a cosine cutoff; set min_score to 0"
                        .into(),
                ));
            }
        }
        if self.model_id.is_empty() || self.model_revision.is_empty() {
            return Err(SemanticError::InvalidConfig(
                "model ID and immutable revision are required".into(),
            ));
        }
        if !(16..=2048).contains(&self.chunk_chars)
            || self.overlap_chars >= self.chunk_chars
            || !(1..=100_000).contains(&self.max_records)
            || !(1..=128 * 1024 * 1024).contains(&self.max_input_bytes)
            || !(1..=1_000_000).contains(&self.max_chunks)
            || !(1..=50_000).contains(&self.max_unique_chunks)
            || !(1..=64 * 1024).contains(&self.max_query_bytes)
            || !(1..=600_000).contains(&self.timeout_ms)
            || !(1..=1000).contains(&self.max_candidates)
            || !self.min_score.is_finite()
            || !(-1.0..=1.0).contains(&self.min_score)
        {
            return Err(SemanticError::InvalidConfig(
                "chunk, candidate, byte, time or score limit outside supported bounds".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChunkMatch {
    pub field: String,
    /// One-based lines in the retained field; not a literal match range.
    pub range: TextRange,
    pub byte_start: usize,
    pub byte_end: usize,
    pub score: f32,
    /// Additional retained field text supplied as context to this chunk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCandidate {
    pub project_id: String,
    pub source_id: String,
    pub record_id: String,
    pub revision: String,
    pub projection_revision: String,
    /// Cosine similarity is a retrieval score, never factual confidence.
    pub score: f32,
    pub chunks: Vec<SemanticChunkMatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank: Option<RerankMatch>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticMetrics {
    pub eligible_records: usize,
    pub indexed_records: usize,
    pub input_bytes: usize,
    pub chunks: usize,
    pub unique_chunks: usize,
    pub query_bytes: usize,
    pub projection_ms: u64,
    pub model_ms: u64,
    pub ranking_ms: u64,
    #[serde(default)]
    pub backend: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranking: Option<RerankMetrics>,
}

/// Immutable candidates and generation identity for the pure query engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedSearch {
    pub fingerprint: String,
    pub input_fingerprint: String,
    #[serde(default)]
    pub compaction_generation: u64,
    pub model_id: String,
    pub model_revision: String,
    pub chunk_version: String,
    pub candidates: Vec<SemanticCandidate>,
    pub omitted: Vec<String>,
    pub metrics: SemanticMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranking: Option<RerankInfo>,
    config: SemanticConfig,
}

impl PreparedSearch {
    /// Bind candidates to the corpus generation used to prepare their records.
    pub fn with_compaction_generation(mut self, generation: u64) -> Result<Self, SemanticError> {
        self.compaction_generation = generation;
        self.fingerprint = self.ranked_fingerprint()?;
        Ok(self)
    }

    pub fn validate(
        &self,
        records: &[Record],
        query_text: &str,
        policy: &RedactionPolicy,
    ) -> Result<(), SemanticError> {
        self.config.validate()?;
        if self.model_id != self.config.model_id
            || self.model_revision != self.config.model_revision
            || self.chunk_version != CHUNK_VERSION
            || input_fingerprint(records, query_text, &self.config, policy)?
                != self.input_fingerprint
            || self.ranked_fingerprint()? != self.fingerprint
        {
            return Err(SemanticError::Stale);
        }
        Ok(())
    }

    fn ranked_fingerprint(&self) -> Result<String, SemanticError> {
        Ok(hash(&serde_json::to_vec(&(
            &self.input_fingerprint,
            self.compaction_generation,
            &self.model_id,
            &self.model_revision,
            &self.chunk_version,
            &self.candidates,
            &self.omitted,
            &self.reranking,
        ))?))
    }
}

/// Versioned JSON protocol for an explicit local executable. Inputs are masked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingRequest {
    pub protocol: u32,
    pub model_id: String,
    pub model_revision: String,
    pub query: String,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingResponse {
    pub protocol: u32,
    pub model_id: String,
    pub model_revision: String,
    pub query: Vec<f32>,
    pub documents: Vec<Vec<f32>>,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
}

pub trait EmbeddingProvider {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        timeout_ms: u64,
    ) -> Result<EmbeddingResponse, SemanticError>;
}

pub fn prepare(
    records: &[Record],
    query_text: &str,
    config: &SemanticConfig,
    policy: &RedactionPolicy,
) -> Result<PreparedSearch, SemanticError> {
    prepare_with_provider(
        records,
        query_text,
        config,
        policy,
        &process::LocalCommand {
            command: &config.command,
        },
    )
}

pub fn prepare_with_provider(
    records: &[Record],
    query_text: &str,
    config: &SemanticConfig,
    policy: &RedactionPolicy,
    provider: &impl EmbeddingProvider,
) -> Result<PreparedSearch, SemanticError> {
    prepare_with_backends(
        records,
        query_text,
        config,
        policy,
        provider,
        &process::LocalCommand {
            command: config.rerank.as_ref().map_or(&[], |r| r.command.as_slice()),
        },
    )
}

pub fn prepare_with_backends(
    records: &[Record],
    query_text: &str,
    config: &SemanticConfig,
    policy: &RedactionPolicy,
    provider: &impl EmbeddingProvider,
    reranker: &impl RerankProvider,
) -> Result<PreparedSearch, SemanticError> {
    config.validate()?;
    let started = Instant::now();
    let query = policy.redact(query_text)?;
    if query.trim().is_empty() {
        return Err(SemanticError::InvalidConfig(
            "query text must be nonempty".into(),
        ));
    }
    if query.len() > config.max_query_bytes {
        return Err(SemanticError::BudgetExceeded(
            "query bytes; shorten the query".into(),
        ));
    }
    let mut ordered: Vec<_> = records.iter().collect();
    ordered.sort_by(|a, b| {
        (&a.project_id, &a.source_id, &a.id).cmp(&(&b.project_id, &b.source_id, &b.id))
    });
    let input_fingerprint = input_fingerprint_refs(&ordered, &query, config, policy)?;
    let mut plan = ChunkPlan::default();
    let mut omitted = Vec::new();
    let mut metrics = SemanticMetrics {
        eligible_records: records.len(),
        query_bytes: query.len(),
        ..Default::default()
    };
    let mut seen = BTreeSet::new();
    for record in ordered {
        if !seen.insert((&record.project_id, &record.source_id, &record.id)) {
            return Err(SemanticError::InvalidConfig(
                "duplicate eligible record identity".into(),
            ));
        }
        if !matches!(
            record.availability,
            Availability::Available | Availability::Redacted
        ) {
            continue;
        }
        let fields = projection(record, policy)?;
        let bytes = fields.iter().map(|(_, text)| text.len()).sum::<usize>();
        if metrics.indexed_records >= config.max_records
            || metrics.input_bytes.saturating_add(bytes) > config.max_input_bytes
        {
            omitted.push(format!(
                "record {}:{} omitted: record/input-byte processing limit",
                record.source_id, record.id
            ));
            continue;
        }
        metrics.indexed_records += 1;
        metrics.input_bytes += bytes;
        let revision = hash(&serde_json::to_vec(&fields)?);
        let title = fields
            .iter()
            .find(|(field, _)| field == "title")
            .map(|(_, text)| text.as_str())
            .unwrap_or_default();
        let compact_title = title.chars().count() <= 96;
        let has_content = fields
            .iter()
            .any(|(field, text)| field != "title" && !text.is_empty());
        for (field, text) in &fields {
            // Generic titles alone are poor evidence candidates. A short title
            // accompanies content instead; a long title still gets full chunks.
            if field == "title" && compact_title && has_content {
                continue;
            }
            let context = if field != "title" && compact_title {
                title
            } else {
                ""
            };
            if !plan.add_field(record, &revision, field, text, context, config)? {
                omitted.push(format!("record {}:{} field {field} partially indexed: chunk limit; remaining text was not inspected",record.source_id, record.id));
            }
        }
        if millis(started) > config.timeout_ms {
            return Err(SemanticError::BudgetExceeded("projection deadline".into()));
        }
    }
    metrics.projection_ms = millis(started);
    metrics.chunks = plan.chunks.len();
    metrics.unique_chunks = plan.documents.len();
    let mut prepared = PreparedSearch {
        fingerprint: String::new(),
        input_fingerprint,
        compaction_generation: 0,
        model_id: config.model_id.clone(),
        model_revision: config.model_revision.clone(),
        chunk_version: CHUNK_VERSION.into(),
        candidates: Vec::new(),
        omitted,
        metrics,
        config: config.clone(),
        reranking: None,
    };
    if !plan.documents.is_empty() {
        let request = EmbeddingRequest {
            protocol: 1,
            model_id: config.model_id.clone(),
            model_revision: config.model_revision.clone(),
            query: query.clone(),
            documents: plan.documents,
        };
        let remaining = config
            .timeout_ms
            .checked_sub(millis(started))
            .filter(|n| *n > 0)
            .ok_or_else(|| SemanticError::BudgetExceeded("embedding deadline".into()))?;
        let model_started = Instant::now();
        let response = provider.embed(&request, remaining)?;
        prepared.metrics.model_ms = millis(model_started);
        if millis(started) > config.timeout_ms {
            return Err(SemanticError::BudgetExceeded("embedding deadline".into()));
        }
        validate_embeddings(&request, &response)?;
        prepared.metrics.backend = response.metrics;
        let ranking_started = Instant::now();
        let mut candidates: BTreeMap<(String, String, String), SemanticCandidate> = BTreeMap::new();
        let scores: Result<Vec<_>, _> = response
            .documents
            .iter()
            .map(|vector| cosine(&response.query, vector))
            .collect();
        let scores = scores?;
        for chunk in plan.chunks {
            let score = *scores
                .get(chunk.embedding)
                .ok_or_else(|| SemanticError::InvalidResponse("missing document vector".into()))?;
            if config.rerank.is_none() && score < config.min_score {
                continue;
            }
            let key = (
                chunk.project.clone(),
                chunk.source.clone(),
                chunk.record.clone(),
            );
            let candidate = candidates.entry(key).or_insert_with(|| SemanticCandidate {
                project_id: chunk.project,
                source_id: chunk.source,
                record_id: chunk.record,
                revision: chunk.revision,
                projection_revision: chunk.projection_revision,
                score,
                chunks: Vec::new(),
                rerank: None,
            });
            candidate.score = candidate.score.max(score);
            candidate.chunks.push(SemanticChunkMatch {
                field: chunk.field,
                range: chunk.range,
                byte_start: chunk.start,
                byte_end: chunk.end,
                score,
                context_fields: chunk.context_fields,
            });
        }
        for candidate in candidates.values_mut() {
            candidate.chunks.sort_by(|a, b| {
                b.score
                    .total_cmp(&a.score)
                    .then(a.field.cmp(&b.field))
                    .then(a.byte_start.cmp(&b.byte_start))
            });
            let mut selected: Vec<SemanticChunkMatch> = Vec::new();
            for chunk in candidate.chunks.drain(..) {
                if !selected.iter().any(|other| {
                    other.field == chunk.field
                        && chunk.byte_start < other.byte_end
                        && other.byte_start < chunk.byte_end
                }) {
                    selected.push(chunk);
                }
                if selected.len() == 3 {
                    break;
                }
            }
            candidate.chunks = selected;
        }
        prepared.candidates = candidates.into_values().collect();
        prepared.candidates.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then((&a.project_id, &a.source_id, &a.record_id).cmp(&(
                    &b.project_id,
                    &b.source_id,
                    &b.record_id,
                )))
        });
        prepared.metrics.ranking_ms = millis(ranking_started);
        if let Some(rerank) = &config.rerank {
            rerank::apply(
                &mut prepared,
                records,
                &query,
                rerank,
                policy,
                reranker,
                started
                    .checked_add(std::time::Duration::from_millis(config.timeout_ms))
                    .ok_or_else(|| SemanticError::BudgetExceeded("reranking deadline".into()))?,
            )?;
        }
        if prepared.candidates.len() > config.max_candidates {
            prepared.omitted.push(format!(
                "ranked candidates beyond top {} omitted",
                config.max_candidates
            ));
            prepared.candidates.truncate(config.max_candidates);
        }
    }
    if millis(started) > config.timeout_ms {
        return Err(SemanticError::BudgetExceeded("ranking deadline".into()));
    }
    prepared.fingerprint = prepared.ranked_fingerprint()?;
    Ok(prepared)
}

pub fn projection_revision(record: &Record) -> Result<String, SemanticError> {
    Ok(hash(&serde_json::to_vec(&projection(
        record,
        &RedactionPolicy::default(),
    )?)?))
}

fn projection(
    record: &Record,
    policy: &RedactionPolicy,
) -> Result<Vec<(String, String)>, SemanticError> {
    let mut fields = vec![
        ("title".into(), policy.redact(&record.title)?),
        ("body".into(), policy.redact(&record.body)?),
    ];
    if let Some(execution) = &record.execution {
        fields.push(("command".into(), policy.redact(&execution.command)?));
        for (name, value) in [
            ("tool_name", execution.tool_name.as_deref()),
            ("tool_input", execution.tool_input.as_deref()),
        ] {
            if let Some(text) = value {
                fields.push((name.into(), policy.redact(text)?));
            }
        }
    }
    Ok(fields)
}

fn input_fingerprint(
    records: &[Record],
    query: &str,
    config: &SemanticConfig,
    policy: &RedactionPolicy,
) -> Result<String, SemanticError> {
    let mut records: Vec<_> = records.iter().collect();
    records.sort_by(|a, b| {
        (&a.project_id, &a.source_id, &a.id).cmp(&(&b.project_id, &b.source_id, &b.id))
    });
    input_fingerprint_refs(&records, &policy.redact(query)?, config, policy)
}

fn input_fingerprint_refs(
    records: &[&Record],
    query: &str,
    config: &SemanticConfig,
    policy: &RedactionPolicy,
) -> Result<String, SemanticError> {
    let identities: Result<Vec<_>, SemanticError> = records
        .iter()
        .map(|r| {
            Ok((
                &r.project_id,
                &r.source_id,
                &r.id,
                &r.revision,
                r.availability,
                hash(&serde_json::to_vec(&projection(r, policy)?)?),
            ))
        })
        .collect();
    Ok(hash(&serde_json::to_vec(&(
        CHUNK_VERSION,
        config,
        query,
        identities?,
    ))?))
}

#[derive(Default)]
struct ChunkPlan {
    documents: Vec<String>,
    unique: BTreeMap<String, usize>,
    chunks: Vec<Chunk>,
}
struct Chunk {
    project: String,
    source: String,
    record: String,
    revision: String,
    projection_revision: String,
    field: String,
    range: TextRange,
    start: usize,
    end: usize,
    embedding: usize,
    context_fields: Vec<String>,
}
impl ChunkPlan {
    fn add_field(
        &mut self,
        record: &Record,
        projection_revision: &str,
        field: &str,
        text: &str,
        context: &str,
        config: &SemanticConfig,
    ) -> Result<bool, SemanticError> {
        if text.is_empty() {
            return Ok(true);
        }
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
            .collect();
        let length = boundaries.len().saturating_sub(1);
        let mut start_char = 0usize;
        let mut previous_start = 0usize;
        let mut line = 1usize;
        while start_char < length {
            if self.chunks.len() >= config.max_chunks {
                return Ok(false);
            }
            let end_char = start_char.saturating_add(config.chunk_chars).min(length);
            let start = *boundaries
                .get(start_char)
                .ok_or_else(|| SemanticError::InvalidResponse("invalid text boundary".into()))?;
            let end = *boundaries
                .get(end_char)
                .ok_or_else(|| SemanticError::InvalidResponse("invalid text boundary".into()))?;
            let advance = text
                .get(previous_start..start)
                .ok_or_else(|| SemanticError::InvalidResponse("invalid text slice".into()))?;
            line = line.saturating_add(advance.bytes().filter(|b| *b == b'\n').count());
            previous_start = start;
            let chunk = text
                .get(start..end)
                .ok_or_else(|| SemanticError::InvalidResponse("invalid chunk slice".into()))?;
            // Exact duplicate text is embedded once; each original location remains.
            let input = if context.is_empty() {
                chunk.to_owned()
            } else {
                format!("{context}\n{chunk}")
            };
            let key = hash(input.as_bytes());
            let embedding = if let Some(index) = self.unique.get(&key) {
                *index
            } else {
                if self.documents.len() >= config.max_unique_chunks {
                    return Ok(false);
                }
                let index = self.documents.len();
                self.documents.push(input);
                self.unique.insert(key, index);
                index
            };
            let end_line = line.saturating_add(
                chunk
                    .trim_end_matches('\n')
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count(),
            );
            self.chunks.push(Chunk {
                project: record.project_id.clone(),
                source: record.source_id.clone(),
                record: record.id.clone(),
                revision: record.revision.clone(),
                projection_revision: projection_revision.into(),
                field: field.into(),
                range: TextRange {
                    start_line: line,
                    end_line,
                },
                start,
                end,
                embedding,
                context_fields: if context.is_empty() {
                    Vec::new()
                } else {
                    vec!["title".into()]
                },
            });
            if end_char == length {
                break;
            }
            start_char = end_char.saturating_sub(config.overlap_chars);
        }
        Ok(true)
    }
}

fn validate_embeddings(
    request: &EmbeddingRequest,
    response: &EmbeddingResponse,
) -> Result<(), SemanticError> {
    let dimension = response.query.len();
    if response.protocol != 1
        || response.model_id != request.model_id
        || response.model_revision != request.model_revision
        || response.documents.len() != request.documents.len()
        || !(1..=4096).contains(&dimension)
        || response.documents.iter().any(|v| v.len() != dimension)
        || response
            .query
            .iter()
            .chain(response.documents.iter().flatten())
            .any(|v| !v.is_finite())
        || response.metrics.values().any(|v| !v.is_finite())
    {
        return Err(SemanticError::InvalidResponse(
            "model identity, dimensions, count or finite values differ from request".into(),
        ));
    }
    Ok(())
}
fn cosine(a: &[f32], b: &[f32]) -> Result<f32, SemanticError> {
    let dot = a
        .iter()
        .zip(b)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum::<f64>();
    let norm_a = a.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>().sqrt();
    let norm_b = b.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return Err(SemanticError::InvalidResponse(
            "zero embedding vector".into(),
        ));
    }
    Ok((dot / (norm_a * norm_b)).clamp(-1.0, 1.0) as f32)
}
fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
