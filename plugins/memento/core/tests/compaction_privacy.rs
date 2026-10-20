use std::{collections::BTreeMap, error::Error, path::Path};

use memento::{Store, compaction::Policy, ingest, model::*, security::RedactionPolicy};

type TestResult = Result<(), Box<dyn Error>>;

fn record(id: &str, source: &str, kind: RecordKind, body: &str) -> Record {
    Record::new(id, "project", source, kind, body)
}

fn evidence(record: &Record) -> Evidence {
    Evidence {
        source_id: record.source_id.clone(),
        record_id: Some(record.id.clone()),
        revision: record.revision.clone(),
        locator: format!("record:{}", record.id),
        availability: Availability::Available,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    }
}

fn policy(max_entries: usize) -> Policy {
    Policy {
        max_entries,
        max_payload_bytes: 64 * 1024,
        recent_entries: 0,
    }
}

async fn seeded_privacy_store(path: &Path) -> Result<(Store, Record), Box<dyn Error>> {
    let mut store = Store::open(path, RedactionPolicy::default()).await?;
    for source in ["secret", "journal"] {
        store
            .append(Entity::Source(ingest::source(
                source,
                "project",
                SourceKind::Journal,
            )))
            .await?;
    }
    let original = record("A", "secret", RecordKind::Finding, "sensitive original");
    let public = record("B", "journal", RecordKind::Finding, "public evidence");
    let mut old = record(
        "D",
        "journal",
        RecordKind::Decision,
        "sensitive derivative v1",
    );
    old.derived = true;
    old.evidence = vec![evidence(&original)];
    let mut current = record(
        "D",
        "journal",
        RecordKind::Decision,
        "sensitive derivative v2",
    );
    current.derived = true;
    current.evidence = vec![evidence(&public)];
    let mut downstream = record("E", "journal", RecordKind::Decision, "sensitive downstream");
    downstream.derived = true;
    downstream.evidence = vec![evidence(&current)];
    store
        .append_all([
            Entity::Record(original.clone()),
            Entity::Record(public),
            Entity::Record(old),
            Entity::Record(current),
            Entity::Record(downstream),
            Entity::Record(record(
                "noise-1",
                "journal",
                RecordKind::ToolResult,
                "unused",
            )),
            Entity::Record(record(
                "noise-2",
                "journal",
                RecordKind::ToolResult,
                "latest",
            )),
        ])
        .await?;
    Ok((store, original))
}

#[derive(Clone, Copy)]
enum Removal {
    RevokeSource,
    DeleteRecord,
    MissingRecord,
}

async fn remove(store: &mut Store, original: &Record, removal: Removal) -> TestResult {
    let entity = match removal {
        Removal::RevokeSource => {
            let mut source = ingest::source("secret", "project", SourceKind::Journal);
            source.authorized = false;
            Entity::Source(source)
        }
        Removal::DeleteRecord | Removal::MissingRecord => {
            let mut removed = original.clone();
            removed.availability = match removal {
                Removal::DeleteRecord => Availability::Deleted,
                _ => Availability::Missing,
            };
            Entity::Record(removed)
        }
    };
    store.append(entity).await?;
    Ok(())
}

async fn derivatives(
    store: &mut Store,
) -> Result<BTreeMap<String, (String, Availability)>, Box<dyn Error>> {
    Ok(store
        .latest()
        .await?
        .into_iter()
        .filter_map(|entity| match entity {
            Entity::Record(record) if matches!(record.id.as_str(), "D" | "E") => {
                Some((record.id, (record.body, record.availability)))
            }
            _ => None,
        })
        .collect())
}

#[tokio::test]
async fn historical_evidence_preserves_the_baseline_transitive_removal_contract() -> TestResult {
    for removal in [
        Removal::RevokeSource,
        Removal::DeleteRecord,
        Removal::MissingRecord,
    ] {
        let directory = tempfile::tempdir()?;
        let baseline_path = directory.path().join("baseline.sqlite");
        let compacted_path = directory.path().join("compacted.sqlite");
        let (mut baseline, original) = seeded_privacy_store(&baseline_path).await?;
        let (mut compacted, _) = seeded_privacy_store(&compacted_path).await?;
        let report = compacted.compact(Some(policy(64)), true).await?;
        assert!(report.removed_entries > 0);

        // Before the fix, dropping D(v1)'s A evidence left D(v2) and E available
        // after A disappeared or its source was revoked. The baseline scrubs both.
        remove(&mut baseline, &original, removal).await?;
        remove(&mut compacted, &original, removal).await?;
        let expected = derivatives(&mut baseline).await?;
        assert_eq!(expected.len(), 2);
        assert!(expected.values().all(|(body, availability)| {
            body.is_empty()
                && *availability
                    == match removal {
                        Removal::DeleteRecord => Availability::Deleted,
                        _ => Availability::Missing,
                    }
        }));
        assert_eq!(derivatives(&mut compacted).await?, expected);
        assert!(!serde_json::to_string(&compacted.load().await?)?.contains("sensitive"));
        drop(compacted);
        let mut reopened = Store::open(&compacted_path, RedactionPolicy::default()).await?;
        assert_eq!(derivatives(&mut reopened).await?, expected);
        if matches!(removal, Removal::DeleteRecord) {
            assert!(reopened.append(Entity::Record(original)).await.is_err());
        }
    }
    Ok(())
}

#[tokio::test]
async fn mixed_duplicate_and_new_batch_keeps_every_durable_receipt_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "project",
            SourceKind::Journal,
        )))
        .await?;
    store.compact(Some(policy(5)), true).await?;
    let original = record("A", "journal", RecordKind::ToolResult, "first output");
    let first = store.append(Entity::Record(original.clone())).await?;
    for id in ["B", "C"] {
        store
            .append(Entity::Record(record(
                id,
                "journal",
                RecordKind::ToolResult,
                id,
            )))
            .await?;
    }
    let before = store.load().await?;
    let receipts = store
        .append_all([
            Entity::Record(original.clone()),
            Entity::Record(record("D", "journal", RecordKind::ToolResult, "new output")),
        ])
        .await?;
    let duplicate = receipts.first().ok_or("missing duplicate receipt")?;
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.sequence, first.sequence);
    assert!(receipts.iter().all(|receipt| receipt.durable));
    assert!(
        receipts
            .last()
            .is_some_and(|receipt| receipt.compaction.is_some())
    );
    let after = store.load().await?;
    assert!(after.compaction.generation > before.compaction.generation);
    // Before the fix A returned durable=true but was collected by this same batch.
    for receipt in &receipts {
        assert!(
            after
                .entries
                .iter()
                .any(|entry| entry.sequence == receipt.sequence)
        );
    }
    assert!(store.compact(None, false).await?.before.entries <= 5);
    drop(store);
    let mut reopened = Store::open(&path, RedactionPolicy::default()).await?;
    let persisted = reopened.load().await?;
    for receipt in &receipts {
        assert!(
            persisted
                .entries
                .iter()
                .any(|entry| entry.sequence == receipt.sequence)
        );
    }
    let replay = reopened.append(Entity::Record(original)).await?;
    assert!(replay.duplicate);
    assert_eq!(replay.sequence, first.sequence);
    Ok(())
}
