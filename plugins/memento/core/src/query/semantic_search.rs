use super::*;
use crate::security::RedactionPolicy;
use crate::semantic::{PreparedSearch, SemanticError};
use std::collections::BTreeMap;

pub(super) fn request(query: &Query) -> Result<&str, QueryError> {
    if query.operation != Operation::Search || query.code_mapping.is_some() {
        return Err(QueryError::InvalidQuery(
            "semantic mode is supported only by search".into(),
        ));
    }
    let text = query
        .query
        .as_ref()
        .filter(|text| text.mode == SearchMode::Semantic)
        .ok_or_else(|| {
            QueryError::InvalidQuery("semantic search requires query.mode=semantic".into())
        })?;
    if text.text.trim().is_empty() {
        return Err(QueryError::InvalidQuery(
            "semantic query must be nonempty".into(),
        ));
    }
    if !(1..=100).contains(&query.limit.unwrap_or(20)) {
        return Err(QueryError::InvalidQuery(
            "limit must be between 1 and 100".into(),
        ));
    }
    Ok(&text.text)
}

pub(super) fn records(view: &View<'_>, query: &Query) -> Vec<Record> {
    view.records()
        // Use the same evidence-aware projection that a direct read exposes.
        // A visible summary must not carry hidden evidence into model input.
        .filter_map(
            |record| match view.item(&Entity::Record(record.clone())).entity {
                Entity::Record(record) => Some(record),
                _ => None,
            },
        )
        .filter(|record| {
            matches!(
                record.availability,
                Availability::Available | Availability::Redacted
            )
        })
        .filter(|record| filters::record(view, record, query))
        .filter(|record| operations::brief_target(record, query.target.as_ref()))
        .collect()
}

pub(super) fn rows(
    view: &View<'_>,
    query: &Query,
    prepared: &PreparedSearch,
    policy: &RedactionPolicy,
    response: &mut QueryResponse,
) -> Result<Vec<QueryItem>, QueryError> {
    let text = request(query)?;
    let records = records(view, query);
    prepared
        .validate(&records, text, policy)
        .map_err(|error| match error {
            SemanticError::Stale => QueryError::StaleCursor,
            other => QueryError::SemanticUnavailable(other.to_string()),
        })?;
    // Revalidate every candidate against this authorized snapshot. Backend output
    // never supplies the response body or access-control decisions.
    let mut retained = BTreeMap::new();
    for record in records {
        let entity = policy
            .entity(&Entity::Record(record))
            .map_err(|error| QueryError::SemanticUnavailable(error.to_string()))?;
        let Entity::Record(record) = entity else {
            continue;
        };
        retained.insert(
            (
                record.project_id.clone(),
                record.source_id.clone(),
                record.id.clone(),
            ),
            record,
        );
    }
    let mut rows = Vec::new();
    for candidate in &prepared.candidates {
        let key = (
            candidate.project_id.clone(),
            candidate.source_id.clone(),
            candidate.record_id.clone(),
        );
        let record = retained.get(&key).ok_or(QueryError::StaleCursor)?;
        if record.revision != candidate.revision
            || crate::semantic::projection_revision(record)
                .map_err(|error| QueryError::SemanticUnavailable(error.to_string()))?
                != candidate.projection_revision
        {
            return Err(QueryError::StaleCursor);
        }
        let mut item = view.item(&Entity::Record(record.clone()));
        item.semantic = Some(candidate.clone());
        item.warnings.push("semantic similarity identifies a candidate, not factual confidence or a causal relationship; read the cited original".into());
        if prepared
            .reranking
            .as_ref()
            .is_some_and(|info| info.seed.is_some())
        {
            item.warnings.push("ranking used an unverified retrieved context; a high rerank score or repeated source context is not independent corroboration".into());
        }
        if let Some(chunk) = candidate
            .rerank
            .as_ref()
            .and_then(|r| r.chunks.first())
            .or_else(|| candidate.chunks.first())
        {
            let field = match chunk.field.as_str() {
                "body" => Some(record.body.as_str()),
                "title" => Some(record.title.as_str()),
                "tool_input" => record
                    .execution
                    .as_ref()
                    .and_then(|execution| execution.tool_input.as_deref()),
                "tool_name" => record
                    .execution
                    .as_ref()
                    .and_then(|execution| execution.tool_name.as_deref()),
                "command" => record
                    .execution
                    .as_ref()
                    .map(|execution| execution.command.as_str()),
                _ => None,
            };
            item.excerpt = field
                .and_then(|field| field.get(chunk.byte_start..chunk.byte_end))
                .map(str::to_owned);
        }
        rows.push(item);
    }
    response.omitted.extend(prepared.omitted.iter().cloned());
    response.semantic = Some(SemanticSearchInfo {
        fingerprint: prepared.fingerprint.clone(),
        model_id: prepared.model_id.clone(),
        model_revision: prepared.model_revision.clone(),
        chunk_version: prepared.chunk_version.clone(),
        metrics: prepared.metrics.clone(),
        reranking: prepared.reranking.clone(),
    });
    Ok(rows)
}
