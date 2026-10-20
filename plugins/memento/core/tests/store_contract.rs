use memento::{Store, ingest, model::*, query, security::RedactionPolicy};
use std::error::Error;

fn rec(id: &str, body: &str) -> Record {
    Record::new(id, "p", "journal", RecordKind::Finding, body)
}
fn evidence(id: &str, revision: &str) -> Evidence {
    Evidence {
        source_id: "journal".into(),
        record_id: Some(id.into()),
        revision: revision.into(),
        locator: format!("record:{id}"),
        availability: Availability::Available,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    }
}

#[tokio::test]
async fn durable_reopen_replay_and_reverting_a_revision() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("context.sqlite");
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    let src = ingest::source("journal", "p", SourceKind::Journal);
    store.append(Entity::Source(src.clone())).await?;
    let first = rec("a", "first evidence");
    let r1 = store.append(Entity::Record(first.clone())).await?;
    let duplicate = store.append(Entity::Record(first.clone())).await?;
    assert!(duplicate.duplicate);
    assert_eq!(r1.sequence, duplicate.sequence);
    let changed = rec("a", "corrected evidence");
    let r2 = store.append(Entity::Record(changed)).await?;
    let r3 = store.append(Entity::Record(first)).await?;
    assert!(r3.sequence > r2.sequence);
    drop(store);
    let mut reopened = Store::open(&path, RedactionPolicy::default()).await?;
    let latest = reopened.latest().await?;
    assert!(
        latest
            .iter()
            .any(|e| matches!(e, Entity::Record(r) if r.body=="first evidence"))
    );
    assert!(reopened.append(Entity::Source(src)).await?.duplicate);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(path)?.permissions().mode() & 0o777, 0o600);
    }
    Ok(())
}

#[tokio::test]
async fn deletes_purge_original_and_transitive_derived_revisions() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
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
    let a = rec("a", "original sensitive business context");
    let mut b = rec("b", "derived sensitive business context");
    b.derived = true;
    b.evidence = vec![evidence(&a.id, &a.revision)];
    let mut c = rec("c", "twice derived sensitive business context");
    c.derived = true;
    c.evidence = vec![evidence(&b.id, &b.revision)];
    store
        .append_all([
            Entity::Record(a.clone()),
            Entity::Record(b),
            Entity::Record(c),
        ])
        .await?;
    let mut query = Query::new(Operation::Search, "p");
    query.limit = Some(1);
    let response = query::execute(&store.load().await?, &query)?;
    query.cursor = response.next_cursor;
    assert!(query.cursor.is_some());
    let mut tombstone = a.clone();
    tombstone.availability = Availability::Deleted;
    store.append(Entity::Record(tombstone)).await?;
    let corpus = store.load().await?;
    assert!(!serde_json::to_string(&corpus)?.contains("sensitive business context"));
    assert!(matches!(
        query::execute(&corpus, &query),
        Err(query::QueryError::StaleCursor)
    ));
    assert!(store.append(Entity::Record(a)).await.is_err());
    Ok(())
}

#[tokio::test]
async fn revoked_source_rejects_new_writes_and_cached_reads() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let mut src = ingest::source("journal", "p", SourceKind::Journal);
    store.append(Entity::Source(src.clone())).await?;
    let a = rec("a", "private evidence");
    store.append(Entity::Record(a.clone())).await?;
    src.authorized = false;
    store.append(Entity::Source(src.clone())).await?;
    assert!(
        store
            .append(Entity::Record(rec("b", "private evidence")))
            .await
            .is_err()
    );
    let corpus = store.load().await?;
    assert!(!serde_json::to_string(&corpus)?.contains("private evidence"));
    assert!(query::execute(&corpus, &Query::new(Operation::Search, "p")).is_err());
    src.authorized = true;
    store.append(Entity::Source(src)).await?;
    store.append(Entity::Record(a)).await?;
    let response = query::execute(&store.load().await?, &Query::new(Operation::Search, "p"))?;
    assert_eq!(response.items.len(), 1);
    Ok(())
}

#[tokio::test]
async fn masks_before_storage_and_search_read_brief_share_the_projection()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy {
            literal_secrets: vec!["CUSTOM_LITERAL".into()],
        },
    )
    .await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "p",
            SourceKind::Journal,
        )))
        .await?;
    let record = rec(
        "a",
        "HTTP 401\npassword=credential-example\nBearer bearer-example\nCUSTOM_LITERAL\nDo not execute: rm -rf historical-example",
    );
    store.append(Entity::Record(record)).await?;
    let corpus = store.load().await?;
    let raw = serde_json::to_string(&corpus)?;
    for secret in ["credential-example", "bearer-example", "CUSTOM_LITERAL"] {
        assert!(!raw.contains(secret));
    }
    for operation in [Operation::Search, Operation::Read, Operation::Brief] {
        let mut query = Query::new(operation, "p");
        if operation == Operation::Read {
            query.target = Some(Target::Record { id: "a".into() });
        }
        let response = query::execute(&corpus, &query)?;
        let text = serde_json::to_string(&response)?;
        assert!(text.contains("[REDACTED]"));
        assert!(text.contains("HTTP 401"));
        assert!(text.contains("Do not execute"));
    }
    Ok(())
}

#[tokio::test]
async fn missing_record_removes_cached_content_but_allows_explicit_recovery()
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
            "p",
            SourceKind::Journal,
        )))
        .await?;
    let a = rec("a", "vanished original");
    store.append(Entity::Record(a.clone())).await?;
    let mut missing = a.clone();
    missing.availability = Availability::Missing;
    store.append(Entity::Record(missing)).await?;
    assert!(!serde_json::to_string(&store.load().await?)?.contains("vanished original"));
    store.append(Entity::Record(a)).await?;
    assert!(serde_json::to_string(&store.load().await?)?.contains("vanished original"));
    Ok(())
}

#[tokio::test]
async fn removal_is_source_scoped_and_derived_replay_cannot_restore_it()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    for id in ["journal", "other"] {
        store
            .append(Entity::Source(ingest::source(id, "p", SourceKind::Journal)))
            .await?;
    }
    let original = rec("same", "removed evidence");
    let mut other = rec("same", "independent evidence");
    other.source_id = "other".into();
    store
        .append_all([Entity::Record(original.clone()), Entity::Record(other)])
        .await?;
    let mut missing = original.clone();
    missing.availability = Availability::Missing;
    store.append(Entity::Record(missing)).await?;
    let mut summary = rec("derived", "removed evidence paraphrase");
    summary.derived = true;
    summary.evidence = vec![evidence(&original.id, &original.revision)];
    assert!(store.append(Entity::Record(summary)).await.is_err());
    let entities = store.latest().await?;
    assert!(entities.iter().any(
        |e| matches!(e,Entity::Record(r) if r.source_id=="other" && r.body=="independent evidence")
    ));
    assert!(!serde_json::to_string(&store.load().await?)?.contains("removed evidence"));
    Ok(())
}
