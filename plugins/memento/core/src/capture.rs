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
    Investigation,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<RecordRef>,
}

pub(crate) const OBSERVATION_SOURCE: &str = "memento-checkpoints";

pub(crate) fn context_id(scope: &Scope, event_id: &str) -> String {
    let parts = [
        &scope.project_id,
        &scope.repository,
        &scope.work_id,
        &scope.session_id,
        &scope.turn_id,
        event_id,
    ];
    let key = parts
        .iter()
        .map(|part| format!("{}:{part}", part.len()))
        .collect::<String>();
    format!("checkpoint:{}", crate::security::hash(key.as_bytes()))
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
    #[serde(default)]
    pub pending_investigation: bool,
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
            || !source.is_some_and(|s| s.authorized && (event.context_id.is_none() || s.available))
            || record.association != Association::Explicit
            || !record.work_ids.contains(&scope.work_id)
            || record.session_id.as_deref() != Some(scope.session_id.as_str())
        {
            return Err(Error::Invalid(
                "checkpoint reference is unavailable, unauthorized or outside its scope".into(),
            ));
        }
        if let Some(context) = &event.context_id {
            if record.representation == Representation::Claim {
                crate::store::claims::validate_record(record, entries)
                    .map_err(|error| Error::Invalid(error.to_string()))?;
                if record.context_id.as_ref() != Some(context) || record.partial {
                    return Err(Error::Invalid(
                        "checkpoint claim has another context or incomplete evidence".into(),
                    ));
                }
                let origin = event
                    .origin
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("checkpoint observation is absent".into()))?;
                if !record.evidence.iter().any(|evidence| {
                    evidence.source_id == origin.source_id
                        && evidence.record_id.as_ref() == Some(&origin.record_id)
                        && evidence.revision == origin.revision
                        && matches!(
                            evidence.availability,
                            Availability::Available | Availability::Redacted
                        )
                }) {
                    return Err(Error::Invalid(
                        "checkpoint claim must reference this event's exact observation".into(),
                    ));
                }
                semantic |= event_kind_accepts(event.kind, record);
            }
            continue;
        }
        // Persisted format-1 resolutions retain their original contract.
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

fn event_kind_accepts(event: EventKind, record: &Record) -> bool {
    match event {
        EventKind::UserPrompt => matches!(
            record.kind,
            RecordKind::Request
                | RecordKind::Constraint
                | RecordKind::Feedback
                | RecordKind::Decision
                | RecordKind::Finding
        ),
        EventKind::ToolFailure => {
            record.kind == RecordKind::Finding
                || (record.kind == RecordKind::Attempt
                    && record.attempt_outcome == Some(AttemptOutcome::Failed))
                || (record.kind == RecordKind::Verification
                    && record.verification_outcome == Some(VerificationOutcome::Failed))
        }
        EventKind::Investigation => {
            matches!(record.kind, RecordKind::Finding | RecordKind::Decision)
        }
        EventKind::Verification => {
            record.kind == RecordKind::Verification && record.verification_outcome.is_some()
        }
        EventKind::Mutation => record.kind == RecordKind::Change,
        EventKind::Commit => matches!(record.kind, RecordKind::Decision | RecordKind::Change),
        EventKind::Handoff | EventKind::Interrupt | EventKind::Compaction => matches!(
            record.kind,
            RecordKind::Request
                | RecordKind::Constraint
                | RecordKind::Finding
                | RecordKind::Decision
                | RecordKind::Attempt
                | RecordKind::Change
                | RecordKind::Verification
                | RecordKind::Feedback
        ),
    }
}

fn no_new_context_allowed(event: &Event) -> bool {
    event.kind != EventKind::Commit
        && (event.context_id.is_none()
            || !matches!(
                event.kind,
                EventKind::ToolFailure | EventKind::Verification | EventKind::Mutation
            ))
}

