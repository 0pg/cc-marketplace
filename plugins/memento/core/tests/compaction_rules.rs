use std::collections::BTreeSet;

use memento::{
    compaction::{Policy, plan, plan_pinned},
    ingest,
    model::*,
    query,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn policy() -> Policy {
    Policy {
        recent_entries: 0,
        ..Policy::default()
    }
}

fn entry(sequence: u64, entity: Entity) -> Entry {
    Entry {
        sequence,
        captured_at: "2026-09-29T01:00:00Z".into(),
        entity,
    }
}

fn source(sequence: u64) -> Entry {
    let mut source = ingest::source("journal", "p", SourceKind::Journal);
    source.content_revision = Some(format!("source-revision-{sequence}"));
    entry(sequence, Entity::Source(source))
}

fn record(id: &str, kind: RecordKind, body: &str) -> Record {
    Record::new(id, "p", "journal", kind, body)
}

fn evidence(record: &Record) -> Evidence {
    Evidence {
        source_id: record.source_id.clone(),
        record_id: Some(record.id.clone()),
        revision: record.revision.clone(),
        locator: format!("record:{}", record.id),
        availability: Availability::Available,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
    }
}

fn work() -> Work {
    Work {
        id: "W1".into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        title: "Bound retry concurrency".into(),
        goal: "Avoid HTTP 429 without slowing normal requests".into(),
        status: WorkStatus::Completed,
        observed_at: None,
        evidence: Vec::new(),
        completion_conditions: Vec::new(),
    }
}

fn session(id: &str, status: SessionStatus) -> Session {
    Session {
        id: id.into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        work_ids: vec!["W1".into()],
        status,
        started_at: None,
        ended_at: None,
        worktree_id: None,
        parent_id: None,
        working_directory: None,
    }
}

fn relation(id: &str, from: &str, to: &str, kind: RelationKind) -> Relation {
    Relation {
        id: id.into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        from: Target::Record { id: from.into() },
        to: Target::Record { id: to.into() },
        kind,
        nature: Nature::Reported,
        evidence: Vec::new(),
        applies_to: Vec::new(),
    }
}

fn execution(id: &str) -> Execution {
    Execution {
        id: id.into(),
        command: "cargo test retry".into(),
        tool_name: Some("process".into()),
        tool_input: None,
        cwd: None,
        started_at: None,
        ended_at: None,
        exit_code: Some(1),
        last_observed_state: "failed".into(),
        observed_at: None,
        liveness: Liveness::Stopped,
        before_state: None,
        after_state: None,
        scope: Vec::new(),
        environment: Environment::default(),
    }
}

#[test]
fn native_failure_retains_results_before_relation_creation_in_the_same_source() -> TestResult {
    let mut attempt = record(
        "X1:attempt",
        RecordKind::Attempt,
        "Command completed; see output",
    );
    attempt.attempt_outcome = Some(AttemptOutcome::Failed);
    attempt.attempt_id = Some(attempt.id.clone());
    attempt.execution = Some(execution("X1"));
    let mut output = record(
        "X1:result",
        RecordKind::ToolResult,
        "HTTP 429 from concurrent retries",
    );
    output.attempt_id = Some(attempt.id.clone());
    let mut imported_output = record(
        "imported-result",
        RecordKind::ToolResult,
        "Additional stderr from the same execution",
    );
    imported_output.execution = Some(execution("X1"));
    let mut wrong_source = imported_output.clone();
    wrong_source.source_id = "other".into();
    let mut wrong_execution = imported_output.clone();
    wrong_execution.id = "other-result".into();
    wrong_execution.execution = Some(execution("X2"));
    let entries = vec![
        source(1),
        entry(2, Entity::Record(attempt)),
        entry(3, Entity::Record(output)),
        entry(4, Entity::Record(imported_output)),
        entry(5, Entity::Record(wrong_source)),
        entry(6, Entity::Record(wrong_execution)),
        source(7),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([2, 3, 4, 7]));
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 3 && reason.rule == "attempt_result_reference")
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 4 && reason.rule == "same_execution")
    );
    Ok(())
}

