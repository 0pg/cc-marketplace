//! Integration contracts only: the provider below is deliberately mechanical.
//! Multilingual retrieval quality is evaluated with the separately installed real models.
use std::{collections::BTreeMap, error::Error, sync::Mutex};

use work_context::{
    ingest,
    mapping::MappingRequest,
    model::*,
    query::{self, LocationStatus, QueryError, ResponseStatus},
    security::{RedactionPolicy, hash},
    semantic::{
        self, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, PreparedSearch,
        SemanticConfig, SemanticError,
    },
};

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default)]
struct CaptureProvider {
    requests: Mutex<Vec<EmbeddingRequest>>,
}
impl EmbeddingProvider for CaptureProvider {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        _: u64,
    ) -> Result<EmbeddingResponse, SemanticError> {
        self.requests
            .lock()
            .map_err(|_| SemanticError::InvalidResponse("fixture lock".into()))?
            .push(request.clone());
        Ok(EmbeddingResponse {
            protocol: 1,
            model_id: request.model_id.clone(),
            model_revision: request.model_revision.clone(),
            query: vec![1.0, 0.0],
            documents: request
                .documents
                .iter()
                .map(|text| {
                    if text.contains("TARGET") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect(),
            metrics: BTreeMap::new(),
        })
    }
}

fn config() -> SemanticConfig {
    SemanticConfig {
        model_id: "mechanical-contract-only".into(),
        model_revision: "fixed-1".into(),
        min_score: 0.5,
        ..SemanticConfig::default()
    }
}
fn source(id: &str) -> Source {
    let mut source = ingest::source(id, "project", SourceKind::Journal);
    source.last_captured_at = Some("2026-09-28T10:00:00Z".into());
    source
}
fn record(id: &str, body: &str) -> Record {
    let mut record = Record::new(id, "project", "journal", RecordKind::ToolResult, body);
    record.occurred_at = Some("2026-09-28T10:00:00Z".into());
    record
}
fn append(data: &mut Corpus, entity: Entity) {
    data.entries.push(Entry {
        sequence: data
            .entries
            .last()
            .map_or(1, |entry| entry.sequence.saturating_add(1)),
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}
fn corpus(records: Vec<Record>) -> Corpus {
    let mut data = Corpus::default();
    append(&mut data, Entity::Source(source("journal")));
    for record in records {
        append(&mut data, Entity::Record(record));
    }
    data
}
fn request() -> Query {
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "표현이 다른 이전 오류".into(),
        mode: SearchMode::Semantic,
    });
    query
}
fn prepare(
    data: &Corpus,
    query: &Query,
    config: &SemanticConfig,
    provider: &CaptureProvider,
) -> Result<PreparedSearch, Box<dyn Error>> {
    let records = query::prepare_semantic_records(data, query)?;
    let text = query.query.as_ref().ok_or("query text missing")?;
    Ok(semantic::prepare_with_provider(
        &records,
        &text.text,
        config,
        &RedactionPolicy::default(),
        provider,
    )?)
}
fn execution(command: &str) -> Result<Execution, serde_json::Error> {
    serde_json::from_value(
        serde_json::json!({"id":"execution","command":command,"tool_name":"shell","tool_input":command,"cwd":null,"started_at":null,"ended_at":null,"exit_code":1,"last_observed_state":"finished","observed_at":null,"liveness":"stopped","before_state":null,"after_state":null,"scope":[],"environment":{}}),
    )
}

#[test]
fn scope_and_typed_filters_gate_records_before_the_provider() -> TestResult {
    let mut allowed = record("allowed", "TARGET authorized failed attempt");
    allowed.kind = RecordKind::Attempt;
    allowed.attempt_outcome = Some(AttemptOutcome::Failed);
    allowed.work_ids = vec!["work".into()];
    allowed.session_id = Some("session".into());
    allowed.worktree_id = Some("tree".into());
    let mut records = vec![allowed.clone()];
    for (id, change) in [
        ("success", 0),
        ("result", 1),
        ("other-work", 2),
        ("other-session", 3),
        ("other-tree", 4),
        ("deleted", 5),
        ("foreign-project", 6),
        ("unselected-source", 7),
        ("revoked-source", 8),
    ] {
        let mut r = allowed.clone();
        r.id = id.into();
        r.body = format!("TARGET forbidden {id}");
        match change {
            0 => r.attempt_outcome = Some(AttemptOutcome::Succeeded),
            1 => r.kind = RecordKind::ToolResult,
            2 => r.work_ids = vec!["other".into()],
            3 => r.session_id = Some("other".into()),
            4 => r.worktree_id = Some("other".into()),
            5 => r.availability = Availability::Deleted,
            6 => r.project_id = "foreign".into(),
            7 => r.source_id = "outside".into(),
            _ => r.source_id = "revoked".into(),
        }
        records.push(r);
    }
    let mut data = corpus(records);
    append(&mut data, Entity::Source(source("outside")));
    let mut revoked = source("revoked");
    revoked.authorized = false;
    append(&mut data, Entity::Source(revoked));
    let mut foreign = source("journal");
    foreign.project_id = "foreign".into();
    append(&mut data, Entity::Source(foreign));
    let mut query = request();
    query.scope.source_ids = vec!["journal".into()];
    query.scope.work_ids = vec!["work".into()];
    query.scope.session_ids = vec!["session".into()];
    query.scope.worktree_ids = vec!["tree".into()];
    query.filters.attempt_outcomes = vec![AttemptOutcome::Failed];
    let provider = CaptureProvider::default();
    let prepared = prepare(&data, &query, &config(), &provider)?;
    assert_eq!(prepared.metrics.eligible_records, 1);
    let requests = provider.requests.lock().map_err(|_| "lock")?;
    assert_eq!(requests.len(), 1);
    assert!(requests.first().is_some_and(|request| {
        request
            .documents
            .iter()
            .any(|document| document.contains("authorized failed attempt"))
    }));
    assert!(!serde_json::to_string(&*requests)?.contains("forbidden"));
    drop(requests);
    let result = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.entity.id())
            .collect::<Vec<_>>(),
        ["allowed"]
    );
    query.filters.verification_outcomes = vec![VerificationOutcome::Unknown];
    assert!(query::prepare_semantic_records(&data, &query)?.is_empty());
    let before = provider.requests.lock().map_err(|_| "lock")?.len();
    let empty = prepare(&data, &query, &config(), &provider)?;
    assert!(empty.candidates.is_empty());
    assert_eq!(provider.requests.lock().map_err(|_| "lock")?.len(), before);
    query.scope.source_ids = vec!["revoked".into()];
    assert!(matches!(
        query::prepare_semantic_records(&data, &query),
        Err(QueryError::InvalidScope(_))
    ));
    Ok(())
}

