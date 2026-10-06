//! Positive evidence graphs complement the real-Git tests in runtime_scenarios.
//! These fixtures assert retrieval of recorded links, not automatic attribution.
use memento::{
    ingest,
    model::*,
    query::{QueryResponse, execute},
    security::hash,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "support/evaluation.rs"]
mod evaluation;

fn add(data: &mut Corpus, entity: Entity) {
    let sequence = data
        .entries
        .last()
        .map_or(1, |entry| entry.sequence.saturating_add(1));
    data.entries.push(Entry {
        sequence,
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}

fn corpus() -> Corpus {
    let mut data = Corpus::default();
    add(
        &mut data,
        Entity::Source(ingest::source("journal", "project", SourceKind::Journal)),
    );
    data
}

fn rec(id: &str, work: &str, kind: RecordKind, body: &str) -> Record {
    let mut record = Record::new(id, "project", "journal", kind, body);
    record.work_ids.push(work.into());
    record.association = Association::Explicit;
    record.occurred_at = Some("2026-09-27T09:00:00Z".into());
    record.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some(id.into()),
        revision: record.revision.clone(),
        locator: format!("selected-journal.jsonl#{id}"),
        availability: Availability::Available,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    });
    record
}

fn record_target(id: &str) -> Target {
    Target::Record { id: id.into() }
}

fn commit_target(sha: &str) -> Target {
    Target::Commit {
        repository_id: "repository".into(),
        commit_sha: sha.into(),
    }
}

fn code_target(id: &str) -> Target {
    Target::Code {
        state_id: id.into(),
        path: "retry.rs".into(),
        range: None,
    }
}

fn link(id: &str, from: Target, to: Target, kind: RelationKind) -> Relation {
    Relation {
        id: id.into(),
        project_id: "project".into(),
        source_id: "journal".into(),
        from,
        to,
        kind,
        nature: Nature::Reported,
        evidence: vec![Evidence {
            source_id: "journal".into(),
            record_id: None,
            revision: hash(id.as_bytes()),
            locator: format!("selected-journal.jsonl#link-{id}"),
            availability: Availability::Available,
            range: None,
            purpose: memento::model::EvidencePurpose::Unspecified,
            span: None,
        }],
        applies_to: Vec::new(),
    }
}

fn commit(sha: &str, worktree: &str) -> Entity {
    Entity::Commit(Commit {
        id: format!("commit:{sha}"),
        project_id: "project".into(),
        source_id: "journal".into(),
        repository_id: "repository".into(),
        sha: sha.into(),
        tree: format!("tree:{sha}"),
        parents: Vec::new(),
        paths: vec!["retry.rs".into()],
        message: "retry improvement".into(),
        occurred_at: Some("2026-09-27T09:00:00Z".into()),
        origin_worktree: Some(worktree.into()),
        observed_worktree: Some(worktree.into()),
    })
}

fn state(id: &str, worktree: &str, sha: Option<&str>, content: &str) -> Entity {
    Entity::CodeState(CodeState {
        id: id.into(),
        project_id: "project".into(),
        source_id: "journal".into(),
        repository_id: "repository".into(),
        worktree_id: Some(worktree.into()),
        commit_sha: sha.map(str::to_owned),
        observed_at: "2026-09-28T10:00:00Z".into(),
        changed_during_observation: false,
        files: vec![FileState {
            path: "retry.rs".into(),
            working_hash: Some(hash(content.as_bytes())),
            working_content: Some(content.into()),
            working_kind: WorkingFileKind::File,
            ..FileState::default()
        }],
    })
}

fn trace_query(target: Target) -> Query {
    let mut query = Query::new(Operation::Trace, "project");
    query.target = Some(target);
    query.max_depth = Some(12);
    query.limit = Some(100);
    query
}

fn trace(data: &Corpus, target: Target) -> Result<QueryResponse, memento::query::QueryError> {
    execute(data, &trace_query(target))
}

fn ids(response: &QueryResponse) -> Vec<&str> {
    response.items.iter().map(|item| item.entity.id()).collect()
}

fn comparison_query(from: &str, to: &str) -> Query {
    let mut query = Query::new(Operation::Compare, "project");
    query.from = Some(HistoryPoint {
        code_state_id: Some(from.into()),
        ..HistoryPoint::default()
    });
    query.to = Some(HistoryPoint {
        code_state_id: Some(to.into()),
        ..HistoryPoint::default()
    });
    query
}

