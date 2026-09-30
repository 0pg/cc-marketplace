use work_context::model::*;
use work_context::query::{QueryError, ResponseStatus, execute};

#[path = "support/evaluation.rs"]
mod evaluation;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn source() -> Source {
    Source {
        id: "journal".into(),
        project_id: "project".into(),
        kind: SourceKind::Journal,
        name: "test source".into(),
        location: None,
        content_revision: None,
        authorized: true,
        available: true,
        record_kinds: vec![
            RecordKind::Request,
            RecordKind::Attempt,
            RecordKind::ToolResult,
            RecordKind::Decision,
        ],
        unsupported_filters: Vec::new(),
        last_captured_at: Some("2026-09-28T10:00:00Z".into()),
        last_event_id: None,
        gaps: Vec::new(),
        result_only: false,
        import_completeness: ImportCompleteness::FullSnapshot,
    }
}

fn add(corpus: &mut Corpus, entity: Entity) {
    let sequence = corpus
        .entries
        .last()
        .map_or(1, |e| e.sequence.saturating_add(1));
    corpus.entries.push(Entry {
        sequence,
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}

fn corpus() -> Corpus {
    let mut corpus = Corpus::default();
    add(&mut corpus, Entity::Source(source()));
    corpus
}

fn record(id: &str, kind: RecordKind, body: &str) -> Record {
    let mut record = Record::new(id, "project", "journal", kind, body);
    record.work_ids.push("work".into());
    record.association = Association::Explicit;
    record.occurred_at = Some("2026-09-27T09:00:00Z".into());
    record
}

fn relation(id: &str, from: &str, to: &str, kind: RelationKind) -> Relation {
    Relation {
        id: id.into(),
        project_id: "project".into(),
        source_id: "journal".into(),
        from: Target::Record { id: from.into() },
        to: Target::Record { id: to.into() },
        kind,
        nature: Nature::Observed,
        evidence: Vec::new(),
        applies_to: Vec::new(),
    }
}

fn ids(response: &work_context::query::QueryResponse) -> Vec<String> {
    response
        .items
        .iter()
        .map(|i| i.entity.id().to_owned())
        .collect()
}

#[test]
fn search_recovers_output_and_does_not_inherit_attempt_filter() -> TestResult {
    let mut data = corpus();
    let mut attempt = record("attempt", RecordKind::Attempt, "parallel upload");
    attempt.attempt_outcome = Some(AttemptOutcome::Failed);
    add(&mut data, Entity::Record(attempt));
    add(
        &mut data,
        Entity::Record(record(
            "output",
            RecordKind::ToolResult,
            "HTTP 429: too many requests",
        )),
    );
    add(
        &mut data,
        Entity::Record(record(
            "other",
            RecordKind::ToolResult,
            "HTTP 401: authentication required",
        )),
    );
    add(
        &mut data,
        Entity::Relation(relation(
            "result",
            "output",
            "attempt",
            RelationKind::Supports,
        )),
    );
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "HTTP 429".into(),
        mode: SearchMode::Literal,
    });
    query.filters.attempt_outcomes = vec![AttemptOutcome::Failed];
    assert!(execute(&data, &query)?.items.is_empty());
    query.filters.attempt_outcomes.clear();
    let result = execute(&data, &query)?;
    assert_eq!(ids(&result), vec!["output"]);
    assert!(
        result
            .items
            .iter()
            .any(|i| i.match_locations.contains(&"body".into()))
    );
    query.operation = Operation::Trace;
    query.query = None;
    query.target = Some(Target::Record {
        id: "output".into(),
    });
    query.direction = Some(Direction::Outgoing);
    assert!(ids(&execute(&data, &query)?).contains(&"attempt".into()));
    Ok(())
}

#[test]
fn token_filters_or_values_and_unknown_apply_only_to_the_right_kind() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record("unknown", RecordKind::Attempt, "RATE upload limit")),
    );
    add(
        &mut data,
        Entity::Record(record(
            "verification",
            RecordKind::Verification,
            "rate limit",
        )),
    );
    let mut failed = record("failed", RecordKind::Attempt, "rate limit");
    failed.attempt_outcome = Some(AttemptOutcome::Failed);
    add(&mut data, Entity::Record(failed));
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "rate limit".into(),
        mode: SearchMode::Tokens,
    });
    query.filters.attempt_outcomes = vec![AttemptOutcome::Unknown, AttemptOutcome::Failed];
    query.filters.exclude_attempt_outcomes = vec![AttemptOutcome::Failed];
    assert_eq!(ids(&execute(&data, &query)?), vec!["unknown"]);
    query.filters.verification_outcomes = vec![VerificationOutcome::Unknown];
    assert!(execute(&data, &query)?.items.is_empty());
    query.filters.verification_outcomes.clear();
    let mut unsupported = source();
    unsupported.unsupported_filters = vec!["attempt_outcomes".into()];
    add(&mut data, Entity::Source(unsupported));
    assert!(matches!(
        execute(&data, &query),
        Err(QueryError::UnsupportedFilter(_))
    ));
    query.filters.attempt_outcomes.clear();
    assert!(matches!(
        execute(&data, &query),
        Err(QueryError::UnsupportedFilter(_))
    ));
    Ok(())
}