#[test]
fn dense_execution_group_is_retained_only_when_a_member_is_retained() -> TestResult {
    let mut entries = vec![source(1)];
    for sequence in 2..=2049 {
        let mut output = record(
            &format!("output-{sequence}"),
            RecordKind::ToolResult,
            "Another chunk from the same execution",
        );
        output.execution = Some(execution("long-running-command"));
        entries.push(entry(sequence, Entity::Record(output)));
    }
    entries.push(source(2050));
    let unreferenced = plan(&entries, &policy())?;
    assert_eq!(unreferenced.retained_sequences, BTreeSet::from([2050]));
    assert!(unreferenced.report.fits);

    let pinned = plan_pinned(&entries, &policy(), &BTreeSet::from([2]))?;
    assert_eq!(pinned.retained_sequences, (2..=2050).collect());
    assert!(pinned.report.fits);
    Ok(())
}

#[test]
fn incoming_batch_is_retained_or_rejected_as_a_whole() -> TestResult {
    let entries = vec![
        source(1),
        entry(
            2,
            Entity::Record(record(
                "old",
                RecordKind::ToolResult,
                "Old unreferenced output",
            )),
        ),
        entry(
            3,
            Entity::Record(record(
                "new",
                RecordKind::ToolResult,
                "New output awaiting its decision",
            )),
        ),
        source(4),
    ];
    let pinned = BTreeSet::from([3, 4]);
    let mut bounded = policy();
    bounded.max_entries = 1;
    let result = plan_pinned(&entries, &bounded, &pinned)?;
    assert!(!result.report.fits);
    assert_eq!(result.retained_sequences, BTreeSet::from([3, 4]));
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 3 && reason.rule == "incoming_write")
    );
    bounded.max_entries = 2;
    assert!(plan_pinned(&entries, &bounded, &pinned)?.report.fits);
    assert_eq!(
        plan(&entries, &bounded)?.retained_sequences,
        BTreeSet::from([4])
    );
    assert!(plan_pinned(&entries, &bounded, &BTreeSet::from([99])).is_err());
    Ok(())
}

#[test]
fn completed_retry_keeps_cause_and_original_evidence_but_drops_verbose_output() -> TestResult {
    let output = record(
        "E1",
        RecordKind::ToolResult,
        "HTTP 429 after 4 concurrent requests",
    );
    let mut attempt = record("A1", RecordKind::Attempt, "Concurrent upload retry failed");
    attempt.attempt_outcome = Some(AttemptOutcome::Failed);
    attempt.work_ids = vec!["W1".into()];
    attempt.session_id = Some("S1".into());
    attempt.evidence = vec![evidence(&output)];
    let cause = record(
        "F1",
        RecordKind::Finding,
        "Concurrency exceeded the endpoint limit",
    );
    let entries = vec![
        source(1),
        entry(2, Entity::Work(work())),
        entry(3, Entity::Session(session("S1", SessionStatus::Ended))),
        entry(4, Entity::Record(output)),
        entry(5, Entity::Record(attempt)),
        entry(6, Entity::Record(cause)),
        entry(
            7,
            Entity::Relation(relation("L1", "F1", "A1", RelationKind::Supports)),
        ),
        entry(
            8,
            Entity::Record(record(
                "verbose",
                RecordKind::ToolResult,
                &"irrelevant trace\n".repeat(1000),
            )),
        ),
        entry(
            9,
            Entity::Record(record(
                "D1",
                RecordKind::Decision,
                "Limit the endpoint to two concurrent requests",
            )),
        ),
    ];
    let result = plan(&entries, &policy())?;
    assert!(result.report.fits);
    assert_eq!(
        result.retained_sequences,
        BTreeSet::from([1, 2, 3, 4, 5, 6, 7, 9])
    );
    assert!(result.report.after.payload_bytes < result.report.before.payload_bytes / 2);
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 4 && reason.rule == "exact_evidence_revision")
    );
    assert!(
        result
            .report
            .removals
            .iter()
            .any(|removal| removal.sequence == 8 && removal.entity_id == "verbose")
    );
    Ok(())
}

