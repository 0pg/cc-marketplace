//! The first format upgrade adds a header to the existing metadata row only.
//! SQLite's transaction journal is the rollback copy: no external plaintext
//! backup is needed or retained, and no entity or compaction policy is rewritten.
use std::{
    fs::{File, OpenOptions},
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::*;

pub const CURRENT_FORMAT: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreFormatState {
    Missing,
    Compatible,
    MigrationRequired,
    Migrating,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreFormatStatus {
    pub state: StoreFormatState,
    pub format_version: Option<u32>,
    pub target_format: u32,
    pub read_min: u32,
    pub read_max: u32,
    pub write_min: u32,
    pub write_max: u32,
    pub db_identity: Option<String>,
    pub migration_epoch: Option<u64>,
}

impl StoreFormatStatus {
    fn absent(state: StoreFormatState, version: Option<u32>) -> Self {
        Self {
            state,
            format_version: version,
            target_format: CURRENT_FORMAT,
            read_min: 0,
            read_max: CURRENT_FORMAT,
            write_min: CURRENT_FORMAT,
            write_max: CURRENT_FORMAT,
            db_identity: None,
            migration_epoch: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum StoreFormatError {
    #[error("invalid or unknown Memento store format: {0}")]
    Invalid(String),
    #[error("unsupported Memento store format {version}; this runtime supports format 1")]
    Unsupported { version: u32 },
    #[error("Memento store identity or migration epoch changed; reopen with a compatible runtime")]
    Fenced,
    #[error("selected Memento store does not exist or has not been initialized")]
    Missing,
    #[error("selected Memento store is busy; retry after the current operation completes")]
    Busy,
}

impl StoreFormatError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "store_format_invalid",
            Self::Unsupported { .. } => "store_format_unsupported",
            Self::Fenced => "store_format_fenced",
            Self::Missing => "store_missing",
            Self::Busy => "store_busy",
        }
    }
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::StoreFormat(error) => error.code(),
            Self::Capacity { .. } => "store_capacity_exceeded",
            Self::Io(_) => "io_error",
            Self::Json(_) => "invalid_json",
            Self::Database(_) => "database_error",
            Self::Redaction(_) => "invalid_redaction",
            Self::Invalid(_) => "invalid_input",
            Self::Query(_) => "query_error",
            Self::Compaction(_) => "compaction_error",
            Self::Capture(_) => "capture_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    pub version: u32,
    pub db_identity: String,
    pub migration_epoch: u64,
}

impl Header {
    pub(super) fn status(&self) -> StoreFormatStatus {
        StoreFormatStatus {
            db_identity: Some(self.db_identity.clone()),
            migration_epoch: Some(self.migration_epoch),
            ..StoreFormatStatus::absent(StoreFormatState::Compatible, Some(self.version))
        }
    }

    fn validate(&self) -> Result<()> {
        if self.version != CURRENT_FORMAT {
            return Err(StoreFormatError::Unsupported {
                version: self.version,
            }
            .into());
        }
        if self.db_identity.len() != 64
            || !self.db_identity.bytes().all(|b| b.is_ascii_hexdigit())
            || self.migration_epoch == 0
        {
            return Err(StoreFormatError::Invalid("invalid identity or epoch".into()).into());
        }
        Ok(())
    }
}

/// One empty sidecar serializes schema upgrades and every writer. Locking the
/// SQLite file itself interferes with SQLite's native locks on macOS.
pub(super) async fn lock(path: &Path, create: bool) -> Result<File> {
    if !create && !path.try_exists()? {
        return Err(StoreFormatError::Missing.into());
    }
    let mut lock_path = path.as_os_str().to_owned();
    lock_path.push(".memento-lock");
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(Path::new(&lock_path))
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(StoreFormatError::Missing.into());
        }
        Err(error) => return Err(error.into()),
    };
    for _ in 0..100 {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
    Err(StoreFormatError::Busy.into())
}

pub(super) fn canonical_path(path: &Path) -> Result<PathBuf> {
    if path.try_exists()? {
        return Ok(path.canonicalize()?);
    }
    let name = path
        .file_name()
        .ok_or_else(|| StoreFormatError::Invalid("store must be a file path".into()))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let parent = match parent {
        Some(parent) => parent.canonicalize()?,
        None => std::env::current_dir()?,
    };
    Ok(parent.join(name))
}

#[derive(Debug, toasty::Model)]
#[table = "sqlite_master"]
struct SchemaEntry {
    #[key]
    name: String,
    #[column("type")]
    kind: String,
    tbl_name: String,
    sql: Option<String>,
}

fn normalized(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) async fn validate_schema(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let mut magic = [0; 16];
    if file.read_exact(&mut magic).is_err() || &magic != b"SQLite format 3\0" {
        return Err(StoreFormatError::Invalid("not a known SQLite database".into()).into());
    }
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(SchemaEntry));
    let mut db = builder
        .build(toasty_driver_sqlite::Sqlite::open(path))
        .await?;
    let rows = SchemaEntry::all().exec(&mut db).await.map_err(|error| {
        StoreFormatError::Invalid(format!("cannot inspect SQLite schema: {error}"))
    })?;
    let table = "CREATE TABLE \"stored_entries\" ( \"id\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \"event_key\" TEXT NOT NULL, \"entity_key\" TEXT NOT NULL, \"captured_at\" TEXT NOT NULL, \"payload\" TEXT NOT NULL )";
    let index = "CREATE UNIQUE INDEX \"index_stored_entries_by_event_key\" ON \"stored_entries\" (\"event_key\")";
    if rows.len() != 3
        || !rows.iter().all(|row| {
            let expected = match (row.kind.as_str(), row.name.as_str(), row.tbl_name.as_str()) {
                ("table", "stored_entries", "stored_entries") => table,
                ("index", "index_stored_entries_by_event_key", "stored_entries") => index,
                ("table", "sqlite_sequence", "sqlite_sequence") => {
                    "CREATE TABLE sqlite_sequence(name,seq)"
                }
                _ => return false,
            };
            row.sql
                .as_ref()
                .is_some_and(|sql| normalized(sql) == expected)
        })
    {
        return Err(StoreFormatError::Invalid("unregistered SQLite schema".into()).into());
    }
    Ok(())
}

