use std::{collections::BTreeSet, error::Error};

use memento::{
    compaction::{self, Policy},
    ingest,
    model::*,
    query::{self, QueryError},
};

type TestResult = Result<(), Box<dyn Error>>;

fn entry(sequence: u64, entity: Entity) -> Entry {
    Entry {
        sequence,
        captured_at: "2026-10-04T00:00:00Z".into(),
        entity,
    }
}

fn source(project: &str) -> Source {
    ingest::source("journal", project, SourceKind::Journal)
}

fn record(id: &str, kind: RecordKind, representation: Representation) -> Record {
    let mut record = Record::new(id, "p", "journal", kind, "처리량을 유지한다.");
    record.revision = "v1".into();
    record.representation = representation;
    record.context_id = Some("correction-1".into());
    record
}

fn evidence(record: &Record, purpose: EvidencePurpose) -> Evidence {
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
        purpose,
        span: None,
    }
}

fn policy() -> Policy {
    Policy {
        recent_entries: 0,
        ..Policy::default()
    }
}

#[test]
fn legacy_serialization_preserves_payload_and_auto_revision_input() -> TestResult {
    // Field order and omissions are the format-1 serialization contract used by
    // append_one's byte comparison and note's revision hash input.
    let original = concat!(
        r#"{"id":"old","project_id":"p","source_id":"journal","revision":"","kind":"constraint","nature":"reported","fidelity":"original","availability":"available","title":"","body":"처리량 유지","occurred_at":null,"source_order":null,"actor":null,"work_ids":[],"association":"unassigned","session_id":null,"worktree_id":null,"paths":[],"code_refs":[],"commit_shas":[],"evidence":["#,
        r#"{"source_id":"journal","record_id":"raw","revision":"v1","locator":"message:raw","availability":"available","range":null}"#,
        r#"],"decision_status":null,"attempt_outcome":null,"verification_outcome":null,"attempt_id":null,"execution":null,"applies_to":[],"alternatives":[],"derived":false,"partial":false}"#
    );
    let restored: Record = serde_json::from_str(original)?;
    assert_eq!(restored.representation, Representation::Legacy);
    assert_eq!(restored.context_id, None);
    let first = restored.evidence.first().ok_or("missing evidence")?;
    assert_eq!(first.purpose, EvidencePurpose::Unspecified);
    assert_eq!(first.span, None);
    assert_eq!(serde_json::to_string(&restored)?, original);
    assert_eq!(
        memento::security::hash(&serde_json::to_vec(&restored)?),
        memento::security::hash(original.as_bytes())
    );
    let filters: Filters = serde_json::from_value(serde_json::json!({}))?;
    let value = serde_json::to_value(filters)?;
    assert!(value.get("context_id").is_none());
    assert!(value.get("representations").is_none());
    Ok(())
}

#[test]
fn new_fields_round_trip_and_unknown_vocabulary_is_rejected() -> TestResult {
    let original = record("raw", RecordKind::Feedback, Representation::Evidence);
    let mut claim = record("c1", RecordKind::Constraint, Representation::Claim);
    let mut origin = evidence(&original, EvidencePurpose::Origin);
    origin.span = Some(ByteSpan { start: 0, end: 9 });
    claim.evidence.push(origin);
    claim.derived = true;
    let value = serde_json::to_value(&claim)?;
    assert_eq!(serde_json::from_value::<Record>(value.clone())?, claim);
    assert_eq!(
        value.get("representation"),
        Some(&serde_json::json!("claim"))
    );
    assert_eq!(
        value.pointer("/evidence/0/purpose"),
        Some(&serde_json::json!("origin"))
    );
    let mut invalid = value;
    invalid
        .as_object_mut()
        .ok_or("record serialization is not an object")?
        .insert("representation".into(), serde_json::json!("summary"));
    assert!(serde_json::from_value::<Record>(invalid).is_err());
    assert!(serde_json::from_str::<EvidencePurpose>("\"causes\"").is_err());
    Ok(())
}

