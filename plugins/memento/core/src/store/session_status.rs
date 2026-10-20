//! Passive diagnostics over one retained store snapshot, without source revalidation.
use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct Scope {
    pub project_id: String,
    pub work_id: String,
    pub session_id: String,
}

impl Scope {
    fn validate(&self) -> Result<()> {
        for value in [&self.project_id, &self.work_id, &self.session_id] {
            if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err(Error::Invalid(
                    "session-status requires nonempty project, work and session IDs (max 512 bytes)"
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    NotObserved,
    Observed,
}

#[derive(Debug, Serialize)]
pub struct RecordSummary {
    pub sequence: u64,
    pub captured_at: String,
    pub source_id: String,
    pub source_available: bool,
    pub record_id: String,
    pub revision: String,
    pub kind: RecordKind,
    pub association: Association,
    pub title: String,
    pub representation: Representation,
    pub availability: Availability,
    pub fidelity: Fidelity,
    pub partial: bool,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub store: PathBuf,
    pub scope: Scope,
    pub read_only: bool,
    pub store_format: StoreFormatStatus,
    pub record_count: usize,
    pub last_saved_at: Option<String>,
    pub records: Vec<RecordSummary>,
    pub omitted_records: usize,
    pub checkpoint_state: CheckpointState,
    pub checkpoints: Vec<capture::PersistedSessionStatus>,
    pub compaction: CompactionState,
    pub limitations: Vec<String>,
}

fn summary_title(
    record: &Record,
    latest: &BTreeMap<String, &Entry>,
    authorized: &BTreeSet<&str>,
) -> String {
    let lost_evidence = record.evidence.iter().any(|evidence| {
        !authorized.contains(evidence.source_id.as_str())
            || evidence.availability == Availability::Deleted
            || evidence.record_id.as_ref().is_some_and(|id| {
                let key = Entity::scoped_key(&record.project_id, &evidence.source_id, "record", id);
                matches!(
                    latest.get(&key).map(|entry| &entry.entity),
                    Some(Entity::Record(origin)) if origin.availability == Availability::Deleted
                )
            })
    });
    if record.derived && lost_evidence {
        String::new()
    } else {
        record.title.clone()
    }
}

pub(super) async fn inspect(path: &Path, scope: Scope, limit: usize) -> Result<Report> {
    scope.validate()?;
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("--limit must be between 1 and 100".into()));
    }
    let selected = std::path::absolute(path)?;
    let snapshot = format::passive_snapshot(&selected).await?;
    let mut latest = BTreeMap::<String, &Entry>::new();
    for entry in &snapshot.entries {
        if entry.entity.project_id() == scope.project_id {
            let saved = latest.entry(entry.entity.key()).or_insert(entry);
            if entry.sequence > saved.sequence {
                *saved = entry;
            }
        }
    }
    let sources = latest
        .values()
        .filter_map(|entry| match &entry.entity {
            Entity::Source(source) if source.authorized => Some((source.id.as_str(), source)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let authorized = sources.keys().copied().collect::<BTreeSet<_>>();
    let mut records = latest
        .values()
        .filter_map(|entry| match &entry.entity {
            Entity::Record(record)
                if authorized.contains(record.source_id.as_str())
                    && record.work_ids.contains(&scope.work_id)
                    && record.session_id.as_deref() == Some(&scope.session_id)
                    && record.availability != Availability::Deleted =>
            {
                Some(RecordSummary {
                    sequence: entry.sequence,
                    captured_at: entry.captured_at.clone(),
                    source_id: record.source_id.clone(),
                    source_available: sources
                        .get(record.source_id.as_str())
                        .is_some_and(|source| source.available),
                    record_id: record.id.clone(),
                    revision: record.revision.clone(),
                    kind: record.kind,
                    association: record.association,
                    title: summary_title(record, &latest, &authorized),
                    representation: record.representation,
                    availability: record.availability,
                    fidelity: record.fidelity,
                    partial: record.partial,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    records.sort_by_key(|record| std::cmp::Reverse(record.sequence));
    let record_count = records.len();
    let last_saved_at = records.first().map(|record| record.captured_at.clone());
    records.truncate(limit);
    let checkpoints = snapshot.capture.persisted_session_status(
        &scope.project_id,
        &scope.work_id,
        &scope.session_id,
        &snapshot.entries,
    );
    Ok(Report {
        store: selected,
        scope,
        read_only: true,
        store_format: snapshot.status,
        record_count,
        last_saved_at,
        omitted_records: record_count.saturating_sub(records.len()),
        records,
        checkpoint_state: if checkpoints.is_empty() {
            CheckpointState::NotObserved
        } else {
            CheckpointState::Observed
        },
        checkpoints,
        compaction: snapshot.compaction,
        limitations: vec![
            "Only this selected store and exact project/work/session were inspected; unauthorized sources and deleted records are excluded.".into(),
            "Source access and availability are the persisted state; original files were not revalidated.".into(),
            "Derived titles are withheld when their evidence is inaccessible or deleted.".into(),
            "Stored checkpoint observations do not establish live hook activation; retained records and resolved obligations do not prove semantic completeness.".into(),
        ],
    })
}