#[test]
fn pages_are_stable_on_addition_and_invalidated_on_revision_or_revocation() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record("a", RecordKind::Finding, "one")),
    );
    add(
        &mut data,
        Entity::Record(record("b", RecordKind::Finding, "two")),
    );
    let mut query = Query::new(Operation::Search, "project");
    query.limit = Some(1);
    let page = execute(&data, &query)?;
    assert!(page.checkpoint.is_none());
    query.cursor = page.next_cursor;
    add(
        &mut data,
        Entity::Record(record("aa", RecordKind::Finding, "new")),
    );
    assert_eq!(ids(&execute(&data, &query)?), vec!["b"]);
    add(
        &mut data,
        Entity::Record(record("a", RecordKind::Finding, "edited")),
    );
    assert!(matches!(
        execute(&data, &query),
        Err(QueryError::StaleCursor)
    ));
    query.cursor = None;
    query.cursor = execute(&data, &query)?.next_cursor;
    let mut revoked = source();
    revoked.authorized = false;
    add(&mut data, Entity::Source(revoked));
    assert!(matches!(
        execute(&data, &query),
        Err(QueryError::StaleCursor | QueryError::InvalidScope(_))
    ));
    Ok(())
}

#[test]
fn long_retained_text_is_read_in_bounded_pages_without_loss() -> TestResult {
    let mut data = corpus();
    let body = "가나다 evidence line\n".repeat(2000);
    add(
        &mut data,
        Entity::Record(record("long", RecordKind::ToolResult, &body)),
    );
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Record { id: "long".into() });
    query.budget_bytes = Some(8192);
    let mut recovered = String::new();
    for _ in 0..100 {
        let response = execute(&data, &query)?;
        assert!(serde_json::to_vec(&response)?.len() <= 8192);
        for item in response.items {
            if let Entity::Record(r) = item.entity {
                recovered.push_str(&r.body);
            }
        }
        query.cursor = response.next_cursor;
        if query.cursor.is_none() {
            break;
        }
    }
    assert_eq!(recovered, body);
    query.range = Some(TextRange {
        start_line: 500,
        end_line: 502,
    });
    query.context_lines = Some(1);
    let response = execute(&data, &query)?;
    assert!(
        response
            .items
            .iter()
            .any(|i| matches!(&i.entity, Entity::Record(r) if r.body.lines().count() == 5))
    );
    Ok(())
}

#[test]
fn checkpoint_waits_for_all_late_arrival_pages_and_reports_deletion() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record("base", RecordKind::Request, "original goal")),
    );
    let query = Query::new(Operation::Search, "project");
    let checkpoint = execute(&data, &query)?
        .checkpoint
        .ok_or("missing checkpoint")?;
    add(
        &mut data,
        Entity::Record(record(
            "late-failure",
            RecordKind::Attempt,
            "Sunday failure",
        )),
    );
    add(
        &mut data,
        Entity::Record(record(
            "late-correction",
            RecordKind::Feedback,
            "Sunday correction",
        )),
    );
    let mut delta = Query::new(Operation::Compare, "project");
    delta.since_checkpoint = Some(checkpoint);
    delta.limit = Some(1);
    let first = execute(&data, &delta)?;
    assert!(first.checkpoint.is_none());
    delta.cursor = first.next_cursor.clone();
    let last = execute(&data, &delta)?;
    assert!(last.checkpoint.is_some());
    assert_ne!(ids(&first), ids(&last));
    let mut deleted = record("base", RecordKind::Request, "");
    deleted.availability = Availability::Deleted;
    add(&mut data, Entity::Record(deleted));
    delta.cursor = None;
    delta.since_checkpoint = last.checkpoint;
    let removal = execute(&data, &delta)?;
    assert!(removal.items.iter().any(|i| matches!(&i.entity, Entity::Record(r) if r.id == "base" && r.body.is_empty() && r.availability == Availability::Deleted)));
    Ok(())
}

