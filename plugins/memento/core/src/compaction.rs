//! Deterministic retention rules over captured facts and their explicit dependencies.
//!
//! The planner never rewrites evidence or infers facts from prose. Its fixed point
//! is finite because every retained sequence belongs to the supplied snapshot.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::*;

mod datalog;

const RULE_VERSION: &str = "crepe-fixed-point-v2";
const MAX_REASONS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub max_entries: usize,
    pub max_payload_bytes: usize,
    pub recent_entries: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_payload_bytes: 64 * 1024 * 1024,
            recent_entries: 128,
        }
    }
}

impl Policy {
    pub fn validate(&self) -> Result<(), Error> {
        if self.max_entries == 0 || self.max_payload_bytes == 0 {
            return Err(Error::InvalidPolicy(
                "max_entries and max_payload_bytes must be positive".into(),
            ));
        }
        if self.recent_entries > self.max_entries {
            return Err(Error::InvalidPolicy(
                "recent_entries must not exceed max_entries".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid compaction policy: {0}")]
    InvalidPolicy(String),
    #[error("duplicate captured sequence: {0}")]
    DuplicateSequence(u64),
    #[error("pinned captured sequence is absent: {0}")]
    MissingSequence(u64),
    #[error("compaction payload size overflow")]
    SizeOverflow,
    #[error("cannot serialize retained entity: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Usage {
    pub entries: usize,
    pub payload_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reason {
    pub sequence: u64,
    pub rule: String,
    pub via: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Removal {
    pub sequence: u64,
    pub project_id: String,
    pub source_id: String,
    pub entity_id: String,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub rule_version: String,
    pub policy: Policy,
    pub before: Usage,
    pub after: Usage,
    pub removed_entries: usize,
    pub protected: Usage,
    pub fits: bool,
    pub reasons: Vec<Reason>,
    pub omitted_reasons: usize,
    pub removals: Vec<Removal>,
    pub omitted_removals: usize,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub retained_sequences: BTreeSet<u64>,
    pub report: Report,
}

/// Plan a full compaction. Callers must not apply a plan whose report does not fit.
/// Payload usage counts serialized entities, not SQLite pages or journal bytes.
pub fn plan(entries: &[Entry], policy: &Policy) -> Result<Plan, Error> {
    plan_pinned(entries, policy, &BTreeSet::new())
}

/// Also retain newly accepted writes until the caller's transaction commits.
pub fn plan_pinned(
    entries: &[Entry],
    policy: &Policy,
    pinned: &BTreeSet<u64>,
) -> Result<Plan, Error> {
    policy.validate()?;
    let index = Index::new(entries)?;
    let mut roots = BTreeSet::new();
    for sequence in pinned {
        if !index.entries.contains_key(sequence) {
            return Err(Error::MissingSequence(*sequence));
        }
        roots.insert((*sequence, "incoming_write"));
    }
    for sequence in index.latest.values() {
        if let Some(entry) = index.entries.get(sequence)
            && let Some(rule) = root_rule(&entry.entity)
        {
            roots.insert((*sequence, rule));
        }
    }
    if let Some(sequence) = index.entries.keys().next_back() {
        roots.insert((*sequence, "sequence_frontier"));
    }
    for sequence in index.entries.keys().rev().take(policy.recent_entries) {
        roots.insert((*sequence, "recent_window"));
    }
    let mut dependencies = BTreeSet::new();
    for (sequence, entry) in &index.entries {
        if let Some(latest) = index.latest.get(&entry.entity.key()) {
            dependencies.insert((*sequence, *latest, "latest_entity_revision"));
        }
        for (dependency, rule) in index.dependencies(&entry.entity) {
            dependencies.insert((*sequence, dependency, rule));
        }
        if let Some(relations) = index.strong_relations.get(sequence) {
            for relation in relations {
                dependencies.insert((*sequence, *relation, "strong_relation"));
            }
        }
    }
    let retained = datalog::evaluate(roots, dependencies, index.grouped_dependencies());
    let before = index.usage(index.entries.keys())?;
    let after = index.usage(retained.sequences.iter())?;
    let omitted_reasons = retained
        .sequences
        .len()
        .saturating_sub(retained.reasons.len());
    let removed_entries = before.entries.saturating_sub(after.entries);
    let removals: Vec<_> = index
        .entries
        .iter()
        .filter(|(sequence, _)| !retained.sequences.contains(sequence))
        .take(MAX_REASONS)
        .map(|(sequence, entry)| Removal {
            sequence: *sequence,
            project_id: entry.entity.project_id().into(),
            source_id: entry.entity.source_id().into(),
            entity_id: entry.entity.id().into(),
            rule: if index.latest.get(&entry.entity.key()) == Some(sequence) {
                "unreferenced_history"
            } else {
                "obsolete_revision"
            }
            .into(),
        })
        .collect();
    let omitted_removals = removed_entries.saturating_sub(removals.len());
    let report = Report {
        rule_version: RULE_VERSION.into(),
        policy: policy.clone(),
        before,
        after,
        removed_entries,
        protected: after,
        fits: after.entries <= policy.max_entries
            && after.payload_bytes <= policy.max_payload_bytes,
        reasons: retained.reasons,
        omitted_reasons,
        removals,
        omitted_removals,
    };
    Ok(Plan {
        retained_sequences: retained.sequences,
        report,
    })
}

fn root_rule(entity: &Entity) -> Option<&'static str> {
    match entity {
        Entity::Source(_) => Some("latest_source"),
        Entity::Work(_) => Some("latest_work"),
        Entity::Session(session)
            if matches!(
                session.status,
                SessionStatus::Active | SessionStatus::Unknown
            ) =>
        {
            Some("unfinished_session")
        }
        Entity::Record(record)
            if matches!(
                record.availability,
                Availability::Deleted | Availability::Missing
            ) =>
        {
            Some("removal_tombstone")
        }
        Entity::Record(record)
            if matches!(
                record.kind,
                RecordKind::Request
                    | RecordKind::Constraint
                    | RecordKind::Decision
                    | RecordKind::Feedback
                    | RecordKind::Verification
            ) =>
        {
            Some("durable_record_kind")
        }
        Entity::Record(record)
            if record.kind == RecordKind::Attempt
                && record.attempt_outcome != Some(AttemptOutcome::Succeeded) =>
        {
            Some("failed_or_unresolved_attempt")
        }
        _ => None,
    }
}

type IdIndex<'a> = BTreeMap<(&'a str, &'a str), Vec<u64>>;
type RevisionIndex<'a> = BTreeMap<(&'a str, &'a str, &'a str), Vec<u64>>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DependencyGroup<'a> {
    HistoricalPrivacy(&'a str),
    Execution(&'a str, &'a str, &'a str),
}

#[derive(Default)]
struct Index<'a> {
    entries: BTreeMap<u64, &'a Entry>,
    sizes: BTreeMap<u64, usize>,
    latest: BTreeMap<String, u64>,
    records: IdIndex<'a>,
    artifacts: RevisionIndex<'a>,
    evidence: BTreeMap<(&'a str, &'a str, &'a str, &'a str), Vec<u64>>,
    privacy_lineage: BTreeMap<String, Vec<u64>>,
    works: IdIndex<'a>,
    sessions: IdIndex<'a>,
    code_states: IdIndex<'a>,
    commits: RevisionIndex<'a>,
    commit_shas: IdIndex<'a>,
    attempt_results: RevisionIndex<'a>,
    executions: RevisionIndex<'a>,
    strong_relations: BTreeMap<u64, Vec<u64>>,
}

impl<'a> Index<'a> {
    fn new(entries: &'a [Entry]) -> Result<Self, Error> {
        let mut index = Self::default();
        for entry in entries {
            if index.entries.insert(entry.sequence, entry).is_some() {
                return Err(Error::DuplicateSequence(entry.sequence));
            }
            index
                .sizes
                .insert(entry.sequence, serde_json::to_vec(&entry.entity)?.len());
            index
                .latest
                .entry(entry.entity.key())
                .and_modify(|sequence| *sequence = (*sequence).max(entry.sequence))
                .or_insert(entry.sequence);
            if let Entity::Record(record) = &entry.entity {
                // Scrubbing propagates through any revision of a record, even
                // after its current evidence changes. Keep those old edges.
                if record.evidence.iter().any(|evidence| {
                    evidence.source_id != record.source_id
                        || evidence
                            .record_id
                            .as_ref()
                            .is_some_and(|id| id != &record.id)
                }) {
                    index
                        .privacy_lineage
                        .entry(entry.entity.key())
                        .or_default()
                        .push(entry.sequence);
                }
                index
                    .artifacts
                    .entry((&record.project_id, &record.id, &record.revision))
                    .or_default()
                    .push(entry.sequence);
                index
                    .evidence
                    .entry((
                        &record.project_id,
                        &record.source_id,
                        &record.id,
                        &record.revision,
                    ))
                    .or_default()
                    .push(entry.sequence);
            }
        }
        for sequences in index
            .artifacts
            .values_mut()
            .chain(index.evidence.values_mut())
            .chain(index.privacy_lineage.values_mut())
        {
            sequences.sort_unstable();
        }
        for sequence in index.latest.values() {
            let Some(entry) = index.entries.get(sequence) else {
                continue;
            };
            match &entry.entity {
                Entity::Record(record) => {
                    index
                        .records
                        .entry((&record.project_id, &record.id))
                        .or_default()
                        .push(*sequence);
                    if let Some(attempt) = &record.attempt_id {
                        index
                            .attempt_results
                            .entry((&record.project_id, &record.source_id, attempt))
                            .or_default()
                            .push(*sequence);
                    }
                    if let Some(execution) = &record.execution
                        && !execution.id.is_empty()
                        && matches!(record.kind, RecordKind::Attempt | RecordKind::ToolResult)
                    {
                        index
                            .executions
                            .entry((&record.project_id, &record.source_id, &execution.id))
                            .or_default()
                            .push(*sequence);
                    }
                }
                Entity::Work(work) => index
                    .works
                    .entry((&work.project_id, &work.id))
                    .or_default()
                    .push(*sequence),
                Entity::Session(session) => index
                    .sessions
                    .entry((&session.project_id, &session.id))
                    .or_default()
                    .push(*sequence),
                Entity::CodeState(state) => index
                    .code_states
                    .entry((&state.project_id, &state.id))
                    .or_default()
                    .push(*sequence),
                Entity::Commit(commit) => {
                    index
                        .commits
                        .entry((&commit.project_id, &commit.repository_id, &commit.sha))
                        .or_default()
                        .push(*sequence);
                    index
                        .commit_shas
                        .entry((&commit.project_id, &commit.sha))
                        .or_default()
                        .push(*sequence);
                }
                _ => {}
            }
        }
        for sequence in index.latest.values() {
            let Some(Entry {
                entity: Entity::Relation(relation),
                ..
            }) = index.entries.get(sequence).copied()
            else {
                continue;
            };
            if relation.kind == RelationKind::RelatedTo {
                continue;
            }
            for target in [&relation.from, &relation.to] {
                for endpoint in index.target(&relation.project_id, target) {
                    if index.entries.get(&endpoint).is_some_and(|entry| {
                        matches!(
                            entry.entity,
                            Entity::Record(_) | Entity::CodeState(_) | Entity::Commit(_)
                        )
                    }) {
                        index
                            .strong_relations
                            .entry(endpoint)
                            .or_default()
                            .push(*sequence);
                    }
                }
            }
        }
        Ok(index)
    }

    fn usage<'b>(&self, sequences: impl Iterator<Item = &'b u64>) -> Result<Usage, Error> {
        let mut usage = Usage::default();
        for sequence in sequences {
            if let Some(size) = self.sizes.get(sequence) {
                usage.entries = usage.entries.checked_add(1).ok_or(Error::SizeOverflow)?;
                usage.payload_bytes = usage
                    .payload_bytes
                    .checked_add(*size)
                    .ok_or(Error::SizeOverflow)?;
            }
        }
        Ok(usage)
    }

    fn target(&self, project: &str, target: &Target) -> Vec<u64> {
        match target {
            Target::Record { id } => self.records.get(&(project, id.as_str())),
            Target::Artifact {
                record_id,
                revision,
                ..
            } => self
                .artifacts
                .get(&(project, record_id.as_str(), revision.as_str())),
            Target::Work { id } => self.works.get(&(project, id.as_str())),
            Target::Session { id } => self.sessions.get(&(project, id.as_str())),
            Target::Commit {
                repository_id,
                commit_sha,
            } => self
                .commits
                .get(&(project, repository_id.as_str(), commit_sha.as_str())),
            Target::Code { state_id, .. } => self.code_states.get(&(project, state_id.as_str())),
        }
        .cloned()
        .unwrap_or_default()
    }

    fn evidence(&self, project: &str, evidence: &[Evidence]) -> Vec<(u64, &'static str)> {
        evidence
            .iter()
            .filter_map(|evidence| {
                let id = evidence.record_id.as_deref()?;
                self.evidence.get(&(
                    project,
                    evidence.source_id.as_str(),
                    id,
                    evidence.revision.as_str(),
                ))
            })
            .flatten()
            .map(|sequence| (*sequence, "exact_evidence_revision"))
            .collect()
    }

    fn grouped_dependencies(&self) -> datalog::Groups {
        // Each implicit all-to-all relationship is represented once as members
        // plus one use per record, rather than one edge per pair of records.
        let definitions: BTreeMap<_, _> = self
            .privacy_lineage
            .iter()
            .map(|(key, members)| (DependencyGroup::HistoricalPrivacy(key.as_str()), members))
            .chain(
                self.executions
                    .iter()
                    .map(|(&(project, source, execution), members)| {
                        (
                            DependencyGroup::Execution(project, source, execution),
                            members,
                        )
                    }),
            )
            .collect();
        let mut ids = BTreeMap::new();
        let mut groups = datalog::Groups::default();
        for (id, (key, members)) in definitions.into_iter().enumerate() {
            ids.insert(key, id);
            groups
                .members
                .extend(members.iter().map(|sequence| (id, *sequence)));
        }
        for (sequence, entry) in &self.entries {
            let Entity::Record(record) = &entry.entity else {
                continue;
            };
            if let Some(group) = ids.get(&DependencyGroup::HistoricalPrivacy(&entry.entity.key())) {
                groups
                    .uses
                    .insert((*sequence, *group, "historical_privacy_lineage"));
            }
            if let Some(execution) = &record.execution
                && let Some(group) = ids.get(&DependencyGroup::Execution(
                    &record.project_id,
                    &record.source_id,
                    &execution.id,
                ))
            {
                groups.uses.insert((*sequence, *group, "same_execution"));
            }
        }
        groups
    }

    fn dependencies(&self, entity: &Entity) -> Vec<(u64, &'static str)> {
        let project = entity.project_id();
        let mut dependencies = Vec::new();
        match entity {
            Entity::Work(work) => dependencies.extend(self.evidence(project, &work.evidence)),
            Entity::Session(session) => {
                for work in &session.work_ids {
                    extend(
                        &mut dependencies,
                        self.works.get(&(project, work)),
                        "work_metadata",
                    );
                }
                if let Some(parent) = &session.parent_id {
                    extend(
                        &mut dependencies,
                        self.sessions.get(&(project, parent)),
                        "parent_session_metadata",
                    );
                }
            }
            Entity::Record(record) => {
                dependencies.extend(self.evidence(project, &record.evidence));
                if record.kind == RecordKind::Attempt {
                    extend(
                        &mut dependencies,
                        self.attempt_results
                            .get(&(project, &record.source_id, &record.id)),
                        "attempt_result_reference",
                    );
                }
                for work in &record.work_ids {
                    extend(
                        &mut dependencies,
                        self.works.get(&(project, work)),
                        "work_metadata",
                    );
                }
                if let Some(session) = &record.session_id {
                    extend(
                        &mut dependencies,
                        self.sessions.get(&(project, session)),
                        "session_metadata",
                    );
                }
                if let Some(attempt) = &record.attempt_id {
                    extend(
                        &mut dependencies,
                        self.records.get(&(project, attempt)),
                        "attempt_reference",
                    );
                }
                for code in &record.code_refs {
                    extend(
                        &mut dependencies,
                        self.code_states.get(&(project, &code.state_id)),
                        "code_reference",
                    );
                }
                if let Some(execution) = &record.execution {
                    for state in execution.before_state.iter().chain(&execution.after_state) {
                        extend(
                            &mut dependencies,
                            self.code_states.get(&(project, state)),
                            "execution_code_state",
                        );
                    }
                }
                for sha in &record.commit_shas {
                    extend(
                        &mut dependencies,
                        self.commit_shas.get(&(project, sha)),
                        "commit_reference",
                    );
                }
            }
            Entity::Relation(relation) => {
                dependencies.extend(self.evidence(project, &relation.evidence));
                if relation.kind != RelationKind::RelatedTo {
                    for target in [&relation.from, &relation.to] {
                        dependencies.extend(
                            self.target(project, target)
                                .into_iter()
                                .map(|sequence| (sequence, "relation_endpoint")),
                        );
                    }
                }
            }
            Entity::CodeState(state) => {
                if let Some(sha) = &state.commit_sha {
                    extend(
                        &mut dependencies,
                        self.commits.get(&(project, &state.repository_id, sha)),
                        "code_state_commit",
                    );
                }
            }
            Entity::Source(_) | Entity::Commit(_) => {}
        }
        dependencies
    }
}

fn extend(
    dependencies: &mut Vec<(u64, &'static str)>,
    sequences: Option<&Vec<u64>>,
    rule: &'static str,
) {
    if let Some(sequences) = sequences {
        dependencies.extend(sequences.iter().map(|sequence| (*sequence, rule)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_dependency_fact_counts_grow_linearly() -> Result<(), Error> {
        for execution_group in [false, true] {
            let mut entries = Vec::new();
            for sequence in 1..=1_000 {
                let mut record = Record::new(
                    "lineage",
                    "project",
                    "source",
                    RecordKind::ToolResult,
                    "output",
                );
                record.revision = sequence.to_string();
                if execution_group {
                    record.id = format!("result-{sequence}");
                    record.execution = Some(Execution {
                        id: "shared-execution".into(),
                        command: "test".into(),
                        tool_name: None,
                        tool_input: None,
                        cwd: None,
                        started_at: None,
                        ended_at: None,
                        exit_code: None,
                        last_observed_state: "completed".into(),
                        observed_at: None,
                        liveness: Liveness::Stopped,
                        before_state: None,
                        after_state: None,
                        scope: Vec::new(),
                        environment: Environment::default(),
                    });
                } else {
                    record.evidence.push(Evidence {
                        source_id: "external-source".into(),
                        record_id: None,
                        revision: "external-revision".into(),
                        locator: "external-locator".into(),
                        availability: Availability::Available,
                        range: None,
                    });
                }
                entries.push(Entry {
                    sequence,
                    captured_at: "2026-09-29T01:00:00Z".into(),
                    entity: Entity::Record(record),
                });
            }
            let index = Index::new(&entries)?;
            let groups = index.grouped_dependencies();
            assert_eq!(groups.uses.len(), 1_000);
            assert_eq!(groups.members.len(), 1_000);
            assert!(
                entries
                    .iter()
                    .all(|entry| index.dependencies(&entry.entity).is_empty())
            );
        }
        Ok(())
    }
}
