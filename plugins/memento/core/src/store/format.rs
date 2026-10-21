//! Known format upgrades change the existing metadata row only.
//! SQLite's transaction journal is the rollback copy: no external plaintext
//! backup is needed or retained, and no entity or compaction policy is rewritten.
use std::{
    fs::{File, OpenOptions},
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::*;

pub const CURRENT_FORMAT: u32 = 2;

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
    #[error(
        "unsupported Memento store format {version}; this runtime supports formats 1 through 2"
    )]
    Unsupported { version: u32 },
    #[error("Memento store identity or migration epoch changed; reopen with a compatible runtime")]
    Fenced,
    #[error("selected Memento store does not exist or has not been initialized")]
    Missing,
    #[error("selected Memento store is busy; retry after the current operation completes")]
    Busy,
    #[error(
        "read-only Memento store inspection could not complete; the store may be locked or unreadable"
    )]
    InspectionUnavailable,
}

impl StoreFormatError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "store_format_invalid",
            Self::Unsupported { .. } => "store_format_unsupported",
            Self::Fenced => "store_format_fenced",
            Self::Missing => "store_missing",
            Self::Busy => "store_busy",
            Self::InspectionUnavailable => "store_inspection_unavailable",
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
            Self::RecordInput(error) => error.code,
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
            ..StoreFormatStatus::absent(
                if self.version == CURRENT_FORMAT {
                    StoreFormatState::Compatible
                } else {
                    StoreFormatState::MigrationRequired
                },
                Some(self.version),
            )
        }
    }

    fn validate(&self) -> Result<()> {
        if !(1..=CURRENT_FORMAT).contains(&self.version) {
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
    validate_schema_rows(&rows)
}

fn validate_schema_rows(rows: &[SchemaEntry]) -> Result<()> {
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

pub(super) struct PassiveSnapshot {
    pub status: StoreFormatStatus,
    pub entries: Vec<Entry>,
    pub capture: capture::State,
    pub compaction: CompactionState,
}

fn read_only_uri(path: &Path) -> Result<String> {
    let path = path.canonicalize()?;
    let path = path
        .to_str()
        .ok_or_else(|| StoreFormatError::Invalid("store path must be UTF-8".into()))?;
    let mut uri = String::from("file:");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(&mut uri, "%{byte:02X}")
                .map_err(|error| StoreFormatError::Invalid(error.to_string()))?;
        }
    }
    uri.push_str("?mode=ro");
    Ok(uri)
}

fn validate_passive_file(path: &Path, reject_sidecars: bool) -> Result<()> {
    let mut file = File::open(path)?;
    let mut header = [0; 20];
    if file.read_exact(&mut header).is_err() || header.get(..16) != Some(b"SQLite format 3\0") {
        return Err(StoreFormatError::Invalid("not a known SQLite database".into()).into());
    }
    if !reject_sidecars {
        return Ok(());
    }
    // A read-only WAL connection can create or modify shared-memory sidecars.
    // Do not silently ignore a WAL with immutable=1 or recover a hot journal.
    if header.get(18) == Some(&2) || header.get(19) == Some(&2) {
        return Err(StoreFormatError::Busy.into());
    }
    for suffix in ["-wal", "-journal"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        if Path::new(&sidecar).try_exists()? {
            return Err(StoreFormatError::Busy.into());
        }
    }
    Ok(())
}

/// Read one validated SQLite snapshot without preparing a writable Store.
/// Known legacy rows remain in their original format and need no migration.
pub(super) async fn passive_snapshot(path: &Path) -> Result<PassiveSnapshot> {
    let path = path.to_path_buf();
    // The SQLite driver in Toasty 0.4 can panic while preparing a query against
    // an exclusively locked database. Keep diagnostic failure a typed result.
    tokio::spawn(async move { snapshot(&path, true).await })
        .await
        .map_err(|_| StoreFormatError::InspectionUnavailable)?
}