#[test]
fn exact_old_evidence_keeps_current_revision_and_does_not_resurrect_old_text() -> TestResult {
    let unused = record("E1", RecordKind::ToolResult, "obsolete draft");
    let old = record(
        "E1",
        RecordKind::ToolResult,
        "429 happened at concurrency 4",
    );
    let current = record(
        "E1",
        RecordKind::ToolResult,
        "Corrected observation: concurrency 5",
    );
    let mut decision = record(
        "D1",
        RecordKind::Decision,
        "The original observation justified the initial limit",
    );
    decision.evidence = vec![evidence(&old)];
    let entries = vec![
        source(1),
        entry(2, Entity::Record(unused)),
        entry(3, Entity::Record(old.clone())),
        entry(4, Entity::Record(current.clone())),
        entry(5, Entity::Record(decision)),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([1, 3, 4, 5]));
    let retained = Corpus {
        entries: entries
            .into_iter()
            .filter(|entry| result.retained_sequences.contains(&entry.sequence))
            .collect(),
        ..Corpus::default()
    };
    let mut request = Query::new(Operation::Read, "p");
    request.target = Some(Target::Record { id: "E1".into() });
    let response = query::execute(&retained, &request)?;
    assert!(
        response.items.iter().any(
            |item| matches!(&item.entity, Entity::Record(record) if record.body == current.body)
        )
    );
    request.target = Some(Target::Artifact {
        record_id: "E1".into(),
        revision: old.revision,
        range: None,
    });
    let response = query::execute(&retained, &request)?;
    assert!(
        response
            .items
            .iter()
            .any(|item| matches!(&item.entity, Entity::Record(record) if record.body == old.body))
    );
    Ok(())
}

#[test]
fn changed_derived_evidence_keeps_historical_privacy_dependencies_transitively() -> TestResult {
    let mut secret = record("A", RecordKind::ToolResult, "Original private evidence");
    secret.source_id = "secret".into();
    let public = record("B", RecordKind::ToolResult, "Replacement public evidence");
    let mut old = record(
        "D",
        RecordKind::Finding,
        "Originally derived from the private source",
    );
    old.derived = true;
    old.evidence = vec![evidence(&secret)];
    let mut current = record(
        "D",
        RecordKind::Finding,
        "Current derived claim cites the public source",
    );
    current.derived = true;
    current.evidence = vec![evidence(&public)];
    let mut decision = record("E", RecordKind::Decision, "Adopt the derived finding");
    decision.evidence = vec![evidence(&current)];
    let entries = vec![
        source(1),
        entry(
            2,
            Entity::Source(ingest::source("secret", "p", SourceKind::Journal)),
        ),
        entry(3, Entity::Record(secret)),
        entry(4, Entity::Record(public)),
        entry(5, Entity::Record(old)),
        entry(6, Entity::Record(current)),
        entry(7, Entity::Record(decision)),
        entry(
            8,
            Entity::Record(record(
                "unrelated",
                RecordKind::ToolResult,
                "Disposable output",
            )),
        ),
        source(9),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(
        result.retained_sequences,
        BTreeSet::from([2, 3, 4, 5, 6, 7, 9])
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 5
                && reason.rule == "historical_privacy_lineage"
                && reason.via == Some(6))
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 3 && reason.rule == "exact_evidence_revision")
    );
    Ok(())
}