#[test]
fn shared_origin_and_context_do_not_retain_sibling_claims() -> TestResult {
    let raw = record("raw", RecordKind::Feedback, Representation::Evidence);
    let support = record("support", RecordKind::ToolResult, Representation::Evidence);
    let mut keep = record("keep", RecordKind::Constraint, Representation::Claim);
    keep.evidence = vec![
        evidence(&raw, EvidencePurpose::Origin),
        evidence(&support, EvidencePurpose::Support),
    ];
    keep.derived = true;
    let mut sibling = record("sibling", RecordKind::Finding, Representation::Claim);
    sibling.evidence = vec![evidence(&raw, EvidencePurpose::Origin)];
    sibling.derived = true;
    // The frontier is source metadata, recent protection is disabled, and there
    // are no pins, native executions or strong relations in this scenario.
    let entries = vec![
        entry(1, Entity::Source(source("p"))),
        entry(2, Entity::Record(raw)),
        entry(3, Entity::Record(keep)),
        entry(4, Entity::Record(sibling)),
        entry(5, Entity::Record(support)),
        entry(6, Entity::Source(source("p"))),
    ];
    let plan = compaction::plan(&entries, &policy())?;
    assert_eq!(plan.retained_sequences, BTreeSet::from([2, 3, 5, 6]));
    assert!(
        plan.report
            .reasons
            .iter()
            .any(|reason| { reason.sequence == 2 && reason.rule == "origin_evidence_revision" })
    );
    assert!(
        plan.report
            .reasons
            .iter()
            .any(|reason| { reason.sequence == 5 && reason.rule == "support_evidence_revision" })
    );
    assert_eq!(plan.report.after.entries, 4);
    assert_eq!(compaction::plan(&entries, &policy())?.report, plan.report);
    let mut reordered = entries.clone();
    reordered.reverse();
    assert_eq!(compaction::plan(&reordered, &policy())?.report, plan.report);
    for entry in &mut reordered {
        if let Entity::Record(record) = &mut entry.entity {
            let repeated = record.evidence.clone();
            record.evidence.extend(repeated);
            record.evidence.reverse();
        }
    }
    let repeated = compaction::plan(&reordered, &policy())?;
    assert_eq!(repeated.retained_sequences, plan.retained_sequences);
    assert_eq!(repeated.report.reasons, plan.report.reasons);
    Ok(())
}

#[test]
fn source_evidence_kinds_do_not_create_durable_roots_but_legacy_still_does() -> TestResult {
    let kinds = [
        RecordKind::Request,
        RecordKind::Constraint,
        RecordKind::Decision,
        RecordKind::Feedback,
        RecordKind::Verification,
        RecordKind::Attempt,
    ];
    for kind in kinds {
        let entries = vec![
            entry(
                1,
                Entity::Record(record("raw", kind, Representation::Evidence)),
            ),
            entry(
                2,
                Entity::Record(record("old", kind, Representation::Legacy)),
            ),
            entry(3, Entity::Source(source("p"))),
        ];
        assert_eq!(
            compaction::plan(&entries, &policy())?.retained_sequences,
            BTreeSet::from([2, 3])
        );
    }
    Ok(())
}

#[test]
fn explicitly_recorded_strong_relations_preserve_the_existing_endpoint_policy() -> TestResult {
    let raw = record("raw", RecordKind::Feedback, Representation::Evidence);
    let mut keep = record("keep", RecordKind::Constraint, Representation::Claim);
    keep.evidence = vec![evidence(&raw, EvidencePurpose::Origin)];
    let sibling = record("sibling", RecordKind::Finding, Representation::Claim);
    let related = Relation {
        id: "explicit".into(),
        project_id: "p".into(),
        source_id: "journal".into(),
        from: Target::Record {
            id: "sibling".into(),
        },
        to: Target::Artifact {
            record_id: "raw".into(),
            revision: "v1".into(),
            range: None,
        },
        kind: RelationKind::RelatedTo,
        nature: Nature::Reported,
        evidence: Vec::new(),
        applies_to: Vec::new(),
    };
    let mut entries = vec![
        entry(1, Entity::Source(source("p"))),
        entry(2, Entity::Record(raw)),
        entry(3, Entity::Record(keep)),
        entry(4, Entity::Record(sibling)),
        entry(5, Entity::Relation(related.clone())),
        entry(6, Entity::Source(source("p"))),
    ];
    assert_eq!(
        compaction::plan(&entries, &policy())?.retained_sequences,
        BTreeSet::from([2, 3, 6])
    );
    let mut derived = related;
    derived.kind = RelationKind::DerivedFrom;
    let relation = entries.get_mut(4).ok_or("missing relation entry")?;
    relation.entity = Entity::Relation(derived);
    // This is an explicit old-style strong relation, not an automatically
    // generated origin link. Its existing endpoint closure remains intact.
    assert_eq!(
        compaction::plan(&entries, &policy())?.retained_sequences,
        BTreeSet::from([2, 3, 4, 5, 6])
    );
    Ok(())
}