async fn snapshot(path: &Path, reject_sidecars: bool) -> Result<PassiveSnapshot> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => {
            return Ok(PassiveSnapshot {
                status: StoreFormatStatus::absent(StoreFormatState::Missing, None),
                entries: Vec::new(),
                capture: capture::State::default(),
                compaction: CompactionState::default(),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PassiveSnapshot {
                status: StoreFormatStatus::absent(StoreFormatState::Missing, None),
                entries: Vec::new(),
                capture: capture::State::default(),
                compaction: CompactionState::default(),
            });
        }
        Err(error) => return Err(error.into()),
        _ => {}
    }
    let path = path.canonicalize()?;
    validate_passive_file(&path, reject_sidecars)?;
    let mut builder = toasty::Db::builder();
    builder.models(toasty::models!(SchemaEntry, StoredEntry));
    let mut db = builder
        .build(toasty_driver_sqlite::Sqlite::open(read_only_uri(&path)?))
        .await?;
    let mut transaction = db.transaction_builder().read_only(true).begin().await?;
    let schema = SchemaEntry::all()
        .exec(&mut transaction)
        .await
        .map_err(|error| {
            StoreFormatError::Invalid(format!("cannot inspect SQLite schema: {error}"))
        })?;
    validate_schema_rows(&schema)?;
    let rows = StoredEntry::all().exec(&mut transaction).await?;
    let meta = validate_rows(&rows)?;
    let mut entries = rows
        .into_iter()
        .filter(|row| !retention::is_metadata(row))
        .map(|row| {
            Ok(Entry {
                sequence: row.id,
                captured_at: row.captured_at,
                entity: serde_json::from_str(&row.payload)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.sequence);
    transaction.commit().await?;
    Ok(PassiveSnapshot {
        status: meta.store_format.as_ref().map_or_else(
            || StoreFormatStatus::absent(StoreFormatState::MigrationRequired, Some(0)),
            |header| header.status(),
        ),
        entries,
        capture: meta.capture,
        compaction: meta.compaction,
    })
}

pub(super) async fn inspect(path: &Path) -> Result<StoreFormatStatus> {
    // Normal open/migrate also use format inspection. Preserve their existing
    // ability to operate on WAL stores; only passive session queries reject WAL.
    Ok(snapshot(path, false).await?.status)
}

pub(super) async fn migrate(db: &mut toasty::Db, path: &Path) -> Result<Header> {
    let mut transaction = db.transaction().await?;
    let mut rows = StoredEntry::all().exec(&mut transaction).await?;
    let mut meta = validate_rows(&rows)?;
    if let Some(header) = &meta.store_format
        && header.version == CURRENT_FORMAT
    {
        transaction.commit().await?;
        return Ok(header.clone());
    }
    let header = match &meta.store_format {
        Some(previous) => Header {
            version: CURRENT_FORMAT,
            db_identity: previous.db_identity.clone(),
            migration_epoch: previous
                .migration_epoch
                .checked_add(1)
                .ok_or_else(|| StoreFormatError::Invalid("migration epoch overflow".into()))?,
        },
        None => {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| {
                    StoreFormatError::Invalid(format!("clock before Unix epoch: {error}"))
                })?
                .as_nanos();
            Header {
                version: CURRENT_FORMAT,
                db_identity: hash(
                    format!("{}:{nonce}:{}", path.display(), std::process::id()).as_bytes(),
                ),
                migration_epoch: 1,
            }
        }
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
        migration_epoch = header.migration_epoch,
        "Memento store format upgraded"
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    async fn populated(path: &Path) -> Result<()> {
        let mut store = Store::open(path, RedactionPolicy::default()).await?;
        store
            .append(Entity::Source(crate::ingest::source(
                "journal",
                "project",
                SourceKind::Journal,
            )))
            .await?;
        store
            .append(Entity::Record(Record::new(
                "finding",
                "project",
                "journal",
                RecordKind::Finding,
                "preserved result",
            )))
            .await?;
        Ok(())
    }

    fn names(path: &Path) -> std::io::Result<Vec<std::ffi::OsString>> {
        let mut names = std::fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        names.sort();
        Ok(names)
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn passive_snapshot_reads_readonly_special_path_without_a_lock_sidecar() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("상태 #?% context.sqlite");
        populated(&path).await?;
        let mut lock = path.as_os_str().to_owned();
        lock.push(".memento-lock");
        std::fs::remove_file(Path::new(&lock))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444))?;
        let before = std::fs::read(&path)?;
        let modified = std::fs::metadata(&path)?.modified()?;
        let directory_before = names(directory.path())?;

        let snapshot = passive_snapshot(&path).await?;
        assert_eq!(snapshot.status.state, StoreFormatState::Compatible);
        assert_eq!(snapshot.entries.len(), 2);
        assert!(snapshot.entries.iter().any(|entry| {
            matches!(&entry.entity, Entity::Record(record) if record.id == "finding")
        }));
        let mut builder = toasty::Db::builder();
        builder.models(toasty::models!(StoredEntry));
        let mut db = builder
            .build(toasty_driver_sqlite::Sqlite::open(read_only_uri(&path)?))
            .await?;
        let mut row = StoredEntry::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .next()
            .ok_or("missing fixture row")?;
        assert!(
            row.update()
                .payload("changed".to_owned())
                .exec(&mut db)
                .await
                .is_err()
        );
        drop(db);

        assert_eq!(std::fs::read(&path)?, before);
        assert_eq!(std::fs::metadata(&path)?.modified()?, modified);
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o444
        );
        assert_eq!(names(directory.path())?, directory_before);
        Ok(())
    }

    #[tokio::test]
    async fn passive_snapshot_neither_creates_missing_store_nor_migrates_legacy_rows() -> TestResult
    {
        let directory = tempfile::tempdir()?;
        let missing = directory.path().join("missing/context.sqlite");
        let snapshot = passive_snapshot(&missing).await?;
        assert_eq!(snapshot.status.state, StoreFormatState::Missing);
        assert!(snapshot.entries.is_empty());
        assert!(!directory.path().join("missing").exists());

        let empty = directory.path().join("empty.sqlite");
        std::fs::write(&empty, b"")?;
        assert_eq!(
            passive_snapshot(&empty).await?.status.state,
            StoreFormatState::Missing
        );
        assert!(std::fs::read(&empty)?.is_empty());

        let path = directory.path().join("legacy.sqlite");
        populated(&path).await?;
        let mut builder = toasty::Db::builder();
        builder.models(toasty::models!(StoredEntry));
        let mut db = builder
            .build(toasty_driver_sqlite::Sqlite::open(&path))
            .await?;
        let mut row = StoredEntry::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .find(retention::is_metadata)
            .ok_or("missing metadata")?;
        let mut metadata: serde_json::Value = serde_json::from_str(&row.payload)?;
        metadata
            .as_object_mut()
            .ok_or("metadata not object")?
            .remove("store_format");
        row.update()
            .payload(serde_json::to_string(&metadata)?)
            .exec(&mut db)
            .await?;
        drop(db);
        let before = std::fs::read(&path)?;
        let directory_before = names(directory.path())?;
        let snapshot = passive_snapshot(&path).await?;
        assert_eq!(snapshot.status.state, StoreFormatState::MigrationRequired);
        assert_eq!(snapshot.status.format_version, Some(0));
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(std::fs::read(&path)?, before);
        assert_eq!(names(directory.path())?, directory_before);
        Ok(())
    }

    #[tokio::test]
    async fn passive_snapshot_rejects_journals_and_wal_without_recovery() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("context.sqlite");
        populated(&path).await?;
        for suffix in ["-wal", "-journal"] {
            let mut sidecar = path.as_os_str().to_owned();
            sidecar.push(suffix);
            let sidecar = Path::new(&sidecar);
            std::fs::write(sidecar, b"untouched journal")?;
            let before = std::fs::read(&path)?;
            let directory_before = names(directory.path())?;
            let error = passive_snapshot(&path)
                .await
                .err()
                .ok_or("accepted sidecar")?;
            assert_eq!(error.code(), "store_busy");
            assert_eq!(std::fs::read(&path)?, before);
            assert_eq!(std::fs::read(sidecar)?, b"untouched journal");
            assert_eq!(names(directory.path())?, directory_before);
            std::fs::remove_file(sidecar)?;
        }
        let mut bytes = std::fs::read(&path)?;
        bytes
            .get_mut(18..20)
            .ok_or("missing SQLite header")?
            .copy_from_slice(&[2, 2]);
        std::fs::write(&path, &bytes)?;
        let directory_before = names(directory.path())?;
        let error = passive_snapshot(&path)
            .await
            .err()
            .ok_or("accepted WAL mode")?;
        assert_eq!(error.code(), "store_busy");
        assert_eq!(std::fs::read(&path)?, bytes);
        assert_eq!(names(directory.path())?, directory_before);
        Ok(())
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn passive_snapshot_checks_sidecars_beside_the_canonical_store() -> TestResult {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("context.sqlite");
        let alias = directory.path().join("alias.sqlite");
        populated(&path).await?;
        std::os::unix::fs::symlink(&path, &alias)?;
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push("-journal");
        std::fs::write(Path::new(&sidecar), b"untouched journal")?;
        let before = std::fs::read(&path)?;
        let directory_before = names(directory.path())?;
        let error = passive_snapshot(&alias)
            .await
            .err()
            .ok_or("alias bypassed journal guard")?;
        assert_eq!(error.code(), "store_busy");
        assert_eq!(std::fs::read(&path)?, before);
        assert_eq!(names(directory.path())?, directory_before);
        Ok(())
    }

    #[tokio::test]
    async fn passive_snapshot_contains_driver_failure_under_an_exclusive_sqlite_lock() -> TestResult
    {
        use std::{
            io::{BufRead, BufReader},
            process::{Child, Command, Stdio},
        };

        struct ExclusiveLock(Child);
        impl Drop for ExclusiveLock {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        let directory = tempfile::tempdir()?;
        let path = directory.path().join("context.sqlite");
        populated(&path).await?;
        let before = std::fs::read(&path)?;
        let directory_before = names(directory.path())?;
        let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1])\nc.execute('BEGIN EXCLUSIVE')\nprint('locked',flush=True)\nsys.stdin.readline()\nc.rollback()\nc.close()\n";
        let mut lock = ExclusiveLock(
            Command::new("python3")
                .args(["-c", script])
                .arg(&path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let output = lock.0.stdout.take().ok_or("missing lock handshake")?;
        let mut ready = String::new();
        BufReader::new(output).read_line(&mut ready)?;
        assert_eq!(ready.trim(), "locked");
        let error = passive_snapshot(&path)
            .await
            .err()
            .ok_or("exclusive lock unexpectedly inspected")?;
        assert_eq!(error.code(), "store_inspection_unavailable");
        assert_eq!(std::fs::read(&path)?, before);
        assert_eq!(names(directory.path())?, directory_before);
        Ok(())
    }
}
