//! Bounded storage with one in-place metadata row and transactional collection.
use super::*;

// Entity::scoped_key starts with a decimal length, so no public Entity can use this key.
pub(super) const METADATA_KEY: &str = "__work_context_compaction_v1__";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Metadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_format: Option<format::Header>,
    pub policy: compaction::Policy,
    pub compaction: CompactionState,
    #[serde(default)]
    pub capture: capture::State,
}

fn entries(rows: &[StoredEntry]) -> Result<Vec<Entry>> {
    rows.iter()
        .filter(|row| !is_metadata(row))
        .map(|row| {
            Ok(Entry {
                sequence: row.id,
                captured_at: row.captured_at.clone(),
                entity: serde_json::from_str(&row.payload)?,
            })
        })
        .collect()
}

fn canonical_repository(repository: &Path) -> Result<String> {
    let path = repository.canonicalize()?;
    crate::git::worktree_root(&path)
        .unwrap_or(path)
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid("repository must be UTF-8".into()))
}

pub(super) async fn checkpoint(
    db: &mut toasty::Transaction<'_>,
    request: capture::Request,
    policy: &RedactionPolicy,
) -> Result<capture::Reply> {
    let mut rows = StoredEntry::all().exec(&mut *db).await?;
    let mut meta = read_metadata(&rows)?;
    let read_only = request.read_only();
    let scope = request.scope().clone().canonical()?;
    let observation = match &request {
        capture::Request::Open {
            event_id,
            detail,
            kind,
            ..
        } => Some((event_id, detail, *kind)),
        capture::Request::PrepareCommit {
            event_id, detail, ..
        } => Some((event_id, detail, capture::EventKind::Commit)),
        _ => None,
    };
    if let Some((event_id, detail, kind)) = observation {
        let source = entries(&rows)?
            .into_iter()
            .filter_map(|entry| match entry.entity {
                Entity::Source(source)
                    if source.project_id == scope.project_id
                        && source.id == capture::OBSERVATION_SOURCE =>
                {
                    Some((entry.sequence, source))
                }
                _ => None,
            })
            .max_by_key(|(sequence, _)| *sequence)
            .map(|(_, source)| source);
        if let Some(source) = source {
            if !source.authorized || !source.available || source.kind != SourceKind::Codex {
                return Err(Error::Invalid(
                    "checkpoint observation source is unavailable or conflicting".into(),
                ));
            }
        } else {
            let mut source = crate::ingest::source(
                capture::OBSERVATION_SOURCE,
                &scope.project_id,
                SourceKind::Codex,
            );
            source.name = "Memento bounded checkpoint observations".into();
            source.gaps.push("Only explicitly observed checkpoint details are captured; this is not a complete transcript".into());
            Store::append_one(db, policy, Entity::Source(source)).await?;
        }
        let context = capture::context_id(&scope, event_id);
        let mut record = Record::new(
            &context,
            &scope.project_id,
            capture::OBSERVATION_SOURCE,
            RecordKind::ToolResult,
            detail,
        );
        record.representation = Representation::Evidence;
        record.context_id = Some(context);
        record.association = Association::Explicit;
        record.work_ids = vec![scope.work_id.clone()];
        record.session_id = Some(scope.session_id.clone());
        record.actor = Some(Actor {
            kind: match kind {
                capture::EventKind::UserPrompt => ActorKind::Human,
                capture::EventKind::ToolFailure
                | capture::EventKind::Verification
                | capture::EventKind::Mutation => ActorKind::Tool,
                _ => ActorKind::Agent,
            },
            name: None,
        });
        record.partial =
            detail.contains("[prompt truncated;") || detail.contains("[capture detail truncated;");
        if record.partial {
            record.fidelity = Fidelity::SourceTruncated;
        }
        Store::append_one(db, policy, Entity::Record(record)).await?;
        rows = StoredEntry::all().exec(&mut *db).await?;
    }
    let all = entries(&rows)?;
    let mut commit_not_ready = false;
    if matches!(request, capture::Request::CheckCommit { .. }) {
        let binding = crate::git::index_binding(Path::new(&scope.repository))
            .map_err(|e| Error::Invalid(e.to_string()))?;
        match meta.capture.check_commit_scoped(&scope, &binding, &all) {
            Ok(_) => {}
            Err(capture::Error::CommitNotReady) => commit_not_ready = true,
            Err(error) => return Err(error.into()),
        }
    }
    let mut reply = meta.capture.apply(request, &all, policy)?;
    if commit_not_ready {
        reply.decision = capture::Decision::Block;
        reply.reason = capture::Error::CommitNotReady.to_string();
    }
    if !read_only {
        save_metadata(db, &mut rows, &meta).await?;
        enforce(db, &BTreeSet::new()).await?;
    }
    Ok(reply)
}

