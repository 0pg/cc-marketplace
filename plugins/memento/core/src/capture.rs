//! Bounded, persisted capture obligations. Existence checks do not prove semantic completeness.
use std::{collections::BTreeSet, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{git::IndexBinding, model::*, security::RedactionPolicy};

const MAX_SESSIONS: usize = 128;
const MAX_EVENTS: usize = 128;
const MAX_RECORDS: usize = 64;
const MAX_TEXT: usize = 2048;
const MAX_STOP_RETRIES: u8 = 2;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid capture checkpoint: {0}")]
    Invalid(String),
    #[error("capture checkpoint capacity exceeded; unresolved obligations were preserved")]
    Capacity,
    #[error("commit context checkpoint is missing, pending, incomplete or stale")]
    CommitNotReady,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub project_id: String,
    pub repository: String,
    pub work_id: String,
    pub session_id: String,
    pub turn_id: String,
}

impl Scope {
    pub fn canonical(mut self) -> Result<Self, Error> {
        for value in [
            &self.project_id,
            &self.work_id,
            &self.session_id,
            &self.turn_id,
        ] {
            identifier(value, 512)?;
        }
        let repository = Path::new(&self.repository)
            .canonicalize()
            .map_err(|_| Error::Invalid("repository must be an existing directory".into()))?;
        if !repository.is_dir() {
            return Err(Error::Invalid("repository must be a directory".into()));
        }
        let repository = crate::git::worktree_root(&repository).unwrap_or(repository);
        self.repository = repository
            .to_str()
            .ok_or_else(|| Error::Invalid("repository must be UTF-8".into()))?
            .into();
        Ok(self)
    }