fn code_comparison(data: &Corpus, from: &str, to: &str) -> TestResult {
    let response = execute(data, &comparison_query(from, to))?;
    let comparison = response.comparison.ok_or("missing comparison")?;
    assert_eq!(comparison.mode, "code_only");
    assert_eq!(
        comparison.code.ok_or("missing code comparison")?.status,
        "different"
    );
    Ok(())
}

#[test]
fn partial_commits_retrieve_two_works_by_recorded_ranges_without_transferring_dirty_tests()
-> TestResult {
    let mut data = corpus();
    for (id, goal) in [("retry-work", "retry behavior"), ("log-work", "logging")] {
        add(
            &mut data,
            Entity::Work(Work {
                id: id.into(),
                project_id: "project".into(),
                source_id: "journal".into(),
                title: goal.into(),
                goal: goal.into(),
                status: WorkStatus::Active,
                observed_at: None,
                evidence: Vec::new(),
                completion_conditions: Vec::new(),
            }),
        );
    }
    add(&mut data, commit("C2", "main"));
    add(&mut data, commit("C3", "main"));
    add(
        &mut data,
        state("dirty-K4", "main", None, "retry and logging"),
    );
    add(
        &mut data,
        state("commit-K2", "main", Some("C2"), "retry only"),
    );
    for (id, work, sha, range) in [
        ("P2a", "retry-work", "C2", "retry.rs:1-3"),
        ("P2b", "retry-work", "C3", "retry.rs:2-3"),
        ("P3", "log-work", "C3", "retry.rs:10-12"),
    ] {
        let mut change = rec(
            id,
            work,
            RecordKind::Change,
            "Explicitly recorded hunk attribution",
        );
        change.commit_shas.push(sha.into());
        change.paths.push("retry.rs".into());
        change.applies_to.push(range.into());
        add(&mut data, Entity::Record(change));
        let mut integration = link(
            id,
            record_target(id),
            commit_target(sha),
            RelationKind::IntegratedInto,
        );
        integration.applies_to.push(range.into());
        add(&mut data, Entity::Relation(integration));
    }
    let mut test = rec(
        "T3",
        "retry-work",
        RecordKind::Verification,
        "Dirty working tree passed",
    );
    test.verification_outcome = Some(VerificationOutcome::Passed);
    test.code_refs.push(CodeRef {
        state_id: "dirty-K4".into(),
        path: "retry.rs".into(),
        range: None,
    });
    add(&mut data, Entity::Record(test));
    add(
        &mut data,
        Entity::Relation(link(
            "T3-K4",
            record_target("T3"),
            code_target("dirty-K4"),
            RelationKind::Verifies,
        )),
    );

    let mut query = Query::new(Operation::Trace, "project");
    query.relations = vec![RelationKind::IntegratedInto];
    query.direction = Some(Direction::Incoming);
    query.max_depth = Some(1);
    query.target = Some(commit_target("C2"));
    let first = execute(&data, &query)?;
    assert!(ids(&first).contains(&"P2a"));
    assert!(!ids(&first).contains(&"P3"));
    assert!(!ids(&first).contains(&"T3"));
    assert!(
        first
            .relations
            .iter()
            .any(|relation| relation.applies_to == ["retry.rs:1-3"])
    );
    query.target = Some(commit_target("C3"));
    let second = execute(&data, &query)?;
    assert!(ids(&second).contains(&"P2b"));
    assert!(ids(&second).contains(&"P3"));
    let works: Vec<_> = second
        .items
        .iter()
        .filter_map(|item| match &item.entity {
            Entity::Record(record) => Some(record.work_ids.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(works.contains(&"retry-work".into()));
    assert!(works.contains(&"log-work".into()));
    assert!(
        !second
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::Verifies)
    );
    let mut work_query = Query::new(Operation::Search, "project");
    work_query.scope.work_ids = vec!["retry-work".into()];
    work_query.filters.record_kinds = vec![RecordKind::Change];
    let same_work = execute(&data, &work_query)?;
    assert_eq!(ids(&same_work), vec!["P2a", "P2b"]);
    let work_states = execute(&data, &Query::new(Operation::ListWork, "project"))?;
    assert!(work_states.items.iter().all(
        |item| matches!(&item.entity, Entity::Work(work) if work.status == WorkStatus::Active)
    ));
    code_comparison(&data, "dirty-K4", "commit-K2")?;
    let mut first_commit = query.clone();
    first_commit.target = Some(commit_target("C2"));
    evaluation::capture(
        "SC03",
        "A",
        "C2는 무슨 작업이고 테스트도 됐어? 두 작업의 커밋 범위와 완료 여부를 알려줘.",
        &data,
        &[
            first_commit,
            query,
            work_query,
            Query::new(Operation::ListWork, "project"),
            comparison_query("dirty-K4", "commit-K2"),
        ],
    )?;

    // File-only evidence cannot establish which work owns a committed hunk.
    data.entries.retain(|entry| !matches!(&entry.entity, Entity::Relation(relation) if relation.kind == RelationKind::IntegratedInto));
    for entry in &mut data.entries {
        if let Entity::Record(record) = &mut entry.entity
            && record.kind == RecordKind::Change
        {
            record.applies_to.clear();
            record.commit_shas.clear();
            record.association = Association::Candidate;
        }
    }
    let missing = trace(&data, commit_target("C2"))?;
    assert!(!ids(&missing).contains(&"P2a"));
    assert!(!ids(&missing).contains(&"P3"));
    assert!(
        missing
            .omitted
            .iter()
            .any(|warning| warning.contains("reason_not_recorded"))
    );
    let mut candidates = Query::new(Operation::Brief, "project");
    candidates.filters.paths = vec!["retry.rs".into()];
    let brief = execute(&data, &candidates)?
        .brief
        .ok_or("missing file candidates")?;
    evaluation::capture(
        "SC03",
        "B",
        "C2는 무슨 작업이고 테스트도 됐어? 두 작업의 커밋 범위와 완료 여부를 알려줘.",
        &data,
        &[
            trace_query(commit_target("C2")),
            candidates,
            comparison_query("dirty-K4", "commit-K2"),
            Query::new(Operation::ListWork, "project"),
        ],
    )?;
    assert!(
        brief
            .sections
            .iter()
            .flat_map(|section| &section.claims)
            .filter(|claim| ["P2a", "P2b", "P3"].contains(&claim.record_id.as_str()))
            .all(|claim| claim.applies_to.is_empty()
                && claim
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("candidate or unassigned")))
    );
    Ok(())
}

#[test]
fn rewritten_commit_recovers_original_rationale_attempt_and_verification_only_with_mapping()
-> TestResult {
    let mut data = corpus();
    for sha in ["C5", "C5a", "C6", "C7"] {
        add(&mut data, commit(sha, "feature"));
    }
    add(
        &mut data,
        state("K5", "feature", Some("C5"), "old implementation"),
    );
    add(
        &mut data,
        state("K7", "feature", Some("C7"), "squashed implementation"),
    );
    let mut decision = rec(
        "D3",
        "W5",
        RecordKind::Decision,
        "Choose backoff because normal requests must avoid delay",
    );
    decision.nature = Nature::Reported;
    decision.decision_status = Some(DecisionStatus::Accepted);
    decision.alternatives = vec!["Fixed delay rejected: it delays normal requests".into()];
    add(&mut data, Entity::Record(decision));
    let mut attempt = rec("A5", "W5", RecordKind::Attempt, "Backoff experiment");
    attempt.attempt_outcome = Some(AttemptOutcome::Succeeded);
    add(&mut data, Entity::Record(attempt));
    let mut verification = rec(
        "T4",
        "W5",
        RecordKind::Verification,
        "Tests passed only on C5",
    );
    verification.verification_outcome = Some(VerificationOutcome::Passed);
    verification.commit_shas = vec!["C5".into()];
    verification.code_refs.push(CodeRef {
        state_id: "K5".into(),
        path: "retry.rs".into(),
        range: None,
    });
    add(&mut data, Entity::Record(verification));
    for relation in [
        link(
            "D3-A5",
            record_target("D3"),
            record_target("A5"),
            RelationKind::Supports,
        ),
        link(
            "A5-C5",
            record_target("A5"),
            commit_target("C5"),
            RelationKind::IntegratedInto,
        ),
        link(
            "A5-K5",
            record_target("A5"),
            code_target("K5"),
            RelationKind::Changes,
        ),
        link(
            "T4-K5",
            record_target("T4"),
            code_target("K5"),
            RelationKind::Verifies,
        ),
        link(
            "C5a-C5",
            commit_target("C5a"),
            commit_target("C5"),
            RelationKind::DerivedFrom,
        ),
        link(
            "C7-C5a",
            commit_target("C7"),
            commit_target("C5a"),
            RelationKind::DerivedFrom,
        ),
        link(
            "C7-C6",
            commit_target("C7"),
            commit_target("C6"),
            RelationKind::DerivedFrom,
        ),
    ] {
        add(&mut data, Entity::Relation(relation));
    }
    let response = trace(&data, commit_target("C7"))?;
    for required in [
        "D3",
        "A5",
        "T4",
        "commit:C5",
        "commit:C5a",
        "commit:C6",
        "commit:C7",
    ] {
        assert!(ids(&response).contains(&required), "missing {required}");
    }
    let reasons = response
        .items
        .iter()
        .find_map(|item| match &item.entity {
            Entity::Record(record) if record.id == "D3" => Some(record),
            _ => None,
        })
        .ok_or("missing rationale")?;
    assert_eq!(reasons.alternatives.len(), 1);
    assert!(
        reasons
            .evidence
            .iter()
            .any(|evidence| evidence.locator.ends_with("#D3"))
    );
    assert_eq!(
        response
            .relations
            .iter()
            .filter(|relation| relation.kind == RelationKind::DerivedFrom
                && relation.from == commit_target("C7"))
            .count(),
        2
    );
    assert!(
        response
            .relations
            .iter()
            .filter(|relation| relation.kind == RelationKind::Verifies)
            .all(|relation| relation.to == code_target("K5"))
    );
    code_comparison(&data, "K5", "K7")?;
    evaluation::capture(
        "SC04",
        "A",
        "지금 C7에 들어간 설계 이유와 검증 결과를 보여줘.",
        &data,
        &[
            trace_query(commit_target("C7")),
            comparison_query("K5", "K7"),
        ],
    )?;

    // Same messages and available old records are not a substitute for missing maps.
    data.entries.retain(|entry| !matches!(&entry.entity, Entity::Relation(relation) if relation.kind == RelationKind::DerivedFrom));
    let missing = trace(&data, commit_target("C7"))?;
    evaluation::capture(
        "SC04",
        "B",
        "지금 C7에 들어간 설계 이유와 검증 결과를 보여줘.",
        &data,
        &[trace_query(commit_target("C7"))],
    )?;
    assert_eq!(ids(&missing), vec!["commit:C7"]);
    assert!(
        missing
            .omitted
            .iter()
            .any(|notice| notice.contains("reason_not_recorded"))
    );
    let old = trace(&data, commit_target("C5"))?;
    assert!(ids(&old).contains(&"D3"));
    assert!(ids(&old).contains(&"T4"));
    Ok(())
}

#[test]
fn cherry_pick_and_revert_keep_branch_specific_reasons_and_missing_reason_boundaries() -> TestResult
{
    let mut data = corpus();
    add(&mut data, commit("C8", "main"));
    add(&mut data, commit("C9", "release"));
    add(&mut data, commit("R1", "release"));
    add(
        &mut data,
        state("Kmain", "main", Some("C8"), "new API backoff"),
    );
    add(
        &mut data,
        state(
            "Kpicked",
            "release",
            Some("C9"),
            "release compatible backoff",
        ),
    );
    add(
        &mut data,
        state("Kreverted", "release", Some("R1"), "legacy API behavior"),
    );
    for (id, work, kind, body, tree) in [
        (
            "D4",
            "W6",
            RecordKind::Decision,
            "Main needs new API retries",
            "main",
        ),
        (
            "E9",
            "W7",
            RecordKind::Finding,
            "Conflict resolved by adapting old API call shape",
            "release",
        ),
        (
            "D5",
            "W7",
            RecordKind::Decision,
            "Revert only in release: legacy API compatibility failure",
            "release",
        ),
    ] {
        let mut record = rec(id, work, kind, body);
        record.worktree_id = Some(tree.into());
        record.nature = Nature::Reported;
        add(&mut data, Entity::Record(record));
    }
    let mut test = rec(
        "T5",
        "W7",
        RecordKind::Verification,
        "Release tests passed at C9 before later operational failure",
    );
    test.worktree_id = Some("release".into());
    test.verification_outcome = Some(VerificationOutcome::Passed);
    test.code_refs.push(CodeRef {
        state_id: "Kpicked".into(),
        path: "retry.rs".into(),
        range: None,
    });
    add(&mut data, Entity::Record(test));
    for relation in [
        link(
            "D4-C8",
            record_target("D4"),
            commit_target("C8"),
            RelationKind::Supports,
        ),
        link(
            "C9-C8",
            commit_target("C9"),
            commit_target("C8"),
            RelationKind::DerivedFrom,
        ),
        link(
            "E9-C9",
            record_target("E9"),
            commit_target("C9"),
            RelationKind::Supports,
        ),
        link(
            "C9-Kpicked",
            commit_target("C9"),
            code_target("Kpicked"),
            RelationKind::Changes,
        ),
        link(
            "T5-Kpicked",
            record_target("T5"),
            code_target("Kpicked"),
            RelationKind::Verifies,
        ),
        link(
            "R1-C9",
            commit_target("R1"),
            commit_target("C9"),
            RelationKind::Reverts,
        ),
        link(
            "D5-R1",
            record_target("D5"),
            commit_target("R1"),
            RelationKind::Supports,
        ),
    ] {
        add(&mut data, Entity::Relation(relation));
    }
    let response = trace(&data, commit_target("R1"))?;
    for required in [
        "D4",
        "E9",
        "D5",
        "T5",
        "commit:C8",
        "commit:C9",
        "commit:R1",
    ] {
        assert!(ids(&response).contains(&required), "missing {required}");
    }
    assert!(
        response
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::Reverts
                && relation.from == commit_target("R1")
                && relation.to == commit_target("C9"))
    );
    assert!(
        response
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::DerivedFrom
                && relation.from == commit_target("C9")
                && relation.to == commit_target("C8"))
    );
    assert!(
        response
            .relations
            .iter()
            .filter(|relation| relation.kind == RelationKind::Verifies)
            .all(|relation| relation.to == code_target("Kpicked"))
    );
    code_comparison(&data, "Kpicked", "Kreverted")?;
    let mut main_read = Query::new(Operation::Read, "project");
    main_read.target = Some(code_target("Kmain"));
    let main = execute(&data, &main_read)?;
    evaluation::capture(
        "SC05",
        "A",
        "재시도 개선은 반영됐어? 왜 취소했어? main과 release를 구분해서 알려줘.",
        &data,
        &[
            trace_query(commit_target("R1")),
            comparison_query("Kpicked", "Kreverted"),
            main_read.clone(),
        ],
    )?;
    assert!(main.items.iter().any(|item| matches!(&item.entity, Entity::CodeState(state) if state.worktree_id.as_deref() == Some("main") && state.commit_sha.as_deref() == Some("C8"))));

    // Removing the transplant/revert explanations keeps observed links and main's reason.
    data.entries.retain(|entry| match &entry.entity {
        Entity::Record(record) => !["E9", "D5"].contains(&record.id.as_str()),
        Entity::Relation(relation) => !["E9-C9", "D5-R1"].contains(&relation.id.as_str()),
        _ => true,
    });
    let missing = trace(&data, commit_target("R1"))?;
    evaluation::capture(
        "SC05",
        "B",
        "재시도 개선은 반영됐어? 왜 취소했어? main과 release를 구분해서 알려줘.",
        &data,
        &[
            trace_query(commit_target("R1")),
            comparison_query("Kpicked", "Kreverted"),
            main_read,
        ],
    )?;
    assert!(ids(&missing).contains(&"D4"));
    assert!(ids(&missing).contains(&"commit:R1"));
    assert!(!ids(&missing).contains(&"D5"));
    assert!(!ids(&missing).contains(&"E9"));
    let encoded = serde_json::to_string(&missing)?;
    assert!(!encoded.contains("legacy API compatibility failure"));
    assert!(!encoded.contains("Conflict resolved by adapting"));
    Ok(())
}