pub(super) async fn check_commit(
    db: &mut toasty::Transaction<'_>,
    project: &str,
    repository: &Path,
    binding: &crate::git::IndexBinding,
) -> Result<capture::CommitCheckpoint> {
    let rows = StoredEntry::all().exec(&mut *db).await?;
    Ok(read_metadata(&rows)?.capture.check_commit(
        project,
        &canonical_repository(repository)?,
        binding,
        &entries(&rows)?,
    )?)
}

pub(super) async fn link_commit(
    db: &mut toasty::Transaction<'_>,
    project: &str,
    repository: &Path,
    binding: &crate::git::IndexBinding,
    sha: &str,
) -> Result<()> {
    let mut rows = StoredEntry::all().exec(&mut *db).await?;
    let mut meta = read_metadata(&rows)?;
    let checkpoint = meta.capture.matching_commit(
        project,
        &canonical_repository(repository)?,
        binding,
        &entries(&rows)?,
        false,
        None,
    )?;
    meta.capture.link_commit(&checkpoint, sha)?;
    save_metadata(db, &mut rows, &meta).await?;
    enforce(db, &BTreeSet::new()).await?;
    Ok(())
}

pub(super) fn is_metadata(row: &StoredEntry) -> bool {
    row.entity_key == METADATA_KEY
}

pub(super) fn metadata(row: &StoredEntry) -> Result<Metadata> {
    let value: Metadata = serde_json::from_str(&row.payload)?;
    value.policy.validate()?;
    Ok(value)
}

pub(super) fn read_metadata(rows: &[StoredEntry]) -> Result<Metadata> {
    rows.iter()
        .find(|row| is_metadata(row))
        .map(metadata)
        .transpose()
        .map(|value| value.unwrap_or_default())
}

fn add_size(total: &mut usize, amount: usize) -> Result<()> {
    *total = total
        .checked_add(amount)
        .ok_or_else(|| Error::Invalid("storage size overflow".into()))?;
    Ok(())
}

fn usage(rows: &[StoredEntry]) -> Result<compaction::Usage> {
    let mut payload_bytes = 0;
    for row in rows {
        add_size(&mut payload_bytes, row.payload.len())?;
    }
    Ok(compaction::Usage {
        entries: rows.len(),
        payload_bytes,
    })
}

fn fits(usage: &compaction::Usage, policy: &compaction::Policy) -> bool {
    usage.entries <= policy.max_entries && usage.payload_bytes <= policy.max_payload_bytes
}

fn capacity(usage: &compaction::Usage, policy: &compaction::Policy) -> Error {
    Error::Capacity {
        entries: usage.entries,
        payload_bytes: usage.payload_bytes,
        max_entries: policy.max_entries,
        max_payload_bytes: policy.max_payload_bytes,
    }
}

