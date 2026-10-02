//! Corruption/fencing fixtures here are deliberate mutations of current stores.
//! Real released legacy binaries are exercised by the separate upgrade harness.
use std::{error::Error, path::Path};

use memento::{
    Error as StoreError, Store, ingest,
    model::{Entity, Record, RecordKind, SourceKind},
    security::RedactionPolicy,
    store::StoreFormatState,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const META: &str = "__work_context_compaction_v1__";

#[derive(Debug, toasty::Model)]
#[table = "stored_entries"]
struct Row {
    #[key]
    #[auto]
    id: u64,
    #[unique]
    event_key: String,
    entity_key: String,
    captured_at: String,
    payload: String,
}

async fn raw(path: &Path) -> Result<toasty::Db, Box<dyn Error>> {
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(Row));
    Ok(builder
        .build(toasty_driver_sqlite::Sqlite::open(path))
        .await?)
}

async fn rows(path: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut rows = Row::all().exec(&mut raw(path).await?).await?;
    rows.sort_by_key(|row| row.id);
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({"id":row.id,"event_key":row.event_key,"entity_key":row.entity_key,
                "captured_at":row.captured_at,"payload":row.payload})
        })
        .collect())
}

async fn mutate_metadata(path: &Path, mutate: impl FnOnce(&mut Value) -> TestResult) -> TestResult {
    let mut db = raw(path).await?;
    let mut row = Row::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .find(|row| row.entity_key == META)
        .ok_or("missing metadata")?;
    let mut payload: Value = serde_json::from_str(&row.payload)?;
    mutate(&mut payload)?;
    row.update()
        .payload(serde_json::to_string(&payload)?)
        .exec(&mut db)
        .await?;
    Ok(())
}

async fn populated(path: &Path) -> Result<Store, Box<dyn Error>> {
    let mut store = Store::open(path, RedactionPolicy::default()).await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "p",
            SourceKind::Journal,
        )))
        .await?;
    store
        .append(Entity::Record(Record::new(
            "request",
            "p",
            "journal",
            RecordKind::Request,
            "preserve exact context",
        )))
        .await?;
    Ok(store)
}

#[tokio::test]
async fn status_is_noncreating_nonmutating_and_identity_survives_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("nested/context.sqlite");
    assert_eq!(
        Store::inspect(&path).await?.state,
        StoreFormatState::Missing
    );
    assert!(!path.exists());
    assert!(!directory.path().join("nested").exists());
    let mut store = populated(&path).await?;
    let before = std::fs::read(&path)?;
    let status = Store::inspect(&path).await?;
    assert_eq!(status.state, StoreFormatState::Compatible);
    assert_eq!(status.format_version, Some(1));
    assert_eq!(status.migration_epoch, Some(1));
    assert_eq!(std::fs::read(&path)?, before);
    let corpus = serde_json::to_value(store.load().await?)?;
    drop(store);
    let mut reopened = Store::open(&path, RedactionPolicy::default()).await?;
    assert_eq!(Store::inspect(&path).await?, status);
    assert_eq!(serde_json::to_value(reopened.load().await?)?, corpus);
    assert_eq!(
        Store::migrate(&path, RedactionPolicy::default()).await?,
        status
    );
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[tokio::test]
async fn unknown_file_and_schema_are_rejected_without_initialization() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("not-sqlite");
    std::fs::write(&path, b"not a memento database")?;
    let before = std::fs::read(&path)?;
    let error = Store::inspect(&path)
        .await
        .err()
        .ok_or("accepted unknown file")?;
    assert_eq!(error.code(), "store_format_invalid");
    assert!(
        Store::open(&path, RedactionPolicy::default())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&path)?, before);
    let unknown = directory.path().join("other.sqlite");
    #[derive(Debug, toasty::Model)]
    struct Unrelated {
        #[key]
        #[auto]
        id: u64,
        value: String,
    }
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(Unrelated));
    let db = builder
        .build(toasty_driver_sqlite::Sqlite::open(&unknown))
        .await?;
    db.push_schema().await?;
    let before = std::fs::read(&unknown)?;
    assert_eq!(
        Store::inspect(&unknown)
            .await
            .err()
            .ok_or("accepted unknown schema")?
            .code(),
        "store_format_invalid"
    );
    assert!(
        Store::open(&unknown, RedactionPolicy::default())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&unknown)?, before);
    Ok(())
}

#[tokio::test]
async fn unknown_record_payload_cannot_be_marked_or_silently_discarded() -> TestResult {
    for unknown_outer in [false, true] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("context.sqlite");
        drop(populated(&path).await?);
        mutate_metadata(&path, |meta| {
            meta.as_object_mut()
                .ok_or("metadata must be object")?
                .remove("store_format");
            Ok(())
        })
        .await?;
        let mut db = raw(&path).await?;
        let mut row = Row::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .find(|row| {
                matches!(
                    serde_json::from_str::<Entity>(&row.payload),
                    Ok(Entity::Record(_))
                )
            })
            .ok_or("missing entity row")?;
        let mut payload: Value = serde_json::from_str(&row.payload)?;
        let object = if unknown_outer {
            &mut payload
        } else {
            payload.get_mut("data").ok_or("missing entity data")?
        };
        object
            .as_object_mut()
            .ok_or("entity must be object")?
            .insert(
                "future_field".into(),
                json!("preserve this unknown context"),
            );
        row.update()
            .payload(serde_json::to_string(&payload)?)
            .exec(&mut db)
            .await?;
        drop(db);
        let before = std::fs::read(&path)?;
        assert_eq!(
            Store::inspect(&path)
                .await
                .err()
                .ok_or("accepted unknown payload")?
                .code(),
            "store_format_invalid"
        );
        assert!(
            Store::migrate(&path, RedactionPolicy::default())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path)?, before);
    }
    Ok(())
}