#[test]
fn document_only_research_preserves_historical_reason_and_missing_excerpt_boundary() -> TestResult {
    let mut data = corpus();
    let request = rec(
        "U14",
        "W14",
        RecordKind::Request,
        "The candidate must run locally",
    );
    let mut document = rec(
        "V1",
        "W14",
        RecordKind::Finding,
        "Checked 2026-08-01: candidate A requires the hosted service",
    );
    document.nature = Nature::Reported;
    document.paths = vec!["docs/storage-evaluation.md".into()];
    let original_revision = document.revision.clone();
    let mut decision = rec(
        "D14",
        "W14",
        RecordKind::Decision,
        "Defer A because the recorded deployment conditions do not satisfy the local-only requirement; reconsider if those conditions change",
    );
    decision.nature = Nature::Reported;
    decision.decision_status = Some(DecisionStatus::Rejected);
    decision.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some("V1".into()),
        revision: original_revision.clone(),
        locator: "https://example.invalid/candidate-a/deployment#checked-2026-08-01".into(),
        availability: Availability::Available,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    });
    for record in [request, document, decision] {
        add(&mut data, Entity::Record(record));
    }
    for relation in [
        link(
            "U14-D14",
            record_target("U14"),
            record_target("D14"),
            RelationKind::Supports,
        ),
        link(
            "V1-D14",
            record_target("V1"),
            record_target("D14"),
            RelationKind::Supports,
        ),
    ] {
        add(&mut data, Entity::Relation(relation));
    }
    let response = trace(&data, record_target("D14"))?;
    for required in ["U14", "V1", "D14"] {
        assert!(ids(&response).contains(&required));
    }
    let mut query = Query::new(Operation::Brief, "project");
    query.scope.work_ids = vec!["W14".into()];
    query.purpose = Some(BriefPurpose::Explain);
    let brief = execute(&data, &query)?.brief.ok_or("missing brief")?;
    let claim = brief
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .find(|claim| claim.record_id == "D14")
        .ok_or("missing research decision")?;
    assert_eq!(claim.nature, Nature::Reported);
    assert!(
        claim
            .evidence
            .iter()
            .any(|evidence| evidence.revision == original_revision
                && evidence.locator.starts_with("https://"))
    );
    let mut read = Query::new(Operation::Read, "project");
    read.target = Some(Target::Artifact {
        record_id: "V1".into(),
        revision: original_revision,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
    });
    assert!(serde_json::to_string(&execute(&data, &read)?)?.contains("Checked 2026-08-01"));
    assert!(
        data.entries
            .iter()
            .all(|entry| !matches!(entry.entity, Entity::Commit(_) | Entity::CodeState(_)))
    );

    evaluation::capture(
        "SC13",
        "A",
        "A를 왜 제외했고 다시 검토하려면 어떤 조건이 달라져야 해? 당시 근거도 보여줘.",
        &data,
        &[
            trace_query(record_target("D14")),
            query.clone(),
            read.clone(),
        ],
    )?;

    // The old external excerpt is gone. A current document is not its replacement.
    data.entries
        .retain(|entry| !matches!(&entry.entity, Entity::Record(record) if record.id == "V1"));
    let mut current = rec(
        "V2",
        "W14",
        RecordKind::Finding,
        "Current deployment documentation needs independent verification",
    );
    current.nature = Nature::Reported;
    add(&mut data, Entity::Record(current));
    let missing = trace(&data, record_target("D14"))?;
    assert!(ids(&missing).contains(&"D14"));
    assert!(ids(&missing).contains(&"U14"));
    assert!(!ids(&missing).contains(&"V1"));
    assert!(!ids(&missing).contains(&"V2"));
    let decision = missing
        .items
        .iter()
        .find_map(|item| match &item.entity {
            Entity::Record(record) if record.id == "D14" => Some(record),
            _ => None,
        })
        .ok_or("missing retained decision")?;
    assert!(
        decision
            .evidence
            .iter()
            .any(|evidence| evidence.record_id.as_deref() == Some("V1")
                && evidence.availability == Availability::Missing)
    );
    assert!(execute(&data, &read)?.items.is_empty());
    evaluation::capture(
        "SC13",
        "B",
        "A를 왜 제외했고 다시 검토하려면 어떤 조건이 달라져야 해? 당시 근거도 보여줘.",
        &data,
        &[trace_query(record_target("D14")), query, read],
    )?;
    Ok(())
}