#[test]
fn fresh_revocation_deletion_revision_title_and_command_changes_reject_prepared_results()
-> TestResult {
    let mut original = record("r", "TARGET original body");
    original.execution = Some(execution("old command")?);
    let data = corpus(vec![original.clone()]);
    let query = request();
    let provider = CaptureProvider::default();
    let prepared = prepare(&data, &query, &config(), &provider)?;
    for change in [
        "revoked",
        "deleted",
        "revision",
        "title",
        "command",
        "tool-input",
    ] {
        let mut fresh = data.clone();
        let mut updated = original.clone();
        match change {
            "revoked" => {
                let mut revoked = source("journal");
                revoked.authorized = false;
                append(&mut fresh, Entity::Source(revoked));
            }
            _ => {
                match change {
                    "deleted" => updated.availability = Availability::Deleted,
                    "revision" => updated.revision = "new-native-revision".into(),
                    "title" => updated.title = "corrected title without body change".into(),
                    "command" => {
                        updated.execution.as_mut().ok_or("execution")?.command =
                            "corrected command".into()
                    }
                    _ => {
                        updated.execution.as_mut().ok_or("execution")?.tool_input =
                            Some("corrected native tool input".into())
                    }
                }
                append(&mut fresh, Entity::Record(updated));
            }
        }
        assert!(
            matches!(
                query::execute_semantic(&fresh, &query, &prepared, &RedactionPolicy::default()),
                Err(QueryError::StaleCursor | QueryError::InvalidScope(_))
            ),
            "stale preparation accepted for {change}"
        );
    }
    Ok(())
}

#[test]
fn semantic_pages_keep_snapshot_on_append_and_reject_model_chunk_or_record_changes() -> TestResult {
    let first = record("a", "TARGET first");
    let second = record("b", "TARGET second");
    let mut data = corpus(vec![first.clone(), second]);
    let provider = CaptureProvider::default();
    let mut query = request();
    query.limit = Some(1);
    let prepared = prepare(&data, &query, &config(), &provider)?;
    let page1 = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(page1.items.first().map(|item| item.entity.id()), Some("a"));
    query.cursor = Some(page1.next_cursor.ok_or("page cursor")?);
    append(
        &mut data,
        Entity::Record(record("aa-new", "TARGET added between pages")),
    );
    let same = prepare(&data, &query, &config(), &provider)?;
    assert_eq!(same.fingerprint, prepared.fingerprint);
    let page2 = query::execute_semantic(&data, &query, &same, &RedactionPolicy::default())?;
    assert_eq!(page2.items.first().map(|item| item.entity.id()), Some("b"));
    assert_eq!(page2.query_snapshot, page1.query_snapshot);
    for changed in [
        SemanticConfig {
            model_revision: "fixed-2".into(),
            ..config()
        },
        SemanticConfig {
            chunk_chars: 128,
            overlap_chars: 16,
            ..config()
        },
    ] {
        let new = prepare(&data, &query, &changed, &provider)?;
        assert!(matches!(
            query::execute_semantic(&data, &query, &new, &RedactionPolicy::default()),
            Err(QueryError::StaleCursor)
        ));
    }
    let mut title_changed = first;
    title_changed.title = "new title same body/source revision".into();
    append(&mut data, Entity::Record(title_changed));
    assert!(matches!(
        query::prepare_semantic_records(&data, &query),
        Err(QueryError::StaleCursor)
    ));
    assert!(matches!(
        query::execute_semantic(&data, &query, &same, &RedactionPolicy::default()),
        Err(QueryError::StaleCursor)
    ));
    Ok(())
}