pub(super) async fn save_metadata(
    db: &mut toasty::Transaction<'_>,
    rows: &mut [StoredEntry],
    value: &Metadata,
) -> Result<()> {
    let payload = serde_json::to_string(value)?;
    if let Some(row) = rows.iter_mut().find(|row| is_metadata(row)) {
        if row.payload != payload {
            row.update().payload(payload).exec(db).await?;
        }
    } else {
        toasty::create!(StoredEntry {
            event_key: METADATA_KEY.to_owned(),
            entity_key: METADATA_KEY.to_owned(),
            captured_at: Utc::now().to_rfc3339(),
            payload,
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn enforce(
    db: &mut toasty::Transaction<'_>,
    pinned: &BTreeSet<u64>,
) -> Result<Option<CompactionNotice>> {
    let mut rows = StoredEntry::all().exec(&mut *db).await?;
    let meta = read_metadata(&rows)?;
    let mut current = usage(&rows)?;
    if !rows.iter().any(is_metadata) {
        add_size(&mut current.entries, 1)?;
        add_size(&mut current.payload_bytes, serde_json::to_vec(&meta)?.len())?;
    }
    if fits(&current, &meta.policy) {
        save_metadata(db, &mut rows, &meta).await?;
        return Ok(None);
    }
    let report = compact_pinned(db, None, true, pinned).await?;
    let updated = StoredEntry::filter(StoredEntry::fields().entity_key().eq(METADATA_KEY))
        .exec(&mut *db)
        .await?;
    let generation = read_metadata(&updated)?.compaction.generation;
    Ok(Some(CompactionNotice {
        generation,
        removed_entries: report.removed_entries,
        remaining: report.after,
    }))
}

pub(super) async fn compact(
    db: &mut toasty::Transaction<'_>,
    policy: Option<compaction::Policy>,
    apply: bool,
) -> Result<compaction::Report> {
    compact_pinned(db, policy, apply, &BTreeSet::new()).await
}

async fn compact_pinned(
    db: &mut toasty::Transaction<'_>,
    policy: Option<compaction::Policy>,
    apply: bool,
    pinned: &BTreeSet<u64>,
) -> Result<compaction::Report> {
    let mut rows = StoredEntry::all().exec(&mut *db).await?;
    let mut meta = read_metadata(&rows)?;
    if let Some(policy) = policy {
        policy.validate()?;
        meta.policy = policy;
    }
    let entries = entries(&rows)?;
    let existing: BTreeSet<_> = entries.iter().map(|entry| entry.sequence).collect();
    let mut protected = pinned.clone();
    protected.extend(meta.capture.pinned().intersection(&existing));
    let mut plan = compaction::plan_pinned(&entries, &meta.policy, &protected)?;
    if plan.report.removed_entries > 0 {
        meta.compaction.generation = meta
            .compaction
            .generation
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("compaction generation exhausted".into()))?;
        meta.compaction.removed_entries = meta
            .compaction
            .removed_entries
            .checked_add(
                u64::try_from(plan.report.removed_entries)
                    .map_err(|_| Error::Invalid("compaction count overflow".into()))?,
            )
            .ok_or_else(|| Error::Invalid("compaction count overflow".into()))?;
    }
    // Count all retained JSON, including bounded capture obligations in the metadata row.
    let mut after = compaction::Usage {
        entries: 1,
        payload_bytes: serde_json::to_vec(&meta)?.len(),
    };
    for row in rows
        .iter()
        .filter(|row| !is_metadata(row) && plan.retained_sequences.contains(&row.id))
    {
        add_size(&mut after.entries, 1)?;
        add_size(&mut after.payload_bytes, row.payload.len())?;
    }
    plan.report.before = usage(&rows)?;
    plan.report.fits = fits(&after, &meta.policy);
    plan.report.after = after;
    plan.report.protected = after;
    if !apply {
        return Ok(plan.report);
    }
    if !plan.report.fits {
        return Err(capacity(&plan.report.after, &meta.policy));
    }
    // Keep the surviving sequence IDs unchanged; record removal and epoch in one transaction.
    for row in rows
        .iter()
        .filter(|row| !is_metadata(row) && !plan.retained_sequences.contains(&row.id))
    {
        StoredEntry::filter(StoredEntry::fields().id().eq(row.id))
            .delete()
            .exec(&mut *db)
            .await?;
    }
    save_metadata(db, &mut rows, &meta).await?;
    tracing::info!(
        removed_entries = plan.report.removed_entries,
        generation = meta.compaction.generation,
        "work context compacted"
    );
    Ok(plan.report)
}