#[test]
fn corrected_summary_and_reimported_summary_remain_one_reported_lineage() -> TestResult {
    let mut data = corpus();
    let mut summary = rec(
        "B15",
        "W15",
        RecordKind::Finding,
        "Remaining summary says A was rejected for performance",
    );
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some("original-15".into()),
        revision: "unavailable-old-revision".into(),
        locator: "old-session.jsonl#L10".into(),
        availability: Availability::Missing,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    });
    let mut repeated = rec(
        "B16",
        "W15",
        RecordKind::Finding,
        "Resume summary repeats the performance explanation",
    );
    repeated.derived = true;
    repeated.fidelity = Fidelity::SummaryOnly;
    repeated.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some("B15".into()),
        revision: summary.revision.clone(),
        locator: "selected-journal.jsonl#B15".into(),
        availability: Availability::Available,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    });
    let mut truncated = rec(
        "E15",
        "W15",
        RecordKind::ToolResult,
        "Only the retained beginning of the tool output",
    );
    truncated.fidelity = Fidelity::SourceTruncated;
    let correction = rec(
        "U15",
        "W15",
        RecordKind::Request,
        "Correction: A was excluded because of the license conditions, not performance",
    );
    for record in [summary, repeated, truncated, correction] {
        add(&mut data, Entity::Record(record));
    }
    add(
        &mut data,
        Entity::Relation(link(
            "B16-B15",
            record_target("B16"),
            record_target("B15"),
            RelationKind::DerivedFrom,
        )),
    );
    let mut correction = link(
        "U15-B15",
        record_target("U15"),
        record_target("B15"),
        RelationKind::Supersedes,
    );
    correction.applies_to = vec!["exclusion_reason".into()];
    add(&mut data, Entity::Relation(correction));
    let mut query = Query::new(Operation::Brief, "project");
    query.scope.work_ids = vec!["W15".into()];
    query.purpose = Some(BriefPurpose::Explain);
    let response = execute(&data, &query)?;
    let brief = response.brief.ok_or("missing summary brief")?;
    let claims: Vec<_> = brief
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .collect();
    assert!(claims.iter().any(|claim| claim.record_id == "U15"
        && claim.text.contains("license conditions")
        && claim.fidelity == Fidelity::Original));
    for id in ["B15", "B16"] {
        let claim = claims
            .iter()
            .find(|claim| claim.record_id == id)
            .ok_or("missing summary claim")?;
        assert_eq!(claim.nature, Nature::Reported);
        assert_eq!(claim.fidelity, Fidelity::SummaryOnly);
        assert!(
            claim
                .warnings
                .iter()
                .any(|warning| warning.contains("do not count as independent corroboration"))
        );
    }
    let lineage = trace(&data, record_target("B16"))?;
    for required in ["B15", "B16", "U15"] {
        assert!(ids(&lineage).contains(&required));
    }
    assert!(
        lineage
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::DerivedFrom
                && relation.to == record_target("B15"))
    );
    let mut read = Query::new(Operation::Read, "project");
    read.target = Some(record_target("E15"));
    let retained = execute(&data, &read)?;
    evaluation::capture(
        "SC15",
        "A",
        "A를 안 쓴 진짜 이유와 당시 테스트 출력을 보여줘.",
        &data,
        &[
            query.clone(),
            trace_query(record_target("B16")),
            read.clone(),
        ],
    )?;
    assert!(retained.next_cursor.is_none());
    assert!(retained.items.iter().any(|item| {
        item.warnings
            .iter()
            .any(|warning| warning == "source_truncated")
    }));

    // Without the correction, the surviving summary cannot supply its content.
    data.entries.retain(|entry| match &entry.entity {
        Entity::Record(record) => record.id != "U15",
        Entity::Relation(relation) => relation.id != "U15-B15",
        _ => true,
    });
    let without = execute(&data, &query)?;
    evaluation::capture(
        "SC15",
        "B",
        "A를 안 쓴 진짜 이유와 당시 테스트 출력을 보여줘.",
        &data,
        &[query, trace_query(record_target("B16")), read.clone()],
    )?;
    let encoded = serde_json::to_string(&without)?;
    assert!(!encoded.contains("license conditions"));
    assert!(encoded.contains("summary_only"));
    assert!(encoded.contains("performance"));

    // A separately retained full original must still be readable, not hidden by compaction.
    let original = rec(
        "original-15",
        "W15",
        RecordKind::Finding,
        "Full original public explanation with recorded deployment measurements",
    );
    let revision = original.revision.clone();
    add(&mut data, Entity::Record(original));
    read.target = Some(Target::Artifact {
        record_id: "original-15".into(),
        revision,
        range: None,
    });
    let original = execute(&data, &read)?;
    assert!(original.items.iter().any(|item| matches!(&item.entity, Entity::Record(record) if record.fidelity == Fidelity::Original && record.body.starts_with("Full original"))));
    Ok(())
}