#[test]
fn brief_preserves_partial_correction_proposal_and_summary_provenance() -> TestResult {
    let mut data = corpus();
    let mut original = record(
        "U2",
        RecordKind::Constraint,
        "50 per page; preserve fields; no dependency",
    );
    original.applies_to = vec![
        "admin-page-size".into(),
        "public-page-size".into(),
        "fields".into(),
        "dependencies".into(),
    ];
    add(&mut data, Entity::Record(original));
    let mut update = record("U3", RecordKind::Request, "Admin pages use 200");
    update.applies_to = vec!["admin-page-size".into()];
    add(&mut data, Entity::Record(update));
    let mut proposed = record("D6", RecordKind::Decision, "Add a library");
    proposed.decision_status = Some(DecisionStatus::Proposed);
    proposed.nature = Nature::Reported;
    add(&mut data, Entity::Record(proposed));
    let mut supersedes = relation("partial", "U3", "U2", RelationKind::Supersedes);
    supersedes.applies_to = vec!["admin-page-size".into()];
    add(&mut data, Entity::Relation(supersedes));
    let mut summary = record(
        "summary",
        RecordKind::Status,
        "Tests passed according to an earlier summary",
    );
    summary.fidelity = Fidelity::SummaryOnly;
    summary.derived = true;
    add(&mut data, Entity::Record(summary));
    let response = execute(&data, &Query::new(Operation::Brief, "project"))?;
    let brief = response.brief.ok_or("missing brief")?;
    let claims: Vec<_> = brief.sections.iter().flat_map(|s| &s.claims).collect();
    let old = claims
        .iter()
        .find(|c| c.record_id == "U2")
        .ok_or("missing original constraint")?;
    assert!(old.applies_to.contains(&"dependencies".into()));
    assert!(!old.applies_to.contains(&"admin-page-size".into()));
    assert!(claims.iter().any(|c| c.record_id == "U3"));
    assert!(
        claims
            .iter()
            .any(|c| c.record_id == "D6" && c.nature == Nature::Reported)
    );
    assert!(claims.iter().any(|c| c.record_id == "summary"
        && c.nature == Nature::Reported
        && c.fidelity == Fidelity::SummaryOnly));
    assert!(claims.iter().all(|c| !c.evidence.is_empty()));
    Ok(())
}

#[test]
fn stale_summary_and_source_truncation_are_not_observed_full_originals() -> TestResult {
    let mut data = corpus();
    let original = record("original", RecordKind::Finding, "first finding");
    let revision = original.revision.clone();
    add(&mut data, Entity::Record(original));
    let mut summary = record("summary", RecordKind::Status, "derived claim");
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some("original".into()),
        revision,
        locator: "record:original".into(),
        availability: Availability::Available,
        range: None,
    });
    add(&mut data, Entity::Record(summary));
    add(
        &mut data,
        Entity::Record(record("original", RecordKind::Finding, "corrected finding")),
    );
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Record {
        id: "summary".into(),
    });
    let result = execute(&data, &query)?;
    assert!(
        result
            .items
            .iter()
            .any(|i| i.warnings.iter().any(|w| w.starts_with("stale_summary")))
    );
    let mut truncated = record(
        "truncated",
        RecordKind::ToolResult,
        "actual retained output",
    );
    truncated.fidelity = Fidelity::SourceTruncated;
    add(&mut data, Entity::Record(truncated));
    query.target = Some(Target::Record {
        id: "truncated".into(),
    });
    let result = execute(&data, &query)?;
    assert!(result.next_cursor.is_none());
    assert!(result.items.iter().any(
        |i| matches!(&i.entity, Entity::Record(r) if r.nature == Nature::Observed)
            && i.warnings.contains(&"source_truncated".into())
    ));
    Ok(())
}

#[test]
fn unavailable_source_returns_cached_history_as_partial() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record("known", RecordKind::Finding, "retained evidence")),
    );
    let mut unavailable = source();
    unavailable.available = false;
    add(&mut data, Entity::Source(unavailable));
    let response = execute(&data, &Query::new(Operation::Search, "project"))?;
    assert_eq!(ids(&response), vec!["known"]);
    assert_eq!(response.status, ResponseStatus::Partial);
    Ok(())
}

#[test]
fn explicit_historical_revision_remains_readable_until_access_is_revoked() -> TestResult {
    let mut data = corpus();
    let old = record("doc", RecordKind::Finding, "original explanation");
    let revision = old.revision.clone();
    add(&mut data, Entity::Record(old));
    add(
        &mut data,
        Entity::Record(record("doc", RecordKind::Finding, "revised explanation")),
    );
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Artifact {
        record_id: "doc".into(),
        revision,
        range: None,
    });
    let response = execute(&data, &query)?;
    assert!(
        response
            .items
            .iter()
            .any(|i| matches!(&i.entity, Entity::Record(r) if r.body == "original explanation"))
    );
    let mut tombstone = record("doc", RecordKind::Finding, "");
    tombstone.availability = Availability::Deleted;
    add(&mut data, Entity::Record(tombstone));
    assert!(execute(&data, &query)?.items.is_empty());
    Ok(())
}

