use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    capture, compaction,
    model::*,
    security::{RedactionPolicy, hash},
};

mod format;
mod retention;

pub use format::{CURRENT_FORMAT, StoreFormatError, StoreFormatState, StoreFormatStatus};

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database operation failed: {0}")]
    Database(#[from] toasty::Error),
    #[error("invalid redaction policy: {0}")]
    Redaction(#[from] regex::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Query(#[from] crate::query::QueryError),
    #[error(transparent)]
    Compaction(#[from] compaction::Error),
    #[error(transparent)]
    Capture(#[from] capture::Error),
    #[error(transparent)]
    StoreFormat(#[from] StoreFormatError),
    #[error(
        "storage capacity exceeded: protected context requires {entries} entries and {payload_bytes} payload bytes; limits are {max_entries} entries and {max_payload_bytes} bytes"
    )]
    Capacity {
        entries: usize,
        payload_bytes: usize,
        max_entries: usize,
        max_payload_bytes: usize,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, toasty::Model)]
struct StoredEntry {
    #[key]
    #[auto]
    id: u64,
    #[unique]
    event_key: String,
    entity_key: String,
    captured_at: String,
    payload: String,
}

pub struct Store {
    db: toasty::Db,
    policy: RedactionPolicy,
    path: PathBuf,
    header: format::Header,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub sequence: u64,
    pub entity_id: String,
    pub duplicate: bool,
    pub durable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionNotice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompactionNotice {
    pub generation: u64,
    pub removed_entries: usize,
    pub remaining: compaction::Usage,
}

impl Store {
    pub async fn checkpoint(&mut self, request: capture::Request) -> Result<capture::Reply> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let reply = retention::checkpoint(&mut transaction, request, &self.policy).await?;
        transaction.commit().await?;
        Ok(reply)
    }

    pub async fn check_commit(
        &mut self,
        project: &str,
        repository: &Path,
        binding: &crate::git::IndexBinding,
    ) -> Result<capture::CommitCheckpoint> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let checkpoint =
            retention::check_commit(&mut transaction, project, repository, binding).await?;
        transaction.commit().await?;
        Ok(checkpoint)
    }

    pub async fn link_commit(
        &mut self,
        project: &str,
        repository: &Path,
        binding: &crate::git::IndexBinding,
        sha: &str,
    ) -> Result<()> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        retention::link_commit(&mut transaction, project, repository, binding, sha).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub fn sanitize(&self, entity: &Entity) -> Result<Entity> {
        self.policy.entity(entity)
    }

    pub async fn open(path: &Path, policy: RedactionPolicy) -> Result<Self> {
        if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0) {
            format::inspect(path).await?;
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let selected_path = format::canonical_path(path)?;
        let path = selected_path.as_path();
        let _guard = format::lock(path, true).await?;
        let initialize = match std::fs::metadata(path) {
            Ok(metadata) => metadata.len() == 0,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };
        if !initialize {
            format::validate_schema(path).await?;
        }
        let mut builder = toasty::Db::builder();
        builder.models(toasty::models!(StoredEntry));
        // Pass a filesystem path directly: URL parsing encodes spaces and can
        // interpret filename characters such as '?' and '#' as URL components.
        let mut db = builder
            .build(toasty_driver_sqlite::Sqlite::open(path))
            .await?;
        if initialize {
            db.push_schema().await?;
        }
        let header = format::migrate(&mut db, path).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self {
            db,
            policy,
            path: path.to_owned(),
            header,
        })
    }

    /// Inspect only the explicitly selected store without creating or upgrading it.
    pub async fn inspect(path: &Path) -> Result<StoreFormatStatus> {
        format::inspect(path).await
    }

    /// Mark a validated existing legacy store atomically; never creates a database.
    pub async fn migrate(path: &Path, policy: RedactionPolicy) -> Result<StoreFormatStatus> {
        if format::inspect(path).await?.state == StoreFormatState::Missing {
            return Err(StoreFormatError::Missing.into());
        }
        let selected_path = format::canonical_path(path)?;
        let path = selected_path.as_path();
        let _guard = format::lock(path, false).await?;
        format::validate_schema(path).await?;
        let mut builder = toasty::Db::builder();
        builder.models(toasty::models!(StoredEntry));
        let mut db = builder
            .build(toasty_driver_sqlite::Sqlite::open(path))
            .await?;
        let header = format::migrate(&mut db, path).await?;
        // Redaction affects subsequent writes, never the migration's retained payloads.
        let _ = policy;
        Ok(header.status())
    }

    #[tracing::instrument(skip_all, fields(entity_id = entity.id()))]
    pub async fn append(&mut self, entity: Entity) -> Result<Receipt> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let mut receipt = Self::append_one(&mut transaction, &self.policy, entity).await?;
        let pinned = BTreeSet::from([receipt.sequence]);
        receipt.compaction = retention::enforce(&mut transaction, &pinned).await?;
        transaction.commit().await?;
        Ok(receipt)
    }

    async fn append_one(
        transaction: &mut toasty::Transaction<'_>,
        policy: &RedactionPolicy,
        entity: Entity,
    ) -> Result<Receipt> {
        validate(&entity)?;
        let mut entity = policy.entity(&entity)?;
        let entity_key = entity.key();
        let mut revisions =
            StoredEntry::filter(StoredEntry::fields().entity_key().eq(entity_key.clone()))
                .exec(&mut *transaction)
                .await?;
        revisions.sort_by_key(|r| r.id);
        let previous = revisions
            .last()
            .map(|r| serde_json::from_str::<Entity>(&r.payload))
            .transpose()?;
        // A later observation cannot erase an origin that was actually captured.
        if let (Entity::Commit(commit), Some(Entity::Commit(old))) = (&mut entity, &previous)
            && commit.origin_worktree.is_none()
        {
            commit.origin_worktree.clone_from(&old.origin_worktree);
        }
        if !matches!(&entity, Entity::Source(_)) {
            let key = Entity::scoped_key(
                entity.project_id(),
                entity.source_id(),
                "source",
                entity.source_id(),
            );
            let sources = StoredEntry::filter(StoredEntry::fields().entity_key().eq(key))
                .exec(&mut *transaction)
                .await?;
            let latest = sources
                .iter()
                .max_by_key(|r| r.id)
                .map(|r| serde_json::from_str::<Entity>(&r.payload))
                .transpose()?;
            if !matches!(latest, Some(Entity::Source(s)) if s.authorized) {
                return Err(Error::Invalid(
                    "register an authorized source before writing its records".into(),
                ));
            }
        }
        if matches!((&entity, &previous), (Entity::Record(new), Some(Entity::Record(old))) if old.availability == Availability::Deleted && new.availability != Availability::Deleted)
        {
            return Err(Error::Invalid(
                "deleted record cannot be restored by replay; use a new explicitly recorded ID"
                    .into(),
            ));
        }
        if let Entity::Record(record) = &entity
            && record.derived
            && matches!(
                record.availability,
                Availability::Available | Availability::Redacted
            )
        {
            for evidence in &record.evidence {
                let source_key = Entity::scoped_key(
                    &record.project_id,
                    &evidence.source_id,
                    "source",
                    &evidence.source_id,
                );
                let sources =
                    StoredEntry::filter(StoredEntry::fields().entity_key().eq(source_key))
                        .exec(&mut *transaction)
                        .await?;
                let source = sources
                    .iter()
                    .max_by_key(|r| r.id)
                    .map(|r| serde_json::from_str::<Entity>(&r.payload))
                    .transpose()?;
                if matches!(source, Some(Entity::Source(s)) if !s.authorized) {
                    return Err(Error::Invalid(
                        "derived content refers to a revoked source".into(),
                    ));
                }
                if let Some(id) = &evidence.record_id {
                    let key =
                        Entity::scoped_key(&record.project_id, &evidence.source_id, "record", id);
                    let rows = StoredEntry::filter(StoredEntry::fields().entity_key().eq(key))
                        .exec(&mut *transaction)
                        .await?;
                    let original = rows
                        .iter()
                        .max_by_key(|r| r.id)
                        .map(|r| serde_json::from_str::<Entity>(&r.payload))
                        .transpose()?;
                    if matches!(original, Some(Entity::Record(r)) if matches!(r.availability, Availability::Missing | Availability::Deleted))
                    {
                        return Err(Error::Invalid(
                            "derived content refers to a removed original".into(),
                        ));
                    }
                }
            }
        }
        if let Entity::Record(record) = &mut entity
            && matches!(
                record.availability,
                Availability::Missing | Availability::Deleted
            )
        {
            erase_record(record, record.availability);
        }
        let payload = serde_json::to_string(&entity)?;
        if let Some(row) = revisions.last().filter(|r| r.payload == payload) {
            return Ok(Receipt {
                sequence: row.id,
                entity_id: entity.id().into(),
                duplicate: true,
                durable: true,
                compaction: None,
            });
        }
        // Include the previous sequence so A -> B -> A is a real revision, while
        // concurrent delivery against the same predecessor remains idempotent.
        let event_key =
            hash(format!("{}:{payload}", revisions.last().map_or(0, |r| r.id)).as_bytes());
        // Source revocation and explicit deletion scrub historical revisions too.
        Self::scrub_for(&mut *transaction, &entity).await?;
        let captured_at = Utc::now().to_rfc3339();
        let inserted = toasty::create!(StoredEntry {
            event_key: event_key.clone(),
            entity_key,
            captured_at,
            payload,
        })
        .exec(&mut *transaction)
        .await;
        let receipt: Result<Receipt> = match inserted {
            Ok(row) => Ok(Receipt {
                sequence: row.id,
                entity_id: entity.id().into(),
                duplicate: false,
                durable: true,
                compaction: None,
            }),
            Err(error) => {
                // Another process may have delivered exactly this event concurrently.
                let rows = StoredEntry::filter(StoredEntry::fields().event_key().eq(event_key))
                    .exec(&mut *transaction)
                    .await?;
                if let Some(row) = rows.first() {
                    Ok(Receipt {
                        sequence: row.id,
                        entity_id: entity.id().into(),
                        duplicate: true,
                        durable: true,
                        compaction: None,
                    })
                } else {
                    Err(error.into())
                }
            }
        };
        receipt
    }

    /// A batch is collected only after all of its evidence links are present.
    pub async fn append_all(
        &mut self,
        entities: impl IntoIterator<Item = Entity>,
    ) -> Result<Vec<Receipt>> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let mut receipts = Vec::new();
        let mut pinned = BTreeSet::new();
        for entity in entities {
            let receipt = Self::append_one(&mut transaction, &self.policy, entity).await?;
            pinned.insert(receipt.sequence);
            receipts.push(receipt);
        }
        if let Some(last) = receipts.last_mut() {
            last.compaction = retention::enforce(&mut transaction, &pinned).await?;
        }
        transaction.commit().await?;
        Ok(receipts)
    }

    pub async fn load(&mut self) -> Result<Corpus> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let rows = StoredEntry::all().exec(&mut transaction).await?;
        let mut entries = Vec::new();
        let mut compaction = CompactionState::default();
        for row in rows {
            if retention::is_metadata(&row) {
                compaction = retention::metadata(&row)?.compaction;
                continue;
            }
            entries.push(Entry {
                sequence: row.id,
                captured_at: row.captured_at,
                entity: serde_json::from_str(&row.payload)?,
            });
        }
        entries.sort_by_key(|e| e.sequence);
        transaction.commit().await?;
        Ok(Corpus {
            entries,
            compaction,
        })
    }

    /// Preview by default at the CLI; applying changes rows and policy atomically.
    pub async fn compact(
        &mut self,
        policy: Option<compaction::Policy>,
        apply: bool,
    ) -> Result<compaction::Report> {
        let _guard = format::lock(&self.path, false).await?;
        let mut transaction = self.db.transaction().await?;
        format::fence(&mut transaction, &self.header).await?;
        let report = retention::compact(&mut transaction, policy, apply).await?;
        transaction.commit().await?;
        Ok(report)
    }

    pub async fn latest(&mut self) -> Result<Vec<Entity>> {
        let mut latest = BTreeMap::new();
        for entry in self.load().await?.entries {
            latest.insert(entry.entity.key(), entry.entity);
        }
        Ok(latest.into_values().collect())
    }

    async fn scrub_for(db: &mut toasty::Transaction<'_>, change: &Entity) -> Result<()> {
        let revoked_source = match change {
            Entity::Source(s) if !s.authorized => Some(s.id.as_str()),
            _ => None,
        };
        let deleted_record = match change {
            Entity::Record(r)
                if matches!(
                    r.availability,
                    Availability::Deleted | Availability::Missing
                ) =>
            {
                Some(r.id.as_str())
            }
            _ => None,
        };
        let availability = match change {
            Entity::Record(r) => r.availability,
            _ => Availability::Missing,
        };
        if revoked_source.is_none() && deleted_record.is_none() {
            return Ok(());
        }
        let mut rows = StoredEntry::all().exec(&mut *db).await?;
        rows.retain(|row| !retention::is_metadata(row));
        let mut erased: BTreeSet<(String, String)> = deleted_record
            .into_iter()
            .map(|id| (change.source_id().to_owned(), id.to_owned()))
            .collect();
        loop {
            let before = erased.len();
            for row in &rows {
                let entity: Entity = serde_json::from_str(&row.payload)?;
                if entity.project_id() != change.project_id() {
                    continue;
                }
                if let Entity::Record(record) = entity
                    && (revoked_source == Some(record.source_id.as_str())
                        || record.evidence.iter().any(|e| {
                            revoked_source == Some(e.source_id.as_str())
                                || e.record_id.as_ref().is_some_and(|id| {
                                    erased.contains(&(e.source_id.clone(), id.clone()))
                                })
                        }))
                {
                    erased.insert((record.source_id, record.id));
                }
            }
            if before == erased.len() {
                break;
            }
        }
        for row in &mut rows {
            let mut entity: Entity = serde_json::from_str(&row.payload)?;
            if entity.project_id() != change.project_id() {
                continue;
            }
            let source_match = revoked_source == Some(entity.source_id());
            match &mut entity {
                Entity::Record(r) => {
                    if source_match || erased.contains(&(r.source_id.clone(), r.id.clone())) {
                        erase_record(r, availability);
                    }
                }
                Entity::Work(w)
                    if source_match
                        || w.evidence.iter().any(|e| {
                            revoked_source == Some(e.source_id.as_str())
                                || e.record_id.as_ref().is_some_and(|id| {
                                    erased.contains(&(e.source_id.clone(), id.clone()))
                                })
                        }) =>
                {
                    w.goal.clear();
                    w.title.clear();
                    w.completion_conditions.clear();
                    w.evidence.clear();
                }
                Entity::Relation(r)
                    if source_match
                        || r.evidence.iter().any(|e| {
                            revoked_source == Some(e.source_id.as_str())
                                || e.record_id.as_ref().is_some_and(|id| {
                                    erased.contains(&(e.source_id.clone(), id.clone()))
                                })
                        }) =>
                {
                    r.evidence.clear();
                    r.applies_to.clear();
                }
                Entity::Commit(c) if source_match => {
                    c.message.clear();
                    c.paths.clear();
                }
                Entity::CodeState(c) if source_match => c.files.clear(),
                _ => {}
            }
            row.update()
                .payload(serde_json::to_string(&entity)?)
                .exec(&mut *db)
                .await?;
        }
        Ok(())
    }
}