impl State {
    pub(crate) fn scrub(
        &mut self,
        project: &str,
        revoked_source: Option<&str>,
        erased: &BTreeSet<(String, String)>,
    ) -> bool {
        let mut changed = false;
        for session in &mut self.sessions {
            if session.scope.project_id != project {
                continue;
            }
            for event in &mut session.events {
                let removed = |reference: &RecordRef| {
                    revoked_source == Some(reference.source_id.as_str())
                        || erased
                            .contains(&(reference.source_id.clone(), reference.record_id.clone()))
                };
                let affected = event.origin.as_ref().is_some_and(removed)
                    || match &event.resolution {
                        Resolution::Records { records } => records.iter().any(removed),
                        _ => false,
                    };
                if affected {
                    event.detail = "Checkpoint evidence removed or access revoked".into();
                    event.resolution = Resolution::CaptureIncomplete {
                        reason: "Checkpoint evidence removed or access revoked".into(),
                    };
                    changed = true;
                }
            }
        }
        changed
    }

    pub(crate) fn pinned(&self) -> BTreeSet<u64> {
        self.sessions
            .iter()
            .flat_map(|s| &s.events)
            .flat_map(|e| match &e.resolution {
                Resolution::Records { records } => {
                    records.iter().map(|r| r.sequence).collect::<Vec<_>>()
                }
                Resolution::Pending => e.origin.iter().map(|r| r.sequence).collect(),
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
                        if !no_new_context_allowed(event) {
                            return Err(Error::Invalid(
                                "this checkpoint requires context records or an explicit capture gap"
                                    .into(),
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
                let repairable = match &event.resolution {
                    Resolution::Pending | Resolution::CaptureIncomplete { .. } => true,
                    Resolution::NoNewContext { .. } => !no_new_context_allowed(event),
                    Resolution::Records { .. } => false,
                };
                if !repairable && event.resolution != resolution {
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
        let context_id = context_id(scope, &event_id);
        let origin = snapshot
            .entries
            .iter()
            .filter_map(|entry| match &entry.entity {
                Entity::Record(record)
                    if record.project_id == scope.project_id
                        && record.source_id == OBSERVATION_SOURCE
                        && record.id == context_id
                        && record.context_id.as_ref() == Some(&context_id)
                        && record.body == detail
                        && record.representation == Representation::Evidence =>
                {
                    Some(RecordRef {
                        source_id: record.source_id.clone(),
                        record_id: record.id.clone(),
                        revision: record.revision.clone(),
                        sequence: entry.sequence,
                    })
                }
                _ => None,
            })
            .max_by_key(|reference| reference.sequence);
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
            context_id: Some(context_id),
            origin,
        });
        if kind == EventKind::UserPrompt {
            session.stop_attempts = 0;
        }
        session.closed = false;
        Ok(())
    }

    fn settled(scope: &Scope, event: &Event, entries: &[Entry]) -> bool {
        let resolved = match &event.resolution {
            Resolution::NoNewContext { .. } => no_new_context_allowed(event),
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
        let mut pending_investigation = false;
        let mut incomplete = Vec::new();
        for event in &events {
            match &event.resolution {
                Resolution::Pending => {
                    pending.push(event.event_id.clone());
                    pending_user_prompt |= event.kind == EventKind::UserPrompt;
                    pending_investigation |= event.kind == EventKind::Investigation;
                }
                Resolution::CaptureIncomplete { .. } => incomplete.push(event.event_id.clone()),
                Resolution::NoNewContext { .. } if !no_new_context_allowed(event) => {
                    incomplete.push(event.event_id.clone())
                }
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
        Reply { scope: scope.clone(), events, pending_event_ids: pending, pending_user_prompt, pending_investigation, capture_incomplete_event_ids: incomplete,
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

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn fixture(scope: &Scope, kind: EventKind, context_id: Option<String>) -> State {
        State {
            sessions: vec![Session {
                scope: scope.clone(),
                events: vec![Event {
                    event_id: "observed-event".into(),
                    original_turn_id: scope.turn_id.clone(),
                    kind,
                    detail: "관측한 작업 사건".into(),
                    opened_after_sequence: 0,
                    opened_order: 1,
                    resolution: Resolution::Pending,
                    commit_binding: None,
                    commit_shas: Vec::new(),
                    context_id,
                    origin: None,
                }],
                stop_attempts: 0,
                closed: true,
            }],
            next_order: 1,
        }
    }

    fn scope(directory: &tempfile::TempDir) -> Result<Scope, Error> {
        Scope {
            project_id: "upload-app".into(),
            repository: directory.path().to_string_lossy().into_owned(),
            work_id: "upload-429".into(),
            session_id: "codex-session".into(),
            turn_id: "turn-1".into(),
        }
        .canonical()
    }

    #[test]
    fn legacy_no_new_context_contract_is_preserved() -> TestResult {
        let directory = tempfile::tempdir()?;
        let scope = scope(&directory)?;
        for kind in [
            EventKind::ToolFailure,
            EventKind::Verification,
            EventKind::Mutation,
        ] {
            let mut state = fixture(&scope, kind, None);
            let reply = state.apply(
                Request::Resolve {
                    scope: scope.clone(),
                    event_id: "observed-event".into(),
                    resolution: Resolution::NoNewContext {
                        reason: "구형 체크포인트의 기존 해소 상태".into(),
                    },
                },
                &[],
                &RedactionPolicy::default(),
            )?;
            assert_eq!(reply.decision, Decision::Allow);
            let event = reply.events.first().ok_or("missing legacy event")?;
            assert!(State::settled(&scope, event, &[]));
            assert!(event.context_id.is_none());
        }
        Ok(())
    }

    #[test]
    fn legacy_record_resolution_keeps_its_source_availability_contract() -> TestResult {
        let directory = tempfile::tempdir()?;
        let scope = scope(&directory)?;
        let mut state = fixture(&scope, EventKind::ToolFailure, None);
        let mut source = crate::ingest::source("journal", &scope.project_id, SourceKind::Journal);
        source.available = false;
        let mut record = Record::new(
            "failure",
            &scope.project_id,
            "journal",
            RecordKind::Attempt,
            "업로드가 HTTP 429로 실패했다.",
        );
        record.association = Association::Explicit;
        record.work_ids = vec![scope.work_id.clone()];
        record.session_id = Some(scope.session_id.clone());
        let reference = RecordRef {
            source_id: record.source_id.clone(),
            record_id: record.id.clone(),
            revision: record.revision.clone(),
            sequence: 2,
        };
        let entries = vec![
            Entry {
                sequence: 1,
                captured_at: "2026-10-04T00:00:00Z".into(),
                entity: Entity::Source(source),
            },
            Entry {
                sequence: 2,
                captured_at: "2026-10-04T00:00:01Z".into(),
                entity: Entity::Record(record),
            },
        ];
        let reply = state.apply(
            Request::Resolve {
                scope: scope.clone(),
                event_id: "observed-event".into(),
                resolution: Resolution::Records {
                    records: vec![reference],
                },
            },
            &entries,
            &RedactionPolicy::default(),
        )?;
        assert_eq!(reply.decision, Decision::Allow);
        assert!(State::settled(
            &scope,
            reply.events.first().ok_or("missing legacy event")?,
            &entries,
        ));
        Ok(())
    }

    #[test]
    fn persisted_native_no_new_context_is_incomplete_unsettled_and_repairable() -> TestResult {
        let directory = tempfile::tempdir()?;
        let scope = scope(&directory)?;
        for kind in [
            EventKind::ToolFailure,
            EventKind::Verification,
            EventKind::Mutation,
        ] {
            let mut state = fixture(&scope, kind, Some("native-context".into()));
            state
                .sessions
                .first_mut()
                .and_then(|session| session.events.first_mut())
                .ok_or("missing native event")?
                .resolution = Resolution::NoNewContext {
                reason: "수정 전 구현이 허용한 해소 상태".into(),
            };
            let status = state.reply(&scope, &[]);
            assert_eq!(status.decision, Decision::CaptureIncomplete);
            assert_eq!(status.capture_incomplete_event_ids, vec!["observed-event"]);
            assert!(!State::settled(
                &scope,
                status.events.first().ok_or("missing native event")?,
                &[],
            ));
            let repaired = state.apply(
                Request::Resolve {
                    scope: scope.clone(),
                    event_id: "observed-event".into(),
                    resolution: Resolution::CaptureIncomplete {
                        reason: "의미 레코드가 누락되어 추가 캡처가 필요하다.".into(),
                    },
                },
                &[],
                &RedactionPolicy::default(),
            )?;
            assert_eq!(repaired.decision, Decision::CaptureIncomplete);
            assert!(matches!(
                repaired
                    .events
                    .first()
                    .ok_or("missing repaired event")?
                    .resolution,
                Resolution::CaptureIncomplete { .. }
            ));
        }
        Ok(())
    }
}