fn checked_metadata(rows: &[StoredEntry]) -> Result<retention::Metadata> {
    let metadata: Vec<_> = rows
        .iter()
        .filter(|row| retention::is_metadata(row))
        .collect();
    if metadata.len() > 1
        || metadata
            .iter()
            .any(|row| row.event_key != retention::METADATA_KEY)
    {
        return Err(StoreFormatError::Invalid("invalid metadata row key".into()).into());
    }
    let meta = retention::read_metadata(rows)
        .map_err(|error| StoreFormatError::Invalid(format!("invalid metadata: {error}")))?;
    if let Some(header) = &meta.store_format {
        header.validate()?;
    }
    Ok(meta)
}

fn validate_rows(rows: &[StoredEntry]) -> Result<retention::Metadata> {
    let meta = checked_metadata(rows)?;
    for row in rows {
        if row.id == 0
            || i64::try_from(row.id).is_err()
            || chrono::DateTime::parse_from_rfc3339(&row.captured_at).is_err()
        {
            return Err(
                StoreFormatError::Invalid("invalid row sequence or captured_at".into()).into(),
            );
        }
        if retention::is_metadata(row) {
            continue;
        }
        let payload: serde_json::Value = serde_json::from_str(&row.payload).map_err(|error| {
            StoreFormatError::Invalid(format!("invalid entity payload: {error}"))
        })?;
        if !payload.as_object().is_some_and(|object| {
            object.len() == 2 && object.contains_key("entity") && object.contains_key("data")
        }) {
            return Err(StoreFormatError::Invalid("unknown entity payload fields".into()).into());
        }
        let entity: Entity = serde_json::from_value(payload).map_err(|error| {
            StoreFormatError::Invalid(format!("invalid entity payload: {error}"))
        })?;
        validate(&entity)
            .map_err(|error| StoreFormatError::Invalid(format!("invalid entity: {error}")))?;
        if row.entity_key != entity.key()
            || row.event_key.len() != 64
            || !row.event_key.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(StoreFormatError::Invalid("invalid entity/event key".into()).into());
        }
    }
    Ok(meta)
}

