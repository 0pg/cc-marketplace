use std::{error::Error, fs};

use memento::{Store, adapters::ImportFormat, ingest, model::*, security::RedactionPolicy};

fn record<'a>(entities: &'a [Entity], id: &str) -> Result<&'a Record, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Record(record) if record.id == id => Some(record),
            _ => None,
        })
        .ok_or_else(|| format!("missing record {id}").into())
}

fn source<'a>(entities: &'a [Entity], id: &str) -> Result<&'a Source, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Source(source) if source.id == id => Some(source),
            _ => None,
        })
        .ok_or_else(|| format!("missing source {id}").into())
}

#[tokio::test]
async fn sync_preserves_explicit_bindings_and_identical_record_replays()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("journal.jsonl");
    fs::write(
        &input,
        "{\"id\":\"E1\",\"kind\":\"finding\",\"text\":\"evidence\",\"session_id\":\"S1\"}\n",
    )?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        Some("W1"),
    )
    .await?;
    let old = store.latest().await?;
    let mut session = old
        .iter()
        .find_map(|entity| match entity {
            Entity::Session(session) => Some(session.clone()),
            _ => None,
        })
        .ok_or("missing session")?;
    session.worktree_id = Some("tree-1".into());
    session.parent_id = Some("parent-1".into());
    store.append(Entity::Session(session)).await?;
    let receipts = ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    assert!(
        receipts
            .iter()
            .any(|receipt| receipt.entity_id == "s:E1" && receipt.duplicate)
    );
    let latest = store.latest().await?;
    assert_eq!(record(&latest, "s:E1")?.work_ids, vec!["W1"]);
    assert_eq!(record(&latest, "s:E1")?.association, Association::Explicit);
    let session = latest
        .iter()
        .find_map(|entity| match entity {
            Entity::Session(session) => Some(session),
            _ => None,
        })
        .ok_or("missing preserved session")?;
    assert_eq!(session.work_ids, vec!["W1"]);
    assert_eq!(session.worktree_id.as_deref(), Some("tree-1"));
    assert_eq!(session.parent_id.as_deref(), Some("parent-1"));
    assert_eq!(source(&latest, "s")?.last_event_id.as_deref(), Some("s:E1"));
    let record_versions = store
        .load()
        .await?
        .entries
        .into_iter()
        .filter(|entry| matches!(&entry.entity, Entity::Record(record) if record.id == "s:E1"))
        .count();
    assert_eq!(record_versions, 1);
    Ok(())
}

#[tokio::test]
async fn changed_and_absent_records_are_reconciled_without_inventing_deletion()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("journal.jsonl");
    let e1 = "{\"id\":\"E1\",\"kind\":\"finding\",\"text\":\"original\"}\n";
    let e2 = "{\"id\":\"E2\",\"kind\":\"finding\",\"text\":\"removed original\"}\n";
    fs::write(&input, format!("{e1}{e2}"))?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let original_revision = record(&store.latest().await?, "s:E1")?.revision.clone();
    fs::write(
        &input,
        "{\"id\":\"E1\",\"kind\":\"finding\",\"text\":\"corrected\"}\n",
    )?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    assert_ne!(record(&latest, "s:E1")?.revision, original_revision);
    assert_eq!(record(&latest, "s:E2")?.availability, Availability::Missing);
    assert!(record(&latest, "s:E2")?.body.is_empty());
    assert!(!serde_json::to_string(&store.load().await?)?.contains("removed original"));
    // A reappearing source record is not an explicit user deletion; restore it.
    fs::write(&input, format!("{e1}{e2}"))?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    assert_eq!(
        record(&store.latest().await?, "s:E2")?.body,
        "removed original"
    );
    Ok(())
}

#[tokio::test]
async fn malformed_source_does_not_delete_unparsed_records_or_claim_completeness()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("journal.jsonl");
    fs::write(
        &input,
        "{\"id\":\"E1\",\"kind\":\"finding\",\"text\":\"previous\"}\n",
    )?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    fs::write(&input, "{\"id\":\"E1\",\"kind\":")?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    assert_eq!(
        record(&latest, "s:E1")?.availability,
        Availability::Available
    );
    assert!(
        source(&latest, "s")?
            .gaps
            .iter()
            .any(|gap| gap.contains("incomplete_import"))
    );
    Ok(())
}

#[tokio::test]
async fn result_ids_survive_without_tool_metadata_and_link_only_matching_executions()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("journal.jsonl");
    fs::write(
        &input,
        concat!(
            "{\"id\":\"A1\",\"kind\":\"attempt\",\"text\":\"cargo test\",\"execution_id\":\"X1\"}\n",
            "{\"id\":\"A2\",\"kind\":\"attempt\",\"text\":\"cargo test\",\"execution_id\":\"X2\"}\n",
            "{\"id\":\"E2\",\"kind\":\"tool_result\",\"text\":\"passed\",\"execution_id\":\"X2\"}\n"
        ),
    )?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "s",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    assert_eq!(
        record(&latest, "s:E2")?
            .execution
            .as_ref()
            .map(|execution| execution.id.as_str()),
        Some("s:X2")
    );
    let links: Vec<_> = latest
        .iter()
        .filter_map(|entity| match entity {
            Entity::Relation(relation) => Some(relation),
            _ => None,
        })
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(
        links.first().map(|link| &link.to),
        Some(&Target::Record { id: "s:A2".into() })
    );
    Ok(())
}