#[test]
fn trace_is_bounded_and_does_not_leave_work_scope() -> TestResult {
    let mut data = corpus();
    for id in ["a", "b", "c"] {
        add(
            &mut data,
            Entity::Record(record(id, RecordKind::Decision, id)),
        );
    }
    let mut foreign = record("foreign", RecordKind::Decision, "unrelated work");
    foreign.work_ids = vec!["different-work".into()];
    add(&mut data, Entity::Record(foreign));
    for (id, from, to) in [
        ("ab", "a", "b"),
        ("bc", "b", "c"),
        ("ca", "c", "a"),
        ("foreign", "b", "foreign"),
    ] {
        add(
            &mut data,
            Entity::Relation(relation(id, from, to, RelationKind::Supports)),
        );
    }
    let mut query = Query::new(Operation::Trace, "project");
    query.scope.work_ids = vec!["work".into()];
    query.target = Some(Target::Record { id: "a".into() });
    query.direction = Some(Direction::Outgoing);
    query.max_depth = Some(1);
    let response = execute(&data, &query)?;
    assert!(ids(&response).contains(&"b".into()));
    assert!(!ids(&response).contains(&"c".into()));
    query.max_depth = Some(10);
    let response = execute(&data, &query)?;
    assert!(ids(&response).contains(&"c".into()));
    assert!(!ids(&response).contains(&"foreign".into()));
    Ok(())
}

#[test]
fn timeline_uses_session_order_and_time_filter_excludes_unknown_times() -> TestResult {
    let mut data = corpus();
    let mut first = record("first", RecordKind::Request, "first in source");
    first.session_id = Some("s".into());
    first.source_order = Some(1);
    first.occurred_at = Some("2026-09-27T12:00:00Z".into());
    let mut second = record("second", RecordKind::Feedback, "second in source");
    second.session_id = Some("s".into());
    second.source_order = Some(2);
    second.occurred_at = Some("2026-09-27T11:00:00Z".into());
    let mut unknown = record("unknown", RecordKind::Finding, "no clock");
    unknown.occurred_at = None;
    add(&mut data, Entity::Record(second));
    add(&mut data, Entity::Record(first));
    add(&mut data, Entity::Record(unknown));
    let mut query = Query::new(Operation::Timeline, "project");
    query.scope.session_ids = vec!["s".into()];
    assert_eq!(ids(&execute(&data, &query)?), vec!["first", "second"]);
    query.scope.session_ids.clear();
    query.filters.as_of = Some("2026-09-27T11:30:00Z".into());
    assert_eq!(ids(&execute(&data, &query)?), vec!["second"]);
    query.filters.as_of = None;
    query.filters.time_unknown = Some(true);
    assert_eq!(ids(&execute(&data, &query)?), vec!["unknown"]);
    Ok(())
}

#[test]
fn work_goals_are_available_without_fabricating_request_records() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Work(Work {
            id: "work".into(),
            project_id: "project".into(),
            source_id: "journal".into(),
            title: "A work item".into(),
            goal: "Preserve existing API fields".into(),
            status: WorkStatus::Active,
            observed_at: Some("2026-09-28T10:00:00Z".into()),
            evidence: Vec::new(),
            completion_conditions: Vec::new(),
        }),
    );
    let mut query = Query::new(Operation::ListWork, "project");
    query.filters.work_statuses = vec![WorkStatus::Active];
    assert_eq!(ids(&execute(&data, &query)?), vec!["work"]);
    query.operation = Operation::Brief;
    query.target = Some(Target::Work { id: "work".into() });
    let brief = execute(&data, &query)?.brief.ok_or("missing brief")?;
    assert!(
        brief
            .sections
            .iter()
            .flat_map(|s| &s.claims)
            .any(|c| c.record_id == "work:work"
                && c.nature == Nature::Reported
                && c.text.contains("API"))
    );
    Ok(())
}

#[test]
fn code_only_comparison_does_not_invent_historical_decisions() -> TestResult {
    let mut data = corpus();
    for (id, content) in [("before", "fixed delay"), ("after", "backoff")] {
        add(
            &mut data,
            Entity::CodeState(CodeState {
                id: id.into(),
                project_id: "project".into(),
                source_id: "journal".into(),
                repository_id: "repo".into(),
                worktree_id: Some("tree".into()),
                commit_sha: None,
                observed_at: "2026-09-28T10:00:00Z".into(),
                changed_during_observation: false,
                files: vec![FileState {
                    path: "retry.rs".into(),
                    working_hash: Some(work_context::security::hash(content.as_bytes())),
                    working_content: Some(content.into()),
                    ..FileState::default()
                }],
            }),
        );
    }
    let mut query = Query::new(Operation::Compare, "project");
    query.from = Some(HistoryPoint {
        code_state_id: Some("before".into()),
        ..HistoryPoint::default()
    });
    query.to = Some(HistoryPoint {
        code_state_id: Some("after".into()),
        ..HistoryPoint::default()
    });
    let comparison = execute(&data, &query)?
        .comparison
        .ok_or("missing comparison")?;
    assert_eq!(comparison.mode, "code_only");
    assert!(comparison.added.is_empty());
    let code = comparison.code.ok_or("missing code comparison")?;
    assert_eq!(code.changed_paths, vec!["retry.rs"]);
    assert_eq!(code.status, "different");
    Ok(())
}