#[tokio::test]
async fn future_and_malformed_headers_are_rejected_without_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    drop(populated(&path).await?);
    mutate_metadata(&path, |meta| {
        *meta
            .pointer_mut("/store_format/version")
            .ok_or("missing version")? = json!(2);
        Ok(())
    })
    .await?;
    let before = std::fs::read(&path)?;
    assert_eq!(
        Store::inspect(&path)
            .await
            .err()
            .ok_or("accepted future format")?
            .code(),
        "store_format_unsupported"
    );
    assert!(
        Store::open(&path, RedactionPolicy::default())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&path)?, before);
    mutate_metadata(&path, |meta| {
        *meta
            .pointer_mut("/store_format/version")
            .ok_or("missing version")? = json!(1);
        *meta
            .pointer_mut("/store_format/db_identity")
            .ok_or("missing identity")? = json!("invalid");
        Ok(())
    })
    .await?;
    let before = std::fs::read(&path)?;
    assert_eq!(
        Store::inspect(&path)
            .await
            .err()
            .ok_or("accepted malformed identity")?
            .code(),
        "store_format_invalid"
    );
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[tokio::test]
async fn an_open_writer_and_reader_recheck_epoch_in_each_transaction() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let mut store = populated(&path).await?;
    mutate_metadata(&path, |meta| {
        *meta
            .pointer_mut("/store_format/migration_epoch")
            .ok_or("missing epoch")? = json!(2);
        Ok(())
    })
    .await?;
    let before = rows(&path).await?;
    let error = store
        .append(Entity::Record(Record::new(
            "later",
            "p",
            "journal",
            RecordKind::Finding,
            "must reject",
        )))
        .await
        .err()
        .ok_or("stale writer accepted")?;
    assert_eq!(error.code(), "store_format_fenced");
    assert_eq!(
        store
            .load()
            .await
            .err()
            .ok_or("stale reader accepted")?
            .code(),
        "store_format_fenced"
    );
    assert_eq!(rows(&path).await?, before);
    drop(store);
    let mut reopened = Store::open(&path, RedactionPolicy::default()).await?;
    reopened
        .append(Entity::Record(Record::new(
            "later",
            "p",
            "journal",
            RecordKind::Finding,
            "now compatible",
        )))
        .await?;
    Ok(())
}

#[tokio::test]
async fn deliberate_header_removal_is_marked_once_without_entity_rewrites() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    drop(populated(&path).await?);
    mutate_metadata(&path, |meta| {
        meta.as_object_mut()
            .ok_or("metadata is not object")?
            .remove("store_format");
        Ok(())
    })
    .await?;
    let before = rows(&path).await?;
    let bytes = std::fs::read(&path)?;
    assert_eq!(
        Store::inspect(&path).await?.state,
        StoreFormatState::MigrationRequired
    );
    assert_eq!(std::fs::read(&path)?, bytes);
    let migrated = Store::migrate(&path, RedactionPolicy::default()).await?;
    assert_eq!(migrated.state, StoreFormatState::Compatible);
    let after = rows(&path).await?;
    let entities = |rows: Vec<Value>| {
        rows.into_iter()
            .filter(|row| row.get("entity_key") != Some(&json!(META)))
            .collect::<Vec<_>>()
    };
    assert_eq!(entities(after), entities(before));
    assert_eq!(
        Store::migrate(&path, RedactionPolicy::default()).await?,
        migrated
    );
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 2);
    Ok(())
}

#[tokio::test]
async fn header_growth_over_capacity_rolls_back_without_compaction_or_policy_growth() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    drop(populated(&path).await?);
    mutate_metadata(&path, |meta| {
        meta.as_object_mut()
            .ok_or("metadata is not object")?
            .remove("store_format");
        *meta
            .pointer_mut("/policy/max_payload_bytes")
            .ok_or("missing capacity")? = json!(1);
        Ok(())
    })
    .await?;
    let before = rows(&path).await?;
    assert!(matches!(
        Store::migrate(&path, RedactionPolicy::default()).await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(rows(&path).await?, before);
    assert_eq!(
        Store::inspect(&path).await?.state,
        StoreFormatState::MigrationRequired
    );
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 2);
    Ok(())
}

#[tokio::test]
async fn two_first_commands_share_one_initialization_without_losing_a_record() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("context.sqlite");
    let writer = |project: &'static str| {
        let path = path.clone();
        async move {
            let mut store = Store::open(&path, RedactionPolicy::default()).await?;
            store
                .append(Entity::Source(ingest::source(
                    "journal",
                    project,
                    SourceKind::Journal,
                )))
                .await?;
            store
                .append(Entity::Record(Record::new(
                    "request",
                    project,
                    "journal",
                    RecordKind::Request,
                    project,
                )))
                .await?;
            Ok::<_, memento::Error>(())
        }
    };
    let (first, second) = tokio::join!(writer("project-a"), writer("project-b"));
    first?;
    second?;
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    let corpus = store.load().await?;
    assert_eq!(corpus.entries.len(), 4);
    assert!(
        corpus
            .entries
            .iter()
            .any(|entry| entry.entity.project_id() == "project-a")
    );
    assert!(
        corpus
            .entries
            .iter()
            .any(|entry| entry.entity.project_id() == "project-b")
    );
    Ok(())
}

#[tokio::test]
async fn migration_never_creates_an_unselected_or_missing_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("other/context.sqlite");
    let error = Store::migrate(&missing, RedactionPolicy::default())
        .await
        .err()
        .ok_or("created missing store")?;
    assert_eq!(error.code(), "store_missing");
    assert!(!directory.path().join("other").exists());
    Ok(())
}