#[test]
fn dense_privacy_lineage_keeps_every_historical_revision_and_its_exact_original() -> TestResult {
    let mut entries = vec![source(1)];
    for index in 0..1024 {
        let original = record(
            &format!("original-{index}"),
            RecordKind::ToolResult,
            "Independent original evidence",
        );
        let mut revision = record(
            "derived-finding",
            RecordKind::Finding,
            &format!("Historical finding revision {index}"),
        );
        revision.derived = true;
        revision.evidence = vec![evidence(&original)];
        entries.push(entry(2 + index * 2, Entity::Record(original)));
        entries.push(entry(3 + index * 2, Entity::Record(revision)));
    }
    entries.push(source(2050));
    let unreferenced = plan(&entries, &policy())?;
    assert_eq!(unreferenced.retained_sequences, BTreeSet::from([2050]));
    assert!(unreferenced.report.fits);

    let pinned = plan_pinned(&entries, &policy(), &BTreeSet::from([3]))?;
    assert_eq!(pinned.retained_sequences, (2..=2050).collect());
    assert!(pinned.report.fits);
    Ok(())
}

#[test]
fn privacy_lineage_keeps_external_locators_but_not_local_or_self_only_revisions() -> TestResult {
    let original = record(
        "A",
        RecordKind::ToolResult,
        "A separate record in the same source",
    );
    let mut self_only = record(
        "D",
        RecordKind::Decision,
        "Early draft with a self reference",
    );
    self_only.evidence = vec![evidence(&self_only)];
    let mut local_locator = record(
        "D",
        RecordKind::Decision,
        "Imported draft with its own source locator",
    );
    let mut locator = evidence(&local_locator);
    locator.record_id = None;
    local_locator.evidence = vec![locator.clone()];
    let mut same_source_dependency = record(
        "D",
        RecordKind::Decision,
        "This revision depended on another local record",
    );
    same_source_dependency.evidence = vec![evidence(&original)];
    let mut external_locator = record(
        "D",
        RecordKind::Decision,
        "This revision depended on an external source locator",
    );
    locator.source_id = "secret".into();
    external_locator.evidence = vec![locator];
    let latest = record(
        "D",
        RecordKind::Decision,
        "Latest decision no longer cites the older evidence",
    );
    let entries = vec![
        source(1),
        entry(
            2,
            Entity::Source(ingest::source("secret", "p", SourceKind::Journal)),
        ),
        entry(3, Entity::Record(original)),
        entry(4, Entity::Record(self_only)),
        entry(5, Entity::Record(local_locator)),
        entry(6, Entity::Record(same_source_dependency)),
        entry(7, Entity::Record(external_locator)),
        entry(8, Entity::Record(latest)),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(
        result.retained_sequences,
        BTreeSet::from([1, 2, 3, 6, 7, 8])
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 6 && reason.rule == "historical_privacy_lineage")
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 7 && reason.rule == "historical_privacy_lineage")
    );
    Ok(())
}

#[test]
fn partial_supersession_keeps_both_decisions_and_original_scope() -> TestResult {
    let mut old = record(
        "D1",
        RecordKind::Decision,
        "Use four workers for all endpoints",
    );
    old.decision_status = Some(DecisionStatus::Accepted);
    let mut new = record(
        "D2",
        RecordKind::Decision,
        "Use two workers for uploads; keep four for reads",
    );
    new.decision_status = Some(DecisionStatus::Accepted);
    let mut supersedes = relation("L1", "D2", "D1", RelationKind::Supersedes);
    supersedes.applies_to = vec!["upload endpoint concurrency".into()];
    let entries = vec![
        source(1),
        entry(2, Entity::Record(old)),
        entry(3, Entity::Record(new)),
        entry(4, Entity::Relation(supersedes)),
        source(5),
    ];
    let before = serde_json::to_string(&entries)?;
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([2, 3, 4, 5]));
    assert_eq!(serde_json::to_string(&entries)?, before);
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 4 && reason.rule == "strong_relation")
    );
    Ok(())
}