#[test]
fn identical_source_and_record_ids_never_cross_project_boundaries() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record(
            "same-id",
            RecordKind::Finding,
            "project A retained fact",
        )),
    );
    let checkpoint = execute(&data, &Query::new(Operation::Search, "project"))?.checkpoint;
    let mut other_source = source();
    other_source.project_id = "other-project".into();
    add(&mut data, Entity::Source(other_source));
    let mut other_record = record("same-id", RecordKind::Finding, "PROJECT B PRIVATE BODY");
    other_record.project_id = "other-project".into();
    add(&mut data, Entity::Record(other_record));
    for operation in [
        Operation::Search,
        Operation::Timeline,
        Operation::Brief,
        Operation::Sources,
    ] {
        let response = execute(&data, &Query::new(operation, "project"))?;
        assert!(!serde_json::to_string(&response)?.contains("PROJECT B PRIVATE BODY"));
        assert!(
            response
                .items
                .iter()
                .all(|item| item.entity.project_id() == "project")
        );
    }
    let mut delta = Query::new(Operation::Compare, "project");
    delta.since_checkpoint = checkpoint;
    assert!(execute(&data, &delta)?.items.is_empty());
    Ok(())
}

#[test]
fn record_targets_require_source_scope_when_identifiers_collide() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record("same", RecordKind::Finding, "journal fact")),
    );
    let mut other_source = source();
    other_source.id = "other-source".into();
    add(&mut data, Entity::Source(other_source));
    let mut other_record = record("same", RecordKind::Finding, "other source fact");
    other_record.source_id = "other-source".into();
    add(&mut data, Entity::Record(other_record));
    for operation in [Operation::Read, Operation::Trace, Operation::Brief] {
        let mut query = Query::new(operation, "project");
        query.target = Some(Target::Record { id: "same".into() });
        assert!(matches!(
            execute(&data, &query),
            Err(QueryError::InvalidScope(_))
        ));
        query.scope.source_ids = vec!["journal".into()];
        let response = execute(&data, &query)?;
        assert!(!serde_json::to_string(&response)?.contains("other source fact"));
    }
    Ok(())
}

#[test]
fn rendered_revision_pins_masked_text_independently_of_source_revision() -> TestResult {
    let mut data = corpus();
    let mut retained = record(
        "masked",
        RecordKind::ToolResult,
        &"retained [REDACTED] output\n".repeat(1000),
    );
    retained.availability = Availability::Redacted;
    let original_revision = retained.revision.clone();
    add(&mut data, Entity::Record(retained.clone()));
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Record {
        id: "masked".into(),
    });
    query.budget_bytes = Some(8192);
    let first = execute(&data, &query)?;
    assert!(
        first
            .items
            .iter()
            .all(|item| item.rendered_revision.as_deref()
                == Some(work_context::security::hash(retained.body.as_bytes()).as_str()))
    );
    query.cursor = first.next_cursor;
    retained.body = "replacement policy masks more content".repeat(1000);
    assert_eq!(retained.revision, original_revision);
    add(&mut data, Entity::Record(retained));
    assert!(matches!(
        execute(&data, &query),
        Err(QueryError::StaleCursor)
    ));
    Ok(())
}

fn code_state(id: &str, worktree: &str, path: &str, text: &str) -> CodeState {
    CodeState {
        id: id.into(),
        project_id: "project".into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        worktree_id: Some(worktree.into()),
        commit_sha: None,
        observed_at: "2026-09-28T10:00:00Z".into(),
        changed_during_observation: false,
        files: vec![FileState {
            path: path.into(),
            working_hash: Some(work_context::security::hash(text.as_bytes())),
            working_content: Some(text.into()),
            ..FileState::default()
        }],
    }
}