#[test]
fn excerpts_reference_the_original_unicode_field_chunk_without_literal_match_ranges() -> TestResult
{
    let body = format!(
        "{}TARGET 중간 오류 설명\n{}",
        "무관한 관측\n".repeat(700),
        "끝부분\n".repeat(100)
    );
    let mut title = record("title", "");
    title.title = format!("{}TARGET 제목", "오래된 제목 설명 ".repeat(30));
    let mut command = record("command", "command output without keyword");
    command.execution = Some(execution("TARGET native command argument")?);
    let data = corpus(vec![record("body", &body), title, command]);
    let query = request();
    let provider = CaptureProvider::default();
    let prepared = prepare(&data, &query, &config(), &provider)?;
    let result = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(result.items.len(), 3);
    for item in &result.items {
        assert!(item.match_ranges.is_empty());
        assert!(item.match_locations.is_empty());
        let chunk = item
            .semantic
            .as_ref()
            .and_then(|candidate| candidate.chunks.first())
            .ok_or("semantic chunk")?;
        let original = data
            .entries
            .iter()
            .find_map(|entry| {
                if let Entity::Record(record) = &entry.entity {
                    if record.id == item.entity.id() {
                        Some(record)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or("original record")?;
        let field = match chunk.field.as_str() {
            "body" => original.body.as_str(),
            "title" => original.title.as_str(),
            "command" => original
                .execution
                .as_ref()
                .ok_or("execution")?
                .command
                .as_str(),
            "tool_input" => original
                .execution
                .as_ref()
                .and_then(|execution| execution.tool_input.as_deref())
                .ok_or("tool input")?,
            other => return Err(format!("unexpected field {other}").into()),
        };
        assert_eq!(
            item.excerpt.as_deref(),
            field.get(chunk.byte_start..chunk.byte_end)
        );
        assert!(
            item.excerpt
                .as_ref()
                .is_some_and(|excerpt| excerpt.contains("TARGET"))
        );
        assert_eq!(
            chunk.range.start_line,
            field
                .get(..chunk.byte_start)
                .ok_or("byte start")?
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1
        );
        assert!(
            item.warnings
                .iter()
                .any(|warning| warning.contains("not factual confidence"))
        );
        if item.entity.id() == "body" {
            assert!(chunk.range.start_line > 500);
            assert!(
                matches!(&item.entity,Entity::Record(record) if !record.body.contains("TARGET"))
            );
        }
    }
    Ok(())
}

#[test]
fn unavailable_backend_and_unsupported_operations_are_not_no_matches() -> TestResult {
    let data = corpus(vec![record("r", "ordinary output without target")]);
    let query = request();
    assert!(matches!(
        query::execute(&data, &query),
        Err(QueryError::SemanticUnavailable(_))
    ));
    let records = query::prepare_semantic_records(&data, &query)?;
    assert!(matches!(
        semantic::prepare(&records, "question", &config(), &RedactionPolicy::default()),
        Err(SemanticError::Unavailable(_))
    ));
    let provider = CaptureProvider::default();
    let prepared = prepare(&data, &query, &config(), &provider)?;
    let response = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(response.status, ResponseStatus::NoMatches);
    assert!(response.items.is_empty());
    for operation in [Operation::Timeline, Operation::Brief, Operation::Read] {
        let mut incompatible = query.clone();
        incompatible.operation = operation;
        assert!(matches!(
            query::prepare_semantic_records(&data, &incompatible),
            Err(QueryError::InvalidQuery(_))
        ));
    }
    let mut literal = query.clone();
    literal.query.as_mut().ok_or("text")?.mode = SearchMode::Literal;
    assert!(matches!(
        query::execute_semantic(&data, &literal, &prepared, &RedactionPolicy::default()),
        Err(QueryError::InvalidQuery(_))
    ));
    Ok(())
}

#[test]
fn unavailable_destination_keeps_the_original_trace_and_never_transfers_relations() -> TestResult {
    let original = CodeState {
        id: "before".into(),
        project_id: "project".into(),
        source_id: "journal".into(),
        repository_id: "repository".into(),
        worktree_id: Some("tree".into()),
        commit_sha: None,
        observed_at: "2026-09-28T10:00:00Z".into(),
        changed_during_observation: false,
        files: vec![FileState {
            path: "src/retry.rs".into(),
            working_kind: WorkingFileKind::File,
            working_hash: Some(hash(b"fn retry() {}\n")),
            working_content: Some("fn retry() {}\n".into()),
            ..FileState::default()
        }],
    };
    let target = Target::Code {
        state_id: "before".into(),
        path: "src/retry.rs".into(),
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
    };
    let mut decision = record(
        "D15",
        "The old retry function was selected for bounded retries. No destination verification exists.",
    );
    decision.kind = RecordKind::Decision;
    decision.decision_status = Some(DecisionStatus::Accepted);
    let mut data = corpus(vec![decision]);
    append(&mut data, Entity::CodeState(original));
    append(
        &mut data,
        Entity::Relation(Relation {
            id: "historical-reason".into(),
            project_id: "project".into(),
            source_id: "journal".into(),
            from: target.clone(),
            to: Target::Record { id: "D15".into() },
            kind: RelationKind::RelatedTo,
            nature: Nature::Reported,
            evidence: Vec::new(),
            applies_to: vec!["before".into()],
        }),
    );
    let mut query = Query::new(Operation::Trace, "project");
    query.target = Some(target.clone());
    query.code_mapping = Some(MappingRequest {
        target_state_id: "not-retained".into(),
        target_source_id: Some("journal".into()),
        paths: vec!["src/new.rs".into()],
    });
    let response = query::execute(&data, &query)?;
    let location = response.location.ok_or("location")?;
    assert_eq!(location.status, LocationStatus::Exact);
    assert_eq!(location.mapping_status, Some(LocationStatus::Unavailable));
    assert!(location.mapping.is_none());
    assert_eq!(response.status, ResponseStatus::Partial);
    assert!(response.items.iter().any(|item| item.entity.id() == "D15"));
    assert!(
        response
            .relations
            .iter()
            .all(|relation| relation.from == target && relation.kind == RelationKind::RelatedTo)
    );
    assert!(
        location
            .notes
            .iter()
            .any(|note| note.contains("historical read/trace remains available"))
    );
    Ok(())
}

#[test]
fn custom_policy_masks_model_input_and_rendered_candidates_consistently() -> TestResult {
    let mut original = record("r", "TARGET private-example Bearer abcsecret body");
    original.title = "private-example".into();
    original.execution = Some(execution("TARGET private-example password=my-secret")?);
    let data = corpus(vec![original]);
    let mut query = request();
    query.query.as_mut().ok_or("query")?.text = "why private-example Bearer abcsecret".into();
    let policy = RedactionPolicy {
        literal_secrets: vec!["private-example".into()],
    };
    let provider = CaptureProvider::default();
    let records = query::prepare_semantic_records(&data, &query)?;
    let prepared = semantic::prepare_with_provider(
        &records,
        &query.query.as_ref().ok_or("query")?.text,
        &config(),
        &policy,
        &provider,
    )?;
    let response = query::execute_semantic(&data, &query, &prepared, &policy)?;
    assert_eq!(response.items.len(), 1);
    let input = serde_json::to_string(&*provider.requests.lock().map_err(|_| "lock")?)?;
    let output = serde_json::to_string(&response)?;
    for forbidden in ["private-example", "abcsecret", "my-secret"] {
        assert!(!input.contains(forbidden));
        assert!(!output.contains(forbidden));
    }
    assert!(input.contains("[REDACTED]"));
    assert!(output.contains("[REDACTED]"));
    Ok(())
}

#[test]
fn revoked_or_deleted_data_cannot_reappear_from_an_old_semantic_cursor() -> TestResult {
    let first = record("a", "TARGET first");
    let data = corpus(vec![first.clone(), record("b", "TARGET second")]);
    let mut query = request();
    query.limit = Some(1);
    let provider = CaptureProvider::default();
    let prepared = prepare(&data, &query, &config(), &provider)?;
    query.cursor =
        query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?.next_cursor;
    assert!(query.cursor.is_some());
    for change in ["delete", "revoke"] {
        let mut current = data.clone();
        if change == "delete" {
            let mut deleted = first.clone();
            deleted.availability = Availability::Deleted;
            deleted.body.clear();
            append(&mut current, Entity::Record(deleted));
        } else {
            let mut revoked = source("journal");
            revoked.authorized = false;
            append(&mut current, Entity::Source(revoked));
        }
        assert!(matches!(
            query::prepare_semantic_records(&current, &query),
            Err(QueryError::StaleCursor)
        ));
        assert!(matches!(
            query::execute_semantic(&current, &query, &prepared, &RedactionPolicy::default()),
            Err(QueryError::StaleCursor)
        ));
    }
    Ok(())
}
