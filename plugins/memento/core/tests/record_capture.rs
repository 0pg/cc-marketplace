use memento::{
    Store, capture, ingest,
    model::*,
    security::RedactionPolicy,
    store::{RecordSelection, ResolveRecords},
};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn fixture() -> TestResult<(tempfile::TempDir, Store, capture::Scope, capture::Reply)> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "p",
            SourceKind::Journal,
        )))
        .await?;
    let scope = capture::Scope {
        project_id: "p".into(),
        repository: directory
            .path()
            .canonicalize()?
            .to_string_lossy()
            .into_owned(),
        work_id: "w".into(),
        session_id: "s".into(),
        turn_id: "t".into(),
    };
    let reply = store
        .checkpoint(capture::Request::Open {
            scope: scope.clone(),
            event_id: "request".into(),
            kind: capture::EventKind::UserPrompt,
            detail: "Preserve the GC collection condition.".into(),
            commit_binding: None,
        })
        .await?;
    Ok((directory, store, scope, reply))
}

fn batch(
    scope: &capture::Scope,
    reply: &capture::Reply,
) -> TestResult<(Vec<Entity>, ResolveRecords)> {
    let event = reply
        .events
        .iter()
        .find(|event| event.event_id == "request")
        .ok_or_else(|| std::io::Error::other("missing request"))?;
    let origin = event
        .origin
        .as_ref()
        .ok_or_else(|| std::io::Error::other("missing origin"))?;
    let mut claim = Record::new(
        "claim",
        "p",
        "journal",
        RecordKind::Request,
        "Preserve the GC collection condition.",
    );
    claim.representation = Representation::Claim;
    claim.derived = true;
    claim.fidelity = Fidelity::SummaryOnly;
    claim.context_id.clone_from(&event.context_id);
    claim.work_ids = vec![scope.work_id.clone()];
    claim.session_id = Some(scope.session_id.clone());
    claim.association = Association::Explicit;
    claim.evidence = serde_json::from_value(json!([{
        "source_id": origin.source_id, "record_id": origin.record_id,
        "revision": origin.revision, "locator": "request",
        "availability": "available", "purpose": "origin",
        "range": {"start_line": 1, "end_line": 1}
    }]))?;
    Ok((
        vec![Entity::Record(claim)],
        ResolveRecords {
            scope: scope.clone(),
            event_id: "request".into(),
            records: vec![RecordSelection {
                source_id: "journal".into(),
                record_id: "claim".into(),
            }],
        },
    ))
}

async fn pending(store: &mut Store, scope: &capture::Scope) -> TestResult<Vec<String>> {
    Ok(store
        .checkpoint(capture::Request::Status {
            scope: scope.clone(),
        })
        .await?
        .pending_event_ids)
}

#[tokio::test]
async fn verified_record_and_resolution_commit_once_and_retry_is_idempotent() -> TestResult {
    let (_directory, mut store, scope, reply) = fixture().await?;
    let (entities, resolve) = batch(&scope, &reply)?;
    let saved = store.append_and_resolve(entities, resolve).await?;
    assert_eq!(
        saved.records.first().map(|record| record.body.as_str()),
        Some("Preserve the GC collection condition.")
    );
    assert!(saved.receipts.iter().all(|receipt| receipt.durable));
    assert_eq!(
        saved.checkpoint.as_ref().map(|reply| reply.decision),
        Some(capture::Decision::Allow)
    );
    let sequence = saved.receipts.first().map(|receipt| receipt.sequence);
    let before = store.load().await?.entries.len();
    let (entities, resolve) = batch(&scope, &reply)?;
    let repeated = store.append_and_resolve(entities, resolve).await?;
    assert!(repeated.receipts.iter().all(|receipt| receipt.duplicate));
    assert_eq!(
        repeated.receipts.first().map(|receipt| receipt.sequence),
        sequence
    );
    assert_eq!(store.load().await?.entries.len(), before);
    assert!(pending(&mut store, &scope).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn evidence_validation_failure_keeps_checkpoint_and_writes_nothing() -> TestResult {
    let (_directory, mut store, scope, reply) = fixture().await?;
    let (mut entities, resolve) = batch(&scope, &reply)?;
    if let Some(Entity::Record(claim)) = entities.first_mut()
        && let Some(evidence) = claim.evidence.first_mut()
    {
        evidence.revision = "unretained".into();
    }
    let before = store.load().await?.entries.len();
    assert!(store.append_and_resolve(entities, resolve).await.is_err());
    assert_eq!(store.load().await?.entries.len(), before);
    assert_eq!(pending(&mut store, &scope).await?, vec!["request"]);
    Ok(())
}

#[tokio::test]
async fn resolution_failure_rolls_back_otherwise_valid_records() -> TestResult {
    let (_directory, mut store, scope, reply) = fixture().await?;
    let (entities, mut resolve) = batch(&scope, &reply)?;
    resolve.event_id = "another-event".into();
    let before = store.load().await?.entries.len();
    assert!(store.append_and_resolve(entities, resolve).await.is_err());
    assert_eq!(store.load().await?.entries.len(), before);
    assert_eq!(pending(&mut store, &scope).await?, vec!["request"]);
    Ok(())
}

#[tokio::test]
async fn another_work_or_unselected_record_cannot_release_request() -> TestResult {
    let (_directory, mut store, scope, reply) = fixture().await?;
    let (entities, mut resolve) = batch(&scope, &reply)?;
    resolve.scope.work_id = "other-work".into();
    assert!(store.append_and_resolve(entities, resolve).await.is_err());
    let (entities, mut resolve) = batch(&scope, &reply)?;
    resolve
        .records
        .first_mut()
        .ok_or_else(|| std::io::Error::other("selection"))?
        .record_id = "absent".into();
    assert!(store.append_and_resolve(entities, resolve).await.is_err());
    assert_eq!(pending(&mut store, &scope).await?, vec!["request"]);
    assert!(
        !store
            .latest()
            .await?
            .iter()
            .any(|entity| entity.id() == "claim")
    );
    Ok(())
}

#[tokio::test]
async fn focus_is_human_scoped_bounded_and_absent_for_recovery() -> TestResult {
    let (_directory, mut store, scope, reply) = fixture().await?;
    assert_eq!(
        reply.request_focus.as_deref(),
        Some("Preserve the GC collection condition.")
    );
    let other = store
        .checkpoint(capture::Request::Status {
            scope: capture::Scope {
                turn_id: "other".into(),
                ..scope.clone()
            },
        })
        .await?;
    assert!(other.request_focus.is_none());
    let recovered = store
        .checkpoint(capture::Request::RecoverRequest {
            scope: capture::Scope {
                turn_id: "recovered".into(),
                ..scope.clone()
            },
            event_id: "recovery".into(),
            detail: "Agent recovery notice".into(),
        })
        .await?;
    assert!(recovered.request_focus.is_none());
    let large = store
        .checkpoint(capture::Request::Open {
            scope: capture::Scope {
                turn_id: "large".into(),
                ..scope
            },
            event_id: "large".into(),
            kind: capture::EventKind::UserPrompt,
            detail: "질문".repeat(400),
            commit_binding: None,
        })
        .await?
        .summarize();
    assert!(
        large
            .request_focus
            .as_ref()
            .is_some_and(|focus| focus.len() <= 1024)
    );
    assert!(large.request_focus_truncated);
    Ok(())
}