#[test]
fn sc12_code_locations_require_a_basis_and_trace_the_selected_decision() -> TestResult {
    use work_context::query::LocationStatus;
    let mut data = corpus();
    for (state, tree, decision, body, code) in [
        (
            "main-state",
            "main",
            "D14",
            "fixed retry preserves old behavior",
            "fn retry(attempt: u32) { let delay_ms = 100; sleep(delay_ms); }\n",
        ),
        (
            "experiment-state",
            "experiment",
            "D15",
            "backoff after errors avoids normal latency",
            "fn retry(attempt: u32) { let delay_ms = 100 * 2_u64.pow(attempt); sleep(delay_ms); }\n",
        ),
    ] {
        add(
            &mut data,
            Entity::CodeState(code_state(state, tree, "src/retry.rs", code)),
        );
        add(
            &mut data,
            Entity::Record(record(decision, RecordKind::Decision, body)),
        );
        let mut edge = relation(
            &format!("{decision}-code"),
            decision,
            "unused",
            RelationKind::Supports,
        );
        edge.to = Target::Code {
            state_id: state.into(),
            path: "src/retry.rs".into(),
            range: None,
        };
        add(&mut data, Entity::Relation(edge));
    }
    let mut query = Query::new(Operation::Trace, "project");
    query.target = Some(Target::Code {
        state_id: String::new(),
        path: "src/retry.rs".into(),
        range: None,
    });
    let ambiguous = execute(&data, &query)?;
    assert_eq!(
        ambiguous.location.ok_or("missing resolution")?.status,
        LocationStatus::Ambiguous
    );
    assert!(ambiguous.items.is_empty());
    assert!(ambiguous.relations.is_empty());
    let candidate_reads: Vec<_> = ["main-state", "experiment-state"]
        .into_iter()
        .map(|state_id| {
            let mut read = Query::new(Operation::Read, "project");
            read.target = Some(Target::Code {
                state_id: state_id.into(),
                path: "src/retry.rs".into(),
                range: None,
            });
            read
        })
        .collect();
    let mut ambiguous_queries = vec![Query::new(Operation::Sources, "project"), query.clone()];
    ambiguous_queries.extend(candidate_reads);
    evaluation::capture(
        "SC12",
        "B",
        "retry에서 왜 backoff를 썼어? 현재 checkout은 알 수 없고 파일명 src/retry.rs만 알고 있어.",
        &data,
        &ambiguous_queries,
    )?;
    query.scope.worktree_ids = vec!["experiment".into()];
    let exact = execute(&data, &query)?;
    assert_eq!(
        exact.location.ok_or("missing resolution")?.status,
        LocationStatus::Exact
    );
    // The declared record worktree is needed when traversal is restricted to it.
    query.scope.worktree_ids.clear();
    query.target = Some(Target::Code {
        state_id: "experiment-state".into(),
        path: "src/retry.rs".into(),
        range: None,
    });
    let exact = execute(&data, &query)?;
    assert!(ids(&exact).contains(&"D15".into()));
    assert!(!ids(&exact).contains(&"D14".into()));
    let mut historical_code = query.clone();
    historical_code.operation = Operation::Read;
    let mut decision = Query::new(Operation::Read, "project");
    decision.target = Some(Target::Record { id: "D15".into() });
    evaluation::capture(
        "SC12",
        "A",
        "실험 worktree WX의 experiment-state 당시 src/retry.rs::retry에서 왜 backoff를 썼어? 당시 코드와 결정 근거를 보여줘. 지금 함수는 src/network/retry.rs로 옮겨졌다고 들었어.",
        &data,
        &[
            Query::new(Operation::Sources, "project"),
            query.clone(),
            historical_code,
            decision,
        ],
    )?;
    query.target = Some(Target::Code {
        state_id: "main-state".into(),
        path: "src/moved.rs".into(),
        range: None,
    });
    assert_eq!(
        execute(&data, &query)?
            .location
            .ok_or("missing resolution")?
            .status,
        LocationStatus::Missing
    );
    query.target = Some(Target::Code {
        state_id: "unretained".into(),
        path: "src/retry.rs".into(),
        range: None,
    });
    assert_eq!(
        execute(&data, &query)?
            .location
            .ok_or("missing resolution")?
            .status,
        LocationStatus::Unavailable
    );
    Ok(())
}

#[test]
fn code_text_pages_and_range_reads_preserve_the_retained_rendering() -> TestResult {
    let mut data = corpus();
    let body = "fn retry() { /* 보존된 내용 */ }\n".repeat(1000);
    let mut state = code_state("code", "tree", "retry.rs", &body);
    state.files.push(FileState {
        path: "unrelated.rs".into(),
        working_content: Some("unrelated content".into()),
        ..FileState::default()
    });
    add(&mut data, Entity::CodeState(state));
    let mut query = Query::new(Operation::Read, "project");
    query.budget_bytes = Some(8192);
    query.target = Some(Target::Code {
        state_id: "code".into(),
        path: "retry.rs".into(),
        range: None,
    });
    let mut recovered = String::new();
    for _ in 0..100 {
        let response = execute(&data, &query)?;
        assert!(serde_json::to_vec(&response)?.len() <= 8192);
        for item in response.items {
            assert_eq!(
                item.rendered_revision,
                Some(work_context::security::hash(body.as_bytes()))
            );
            if let Entity::CodeState(state) = item.entity {
                assert_eq!(state.files.len(), 1);
                recovered.push_str(
                    state
                        .files
                        .first()
                        .and_then(|f| f.working_content.as_deref())
                        .ok_or("missing code text")?,
                );
            }
        }
        query.cursor = response.next_cursor;
        if query.cursor.is_none() {
            break;
        }
    }
    assert_eq!(recovered, body);
    query.range = Some(TextRange {
        start_line: 400,
        end_line: 402,
    });
    query.context_lines = Some(1);
    let response = execute(&data, &query)?;
    assert!(response.items.iter().any(|item| matches!(&item.entity, Entity::CodeState(state) if state.files.first().and_then(|f| f.working_content.as_ref()).is_some_and(|body| body.lines().count() == 5))));
    Ok(())
}

