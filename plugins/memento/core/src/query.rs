//! Pure, bounded queries over the retained revision log.
mod code_mapping;
mod filters;
mod operations;
mod semantic_search;
mod view;

use crate::model::*;
use serde::{Deserialize, Serialize};
use view::{Cursor, View};

const HISTORY_COMPACTED_WARNING: &str = "history_compacted: older entries or revisions may no longer be retained; no match does not establish absence";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum QueryError {
    #[error("invalid scope: {0}")]
    InvalidScope(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("unsupported filter: {0}")]
    UnsupportedFilter(String),
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),
    #[error("cursor is stale or does not match this query")]
    StaleCursor,
    #[error("response budget is too small")]
    BudgetTooSmall,
    #[error("checkpoint cannot be compared; rescan required")]
    RescanRequired,
    #[error("semantic search unavailable: {0}")]
    SemanticUnavailable(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Partial,
    NoMatches,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub source_id: String,
    pub record_kinds: Vec<RecordKind>,
    pub gaps: Vec<String>,
    pub result_only: bool,
    pub time_unknown_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFreshness {
    pub source_id: String,
    pub available: bool,
    pub last_captured_at: Option<String>,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRange {
    pub field: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryItem {
    pub entity: Entity,
    pub rendered_revision: Option<String>,
    pub excerpt: Option<String>,
    pub match_locations: Vec<String>,
    pub match_ranges: Vec<MatchRange>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<crate::semantic::SemanticCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefClaim {
    pub record_id: String,
    pub revision: String,
    pub text: String,
    pub nature: Nature,
    pub fidelity: Fidelity,
    pub decision_status: Option<DecisionStatus>,
    pub attempt_outcome: Option<AttemptOutcome>,
    pub verification_outcome: Option<VerificationOutcome>,
    pub evidence: Vec<Evidence>,
    pub applies_to: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Representation::is_legacy")]
    pub representation: Representation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefSection {
    pub name: String,
    pub claims: Vec<BriefClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    pub purpose: BriefPurpose,
    pub sections: Vec<BriefSection>,
    pub conflicts: Vec<Relation>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeComparison {
    pub from_state: Option<String>,
    pub to_state: Option<String>,
    pub status: String,
    pub changed_paths: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub mode: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub code: Option<CodeComparison>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationStatus {
    Exact,
    Mapped,
    Ambiguous,
    Missing,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResolution {
    pub status: LocationStatus,
    pub candidates: Vec<Target>,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<crate::mapping::MappingReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping_status: Option<LocationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchInfo {
    pub fingerprint: String,
    pub model_id: String,
    pub model_revision: String,
    pub chunk_version: String,
    pub metrics: crate::semantic::SemanticMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranking: Option<crate::semantic::RerankInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub operation: Operation,
    pub scope: Scope,
    pub query_snapshot: u64,
    pub checkpoint: Option<String>,
    pub next_cursor: Option<String>,
    pub status: ResponseStatus,
    pub truncated: bool,
    pub omitted: Vec<String>,
    pub coverage: Vec<SourceCoverage>,
    pub freshness: Vec<SourceFreshness>,
    pub items: Vec<QueryItem>,
    pub relations: Vec<Relation>,
    pub brief: Option<Brief>,
    pub comparison: Option<Comparison>,
    pub location: Option<LocationResolution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SemanticSearchInfo>,
}

pub fn execute(corpus: &Corpus, query: &Query) -> Result<QueryResponse, QueryError> {
    execute_inner(corpus, query, None)
}

/// Prepare only authorized, structurally matching records from this query snapshot.
pub fn prepare_semantic_records(corpus: &Corpus, query: &Query) -> Result<Vec<Record>, QueryError> {
    semantic_search::request(query)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(view::decode::<Cursor>)
        .transpose()?;
    let view = View::new(corpus, query, cursor.as_ref())?;
    filters::validate(&view, query)?;
    Ok(semantic_search::records(&view, query))
}

/// Rendering stays pure. The caller reloads current storage after local model work.
pub fn execute_semantic(
    corpus: &Corpus,
    query: &Query,
    prepared: &crate::semantic::PreparedSearch,
    policy: &crate::security::RedactionPolicy,
) -> Result<QueryResponse, QueryError> {
    execute_inner(corpus, query, Some((prepared, policy)))
}

fn execute_inner(
    corpus: &Corpus,
    query: &Query,
    semantic: Option<(
        &crate::semantic::PreparedSearch,
        &crate::security::RedactionPolicy,
    )>,
) -> Result<QueryResponse, QueryError> {
    let semantic_requested = query
        .query
        .as_ref()
        .is_some_and(|q| q.mode == SearchMode::Semantic);
    if semantic_requested {
        semantic_search::request(query)?;
        if semantic.is_none() {
            return Err(QueryError::SemanticUnavailable(
                "configure an explicit local embedding backend; literal/tokens remain available"
                    .into(),
            ));
        }
    } else if semantic.is_some() {
        return Err(QueryError::InvalidQuery(
            "prepared semantic candidates require mode=semantic".into(),
        ));
    }
    let pagination_query = query;
    let cursor = query
        .cursor
        .as_deref()
        .map(view::decode::<Cursor>)
        .transpose()?;
    let mut view = View::new(corpus, query, cursor.as_ref())?;
    let (effective_query, mut location) = operations::resolve_location(&view, query)?;
    code_mapping::attach(&view, query, &mut location)?;
    if let Some((prepared, _)) = semantic {
        if prepared.compaction_generation != corpus.compaction.generation {
            return Err(QueryError::StaleCursor);
        }
        if cursor.as_ref().is_some_and(|cursor| {
            cursor.search_fingerprint.as_deref() != Some(&prepared.fingerprint)
        }) {
            return Err(QueryError::StaleCursor);
        }
        view.search_fingerprint = Some(prepared.fingerprint.clone());
    }
    let query = &effective_query;
    if matches!(query.operation, Operation::Trace | Operation::Brief)
        && let Some(target) = &query.target
        && view
            .entities()
            .filter(|entity| view::target_matches(target, entity))
            .take(2)
            .count()
            > 1
    {
        return Err(QueryError::InvalidScope(
            "target is ambiguous; select a source".into(),
        ));
    }
    filters::validate(&view, query)?;
    let limit = query.limit.unwrap_or(20);
    if !(1..=100).contains(&limit) {
        return Err(QueryError::InvalidQuery(
            "limit must be between 1 and 100".into(),
        ));
    }
    let mut response = view.response(query);
    response.location = location;
    let mut rows = if response.location.as_ref().is_some_and(|location| {
        matches!(
            location.status,
            LocationStatus::Ambiguous | LocationStatus::Missing
        )
    }) {
        Vec::new()
    } else if let Some((prepared, policy)) = semantic {
        semantic_search::rows(&view, query, prepared, policy, &mut response)?
    } else {
        operations::select(&view, query, &mut response)?
    };
    if corpus.compaction.generation > 0
        && query.operation == Operation::Read
        && matches!(query.target, Some(Target::Artifact { .. }))
        && rows.is_empty()
    {
        response.omitted.push(
            "requested artifact revision is not retained; compaction may have removed it".into(),
        );
    }
    if query.operation != Operation::Trace
        && (!semantic_requested || matches!(query.sort, Some(Sort::Oldest | Sort::Newest)))
    {
        operations::sort(&mut rows, query);
    }
    let offset = cursor.as_ref().map_or(0, |x| x.offset);
    let text_offset = cursor.as_ref().map_or(0, |x| x.text_offset);
    if offset > rows.len() {
        return Err(QueryError::StaleCursor);
    }
    let budget = query.budget_bytes.unwrap_or(64 * 1024);
    let mut next_offset = offset;
    let mut next_text = text_offset;
    for row in rows.iter().skip(offset).take(limit) {
        let mut item = row.clone();
        let mut remaining_text = None;
        if query.operation == Operation::Read {
            if let Some(text) = retained_text_mut(&mut item.entity) {
                let body = text.get(next_text..).ok_or(QueryError::StaleCursor)?;
                *text = body.to_owned();
            }
        } else {
            shorten(&mut item, 2048);
        }
        response.items.push(item);
        if query.operation == Operation::Trace {
            response.relations = response
                .items
                .iter()
                .filter_map(|item| match &item.entity {
                    Entity::Relation(r) => Some(r.clone()),
                    _ => None,
                })
                .collect();
        }
        if query.operation == Operation::Brief {
            response.brief = Some(operations::brief(&view, query, &response.items));
        }
        let after_row = next_offset
            .checked_add(1)
            .ok_or(QueryError::BudgetTooSmall)?;
        if page_len(&response, &view, pagination_query, after_row, 0, rows.len())? > budget {
            if query.operation == Operation::Read {
                let index = response
                    .items
                    .len()
                    .checked_sub(1)
                    .ok_or(QueryError::BudgetTooSmall)?;
                let mut body = response
                    .items
                    .get_mut(index)
                    .and_then(|item| retained_text_mut(&mut item.entity))
                    .cloned()
                    .unwrap_or_default();
                while !body.is_empty() {
                    let half = boundary(&body, body.len() / 2);
                    body.truncate(half);
                    if let Some(item) = response.items.get_mut(index) {
                        if let Some(text) = retained_text_mut(&mut item.entity) {
                            text.clone_from(&body);
                        }
                        if !item
                            .warnings
                            .iter()
                            .any(|x| x == "response_truncated; continue with next_cursor")
                        {
                            item.warnings
                                .push("response_truncated; continue with next_cursor".into());
                        }
                    }
                    let after_text = next_text
                        .checked_add(body.len())
                        .ok_or(QueryError::BudgetTooSmall)?;
                    if !body.is_empty()
                        && page_len(
                            &response,
                            &view,
                            pagination_query,
                            next_offset,
                            after_text,
                            rows.len(),
                        )? <= budget
                    {
                        break;
                    }
                }
                if body.is_empty() {
                    response.items.pop();
                    if response.items.is_empty() {
                        return Err(QueryError::BudgetTooSmall);
                    }
                    break;
                }
                remaining_text = Some(
                    next_text
                        .checked_add(body.len())
                        .ok_or(QueryError::BudgetTooSmall)?,
                );
            } else {
                response.items.pop();
                if query.operation == Operation::Trace {
                    response.relations = response
                        .items
                        .iter()
                        .filter_map(|item| match &item.entity {
                            Entity::Relation(r) => Some(r.clone()),
                            _ => None,
                        })
                        .collect();
                }
                if query.operation == Operation::Brief {
                    response.brief = Some(operations::brief(&view, query, &response.items));
                }
                if response.items.is_empty() {
                    return Err(QueryError::BudgetTooSmall);
                }
                break;
            }
        }
        if let Some(text) = remaining_text {
            next_text = text;
            break;
        }
        next_offset = next_offset
            .checked_add(1)
            .ok_or(QueryError::BudgetTooSmall)?;
        next_text = 0;
    }
    if next_offset < rows.len() {
        response.next_cursor = Some(view.cursor(pagination_query, next_offset, next_text)?);
        response.truncated = true;
        response
            .omitted
            .push("more results are available with next_cursor".into());
    } else {
        response.checkpoint = Some(view.checkpoint(query)?);
    }
    if query.operation == Operation::Brief {
        response.brief = Some(operations::brief(&view, query, &response.items));
    }
    if query.operation == Operation::Trace {
        response.relations = response
            .items
            .iter()
            .filter_map(|item| match &item.entity {
                Entity::Relation(relation) => Some(relation.clone()),
                _ => None,
            })
            .collect();
    }
    if is_partial(&response) {
        response.status = ResponseStatus::Partial;
    } else if response.items.is_empty()
        && response.relations.is_empty()
        && response.comparison.is_none()
    {
        response.status = ResponseStatus::NoMatches;
    }
    comparison_page(&mut response);
    // Metadata, relationships, and generated sections also count towards the caller's budget.
    if encoded_len(&response)? > budget {
        return Err(QueryError::BudgetTooSmall);
    }
    Ok(response)
}

fn boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    at
}

fn shorten(item: &mut QueryItem, limit: usize) {
    if let Entity::Record(record) = &mut item.entity
        && record.body.len() > limit
    {
        record.body.truncate(boundary(&record.body, limit));
        item.warnings
            .push("excerpt only; read the record ID for the retained text".into());
    }
    if let Entity::CodeState(state) = &mut item.entity {
        for file in &mut state.files {
            if let Some(text) = &mut file.working_content
                && text.len() > limit
            {
                text.truncate(boundary(text, limit));
                item.warnings.push(format!(
                    "code excerpt only for {}; read the code target for retained content",
                    file.path
                ));
            }
        }
    }
}

fn retained_text_mut(entity: &mut Entity) -> Option<&mut String> {
    match entity {
        Entity::Record(record) => Some(&mut record.body),
        Entity::CodeState(state) if state.files.len() == 1 => state
            .files
            .first_mut()
            .and_then(|file| file.working_content.as_mut()),
        _ => None,
    }
}

fn encoded_len(value: &impl Serialize) -> Result<usize, QueryError> {
    serde_json::to_vec(value)
        .map(|x| x.len())
        .map_err(|e| QueryError::InvalidQuery(e.to_string()))
}

fn page_len(
    response: &QueryResponse,
    view: &View<'_>,
    query: &Query,
    offset: usize,
    text_offset: usize,
    total: usize,
) -> Result<usize, QueryError> {
    let mut candidate = response.clone();
    comparison_page(&mut candidate);
    if offset < total {
        candidate.next_cursor = Some(view.cursor(query, offset, text_offset)?);
        candidate.truncated = true;
        candidate.status = ResponseStatus::Partial;
        candidate
            .omitted
            .push("more results are available with next_cursor".into());
    } else {
        candidate.checkpoint = Some(view.checkpoint(query)?);
        if is_partial(&candidate) {
            candidate.status = ResponseStatus::Partial;
        }
    }
    encoded_len(&candidate)
}

fn comparison_page(response: &mut QueryResponse) {
    if let Some(comparison) = &mut response.comparison {
        let ids: std::collections::BTreeSet<_> =
            response.items.iter().map(|item| item.entity.id()).collect();
        comparison.added.retain(|id| ids.contains(id.as_str()));
        comparison.removed.retain(|id| ids.contains(id.as_str()));
    }
}

fn is_partial(response: &QueryResponse) -> bool {
    response.truncated
        || response.omitted.iter().any(|warning| warning != HISTORY_COMPACTED_WARNING)
        || response.freshness.iter().any(|source| !source.available)
        || response.coverage.iter().any(|source| !source.gaps.is_empty())
        || response.location.as_ref().is_some_and(|location| {
            location.status != LocationStatus::Exact
                || location.mapping_status.is_some_and(|status| !matches!(status, LocationStatus::Mapped | LocationStatus::Exact))
        })
        || response.items.iter().any(|item| {
            matches!(&item.entity, Entity::Record(record) if record.partial || record.fidelity != Fidelity::Original || matches!(record.availability, Availability::Missing | Availability::Unsupported))
                || item.warnings.iter().any(|warning| warning.starts_with("stale_summary"))
        })
}
