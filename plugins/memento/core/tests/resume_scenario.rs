use memento::{Store, ingest, model::*, query, security::RedactionPolicy};
use std::error::Error;

#[path = "support/evaluation.rs"]
mod evaluation;

#[tokio::test]
async fn sc01_resume_recovers_abandoned_attempt_and_degrades_only_missing_original()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "resume",
            SourceKind::Journal,
        )))
        .await?;
    let fixtures = [
        (
            "U1",
            RecordKind::Request,
            "Reduce HTTP 429 without slowing normal requests",
            Nature::Reported,
        ),
        (
            "A1",
            RecordKind::Attempt,
            "Try fixed 500ms delay",
            Nature::Reported,
        ),
        (
            "E1",
            RecordKind::ToolResult,
            "Normal request p95 rose from 100ms to 600ms",
            Nature::Observed,
        ),
        (
            "D1",
            RecordKind::Decision,
            "Reject fixed delay due to normal latency; use backoff only after errors",
            Nature::Reported,
        ),
        (
            "E2",
            RecordKind::Verification,
            "Backoff normal-request tests passed in local debug profile; integration still unverified",
            Nature::Observed,
        ),
        (
            "N1",
            RecordKind::Status,
            "Next: handle Retry-After and verify timeout boundary",
            Nature::Reported,
        ),
    ];
    let mut original = None;
    for (index, (id, kind, body, nature)) in fixtures.into_iter().enumerate() {
        let mut record = Record::new(id, "resume", "journal", kind, body);
        record.nature = nature;
        record.work_ids = vec!["W1".into()];
        record.association = Association::Explicit;
        record.session_id = Some(if index < 4 { "S1" } else { "S2" }.into());
        record.source_order = Some(u64::try_from(index)?);
        if id == "A1" {
            record.attempt_outcome = Some(AttemptOutcome::Abandoned);
        }
        if id == "D1" {
            record.decision_status = Some(DecisionStatus::Accepted);
            record.alternatives = vec!["fixed delay".into()];
        }
        if id == "E2" {
            record.verification_outcome = Some(VerificationOutcome::Passed);
            record.applies_to = vec!["local debug normal requests".into()];
        }
        if id == "E1" {
            original = Some(record.clone());
        }
        store.append(Entity::Record(record)).await?;
    }
    let original = original.ok_or("missing test fixture")?;
    for (id, from, to, kind) in [
        ("failure", "E1", "A1", RelationKind::RespondsTo),
        ("reason", "E1", "D1", RelationKind::Supports),
        ("next", "D1", "N1", RelationKind::RelatedTo),
    ] {
        store
            .append(Entity::Relation(Relation {
                id: id.into(),
                project_id: "resume".into(),
                source_id: "journal".into(),
                from: Target::Record { id: from.into() },
                to: Target::Record { id: to.into() },
                kind,
                nature: Nature::Reported,
                evidence: vec![Evidence {
                    source_id: "journal".into(),
                    record_id: Some("E1".into()),
                    revision: original.revision.clone(),
                    locator: "record:E1".into(),
                    availability: Availability::Available,
                    range: None,
                    purpose: memento::model::EvidencePurpose::Unspecified,
                    span: None,
                }],
                applies_to: Vec::new(),
            }))
            .await?;
    }
    let mut query = Query::new(Operation::Brief, "resume");
    query.scope.work_ids = vec!["W1".into()];
    query.purpose = Some(BriefPurpose::Resume);
    let corpus = store.load().await?;
    evaluation::capture(
        "SC01",
        "A",
        "어제 왜 고정 대기를 포기했고, 지금 어떤 제약 아래 어디서 이어가야 하나? 실제로 확인된 검증 범위도 알려줘.",
        &corpus,
        &[query.clone()],
    )?;
    let full = query::execute(&corpus, &query)?;
    let claims = full
        .brief
        .ok_or("missing brief")?
        .sections
        .into_iter()
        .flat_map(|s| s.claims)
        .collect::<Vec<_>>();
    for id in ["U1", "A1", "E1", "D1", "E2", "N1"] {
        assert!(claims.iter().any(|r| r.record_id == id));
    }
    assert!(
        claims
            .iter()
            .any(|r| r.record_id == "A1" && r.attempt_outcome == Some(AttemptOutcome::Abandoned))
    );
    assert!(
        claims
            .iter()
            .any(|r| r.record_id == "E2" && r.applies_to == ["local debug normal requests"])
    );
    let mut missing = original;
    missing.availability = Availability::Missing;
    store.append(Entity::Record(missing)).await?;
    let corpus = store.load().await?;
    evaluation::capture(
        "SC01",
        "B",
        "어제 왜 고정 대기를 포기했고, 지금 어떤 제약 아래 어디서 이어가야 하나? 실제로 확인된 검증 범위도 알려줘.",
        &corpus,
        &[query.clone()],
    )?;
    let partial = query::execute(&corpus, &query)?;
    let text = serde_json::to_string(&partial)?;
    assert!(!text.contains("600ms"));
    let claims = partial
        .brief
        .ok_or("missing partial brief")?
        .sections
        .into_iter()
        .flat_map(|s| s.claims)
        .collect::<Vec<_>>();
    for id in ["U1", "A1", "D1", "E2", "N1"] {
        assert!(
            claims
                .iter()
                .any(|r| r.record_id == id && !r.text.is_empty())
        );
    }
    assert!(claims.iter().any(|r| r.record_id == "D1"
        && r.nature == Nature::Reported
        && r.text.contains("normal latency")));
    assert_eq!(partial.status, query::ResponseStatus::Partial);
    Ok(())
}