#[tokio::test]
async fn initial_git_only_history_and_new_native_transcript_keep_distinct_coverage() -> TestResult {
    use memento::{Store, adapters::ImportFormat, security::RedactionPolicy};
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    store
        .append(Entity::Source(ingest::source(
            "git",
            "project",
            SourceKind::Git,
        )))
        .await?;
    for sha in ["C16", "C17"] {
        let Entity::Commit(mut checkpoint) = commit(sha, "main") else {
            return Err("invalid fixture commit".into());
        };
        checkpoint.source_id = "git".into();
        store.append(Entity::Commit(checkpoint)).await?;
    }
    let old = trace(&store.load().await?, commit_target("C16"))?;
    assert!(
        old.omitted
            .iter()
            .any(|warning| warning.contains("reason_not_recorded"))
    );
    let transcript = directory.path().join("selected-rollout.jsonl");
    std::fs::write(
        &transcript,
        concat!(
            "{\"timestamp\":\"2026-09-28T08:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"S16\",\"cwd\":\"/selected-repository\"}}\n",
            "{\"timestamp\":\"2026-09-28T08:01:00Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"new-rationale\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"Timeout changed to accommodate the observed queue delay; full production load verification is still pending\"}]}}\n"
        ),
    )?;
    ingest::import(
        &mut store,
        "project",
        "journal",
        &transcript,
        ImportFormat::CodexJsonl,
        Some("W16"),
    )
    .await?;
    store
        .append(Entity::Relation(link(
            "reason-C17",
            record_target("journal:message:new-rationale"),
            commit_target("C17"),
            RelationKind::Supports,
        )))
        .await?;
    let data = store.load().await?;
    let new = trace(&data, commit_target("C17"))?;
    assert!(ids(&new).contains(&"journal:message:new-rationale"));
    assert!(new.items.iter().any(|item| matches!(&item.entity, Entity::Record(record) if record.body.contains("observed queue delay") && record.body.contains("still pending") && record.nature == Nature::Reported && record.work_ids == ["W16"])));
    let old = trace(&data, commit_target("C16"))?;
    assert!(!ids(&old).contains(&"journal:message:new-rationale"));
    assert!(
        old.omitted
            .iter()
            .any(|warning| warning.contains("reason_not_recorded"))
    );
    let sources = execute(&data, &Query::new(Operation::Sources, "project"))?;
    evaluation::capture(
        "SC16",
        "A",
        "C16은 왜 이 timeout을 골랐고 C17에서는 왜 바뀌었어? 과거와 신규 기록 범위를 구분해줘.",
        &data,
        &[
            Query::new(Operation::Sources, "project"),
            trace_query(commit_target("C16")),
            trace_query(commit_target("C17")),
        ],
    )?;
    assert!(sources.items.iter().any(|item| matches!(&item.entity, Entity::Source(source) if source.id == "git" && source.result_only)));
    assert!(sources.items.iter().any(|item| matches!(&item.entity, Entity::Source(source) if source.id == "journal" && !source.result_only && source.last_captured_at.is_some() && source.last_event_id.as_deref() == Some("journal:message:new-rationale"))));

    // A clone with only its Git objects represents a separate store, not revoked data.
    let git_only = Corpus {
        compaction: data.compaction,
        entries: data
            .entries
            .into_iter()
            .filter(|entry| entry.entity.source_id() == "git")
            .collect(),
    };
    let unavailable = trace(&git_only, commit_target("C17"))?;
    assert!(
        unavailable
            .omitted
            .iter()
            .any(|warning| warning.contains("reason_not_recorded"))
    );
    assert!(!serde_json::to_string(&unavailable)?.contains("observed queue delay"));
    let sources = execute(&git_only, &Query::new(Operation::Sources, "project"))?;
    evaluation::capture(
        "SC16",
        "B",
        "C16은 왜 이 timeout을 골랐고 C17에서는 왜 바뀌었어? 과거와 신규 기록 범위를 구분해줘.",
        &git_only,
        &[
            Query::new(Operation::Sources, "project"),
            trace_query(commit_target("C16")),
            trace_query(commit_target("C17")),
        ],
    )?;
    assert_eq!(sources.items.len(), 1);
    assert!(
        sources
            .items
            .iter()
            .all(|item| matches!(&item.entity, Entity::Source(source) if source.result_only))
    );
    Ok(())
}