#[test]
fn native_execution_group_still_retains_observed_evidence() -> TestResult {
    let execution = Execution {
        id: "run-1".into(),
        command: "cargo test".into(),
        tool_name: None,
        tool_input: None,
        cwd: None,
        started_at: None,
        ended_at: None,
        exit_code: Some(1),
        last_observed_state: "completed".into(),
        observed_at: None,
        liveness: Liveness::Stopped,
        before_state: None,
        after_state: None,
        scope: Vec::new(),
        environment: Environment::default(),
    };
    let mut raw = record("raw", RecordKind::ToolResult, Representation::Evidence);
    raw.execution = Some(execution.clone());
    let mut attempt = record("attempt", RecordKind::Attempt, Representation::Claim);
    attempt.execution = Some(execution);
    attempt.attempt_outcome = Some(AttemptOutcome::Failed);
    let entries = vec![
        entry(1, Entity::Record(raw)),
        entry(2, Entity::Record(attempt)),
        entry(3, Entity::Source(source("p"))),
    ];
    assert_eq!(
        compaction::plan(&entries, &policy())?.retained_sequences,
        BTreeSet::from([1, 2, 3])
    );
    Ok(())
}

#[test]
fn context_filters_are_project_scoped_and_support_all_representations() -> TestResult {
    let raw = record("raw", RecordKind::Feedback, Representation::Evidence);
    let claim = record("claim", RecordKind::Constraint, Representation::Claim);
    let mut other_context = record("unrelated", RecordKind::Constraint, Representation::Claim);
    other_context.context_id = Some("other".into());
    let mut other_project = claim.clone();
    other_project.project_id = "q".into();
    let data = Corpus {
        entries: vec![
            entry(1, Entity::Source(source("p"))),
            entry(2, Entity::Source(source("q"))),
            entry(3, Entity::Record(raw)),
            entry(4, Entity::Record(claim)),
            entry(5, Entity::Record(other_context)),
            entry(6, Entity::Record(other_project)),
        ],
        ..Corpus::default()
    };
    let mut query = Query::new(Operation::Search, "p");
    query.filters.context_id = Some("correction-1".into());
    let result = query::execute(&data, &query)?;
    let ids: BTreeSet<_> = result.items.iter().map(|item| item.entity.id()).collect();
    assert_eq!(ids, BTreeSet::from(["claim", "raw"]));
    query.filters.representations = vec![Representation::Evidence];
    let result = query::execute(&data, &query)?;
    assert_eq!(result.items.len(), 1);
    assert_eq!(
        result.items.first().ok_or("missing evidence")?.entity.id(),
        "raw"
    );
    query.filters.representations = vec![Representation::Claim];
    assert_eq!(query::execute(&data, &query)?.items.len(), 1);
    query.filters.context_id = Some("".into());
    assert!(matches!(
        query::execute(&data, &query),
        Err(QueryError::InvalidQuery(_))
    ));
    Ok(())
}

#[test]
fn brief_does_not_present_source_evidence_as_a_semantic_claim() -> TestResult {
    let mut raw = record("raw", RecordKind::Decision, Representation::Evidence);
    raw.decision_status = Some(DecisionStatus::Accepted);
    let mut claim = record("claim", RecordKind::Decision, Representation::Claim);
    claim.decision_status = Some(DecisionStatus::Accepted);
    claim.evidence = vec![evidence(&raw, EvidencePurpose::Origin)];
    claim.derived = true;
    let data = Corpus {
        entries: vec![
            entry(1, Entity::Source(source("p"))),
            entry(2, Entity::Record(raw)),
            entry(3, Entity::Record(claim)),
        ],
        ..Corpus::default()
    };
    let mut query = Query::new(Operation::Brief, "p");
    query.filters.context_id = Some("correction-1".into());
    let result = query::execute(&data, &query)?;
    assert_eq!(result.items.len(), 1);
    let brief = result.brief.ok_or("missing brief")?;
    let claims: Vec<_> = brief
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .collect();
    assert_eq!(claims.len(), 1);
    let first = claims.first().ok_or("missing claim")?;
    assert_eq!(first.record_id, "claim");
    assert_eq!(first.representation, Representation::Claim);
    assert_eq!(first.context_id.as_deref(), Some("correction-1"));
    assert_eq!(
        first.evidence.first().ok_or("missing origin")?.purpose,
        EvidencePurpose::Origin
    );
    query.operation = Operation::Search;
    query.filters.decision_statuses = vec![DecisionStatus::Accepted];
    assert_eq!(query::execute(&data, &query)?.items.len(), 1);
    Ok(())
}

