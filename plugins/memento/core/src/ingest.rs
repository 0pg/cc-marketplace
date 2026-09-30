use std::{collections::BTreeSet, path::Path};

use crate::{
    adapters::*,
    model::*,
    security::hash,
    store::{Error, Receipt, Result, Store},
};
use chrono::Utc;

pub fn source(id: &str, project: &str, kind: SourceKind) -> Source {
    Source {
        id: id.into(),
        project_id: project.into(),
        kind,
        name: id.into(),
        location: None,
        content_revision: None,
        authorized: true,
        available: true,
        record_kinds: Vec::new(),
        unsupported_filters: Vec::new(),
        last_captured_at: None,
        last_event_id: None,
        gaps: Vec::new(),
        result_only: kind == SourceKind::Git,
        import_completeness: ImportCompleteness::FullSnapshot,
    }
}

pub async fn import(
    store: &mut Store,
    project: &str,
    source_id: &str,
    path: &Path,
    format: ImportFormat,
    work_id: Option<&str>,
) -> Result<Vec<Receipt>> {
    import_with_completeness(
        store,
        project,
        source_id,
        path,
        format,
        work_id,
        ImportCompleteness::FullSnapshot,
    )
    .await
}

/// Full-file snapshots can reconcile absent records. Explicit partial/delta
/// inputs only upsert supplied records; an absent ID is never a deletion event.
pub async fn import_with_completeness(
    store: &mut Store,
    project: &str,
    source_id: &str,
    path: &Path,
    format: ImportFormat,
    work_id: Option<&str>,
    completeness: ImportCompleteness,
) -> Result<Vec<Receipt>> {
    let path = path.canonicalize()?;
    let report = import_file(&path, format).map_err(|e| Error::Invalid(e.to_string()))?;
    if completeness != ImportCompleteness::FullSnapshot
        && (format == ImportFormat::Document
            || report
                .gaps
                .iter()
                .any(|gap| gap.code == ImportGapCode::LineIdentity))
    {
        return Err(Error::Invalid(
            "partial/delta import requires stable native record IDs; document fragments and line-identified exports cannot be merged safely".into(),
        ));
    }
    let previous = store.latest().await?;
    let previous_source = previous.iter().find_map(|entity| match entity {
        Entity::Source(source) if source.project_id == project && source.id == source_id => {
            Some(source)
        }
        _ => None,
    });
    let mut src = source(
        source_id,
        project,
        match format {
            ImportFormat::CodexJsonl => SourceKind::Codex,
            ImportFormat::Document => SourceKind::Document,
            ImportFormat::JournalJsonl => SourceKind::Journal,
        },
    );
    src.location = Some(path.to_string_lossy().into_owned());
    src.import_completeness = completeness;
    if let Some(previous) = previous_source
        && (previous.kind != src.kind
            || previous
                .location
                .as_ref()
                .is_some_and(|location| Some(location) != src.location.as_ref()))
    {
        return Err(Error::Invalid(
            "source id already belongs to another file or format".into(),
        ));
    }
    src.content_revision = Some(report.source_revision);
    src.last_captured_at = Some(Utc::now().to_rfc3339());
    src.last_event_id = report
        .records
        .last()
        .map(|r| format!("{source_id}:{}", r.source_record_id));
    src.gaps = report
        .gaps
        .iter()
        .map(|g| format!("{:?} at {:?}: {}", g.code, g.line, g.detail))
        .collect();
    let safe_to_reconcile = completeness == ImportCompleteness::FullSnapshot
        && report.gaps.iter().all(|gap| {
            !matches!(
                gap.code,
                ImportGapCode::MalformedRecord
                    | ImportGapCode::PartialTail
                    | ImportGapCode::UnsupportedRecord
                    | ImportGapCode::UnsupportedContent
                    | ImportGapCode::ConflictingIdentity
            )
        });
    if !safe_to_reconcile {
        src.gaps
            .push("incomplete_import: absent cached records were not classified as removed".into());
    }
    if completeness != ImportCompleteness::FullSnapshot {
        src.gaps.push(format!(
            "{:?}_input: only supplied records were merged; absent records and explicit deletion events are not reconciled",
            completeness
        ).to_lowercase());
    }
    src.record_kinds = supported_kinds(format);
    src.unsupported_filters = unsupported_filters(format);
    let mut records = Vec::new();
    let mut sessions: BTreeSet<_> = report
        .sessions
        .iter()
        .map(|session| format!("{source_id}:{}", session.id))
        .collect();
    for imported in report.records {
        let kind = serde_json::from_value(serde_json::to_value(imported.kind)?)?;
        let id = format!("{source_id}:{}", imported.source_record_id);
        let mut record = Record::new(&id, project, source_id, kind, &imported.text);
        let old_record = previous.iter().find_map(|entity| match entity {
            Entity::Record(old)
                if old.project_id == project && old.source_id == source_id && old.id == id =>
            {
                Some(old)
            }
            _ => None,
        });
        record.revision = imported.revision;
        record.title = imported
            .text
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(100)
            .collect();
        record.nature = serde_json::from_value(serde_json::to_value(imported.nature)?)?;
        record.occurred_at = imported.occurred_at.map(|t| t.to_rfc3339());
        record.source_order = Some(imported.source_order);
        record.session_id = imported.session_id.map(|s| format!("{source_id}:{s}"));
        if record.session_id.is_none() {
            record.session_id = old_record.and_then(|old| old.session_id.clone());
        }
        if let Some(session) = &record.session_id {
            sessions.insert(session.clone());
        }
        record.work_ids = work_id
            .map(str::to_owned)
            .or(imported.work_id)
            .into_iter()
            .collect();
        record.association = if record.work_ids.is_empty() {
            Association::Unassigned
        } else {
            Association::Explicit
        };
        if record.work_ids.is_empty()
            && let Some(old) = old_record
        {
            record.work_ids = old.work_ids.clone();
            record.association = old.association;
        }
        if format == ImportFormat::Document {
            record.paths.push(path.to_string_lossy().into_owned());
        }
        record.actor = imported.actor.map(|name| Actor {
            kind: match name.as_str() {
                "user" => ActorKind::Human,
                "assistant" => ActorKind::Agent,
                "tool" => ActorKind::Tool,
                _ => ActorKind::Unknown,
            },
            name: Some(name),
        });
        record.derived = imported.derived;
        record.partial = imported.partial;
        record.fidelity = if imported.source_truncated {
            Fidelity::SourceTruncated
        } else if imported.evidence_level == EvidenceLevel::SummaryOnly {
            Fidelity::SummaryOnly
        } else {
            Fidelity::Original
        };
        record.evidence.push(Evidence {
            source_id: source_id.into(),
            record_id: None,
            revision: record.revision.clone(),
            locator: imported.locator,
            availability: Availability::Available,
            range: None,
        });
        for reference in imported.references {
            record.evidence.push(Evidence {
                source_id: source_id.into(),
                record_id: None,
                revision: reference.revision.unwrap_or_else(|| "unavailable".into()),
                locator: reference.locator,
                availability: Availability::Missing,
                range: None,
            });
        }
        if let Some(execution_id) = imported.execution_id.map(|id| format!("{source_id}:{id}")) {
            let tool = imported.tool.unwrap_or_default();
            record.execution = Some(Execution {
                id: execution_id,
                command: tool.input.clone().unwrap_or_else(|| tool.name.clone()),
                tool_name: (!tool.name.is_empty()).then_some(tool.name),
                tool_input: tool.input,
                cwd: tool.working_directory,
                started_at: if kind == RecordKind::Attempt {
                    record.occurred_at.clone()
                } else {
                    None
                },
                ended_at: if matches!(
                    tool.state,
                    ExecutionState::Completed | ExecutionState::Failed
                ) {
                    record.occurred_at.clone()
                } else {
                    None
                },
                exit_code: tool.exit_code,
                last_observed_state: format!("{:?}", tool.state).to_lowercase(),
                observed_at: record.occurred_at.clone(),
                liveness: Liveness::Unknown,
                before_state: None,
                after_state: None,
                scope: Vec::new(),
                environment: Environment::default(),
            });
            if kind == RecordKind::Attempt {
                record.attempt_outcome = Some(match tool.state {
                    ExecutionState::Running => AttemptOutcome::Running,
                    ExecutionState::Completed => AttemptOutcome::Succeeded,
                    ExecutionState::Failed => AttemptOutcome::Failed,
                    ExecutionState::Unknown => AttemptOutcome::Unknown,
                });
            }
        }
        resolve_references(&mut record, &previous);
        records.push(record);
    }
    src.record_kinds.sort();
    src.record_kinds.dedup();
    // Register authorization first, but do not acknowledge a completed capture
    // before records, sessions, and relationships are durable.
    let mut opening = src.clone();
    opening.content_revision = previous_source.and_then(|source| source.content_revision.clone());
    opening.last_captured_at = previous_source.and_then(|source| source.last_captured_at.clone());
    opening.last_event_id = previous_source.and_then(|source| source.last_event_id.clone());
    opening.gaps =
        vec!["import_in_progress: last completed capture checkpoint is unchanged".into()];
    let mut entities = vec![Entity::Source(opening)];
    for id in sessions {
        let mut session = previous
            .iter()
            .find_map(|entity| match entity {
                Entity::Session(session)
                    if session.project_id == project
                        && session.source_id == source_id
                        && session.id == id =>
                {
                    Some(session.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| Session {
                id: id.clone(),
                project_id: project.into(),
                source_id: source_id.into(),
                work_ids: Vec::new(),
                status: SessionStatus::Unknown,
                started_at: None,
                ended_at: None,
                worktree_id: None,
                working_directory: None,
                parent_id: None,
            });
        for record in records
            .iter()
            .filter(|record| record.session_id.as_ref() == Some(&id))
        {
            session.work_ids.extend(record.work_ids.clone());
        }
        session.work_ids.sort();
        session.work_ids.dedup();
        if let Some(metadata) = report
            .sessions
            .iter()
            .rev()
            .find(|metadata| format!("{source_id}:{}", metadata.id) == id)
        {
            if let Some(started_at) = metadata.started_at {
                session.started_at = Some(started_at.to_rfc3339());
            }
            if let Some(cwd) = &metadata.working_directory {
                session.working_directory = Some(cwd.clone());
            }
            if let Some(parent) = &metadata.parent_id {
                let (reference, resolved) = session_reference(
                    project,
                    source_id,
                    src.kind,
                    parent,
                    &previous,
                    &report.sessions,
                );
                session.parent_id = Some(reference);
                if !resolved {
                    src.gaps.push("session_parent_unresolved: original parent ID retained; the referenced source has not been uniquely matched".into());
                }
            }
            if let Some(parent) = &metadata.forked_from_id {
                let (reference, resolved) = session_reference(
                    project,
                    source_id,
                    src.kind,
                    parent,
                    &previous,
                    &report.sessions,
                );
                if !resolved {
                    src.gaps.push("session_fork_unresolved: original fork ID retained; the referenced source has not been uniquely matched".into());
                }
                entities.push(Entity::Relation(Relation {
                    id: format!("{source_id}:fork:{}:{parent}", metadata.id),
                    project_id: project.into(),
                    source_id: source_id.into(),
                    from: Target::Session { id: id.clone() },
                    to: Target::Session { id: reference },
                    kind: RelationKind::ForkedFrom,
                    nature: Nature::Observed,
                    evidence: vec![Evidence {
                        source_id: source_id.into(),
                        record_id: None,
                        revision: metadata.revision.clone(),
                        locator: metadata.locator.clone(),
                        availability: Availability::Available,
                        range: None,
                    }],
                    applies_to: Vec::new(),
                }));
            }
        }
        entities.push(Entity::Session(session));
    }
    if safe_to_reconcile {
        let present: BTreeSet<_> = records.iter().map(|record| record.id.as_str()).collect();
        for entity in &previous {
            let Entity::Record(old) = entity else {
                continue;
            };
            if old.project_id == project
                && old.source_id == source_id
                && !present.contains(old.id.as_str())
                && !matches!(
                    old.availability,
                    Availability::Missing | Availability::Deleted
                )
            {
                let mut missing = old.clone();
                missing.availability = Availability::Missing;
                missing.body.clear();
                missing.title.clear();
                missing.alternatives.clear();
                missing.execution = None;
                for evidence in &mut missing.evidence {
                    evidence.availability = Availability::Missing;
                }
                entities.push(Entity::Record(missing));
            }
        }
    }
    // The only automatically created causal edge is an explicit invocation/result ID.
    // A partial page can carry the result of a call captured on an earlier page.
    // Use only retained calls in this exact source and let current revisions win.
    let current_ids: BTreeSet<_> = records.iter().map(|record| record.id.as_str()).collect();
    let previous_calls: Vec<_> = previous
        .iter()
        .filter_map(|entity| match entity {
            Entity::Record(record)
                if record.project_id == project
                    && record.source_id == source_id
                    && record.kind == RecordKind::Attempt
                    && !current_ids.contains(record.id.as_str())
                    && matches!(
                        record.availability,
                        Availability::Available | Availability::Redacted
                    )
                    && !safe_to_reconcile =>
            {
                Some(record)
            }
            _ => None,
        })
        .collect();
    for result in &records {
        if result.kind != RecordKind::ToolResult {
            continue;
        }
        if let Some(execution) = &result.execution {
            for call in records
                .iter()
                .chain(previous_calls.iter().copied())
                .filter(|r| {
                    r.kind == RecordKind::Attempt
                        && r.execution.as_ref().is_some_and(|e| e.id == execution.id)
                })
            {
                entities.push(Entity::Relation(Relation {
                    id: format!("{}:result-of:{}", result.id, call.id),
                    project_id: project.into(),
                    source_id: source_id.into(),
                    from: Target::Record {
                        id: result.id.clone(),
                    },
                    to: Target::Record {
                        id: call.id.clone(),
                    },
                    kind: RelationKind::RespondsTo,
                    nature: Nature::Observed,
                    evidence: result.evidence.clone(),
                    applies_to: Vec::new(),
                }));
            }
        }
    }
    entities.extend(records.into_iter().map(Entity::Record));
    entities.push(Entity::Source(src));
    store.append_all(entities).await
}

fn session_reference(
    project: &str,
    source: &str,
    source_kind: SourceKind,
    native_id: &str,
    known: &[Entity],
    imported: &[ImportedSession],
) -> (String, bool) {
    if imported.iter().any(|session| session.id == native_id) {
        return (format!("{source}:{native_id}"), true);
    }
    let permitted: BTreeSet<_> = known
        .iter()
        .filter_map(|entity| match entity {
            Entity::Source(source)
                if source.project_id == project
                    && source.authorized
                    && source.available
                    && source.kind == source_kind =>
            {
                Some(source.id.as_str())
            }
            _ => None,
        })
        .collect();
    let mut matches = known.iter().filter_map(|entity| match entity {
        Entity::Session(session)
            if session.project_id == project
                && permitted.contains(session.source_id.as_str())
                && session.id == format!("{}:{native_id}", session.source_id) =>
        {
            Some(session.id.clone())
        }
        _ => None,
    });
    if let Some(id) = matches.next()
        && matches.next().is_none()
    {
        (id, true)
    } else {
        (format!("{source}:{native_id}"), false)
    }
}

fn supported_kinds(format: ImportFormat) -> Vec<RecordKind> {
    match format {
        ImportFormat::Document => vec![RecordKind::Finding],
        ImportFormat::CodexJsonl => vec![
            RecordKind::Request,
            RecordKind::Finding,
            RecordKind::Attempt,
            RecordKind::ToolResult,
        ],
        ImportFormat::JournalJsonl => vec![
            RecordKind::Request,
            RecordKind::Constraint,
            RecordKind::Finding,
            RecordKind::Decision,
            RecordKind::Attempt,
            RecordKind::ToolResult,
            RecordKind::Change,
            RecordKind::Verification,
            RecordKind::Feedback,
            RecordKind::Status,
        ],
    }
}

fn unsupported_filters(format: ImportFormat) -> Vec<String> {
    let mut filters = vec!["decision_statuses", "verification_outcomes", "commit_shas"];
    if format == ImportFormat::Document {
        filters.push("attempt_outcomes");
    } else {
        filters.push("paths");
    }
    filters.into_iter().map(str::to_owned).collect()
}

fn resolve_references(record: &mut Record, known: &[Entity]) {
    let allowed: BTreeSet<_> = known
        .iter()
        .filter_map(|entity| match entity {
            Entity::Source(source)
                if source.project_id == record.project_id && source.authorized =>
            {
                Some(source.id.as_str())
            }
            _ => None,
        })
        .collect();
    for evidence in record.evidence.iter_mut().skip(1) {
        let mut candidates = known.iter().filter_map(|entity| match entity {
            Entity::Record(candidate)
                if candidate.id != record.id
                    && candidate.project_id == record.project_id
                    && allowed.contains(candidate.source_id.as_str())
                    && !matches!(
                        candidate.availability,
                        Availability::Missing | Availability::Deleted
                    )
                    && candidate.revision == evidence.revision
                    && candidate.evidence.iter().any(|origin| {
                        origin.locator == evidence.locator && origin.revision == evidence.revision
                    }) =>
            {
                Some(candidate)
            }
            _ => None,
        });
        if let Some(candidate) = candidates.next()
            && candidates.next().is_none()
        {
            evidence.source_id = candidate.source_id.clone();
            evidence.record_id = Some(candidate.id.clone());
            evidence.availability = candidate.availability;
        }
    }
}

/// Re-check only explicitly registered paths. A disappeared source cannot be
/// served from cached raw text; a changed file is stale until explicitly synced.
pub async fn revalidate(store: &mut Store) -> Result<()> {
    for entity in store.latest().await? {
        let Entity::Source(mut src) = entity else {
            continue;
        };
        if !src.authorized || src.kind == SourceKind::Git {
            continue;
        }
        let Some(path) = &src.location else {
            continue;
        };
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut changed = !src.available;
                src.available = true;
                let current = format!("sha256:{}", hash(&bytes));
                if src
                    .content_revision
                    .as_deref()
                    .is_some_and(|r| r != current)
                {
                    let gap =
                        "source_changed: cached revision retained; sync to read the current source"
                            .to_string();
                    if !src.gaps.contains(&gap) {
                        src.gaps.push(gap);
                        changed = true;
                    }
                }
                if changed {
                    store.append(Entity::Source(src)).await?;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                ) =>
            {
                src.available = false;
                src.authorized = false;
                src.gaps = vec!["source_removed_or_access_revoked".into()];
                store.append(Entity::Source(src)).await?;
            }
            Err(_) => {
                let gap = "source_unavailable: cached records may be stale".to_owned();
                let changed = src.available || !src.gaps.contains(&gap);
                src.available = false;
                if !src.gaps.contains(&gap) {
                    src.gaps.push(gap);
                }
                if changed {
                    store.append(Entity::Source(src)).await?;
                }
            }
        }
    }
    Ok(())
}