#[tokio::test]
async fn documents_report_revision_changes_and_revocation_requires_explicit_import()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("design.md");
    fs::write(&input, "first design")?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "docs",
        &input,
        ImportFormat::Document,
        None,
    )
    .await?;
    fs::write(&input, "revised design")?;
    ingest::revalidate(&mut store).await?;
    let latest = store.latest().await?;
    assert!(
        source(&latest, "docs")?
            .gaps
            .iter()
            .any(|gap| gap.starts_with("source_changed:"))
    );
    assert_eq!(record(&latest, "docs:document")?.body, "first design");
    fs::remove_file(&input)?;
    ingest::revalidate(&mut store).await?;
    let latest = store.latest().await?;
    assert!(!source(&latest, "docs")?.authorized);
    assert!(!source(&latest, "docs")?.available);
    fs::write(&input, "revised design")?;
    ingest::revalidate(&mut store).await?;
    assert!(!source(&store.latest().await?, "docs")?.authorized);
    ingest::import(
        &mut store,
        "p",
        "docs",
        &input,
        ImportFormat::Document,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    assert!(source(&latest, "docs")?.authorized);
    assert_eq!(record(&latest, "docs:document")?.body, "revised design");
    assert_eq!(
        source(&latest, "docs")?.record_kinds,
        vec![RecordKind::Finding]
    );
    assert!(record(&latest, "docs:document")?.occurred_at.is_none());
    Ok(())
}

#[tokio::test]
async fn summary_references_resolve_only_by_exact_locator_revision_and_masking_is_shared()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("design.md");
    fs::write(&input, "original SOURCE_SECRET")?;
    let mut store = Store::open(
        &directory.path().join("context.db"),
        RedactionPolicy {
            literal_secrets: vec!["SOURCE_SECRET".into()],
        },
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "docs",
        &input,
        ImportFormat::Document,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    let original = record(&latest, "docs:document")?;
    let locator = original
        .evidence
        .first()
        .ok_or("missing locator")?
        .locator
        .clone();
    let journal = directory.path().join("summary.jsonl");
    fs::write(
        &journal,
        serde_json::to_string(&serde_json::json!({
            "id":"S1", "kind":"finding", "text":"summary SOURCE_SECRET", "evidence_level":"summary_only",
            "references":[{"locator":locator,"revision":original.revision}]
        }))?,
    )?;
    ingest::import(
        &mut store,
        "p",
        "summaries",
        &journal,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    let summary = record(&latest, "summaries:S1")?;
    assert!(summary.derived);
    assert_eq!(summary.fidelity, Fidelity::SummaryOnly);
    assert!(
        summary
            .evidence
            .iter()
            .any(|evidence| evidence.record_id.as_deref() == Some("docs:document"))
    );
    assert!(!serde_json::to_string(&store.load().await?)?.contains("SOURCE_SECRET"));
    Ok(())
}

#[tokio::test]
async fn native_ancestry_resolves_only_explicit_selected_session_sources()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let parent = directory.path().join("parent.jsonl");
    let child = directory.path().join("child.jsonl");
    fs::write(
        &parent,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"parent\",\"cwd\":\"/repo\"}}\n",
    )?;
    fs::write(
        &child,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"child\",\"cwd\":\"/repo\",\"parent_thread_id\":\"parent\",\"forked_from_id\":\"parent\"}}\n",
    )?;
    ingest::import(
        &mut store,
        "p",
        "child-source",
        &child,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    let before = store.latest().await?;
    assert!(before.iter().any(|entity| matches!(entity, Entity::Session(session) if session.parent_id.as_deref() == Some("child-source:parent"))));
    assert!(
        source(&before, "child-source")?
            .gaps
            .iter()
            .any(|gap| gap.contains("session_parent_unresolved"))
    );
    ingest::import(
        &mut store,
        "p",
        "parent-source",
        &parent,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "child-source",
        &child,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    let after = store.latest().await?;
    assert!(after.iter().any(|entity| matches!(entity, Entity::Session(session) if session.id == "child-source:child" && session.parent_id.as_deref() == Some("parent-source:parent") && session.status == SessionStatus::Unknown)));
    assert!(after.iter().any(|entity| matches!(entity, Entity::Relation(relation) if relation.kind == RelationKind::ForkedFrom && relation.to == (Target::Session { id: "parent-source:parent".into() }))));
    assert!(
        !source(&after, "child-source")?
            .gaps
            .iter()
            .any(|gap| gap.contains("session_parent_unresolved"))
    );
    assert!(!after.iter().any(|entity| matches!(entity, Entity::Relation(relation) if relation.kind == RelationKind::IntegratedInto)));
    Ok(())
}