#[test]
fn source_coverage_reports_retained_limitations_and_last_durable_record() -> TestResult {
    let mut data = Corpus::default();
    let mut src = source();
    src.record_kinds.clear();
    src.last_captured_at = None;
    add(&mut data, Entity::Source(src));
    let mut missing = record("missing", RecordKind::Verification, "");
    missing.availability = Availability::Missing;
    add(&mut data, Entity::Record(missing));
    let mut summary = record("summary", RecordKind::Status, "reported result");
    summary.fidelity = Fidelity::SummaryOnly;
    add(&mut data, Entity::Record(summary));
    let response = execute(&data, &Query::new(Operation::Sources, "project"))?;
    assert!(response.coverage.iter().any(|c| {
        c.record_kinds.contains(&RecordKind::Verification)
            && c.gaps
                .iter()
                .any(|g| g.contains("missing retained bodies: 1"))
            && c.gaps.iter().any(|g| g.contains("summary_only"))
    }));
    assert!(
        response
            .freshness
            .iter()
            .any(|s| s.last_event_id.as_deref() == Some("summary"))
    );
    assert!(matches!(
        execute(&data, &Query::new(Operation::Compare, "project")),
        Err(QueryError::RescanRequired)
    ));
    Ok(())
}

#[test]
fn missing_and_ignored_working_files_are_not_exact_code_locations() -> TestResult {
    use work_context::query::LocationStatus;
    for (kind, expected) in [
        (WorkingFileKind::Missing, LocationStatus::Missing),
        (WorkingFileKind::Ignored, LocationStatus::Unavailable),
        (WorkingFileKind::Symlink, LocationStatus::Unavailable),
    ] {
        let mut data = corpus();
        let mut state = code_state("state", "tree", "code.rs", "not readable working content");
        if let Some(file) = state.files.first_mut() {
            file.working_kind = kind;
            file.working_content = None;
            file.working_hash = None;
            file.head_hash = Some("old git object".into());
        }
        add(&mut data, Entity::CodeState(state));
        let mut query = Query::new(Operation::Read, "project");
        query.target = Some(Target::Code {
            state_id: "state".into(),
            path: "code.rs".into(),
            range: None,
        });
        let response = execute(&data, &query)?;
        assert_eq!(
            response.location.ok_or("missing location")?.status,
            expected
        );
        assert_eq!(response.status, ResponseStatus::Partial);
    }
    Ok(())
}

#[test]
fn retained_original_revision_is_available_but_missing_original_is_not() -> TestResult {
    let mut data = corpus();
    let original = record("original", RecordKind::Finding, "original text");
    let revision = original.revision.clone();
    add(&mut data, Entity::Record(original.clone()));
    let mut summary = record("summary", RecordKind::Status, "summary of original");
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some("original".into()),
        revision,
        locator: "record:original".into(),
        availability: Availability::Available,
        range: None,
    });
    add(&mut data, Entity::Record(summary));
    let mut query = Query::new(Operation::Read, "project");
    query.target = Some(Target::Record {
        id: "summary".into(),
    });
    add(
        &mut data,
        Entity::Record(record("original", RecordKind::Finding, "edited text")),
    );
    let retained = execute(&data, &query)?;
    assert!(retained.items.iter().any(|i| matches!(&i.entity, Entity::Record(r) if r.evidence.iter().any(|e| e.availability == Availability::Available))));
    let mut missing = original;
    missing.availability = Availability::Missing;
    missing.body.clear();
    add(&mut data, Entity::Record(missing));
    let gone = execute(&data, &query)?;
    assert!(gone.items.iter().any(|i| matches!(&i.entity, Entity::Record(r) if r.evidence.iter().any(|e| e.availability == Availability::Missing))));
    assert_eq!(gone.status, ResponseStatus::Partial);
    Ok(())
}