#[test]
fn cyclic_evidence_and_relations_reach_a_finite_deterministic_closure() -> TestResult {
    let mut first = record("F1", RecordKind::Finding, "First observation");
    let mut second = record("F2", RecordKind::Finding, "Second observation");
    first.evidence = vec![evidence(&second)];
    second.evidence = vec![evidence(&first)];
    let mut decision = record("D1", RecordKind::Decision, "Keep both observations visible");
    decision.evidence = vec![evidence(&first)];
    let mut entries = vec![
        source(1),
        entry(2, Entity::Record(first)),
        entry(3, Entity::Record(second)),
        entry(4, Entity::Record(decision)),
        entry(
            5,
            Entity::Relation(relation("L1", "F1", "F2", RelationKind::Contradicts)),
        ),
        entry(
            6,
            Entity::Relation(relation("L2", "F2", "F1", RelationKind::Supports)),
        ),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences.len(), entries.len());
    assert_eq!(result.report.reasons.len(), entries.len());
    entries.reverse();
    let reversed = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, reversed.retained_sequences);
    assert_eq!(result.report, reversed.report);
    Ok(())
}

fn determinism_fixture() -> Vec<Entry> {
    let mut private = record(
        "private",
        RecordKind::ToolResult,
        "Private original evidence",
    );
    private.source_id = "secret".into();
    let old_output = record(
        "output",
        RecordKind::ToolResult,
        "Initial failed observation",
    );
    let new_output = record(
        "output",
        RecordKind::ToolResult,
        "Corrected failed observation",
    );
    let mut old_finding = record(
        "F1",
        RecordKind::Finding,
        "Finding based on private evidence",
    );
    old_finding.derived = true;
    old_finding.evidence = vec![evidence(&private)];
    let mut current_finding = record("F1", RecordKind::Finding, "Finding with revised evidence");
    current_finding.derived = true;
    let mut second_finding = record("F2", RecordKind::Finding, "Corroborating finding");
    current_finding.evidence = vec![evidence(&second_finding), evidence(&old_output)];
    second_finding.evidence = vec![evidence(&current_finding)];
    let mut decision = record("D1", RecordKind::Decision, "Adopt the linked findings");
    decision.work_ids = vec!["W1".into()];
    decision.evidence = vec![evidence(&current_finding), evidence(&second_finding)];
    let mut supports = relation("L1", "F1", "F2", RelationKind::Supports);
    supports.evidence = vec![evidence(&old_output), evidence(&private)];
    let mut tombstone = record("deleted", RecordKind::ToolResult, "");
    tombstone.availability = Availability::Deleted;
    vec![
        source(1),
        entry(
            2,
            Entity::Source(ingest::source("secret", "p", SourceKind::Journal)),
        ),
        entry(3, Entity::Work(work())),
        entry(4, Entity::Record(private)),
        entry(5, Entity::Record(old_output)),
        entry(6, Entity::Record(new_output)),
        entry(7, Entity::Record(old_finding)),
        entry(8, Entity::Record(current_finding)),
        entry(9, Entity::Record(second_finding)),
        entry(10, Entity::Record(decision)),
        entry(11, Entity::Relation(supports)),
        entry(
            12,
            Entity::Relation(relation("L2", "output", "F2", RelationKind::Contradicts)),
        ),
        entry(13, Entity::Record(tombstone)),
        entry(
            14,
            Entity::Record(record(
                "unused",
                RecordKind::ToolResult,
                "Disposable output",
            )),
        ),
        entry(
            15,
            Entity::Record(record(
                "incoming",
                RecordKind::ToolResult,
                "New unlinked output",
            )),
        ),
        source(16),
    ]
}

fn permuted(entries: &[Entry], seed: u32) -> Vec<Entry> {
    let mut result = entries.to_vec();
    result.sort_by_key(|entry| {
        entry
            .sequence
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(seed)
    });
    result
}