#[test]
fn observed_atomic_claim_keeps_native_provenance_across_query_projections() -> TestResult {
    let mut raw = record(
        "native-output",
        RecordKind::ToolResult,
        Representation::Evidence,
    );
    raw.source_id = "native".into();
    raw.body = "test smoke::health ... ok\n".into();
    raw.revision = memento::security::hash(raw.body.as_bytes());
    let mut origin = evidence(&raw, EvidencePurpose::Origin);
    origin.locator = "tool-output:smoke:stdout".into();
    origin.span = Some(ByteSpan {
        start: 0,
        end: raw.body.len(),
    });
    let mut claim = record("claim", RecordKind::Finding, Representation::Claim);
    claim.body = "The smoke::health test passed.".into();
    claim.nature = Nature::Observed;
    claim.fidelity = Fidelity::SummaryOnly;
    claim.derived = true;
    claim.evidence = vec![origin.clone()];
    let mut legacy = claim.clone();
    legacy.id = "legacy-summary".into();
    legacy.representation = Representation::Legacy;
    let mut evidence_summary = legacy.clone();
    evidence_summary.id = "evidence-summary".into();
    evidence_summary.kind = RecordKind::ToolResult;
    evidence_summary.representation = Representation::Evidence;
    let data = Corpus {
        entries: vec![
            entry(1, Entity::Source(source("p"))),
            entry(
                2,
                Entity::Source(ingest::source("native", "p", SourceKind::Codex)),
            ),
            entry(3, Entity::Record(raw.clone())),
            entry(4, Entity::Record(claim.clone())),
            entry(5, Entity::Record(legacy.clone())),
            entry(6, Entity::Record(evidence_summary.clone())),
        ],
        ..Corpus::default()
    };
    let persisted = serde_json::to_vec(&data)?;

    for (stored, expected_nature) in [
        (&claim, Nature::Observed),
        (&legacy, Nature::Reported),
        (&evidence_summary, Nature::Reported),
    ] {
        let mut read = Query::new(Operation::Read, "p");
        read.target = Some(Target::Artifact {
            record_id: stored.id.clone(),
            revision: stored.revision.clone(),
            range: None,
        });
        let result = query::execute(&data, &read)?;
        assert_eq!(result.items.len(), 1);
        let item = result.items.first().ok_or("missing summary item")?;
        let Entity::Record(projected) = &item.entity else {
            return Err("expected a summary record".into());
        };
        let mut expected = (*stored).clone();
        expected.nature = expected_nature;
        assert_eq!(projected, &expected);
        assert!(
            item.warnings
                .iter()
                .any(|warning| warning == "summary_only")
        );
    }

    let brief = query::execute(&data, &Query::new(Operation::Brief, "p"))?;
    assert_eq!(brief.items.len(), 2);
    let item = brief
        .items
        .iter()
        .find(|item| item.entity.id() == claim.id)
        .ok_or("missing atomic claim item")?;
    let Entity::Record(projected) = &item.entity else {
        return Err("expected the atomic claim record".into());
    };
    assert_eq!(projected, &claim);
    let brief_claims: Vec<_> = brief
        .brief
        .as_ref()
        .ok_or("missing brief")?
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .collect();
    assert_eq!(brief_claims.len(), 2);
    let projected = brief_claims
        .iter()
        .find(|item| item.record_id == claim.id)
        .ok_or("missing atomic BriefClaim")?;
    assert_eq!(projected.nature, Nature::Observed);
    assert_eq!(projected.fidelity, Fidelity::SummaryOnly);
    assert_eq!(projected.representation, Representation::Claim);
    assert_eq!(projected.context_id, claim.context_id);
    assert_eq!(projected.text, claim.body);
    assert_eq!(projected.evidence, vec![origin.clone()]);
    assert!(
        projected
            .warnings
            .iter()
            .any(|warning| warning == "summary_only")
    );
    assert!(
        projected
            .warnings
            .iter()
            .any(|warning| warning.contains("do not count as independent corroboration"))
    );
    assert!(brief_claims.iter().any(|item| item.record_id == legacy.id
        && item.nature == Nature::Reported
        && item.fidelity == Fidelity::SummaryOnly));

    let mut search = Query::new(Operation::Search, "p");
    search.filters.representations = vec![Representation::Claim];
    search.filters.natures = vec![Nature::Observed];
    let result = query::execute(&data, &search)?;
    assert_eq!(result.items.len(), 1);
    assert_eq!(
        result.items.first().ok_or("missing observed claim")?.entity,
        Entity::Record(claim)
    );
    search.filters.natures = vec![Nature::Reported];
    assert!(query::execute(&data, &search)?.items.is_empty());

    let mut read = Query::new(Operation::Read, "p");
    read.target = Some(Target::Artifact {
        record_id: origin.record_id.ok_or("missing origin identity")?,
        revision: origin.revision,
        range: None,
    });
    let result = query::execute(&data, &read)?;
    assert_eq!(result.items.len(), 1);
    let Entity::Record(original) = &result.items.first().ok_or("missing origin")?.entity else {
        return Err("expected native output evidence".into());
    };
    assert_eq!(original.body.as_bytes(), b"test smoke::health ... ok\n");
    assert_eq!(original.nature, Nature::Observed);
    assert_eq!(original.fidelity, Fidelity::Original);
    assert_eq!(original.representation, Representation::Evidence);
    assert!(!original.derived);
    assert_eq!(original.revision, raw.revision);
    assert_eq!(serde_json::to_vec(&data)?, persisted);
    Ok(())
}