pub(super) async fn inspect(path: &Path) -> Result<StoreFormatStatus> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => {
            return Ok(StoreFormatStatus::absent(StoreFormatState::Missing, None));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoreFormatStatus::absent(StoreFormatState::Missing, None));
        }
        Err(error) => return Err(error.into()),
        _ => {}
    }
    // Opening for inspection neither creates a header nor changes permissions.
    validate_schema(path).await?;
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(StoredEntry));
    let mut db = builder
        .build(toasty_driver_sqlite::Sqlite::open(path))
        .await?;
    let mut transaction = db.transaction().await?;
    let rows = StoredEntry::all().exec(&mut transaction).await?;
    let meta = validate_rows(&rows)?;
    transaction.commit().await?;
    Ok(meta.store_format.map_or_else(
        || StoreFormatStatus::absent(StoreFormatState::MigrationRequired, Some(0)),
        |header| header.status(),
    ))
}

pub(super) async fn migrate(db: &mut toasty::Db, path: &Path) -> Result<Header> {
    let mut transaction = db.transaction().await?;
    let mut rows = StoredEntry::all().exec(&mut transaction).await?;
    let mut meta = validate_rows(&rows)?;
    if let Some(header) = meta.store_format {
        transaction.commit().await?;
        return Ok(header);
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| StoreFormatError::Invalid(format!("clock before Unix epoch: {error}")))?
        .as_nanos();
    let header = Header {
        version: CURRENT_FORMAT,
        db_identity: hash(format!("{}:{nonce}:{}", path.display(), std::process::id()).as_bytes()),
        migration_epoch: 1,
    };
    meta.store_format = Some(header.clone());
    let metadata_bytes = serde_json::to_vec(&meta)?.len();
    let mut usage = compaction::Usage {
        entries: 1,
        payload_bytes: metadata_bytes,
    };
    for row in rows.iter().filter(|row| !retention::is_metadata(row)) {
        usage.entries = usage
            .entries
            .checked_add(1)
            .ok_or_else(|| StoreFormatError::Invalid("entry count overflow".into()))?;
        usage.payload_bytes = usage
            .payload_bytes
            .checked_add(row.payload.len())
            .ok_or_else(|| StoreFormatError::Invalid("payload size overflow".into()))?;
    }
    if usage.entries > meta.policy.max_entries
        || usage.payload_bytes > meta.policy.max_payload_bytes
    {
        return Err(Error::Capacity {
            entries: usage.entries,
            payload_bytes: usage.payload_bytes,
            max_entries: meta.policy.max_entries,
            max_payload_bytes: meta.policy.max_payload_bytes,
        });
    }
    retention::save_metadata(&mut transaction, &mut rows, &meta).await?;
    transaction.commit().await?;
    tracing::info!(
        store_format = CURRENT_FORMAT,
        migration_epoch = 1,
        "Memento store format marked"
    );
    Ok(header)
}

pub(super) async fn fence(
    transaction: &mut toasty::Transaction<'_>,
    expected: &Header,
) -> Result<()> {
    let rows = StoredEntry::filter(
        StoredEntry::fields()
            .entity_key()
            .eq(retention::METADATA_KEY),
    )
    .exec(transaction)
    .await?;
    let meta = checked_metadata(&rows)?;
    if meta.store_format.as_ref() != Some(expected) {
        return Err(StoreFormatError::Fenced.into());
    }
    Ok(())
}