#[test]
fn snapshot_permutations_preserve_full_reports_at_the_capacity_boundary() -> TestResult {
    let entries = determinism_fixture();
    let pinned = BTreeSet::from([15]);
    let expected: BTreeSet<_> = (2..=13).chain([15, 16]).collect();
    for max_entries in [13, 14] {
        let bounded = Policy {
            max_entries,
            recent_entries: 2,
            ..policy()
        };
        let original = plan_pinned(&entries, &bounded, &pinned)?;
        assert_eq!(original.retained_sequences, expected);
        assert_eq!(original.report.fits, max_entries == expected.len());
        for seed in [1, 7, 19, 31] {
            let shuffled = permuted(&entries, seed);
            assert_ne!(
                shuffled
                    .iter()
                    .map(|entry| entry.sequence)
                    .collect::<Vec<_>>(),
                entries
                    .iter()
                    .map(|entry| entry.sequence)
                    .collect::<Vec<_>>()
            );
            let candidate = plan_pinned(&shuffled, &bounded, &pinned)?;
            assert_eq!(candidate.retained_sequences, original.retained_sequences);
            assert_eq!(candidate.report, original.report);
        }
    }
    Ok(())
}

#[test]
fn repeated_and_reordered_domain_dependencies_preserve_retention_and_reasons() -> TestResult {
    let entries = determinism_fixture();
    let pinned = BTreeSet::from([15]);
    let original = plan_pinned(&entries, &policy(), &pinned)?;
    for seed in [1, 7, 19, 31] {
        let mut duplicated = permuted(&entries, seed);
        for entry in &mut duplicated {
            let evidence = match &mut entry.entity {
                Entity::Record(record) => {
                    record.work_ids.extend(record.work_ids.clone());
                    &mut record.evidence
                }
                Entity::Relation(relation) => &mut relation.evidence,
                _ => continue,
            };
            let original_evidence = evidence.clone();
            evidence.reverse();
            evidence.extend(original_evidence.iter().cloned());
            evidence.extend(original_evidence.into_iter().rev());
        }
        let candidate = plan_pinned(&duplicated, &policy(), &pinned)?;
        assert_eq!(candidate.retained_sequences, original.retained_sequences);
        assert_eq!(candidate.report.reasons, original.report.reasons);
        assert_eq!(candidate.report.removals, original.report.removals);
        assert_eq!(
            candidate.report.omitted_reasons,
            original.report.omitted_reasons
        );
        assert!(candidate.report.after.payload_bytes > original.report.after.payload_bytes);
    }
    Ok(())
}

#[test]
fn duplicate_captured_sequences_are_rejected_instead_of_deduplicated_as_facts() {
    let original = source(1);
    let same = original.clone();
    let conflicting = entry(
        1,
        Entity::Record(record("D1", RecordKind::Decision, "Different row")),
    );
    for duplicate in [same, conflicting] {
        for entries in [
            vec![original.clone(), duplicate.clone()],
            vec![duplicate, original.clone()],
        ] {
            assert!(matches!(
                plan(&entries, &policy()),
                Err(memento::compaction::Error::DuplicateSequence(1))
            ));
        }
    }
}

#[test]
fn exact_evidence_does_not_pin_same_id_in_another_source_or_project() -> TestResult {
    let original = record("E1", RecordKind::ToolResult, "Observed failure");
    let mut other_source = original.clone();
    other_source.source_id = "other".into();
    let mut other_project = original.clone();
    other_project.project_id = "other-project".into();
    let mut decision = record("D1", RecordKind::Decision, "Chosen after the local failure");
    decision.evidence = vec![evidence(&original)];
    let entries = vec![
        source(1),
        entry(2, Entity::Record(original)),
        entry(3, Entity::Record(other_source)),
        entry(4, Entity::Record(other_project)),
        entry(5, Entity::Record(decision)),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([1, 2, 5]));
    Ok(())
}