#[test]
fn large_history_comparison_metadata_is_paged_and_cursors_advance() -> TestResult {
    let mut data = corpus();
    for n in 0..80 {
        let id = format!("record-{n:03}-with-a-long-identity-to-exercise-metadata-budget");
        add(
            &mut data,
            Entity::Record(record(&id, RecordKind::Finding, "retained observation")),
        );
    }
    let mut query = Query::new(Operation::Compare, "project");
    query.limit = Some(1);
    query.budget_bytes = Some(4096);
    query.from = Some(HistoryPoint {
        occurred_at: Some("2026-09-26T00:00:00Z".into()),
        ..HistoryPoint::default()
    });
    query.to = Some(HistoryPoint {
        occurred_at: Some("2026-09-28T00:00:00Z".into()),
        ..HistoryPoint::default()
    });
    let mut added = std::collections::BTreeSet::new();
    let mut cursors = std::collections::BTreeSet::new();
    for _ in 0..100 {
        let response = execute(&data, &query)?;
        assert!(serde_json::to_vec(&response)?.len() <= 4096);
        if let Some(comparison) = response.comparison {
            assert!(comparison.added.len() <= 1);
            added.extend(comparison.added);
        }
        query.cursor = response.next_cursor;
        if let Some(cursor) = &query.cursor {
            assert!(cursors.insert(cursor.clone()));
        } else {
            break;
        }
    }
    assert_eq!(added.len(), 80);
    Ok(())
}

#[test]
fn search_ranges_point_to_exact_one_based_lines_and_drive_read() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record(
            "log",
            RecordKind::ToolResult,
            "first line\nHTTP 429\nretry after 5\nlast line",
        )),
    );
    let mut search = Query::new(Operation::Search, "project");
    search.query = Some(TextQuery {
        text: "HTTP 429\nretry".into(),
        mode: SearchMode::Literal,
    });
    let result = execute(&data, &search)?;
    let range = result
        .items
        .iter()
        .flat_map(|item| &item.match_ranges)
        .find(|hit| hit.field == "body")
        .ok_or("missing body locator")?
        .range
        .clone();
    assert_eq!(
        range,
        TextRange {
            start_line: 2,
            end_line: 3
        }
    );
    let mut read = Query::new(Operation::Read, "project");
    read.target = Some(Target::Record { id: "log".into() });
    read.range = Some(range);
    assert!(execute(&data, &read)?.items.iter().any(|item| matches!(&item.entity, Entity::Record(record) if record.body == "HTTP 429\nretry after 5")));
    Ok(())
}

#[test]
fn token_search_maps_expanding_unicode_lowercase_back_to_original_text() -> TestResult {
    let mut data = corpus();
    let body = format!("{}\nHTTP ERROR\nlast line", "İ".repeat(1000));
    add(
        &mut data,
        Entity::Record(record("unicode", RecordKind::ToolResult, &body)),
    );
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "http error".into(),
        mode: SearchMode::Tokens,
    });
    let response = execute(&data, &query)?;
    let item = response
        .items
        .first()
        .ok_or("missing unicode search result")?;
    assert!(
        item.excerpt
            .as_ref()
            .is_some_and(|excerpt| excerpt.contains("HTTP ERROR"))
    );
    assert!(
        item.match_ranges
            .iter()
            .filter(|hit| hit.field == "body")
            .all(|hit| hit.range
                == TextRange {
                    start_line: 2,
                    end_line: 2
                })
    );
    query.query = Some(TextQuery {
        text: "i\u{307}".into(),
        mode: SearchMode::Tokens,
    });
    let response = execute(&data, &query)?;
    assert!(
        response
            .items
            .iter()
            .flat_map(|item| &item.match_ranges)
            .any(|hit| hit.field == "body"
                && hit.range
                    == TextRange {
                        start_line: 1,
                        end_line: 1
                    })
    );
    Ok(())
}

#[test]
fn checkpoint_changes_do_not_reveal_relations_to_forbidden_endpoints() -> TestResult {
    let mut data = corpus();
    add(
        &mut data,
        Entity::Record(record(
            "public-record",
            RecordKind::Finding,
            "public evidence",
        )),
    );
    let checkpoint = execute(&data, &Query::new(Operation::Search, "project"))?.checkpoint;
    let mut secret_source = source();
    secret_source.id = "secret".into();
    secret_source.authorized = false;
    add(&mut data, Entity::Source(secret_source));
    let mut secret = record(
        "PRIVATE_RECORD_IDENTIFIER",
        RecordKind::Finding,
        "PRIVATE_BODY",
    );
    secret.source_id = "secret".into();
    add(&mut data, Entity::Record(secret));
    add(
        &mut data,
        Entity::Relation(relation(
            "relation-to-private",
            "public-record",
            "PRIVATE_RECORD_IDENTIFIER",
            RelationKind::Supports,
        )),
    );
    let mut query = Query::new(Operation::Compare, "project");
    query.since_checkpoint = checkpoint;
    let response = execute(&data, &query)?;
    let serialized = serde_json::to_string(&response)?;
    assert!(!serialized.contains("PRIVATE_RECORD_IDENTIFIER"));
    assert!(!serialized.contains("PRIVATE_BODY"));
    assert!(!serialized.contains("relation-to-private"));
    assert!(response.items.is_empty());
    Ok(())
}