fn erase_record(record: &mut Record, availability: Availability) {
    record.body.clear();
    record.title.clear();
    record.alternatives.clear();
    record.execution = None;
    record.paths.clear();
    record.code_refs.clear();
    record.applies_to.clear();
    record.commit_shas.clear();
    for evidence in &mut record.evidence {
        evidence.locator.clear();
        evidence.availability = availability;
    }
    record.availability = availability;
}

fn validate(entity: &Entity) -> Result<()> {
    if entity.id().trim().is_empty()
        || entity.project_id().trim().is_empty()
        || entity.source_id().trim().is_empty()
    {
        return Err(Error::Invalid(
            "entity id, project_id and source_id are required".into(),
        ));
    }
    if let Entity::Record(r) = entity {
        if r.revision.is_empty() {
            return Err(Error::Invalid("record revision is required".into()));
        }
        if r.derived && r.evidence.is_empty() {
            return Err(Error::Invalid(
                "derived records require versioned evidence".into(),
            ));
        }
        if r.kind != RecordKind::Decision && r.decision_status.is_some()
            || r.kind != RecordKind::Attempt && r.attempt_outcome.is_some()
            || r.kind != RecordKind::Verification && r.verification_outcome.is_some()
        {
            return Err(Error::Invalid(
                "status filter field does not apply to record kind".into(),
            ));
        }
        if let Some(t) = &r.occurred_at {
            chrono::DateTime::parse_from_rfc3339(t)
                .map_err(|_| Error::Invalid("occurred_at must be RFC3339".into()))?;
        }
        for evidence in &r.evidence {
            if evidence.revision.is_empty() {
                return Err(Error::Invalid("evidence revision is required".into()));
            }
        }
    }
    Ok(())
}