    fn same_session(&self, other: &Self) -> bool {
        self.project_id == other.project_id
            && self.repository == other.repository
            && self.work_id == other.work_id
            && self.session_id == other.session_id
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    UserPrompt,
    ToolFailure,
    Verification,
    Mutation,
    Commit,
    Handoff,
    Interrupt,
    Compaction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordRef {
    pub source_id: String,
    pub record_id: String,
    pub revision: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Resolution {
    Pending,
    Records { records: Vec<RecordRef> },
    NoNewContext { reason: String },
    CaptureIncomplete { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_id: String,
    pub original_turn_id: String,
    pub kind: EventKind,
    pub detail: String,
    pub opened_after_sequence: u64,
    #[serde(default)]
    pub opened_order: u64,
    pub resolution: Resolution,
    pub commit_binding: Option<IndexBinding>,
    #[serde(default)]
    pub commit_shas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    #[serde(default)]
    sessions: Vec<Session>,
    #[serde(default)]
    next_order: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    scope: Scope,
    events: Vec<Event>,
    stop_attempts: u8,
    closed: bool,
}

struct CaptureSnapshot<'a> {
    frontier: u64,
    order: u64,
    entries: &'a [Entry],
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Open {
        scope: Scope,
        event_id: String,
        kind: EventKind,
        detail: String,
        #[serde(default)]
        commit_binding: Option<IndexBinding>,
    },
    PrepareCommit {
        scope: Scope,
        event_id: String,
        detail: String,
    },
    Status {
        scope: Scope,
    },
    Resolve {
        scope: Scope,
        event_id: String,
        resolution: Resolution,
    },
    Stop {
        scope: Scope,
    },
    CheckCommit {
        scope: Scope,
    },
}

impl Request {
    pub(crate) fn scope(&self) -> &Scope {
        match self {
            Self::Open { scope, .. }
            | Self::PrepareCommit { scope, .. }
            | Self::Status { scope }
            | Self::Resolve { scope, .. }
            | Self::Stop { scope }
            | Self::CheckCommit { scope } => scope,
        }
    }
    pub(crate) fn read_only(&self) -> bool {
        matches!(self, Self::Status { .. } | Self::CheckCommit { .. })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Block,
    CaptureIncomplete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub scope: Scope,
    pub events: Vec<Event>,
    pub pending_event_ids: Vec<String>,
    pub pending_user_prompt: bool,
    pub capture_incomplete_event_ids: Vec<String>,
    #[serde(default)]
    pub omitted_pending_event_ids: usize,
    #[serde(default)]
    pub omitted_capture_incomplete_event_ids: usize,
    pub stop_attempts: u8,
    pub decision: Decision,
    pub reason: String,
    pub durable: bool,
}

impl Reply {
    /// Decisions use all obligations; hook output only samples their identifiers.
    pub fn summarize(mut self) -> Self {
        self.events.clear();
        self.omitted_pending_event_ids = self.pending_event_ids.len().saturating_sub(8);
        self.omitted_capture_incomplete_event_ids =
            self.capture_incomplete_event_ids.len().saturating_sub(8);
        self.pending_event_ids.truncate(8);
        self.capture_incomplete_event_ids.truncate(8);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitCheckpoint {
    pub scope: Scope,
    pub event_id: String,
    pub binding: IndexBinding,
}

fn text(value: &str, limit: usize) -> Result<(), Error> {
    if value.trim().is_empty() || value.contains('\0') || value.chars().count() > limit {
        return Err(Error::Invalid(format!(
            "text must be nonempty and at most {limit} characters"
        )));
    }
    Ok(())
}

fn identifier(value: &str, limit: usize) -> Result<(), Error> {
    text(value, limit)?;
    if value.len() > limit || value.chars().any(char::is_control) {
        return Err(Error::Invalid(format!(
            "identifier must be at most {limit} bytes without control characters"
        )));
    }
    Ok(())
}

fn record_refs_valid(
    scope: &Scope,
    event: &Event,
    records: &[RecordRef],
    entries: &[Entry],
) -> Result<(), Error> {
    if records.is_empty() || records.len() > MAX_RECORDS {
        return Err(Error::Invalid(
            "records must contain 1 to 64 exact references".into(),
        ));
    }
    let mut semantic = false;
    let mut sequences = BTreeSet::new();
    for reference in records {
        identifier(&reference.source_id, 512)?;
        identifier(&reference.record_id, 1024)?;
        identifier(&reference.revision, 512)?;
        if !sequences.insert(reference.sequence)
            || reference.sequence <= event.opened_after_sequence
        {
            return Err(Error::Invalid(
                "checkpoint records must be distinct and captured after the event".into(),
            ));
        }
        let record = entries
            .iter()
            .find(|e| e.sequence == reference.sequence)
            .and_then(|e| match &e.entity {
                Entity::Record(r) => Some(r),
                _ => None,
            })
            .ok_or_else(|| Error::Invalid("exact checkpoint record is absent".into()))?;
        let source = entries
            .iter()
            .filter_map(|e| match &e.entity {
                Entity::Source(s)
                    if s.project_id == scope.project_id && s.id == reference.source_id =>
                {
                    Some((e.sequence, s))
                }
                _ => None,
            })
            .max_by_key(|(sequence, _)| *sequence)
            .map(|(_, s)| s);
        if record.project_id != scope.project_id
            || record.source_id != reference.source_id
            || record.id != reference.record_id
            || record.revision != reference.revision
            || !matches!(
                record.availability,
                Availability::Available | Availability::Redacted
            )
            || record.body.trim().is_empty()
            || !source.is_some_and(|s| s.authorized)
            || record.association != Association::Explicit
            || !record.work_ids.contains(&scope.work_id)
            || record.session_id.as_deref() != Some(scope.session_id.as_str())
        {
            return Err(Error::Invalid(
                "checkpoint reference is unavailable, unauthorized or outside its scope".into(),
            ));
        }
        semantic |= matches!(
            record.kind,
            RecordKind::Request
                | RecordKind::Constraint
                | RecordKind::Finding
                | RecordKind::Decision
                | RecordKind::Attempt
                | RecordKind::Change
                | RecordKind::Verification
                | RecordKind::Feedback
        );
    }
    if !semantic {
        return Err(Error::Invalid("a checkpoint needs a semantic context record; status or tool output alone is insufficient".into()));
    }
    Ok(())
}

impl State {
    pub(crate) fn pinned(&self) -> BTreeSet<u64> {
        self.sessions
            .iter()
            .flat_map(|s| &s.events)
            .flat_map(|e| match &e.resolution {
                Resolution::Records { records } => {
                    records.iter().map(|r| r.sequence).collect::<Vec<_>>()
                }
                _ => Vec::new(),
            })
            .collect()
    }

    fn trim(&mut self, entries: &[Entry]) -> Result<(), Error> {
        while self.sessions.len() > MAX_SESSIONS {
            let removable = self.sessions.iter().position(|s| {
                s.closed && s.events.iter().all(|e| Self::settled(&s.scope, e, entries))
            });
            if let Some(index) = removable {
                self.sessions.remove(index);
            } else {
                return Err(Error::Capacity);
            }
        }
        Ok(())
    }

    pub(crate) fn apply(
        &mut self,
        request: Request,
        entries: &[Entry],
        policy: &RedactionPolicy,
    ) -> crate::Result<Reply> {
        let scope = request.scope().clone().canonical()?;
        let existing = self
            .sessions
            .iter()
            .position(|s| s.scope.same_session(&scope));
        if request.read_only() {
            return Ok(self.reply(&scope, entries));
        }
        if existing.is_none() {
            if !matches!(
                request,
                Request::Open { .. } | Request::PrepareCommit { .. } | Request::Stop { .. }
            ) {
                return Err(Error::Invalid("capture session is absent".into()).into());
            }
            self.sessions.push(Session {
                scope: scope.clone(),
                events: Vec::new(),
                stop_attempts: 0,
                closed: false,
            });
        }
        self.trim(entries)?;
        let order = if matches!(
            request,
            Request::Open { .. } | Request::PrepareCommit { .. }
        ) {
            self.next_order = self
                .next_order
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("capture event order exhausted".into()))?;
            self.next_order
        } else {
            self.next_order
        };
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.scope.same_session(&scope))
            .ok_or_else(|| Error::Invalid("capture session is absent".into()))?;
        let frontier = entries.iter().map(|e| e.sequence).max().unwrap_or(0);
        match request {
            Request::Open {
                event_id,
                kind,
                detail,
                commit_binding,
                ..
            } => {
                Self::open(
                    session,
                    &scope,
                    event_id,
                    kind,
                    policy.redact(&detail)?,
                    commit_binding,
                    CaptureSnapshot {
                        frontier,
                        order,
                        entries,
                    },
                )?;
            }
            Request::PrepareCommit {
                event_id, detail, ..
            } => {
                let binding = crate::git::index_binding(Path::new(&scope.repository))
                    .map_err(|e| crate::Error::Invalid(e.to_string()))?;
                Self::open(
                    session,
                    &scope,
                    event_id,
                    EventKind::Commit,
                    policy.redact(&detail)?,
                    Some(binding),
                    CaptureSnapshot {
                        frontier,
                        order,
                        entries,
                    },
                )?;
            }
            Request::Resolve {
                event_id,
                mut resolution,
                ..
            } => {
                let event = session
                    .events
                    .iter_mut()
                    .find(|e| e.event_id == event_id)
                    .ok_or_else(|| Error::Invalid("capture event is absent".into()))?;
                match &mut resolution {
                    Resolution::Pending => {
                        return Err(Error::Invalid(
                            "resolve cannot reset an event to pending".into(),
                        )
                        .into());
                    }
                    Resolution::Records { records } => {
                        record_refs_valid(&scope, event, records, entries)?
                    }
                    Resolution::NoNewContext { reason } => {
                        if event.kind == EventKind::Commit {
                            return Err(Error::Invalid(
                                "commit checkpoints require context records".into(),
                            )
                            .into());
                        }
                        text(reason, MAX_TEXT)?;
                        *reason = policy.redact(reason)?;
                    }
                    Resolution::CaptureIncomplete { reason } => {
                        text(reason, MAX_TEXT)?;
                        *reason = policy.redact(reason)?;
                    }
                }
                if !matches!(
                    event.resolution,
                    Resolution::Pending | Resolution::CaptureIncomplete { .. }
                ) && event.resolution != resolution
                {
                    return Err(Error::Invalid("a resolved event cannot be replaced".into()).into());
                }
                event.resolution = resolution;
            }
            Request::Stop { .. } => {
                if session
                    .events
                    .iter()
                    .any(|e| matches!(e.resolution, Resolution::Pending))
                {
                    if session.stop_attempts < MAX_STOP_RETRIES {
                        session.stop_attempts += 1;
                    } else {
                        for event in &mut session.events {
                            if matches!(event.resolution, Resolution::Pending) {
                                event.resolution = Resolution::CaptureIncomplete {
                                    reason: "capture remained incomplete after two Stop retries"
                                        .into(),
                                };
                            }
                        }
                        session.closed = true;
                    }
                } else {
                    session.closed = true;
                }
            }
            Request::Status { .. } | Request::CheckCommit { .. } => {}
        }
        Ok(self.reply(&scope, entries))
    }

    fn open(
        session: &mut Session,
        scope: &Scope,
        event_id: String,
        kind: EventKind,
        detail: String,
        binding: Option<IndexBinding>,
        snapshot: CaptureSnapshot<'_>,
    ) -> Result<(), Error> {
        identifier(&event_id, 256)?;
        text(&detail, MAX_TEXT)?;
        if (kind == EventKind::Commit) != binding.is_some() {
            return Err(Error::Invalid(
                "commit binding is required only for commit events".into(),
            ));
        }
        if let Some(binding) = &binding {
            identifier(&binding.staged_tree, 128)?;
            if let Some(parent) = &binding.parent_head {
                identifier(parent, 128)?;
            }
        }
        if let Some(existing) = session.events.iter().find(|e| e.event_id == event_id) {
            if existing.kind != kind
                || existing.detail != detail
                || existing.commit_binding != binding
                || existing.original_turn_id != scope.turn_id
            {
                return Err(Error::Invalid(
                    "event ID already identifies different input".into(),
                ));
            }
            return Ok(());
        }
        if session.events.len() >= MAX_EVENTS {
            let settled = session
                .events
                .iter()
                .position(|e| Self::settled(scope, e, snapshot.entries));
            if let Some(index) = settled {
                session.events.remove(index);
            } else {
                return Err(Error::Capacity);
            }
        }
        session.events.push(Event {
            event_id,
            original_turn_id: scope.turn_id.clone(),
            kind,
            detail,
            opened_after_sequence: snapshot.frontier,
            opened_order: snapshot.order,
            resolution: Resolution::Pending,
            commit_binding: binding,
            commit_shas: Vec::new(),
        });
        if kind == EventKind::UserPrompt {
            session.stop_attempts = 0;
        }
        session.closed = false;
        Ok(())
    }

    fn settled(scope: &Scope, event: &Event, entries: &[Entry]) -> bool {
        let resolved = match &event.resolution {
            Resolution::NoNewContext { .. } => true,
            Resolution::Records { records } => {
                record_refs_valid(scope, event, records, entries).is_ok()
            }
            _ => false,
        };
        resolved && (event.kind != EventKind::Commit || !event.commit_shas.is_empty())
    }

    fn reply(&self, scope: &Scope, entries: &[Entry]) -> Reply {
        let session = self.sessions.iter().find(|s| s.scope.same_session(scope));
        let events = session.map_or_else(Vec::new, |s| s.events.clone());
        let mut pending = Vec::new();
        let mut pending_user_prompt = false;
        let mut incomplete = Vec::new();
        for event in &events {
            match &event.resolution {
                Resolution::Pending => {
                    pending.push(event.event_id.clone());
                    pending_user_prompt |= event.kind == EventKind::UserPrompt;
                }
                Resolution::CaptureIncomplete { .. } => incomplete.push(event.event_id.clone()),
                Resolution::Records { records }
                    if record_refs_valid(scope, event, records, entries).is_err() =>
                {
                    incomplete.push(event.event_id.clone())
                }
                _ => {}
            }
        }
        let decision = if !pending.is_empty() {
            Decision::Block
        } else if !incomplete.is_empty() {
            Decision::CaptureIncomplete
        } else {
            Decision::Allow
        };
        Reply { scope: scope.clone(), events, pending_event_ids: pending, pending_user_prompt, capture_incomplete_event_ids: incomplete,
            omitted_pending_event_ids: 0, omitted_capture_incomplete_event_ids: 0,
            stop_attempts: session.map_or(0, |s| s.stop_attempts), decision,
            reason: match decision { Decision::Allow => "all known capture obligations are resolved; semantic completeness is not proven", Decision::Block => "save scoped context records for pending events, then resolve their exact references", Decision::CaptureIncomplete => "capture is incomplete; report the unsaved gap and do not claim persistence" }.into(), durable: true }
    }

    pub(crate) fn check_commit(
        &self,
        project: &str,
        repository: &str,
        binding: &IndexBinding,
        entries: &[Entry],
    ) -> Result<CommitCheckpoint, Error> {
        self.matching_commit(project, repository, binding, entries, true, None)
    }

    pub(crate) fn check_commit_scoped(
        &self,
        scope: &Scope,
        binding: &IndexBinding,
        entries: &[Entry],
    ) -> Result<CommitCheckpoint, Error> {
        self.matching_commit(
            &scope.project_id,
            &scope.repository,
            binding,
            entries,
            true,
            Some(scope),
        )
    }

    pub(crate) fn matching_commit(
        &self,
        project: &str,
        repository: &str,
        binding: &IndexBinding,
        entries: &[Entry],
        require_ready: bool,
        scope: Option<&Scope>,
    ) -> Result<CommitCheckpoint, Error> {
        let (session, event) = self
            .sessions
            .iter()
            .filter(|s| {
                s.scope.project_id == project
                    && s.scope.repository == repository
                    && scope.is_none_or(|scope| s.scope.same_session(scope))
            })
            .flat_map(|session| session.events.iter().map(move |event| (session, event)))
            .filter(|(_, event)| {
                event.kind == EventKind::Commit && event.commit_binding.as_ref() == Some(binding)
            })
            .max_by_key(|(_, event)| event.opened_order)
            .ok_or(Error::CommitNotReady)?;
        if (require_ready && self.reply(&session.scope, entries).decision != Decision::Allow)
            || !matches!(&event.resolution, Resolution::Records { records } if record_refs_valid(&session.scope, event, records, entries).is_ok())
        {
            return Err(Error::CommitNotReady);
        }
        Ok(CommitCheckpoint {
            scope: session.scope.clone(),
            event_id: event.event_id.clone(),
            binding: binding.clone(),
        })
    }

    pub(crate) fn link_commit(
        &mut self,
        checkpoint: &CommitCheckpoint,
        sha: &str,
    ) -> Result<(), Error> {
        text(sha, 128)?;
        let event = self
            .sessions
            .iter_mut()
            .find(|s| s.scope.same_session(&checkpoint.scope))
            .and_then(|s| {
                s.events
                    .iter_mut()
                    .find(|e| e.event_id == checkpoint.event_id)
            })
            .ok_or(Error::CommitNotReady)?;
        if !event.commit_shas.iter().any(|old| old == sha) {
            if event.commit_shas.len() >= MAX_RECORDS {
                return Err(Error::Capacity);
            }
            event.commit_shas.push(sha.into());
        }
        Ok(())
    }
}