#[test]
fn structural_metadata_and_related_to_do_not_pull_all_work_records_back_in() -> TestResult {
    let mut attempt = record("A1", RecordKind::Attempt, "Old successful retry");
    attempt.attempt_outcome = Some(AttemptOutcome::Succeeded);
    attempt.work_ids = vec!["W1".into()];
    let mut attempt_of = relation("L1", "A1", "unused", RelationKind::AttemptOf);
    attempt_of.to = Target::Work { id: "W1".into() };
    let mut output = record("E1", RecordKind::ToolResult, "A long unrelated trace");
    output.session_id = Some("active".into());
    let entries = vec![
        source(1),
        entry(2, Entity::Work(work())),
        entry(3, Entity::Session(session("active", SessionStatus::Active))),
        entry(4, Entity::Session(session("ended", SessionStatus::Ended))),
        entry(5, Entity::Record(attempt)),
        entry(6, Entity::Relation(attempt_of)),
        entry(7, Entity::Record(output)),
        entry(
            8,
            Entity::Record(record(
                "D1",
                RecordKind::Decision,
                "Keep the endpoint limit",
            )),
        ),
        entry(
            9,
            Entity::Relation(relation("L2", "D1", "E1", RelationKind::RelatedTo)),
        ),
        source(10),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([2, 3, 8, 10]));
    Ok(())
}

#[test]
fn hard_caps_report_failure_without_discarding_protected_evidence() -> TestResult {
    let entries = vec![
        source(1),
        entry(
            2,
            Entity::Record(record("D1", RecordKind::Decision, "Accepted design")),
        ),
        entry(
            3,
            Entity::Record(record(
                "D2",
                RecordKind::Decision,
                &"Original rationale\n".repeat(1000),
            )),
        ),
    ];
    let mut bounded = policy();
    bounded.max_entries = 2;
    let result = plan(&entries, &bounded)?;
    assert!(!result.report.fits);
    assert_eq!(result.retained_sequences, BTreeSet::from([1, 2, 3]));
    assert_eq!(result.report.protected.entries, 3);
    bounded.max_entries = 3;
    bounded.max_payload_bytes = 1024;
    let result = plan(&entries, &bounded)?;
    assert!(!result.report.fits);
    assert!(result.report.after.payload_bytes > bounded.max_payload_bytes);
    assert_eq!(result.report.removed_entries, 0);
    Ok(())
}

#[test]
fn revoked_source_and_deleted_record_tombstones_survive_all_windows() -> TestResult {
    let mut revoked = ingest::source("journal", "p", SourceKind::Journal);
    revoked.authorized = false;
    let mut deleted = record("E1", RecordKind::ToolResult, "");
    deleted.availability = Availability::Deleted;
    let mut missing = record("E2", RecordKind::ToolResult, "");
    missing.availability = Availability::Missing;
    let entries = vec![
        source(1),
        entry(2, Entity::Record(record("E1", RecordKind::ToolResult, ""))),
        entry(3, Entity::Record(deleted)),
        entry(4, Entity::Record(missing)),
        entry(5, Entity::Source(revoked)),
        entry(
            6,
            Entity::Source(ingest::source("other", "p", SourceKind::Journal)),
        ),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([3, 4, 5, 6]));
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 3 && reason.rule == "removal_tombstone")
    );
    Ok(())
}

#[test]
fn retained_change_preserves_exact_code_and_commit_references() -> TestResult {
    let commit = Commit {
        id: "commit1".into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        sha: "abc123".into(),
        tree: "tree1".into(),
        parents: Vec::new(),
        paths: vec!["src/retry.rs".into()],
        message: "Bound upload concurrency".into(),
        occurred_at: None,
        origin_worktree: None,
        observed_worktree: None,
    };
    let state = CodeState {
        id: "state1".into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        worktree_id: None,
        commit_sha: Some("abc123".into()),
        observed_at: "2026-09-29T01:00:00Z".into(),
        changed_during_observation: false,
        files: vec![FileState {
            path: "src/retry.rs".into(),
            working_content: Some("const UPLOAD_CONCURRENCY: usize = 2;".into()),
            ..FileState::default()
        }],
    };
    let mut decision = record("D1", RecordKind::Decision, "Bound uploads to two workers");
    decision.code_refs = vec![CodeRef {
        state_id: "state1".into(),
        path: "src/retry.rs".into(),
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
    }];
    let entries = vec![
        source(1),
        entry(2, Entity::Commit(commit)),
        entry(3, Entity::CodeState(state)),
        entry(4, Entity::Record(decision)),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([1, 2, 3, 4]));
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 3 && reason.rule == "code_reference")
    );
    assert!(
        result
            .report
            .reasons
            .iter()
            .any(|reason| reason.sequence == 2 && reason.rule == "code_state_commit")
    );
    Ok(())
}

#[test]
fn a_code_relation_keeps_the_observation_even_when_the_path_was_not_captured() -> TestResult {
    let decision = record(
        "D1",
        RecordKind::Decision,
        "The historical path was not observed",
    );
    let state = CodeState {
        id: "state1".into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        worktree_id: None,
        commit_sha: None,
        observed_at: "2026-09-29T01:00:00Z".into(),
        changed_during_observation: false,
        files: Vec::new(),
    };
    let mut link = relation("L1", "D1", "unused", RelationKind::Changes);
    link.to = Target::Code {
        state_id: "state1".into(),
        path: "src/uncaptured.rs".into(),
        range: None,
    };
    let entries = vec![
        source(1),
        entry(2, Entity::Record(decision)),
        entry(3, Entity::CodeState(state)),
        entry(4, Entity::Relation(link)),
        source(5),
    ];
    let result = plan(&entries, &policy())?;
    assert_eq!(result.retained_sequences, BTreeSet::from([2, 3, 4, 5]));
    Ok(())
}

#[test]
fn policy_and_explanations_are_bounded_and_the_frontier_is_retained() -> TestResult {
    assert!(
        Policy {
            max_entries: 0,
            ..policy()
        }
        .validate()
        .is_err()
    );
    assert!(
        Policy {
            max_payload_bytes: 0,
            ..policy()
        }
        .validate()
        .is_err()
    );
    assert!(
        Policy {
            max_entries: 1,
            recent_entries: 2,
            ..policy()
        }
        .validate()
        .is_err()
    );
    assert!(
        serde_json::from_str::<Policy>(
            r#"{"max_entries":10,"max_payload_bytes":1000,"recent_entries":0,"unknown":true}"#
        )
        .is_err()
    );
    let mut entries = vec![source(1)];
    for sequence in 2..=251 {
        entries.push(entry(
            sequence,
            Entity::Record(record(
                &format!("D{sequence}"),
                RecordKind::Decision,
                "Accepted decision",
            )),
        ));
    }
    for sequence in 252..=502 {
        entries.push(entry(
            sequence,
            Entity::Record(record(
                &format!("E{sequence}"),
                RecordKind::ToolResult,
                "Old verbose output",
            )),
        ));
    }
    let result = plan(&entries, &policy())?;
    assert!(result.retained_sequences.contains(&502));
    assert!(!result.retained_sequences.contains(&501));
    assert_eq!(result.report.reasons.len(), 200);
    assert_eq!(result.report.omitted_reasons, 52);
    assert_eq!(result.report.removals.len(), 200);
    assert_eq!(result.report.omitted_removals, 50);
    let mut recent = policy();
    recent.recent_entries = 3;
    let result = plan(&entries, &recent)?;
    assert!(
        result
            .retained_sequences
            .is_superset(&BTreeSet::from([500, 501, 502]))
    );
    Ok(())
}
