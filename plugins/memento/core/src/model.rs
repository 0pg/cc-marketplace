use serde::{Deserialize, Serialize};

macro_rules! vocabulary {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

vocabulary!(RecordKind {
    Request,
    Constraint,
    Finding,
    Decision,
    Attempt,
    ToolResult,
    Change,
    Verification,
    Feedback,
    Status,
    GitEvent
});
vocabulary!(Nature {
    Observed,
    Reported,
    Inferred
});
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    #[default]
    Legacy,
    Evidence,
    Claim,
}

impl Representation {
    pub(crate) fn is_legacy(&self) -> bool {
        *self == Self::Legacy
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePurpose {
    #[default]
    Unspecified,
    Origin,
    Support,
}

impl EvidencePurpose {
    fn is_unspecified(&self) -> bool {
        *self == Self::Unspecified
    }
}
vocabulary!(Availability {
    Available,
    Redacted,
    Missing,
    Deleted,
    Unsupported
});
vocabulary!(Fidelity {
    Original,
    SummaryOnly,
    SourceTruncated
});
vocabulary!(WorkStatus {
    Planned,
    Active,
    Blocked,
    Completed,
    Abandoned,
    Unknown
});
vocabulary!(SessionStatus {
    Active,
    Ended,
    Unknown
});
vocabulary!(DecisionStatus {
    Proposed,
    Accepted,
    Superseded,
    Rejected,
    Unknown
});
vocabulary!(AttemptOutcome {
    Succeeded,
    Failed,
    Abandoned,
    Running,
    Unknown
});
vocabulary!(VerificationOutcome {
    Passed,
    Failed,
    Running,
    Skipped,
    Unknown
});
vocabulary!(Liveness {
    Running,
    Stopped,
    Unknown
});
vocabulary!(ActorKind {
    Human,
    Agent,
    Tool,
    Unknown
});
vocabulary!(Association {
    Explicit,
    Candidate,
    Unassigned
});
vocabulary!(RelationKind {
    RespondsTo,
    Supports,
    Contradicts,
    Supersedes,
    AttemptOf,
    Verifies,
    Changes,
    RelatedTo,
    ForkedFrom,
    IntegratedInto,
    DerivedFrom,
    Reverts
});
vocabulary!(SourceKind {
    Journal,
    Codex,
    Document,
    Git
});
vocabulary!(Operation {
    Sources,
    ListWork,
    Search,
    Read,
    Timeline,
    Trace,
    Compare,
    Brief
});
vocabulary!(Direction {
    Incoming,
    Outgoing,
    Both
});
vocabulary!(SearchMode {
    Literal,
    Tokens,
    Semantic
});

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportCompleteness {
    #[default]
    FullSnapshot,
    Partial,
    Delta,
}
vocabulary!(Sort {
    Relevance,
    Oldest,
    Newest
});
vocabulary!(BriefPurpose {
    Resume,
    Explain,
    InvestigateFailure,
    Review
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub project_id: String,
    pub kind: SourceKind,
    pub name: String,
    pub location: Option<String>,
    pub content_revision: Option<String>,
    pub authorized: bool,
    pub available: bool,
    #[serde(default)]
    pub record_kinds: Vec<RecordKind>,
    #[serde(default)]
    pub unsupported_filters: Vec<String>,
    pub last_captured_at: Option<String>,
    pub last_event_id: Option<String>,
    #[serde(default)]
    pub gaps: Vec<String>,
    #[serde(default)]
    pub result_only: bool,
    #[serde(default)]
    pub import_completeness: ImportCompleteness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Work {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub title: String,
    pub goal: String,
    pub status: WorkStatus,
    pub observed_at: Option<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub completion_conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    #[serde(default)]
    pub work_ids: Vec<String>,
    pub status: SessionStatus,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub worktree_id: Option<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub kind: ActorKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub source_id: String,
    pub record_id: Option<String>,
    pub revision: String,
    pub locator: String,
    pub availability: Availability,
    pub range: Option<TextRange>,
    #[serde(default, skip_serializing_if = "EvidencePurpose::is_unspecified")]
    pub purpose: EvidencePurpose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<ByteSpan>,
}

/// Zero-based, half-open UTF-8 byte range in the retained evidence body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodeRef {
    pub state_id: String,
    pub path: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub os: Option<String>,
    pub toolchain: Option<String>,
    pub profile: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub exit_code: Option<i32>,
    pub last_observed_state: String,
    pub observed_at: Option<String>,
    pub liveness: Liveness,
    pub before_state: Option<String>,
    pub after_state: Option<String>,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub revision: String,
    pub kind: RecordKind,
    pub nature: Nature,
    pub fidelity: Fidelity,
    pub availability: Availability,
    pub title: String,
    pub body: String,
    pub occurred_at: Option<String>,
    pub source_order: Option<u64>,
    pub actor: Option<Actor>,
    #[serde(default)]
    pub work_ids: Vec<String>,
    pub association: Association,
    pub session_id: Option<String>,
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub code_refs: Vec<CodeRef>,
    #[serde(default)]
    pub commit_shas: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    pub decision_status: Option<DecisionStatus>,
    pub attempt_outcome: Option<AttemptOutcome>,
    pub verification_outcome: Option<VerificationOutcome>,
    pub attempt_id: Option<String>,
    pub execution: Option<Execution>,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub alternatives: Vec<String>,
    #[serde(default)]
    pub derived: bool,
    #[serde(default)]
    pub partial: bool,
    #[serde(default, skip_serializing_if = "Representation::is_legacy")]
    pub representation: Representation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

impl Record {
    pub fn new(id: &str, project: &str, source: &str, kind: RecordKind, body: &str) -> Self {
        Self {
            id: id.into(),
            project_id: project.into(),
            source_id: source.into(),
            revision: crate::security::hash(body.as_bytes()),
            kind,
            nature: Nature::Observed,
            fidelity: Fidelity::Original,
            availability: Availability::Available,
            title: String::new(),
            body: body.into(),
            occurred_at: None,
            source_order: None,
            actor: None,
            work_ids: Vec::new(),
            association: Association::Unassigned,
            session_id: None,
            worktree_id: None,
            paths: Vec::new(),
            code_refs: Vec::new(),
            commit_shas: Vec::new(),
            evidence: Vec::new(),
            decision_status: None,
            attempt_outcome: None,
            verification_outcome: None,
            attempt_id: None,
            execution: None,
            applies_to: Vec::new(),
            alternatives: Vec::new(),
            derived: false,
            partial: false,
            representation: Representation::Legacy,
            context_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Record {
        id: String,
    },
    Work {
        id: String,
    },
    Session {
        id: String,
    },
    Code {
        #[serde(default)]
        state_id: String,
        path: String,
        range: Option<TextRange>,
    },
    Artifact {
        record_id: String,
        revision: String,
        range: Option<TextRange>,
    },
    Commit {
        repository_id: String,
        commit_sha: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub from: Target,
    pub to: Target,
    pub kind: RelationKind,
    pub nature: Nature,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkingFileKind {
    File,
    Symlink,
    Directory,
    Missing,
    Ignored,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileChange {
    Unmodified,
    Modified,
    TypeChanged,
    Added,
    Deleted,
    Renamed,
    Copied,
    Unmerged,
    Untracked,
    Ignored,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileStatus {
    pub index: FileChange,
    pub working: FileChange,
    pub original_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub mode: String,
    pub object_id: String,
    /// Git index stage: 0 normal, 1 base, 2 ours, 3 theirs.
    pub stage: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileState {
    pub path: String,
    /// Git object ID, using this repository's object format.
    pub head_hash: Option<String>,
    /// Stage-zero Git object ID; conflicts retain all entries in index_entries.
    pub index_hash: Option<String>,
    /// SHA-256 of raw working bytes. Compare only with the same layer/algorithm;
    /// Git object IDs include a blob header and may also use SHA-1.
    pub working_hash: Option<String>,
    pub working_content: Option<String>,
    #[serde(default)]
    pub working_kind: WorkingFileKind,
    #[serde(default)]
    pub index_entries: Vec<IndexEntry>,
    #[serde(default)]
    pub status: Option<FileStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodeState {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub repository_id: String,
    pub worktree_id: Option<String>,
    pub commit_sha: Option<String>,
    pub observed_at: String,
    pub changed_during_observation: bool,
    pub files: Vec<FileState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub repository_id: String,
    pub sha: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub paths: Vec<String>,
    pub message: String,
    pub occurred_at: Option<String>,
    pub origin_worktree: Option<String>,
    pub observed_worktree: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "entity", content = "data", rename_all = "snake_case")]
#[allow(
    clippy::large_enum_variant,
    reason = "Entities are the explicit JSON interchange model; boxing every record adds API indirection without reducing retained payload size"
)]
pub enum Entity {
    Source(Source),
    Work(Work),
    Session(Session),
    Record(Record),
    Relation(Relation),
    CodeState(CodeState),
    Commit(Commit),
}

impl Entity {
    pub fn id(&self) -> &str {
        match self {
            Self::Source(x) => &x.id,
            Self::Work(x) => &x.id,
            Self::Session(x) => &x.id,
            Self::Record(x) => &x.id,
            Self::Relation(x) => &x.id,
            Self::CodeState(x) => &x.id,
            Self::Commit(x) => &x.id,
        }
    }
    pub fn project_id(&self) -> &str {
        match self {
            Self::Source(x) => &x.project_id,
            Self::Work(x) => &x.project_id,
            Self::Session(x) => &x.project_id,
            Self::Record(x) => &x.project_id,
            Self::Relation(x) => &x.project_id,
            Self::CodeState(x) => &x.project_id,
            Self::Commit(x) => &x.project_id,
        }
    }
    pub fn source_id(&self) -> &str {
        match self {
            Self::Source(x) => &x.id,
            Self::Work(x) => &x.source_id,
            Self::Session(x) => &x.source_id,
            Self::Record(x) => &x.source_id,
            Self::Relation(x) => &x.source_id,
            Self::CodeState(x) => &x.source_id,
            Self::Commit(x) => &x.source_id,
        }
    }
    pub fn scoped_key(project: &str, source: &str, kind: &str, id: &str) -> String {
        format!(
            "{}:{project}{}:{source}{}:{kind}{}:{id}",
            project.len(),
            source.len(),
            kind.len(),
            id.len()
        )
    }
    pub fn key(&self) -> String {
        let kind = match self {
            Self::Source(_) => "source",
            Self::Work(_) => "work",
            Self::Session(_) => "session",
            Self::Record(_) => "record",
            Self::Relation(_) => "relation",
            Self::CodeState(_) => "code_state",
            Self::Commit(_) => "commit",
        };
        Self::scoped_key(self.project_id(), self.source_id(), kind, self.id())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub sequence: u64,
    pub captured_at: String,
    pub entity: Entity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Corpus {
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub compaction: CompactionState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionState {
    pub generation: u64,
    pub removed_entries: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub project_id: String,
    #[serde(default)]
    pub work_ids: Vec<String>,
    #[serde(default)]
    pub session_ids: Vec<String>,
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub worktree_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representations: Vec<Representation>,
    #[serde(default)]
    pub record_kinds: Vec<RecordKind>,
    #[serde(default)]
    pub exclude_record_kinds: Vec<RecordKind>,
    #[serde(default)]
    pub work_statuses: Vec<WorkStatus>,
    #[serde(default)]
    pub exclude_work_statuses: Vec<WorkStatus>,
    #[serde(default)]
    pub session_statuses: Vec<SessionStatus>,
    #[serde(default)]
    pub exclude_session_statuses: Vec<SessionStatus>,
    #[serde(default)]
    pub decision_statuses: Vec<DecisionStatus>,
    #[serde(default)]
    pub exclude_decision_statuses: Vec<DecisionStatus>,
    #[serde(default)]
    pub attempt_outcomes: Vec<AttemptOutcome>,
    #[serde(default)]
    pub exclude_attempt_outcomes: Vec<AttemptOutcome>,
    #[serde(default)]
    pub verification_outcomes: Vec<VerificationOutcome>,
    #[serde(default)]
    pub exclude_verification_outcomes: Vec<VerificationOutcome>,
    #[serde(default)]
    pub actors: Vec<String>,
    #[serde(default)]
    pub exclude_actors: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub commit_shas: Vec<String>,
    #[serde(default)]
    pub exclude_commit_shas: Vec<String>,
    #[serde(default)]
    pub natures: Vec<Nature>,
    #[serde(default)]
    pub exclude_natures: Vec<Nature>,
    pub occurred_from: Option<String>,
    pub occurred_to: Option<String>,
    pub as_of: Option<String>,
    pub time_unknown: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextQuery {
    pub text: String,
    pub mode: SearchMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryPoint {
    pub occurred_at: Option<String>,
    #[serde(default)]
    pub session_last_records: Vec<String>,
    pub code_state_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub operation: Operation,
    pub scope: Scope,
    #[serde(default)]
    pub filters: Filters,
    pub query: Option<TextQuery>,
    pub target: Option<Target>,
    pub direction: Option<Direction>,
    #[serde(default)]
    pub relations: Vec<RelationKind>,
    pub max_depth: Option<usize>,
    pub sort: Option<Sort>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub budget_bytes: Option<usize>,
    pub purpose: Option<BriefPurpose>,
    pub range: Option<TextRange>,
    pub context_lines: Option<usize>,
    pub from: Option<HistoryPoint>,
    pub to: Option<HistoryPoint>,
    pub since_checkpoint: Option<String>,
    #[serde(default)]
    pub code_mapping: Option<crate::mapping::MappingRequest>,
}

impl Query {
    pub fn new(operation: Operation, project: &str) -> Self {
        Self {
            operation,
            scope: Scope {
                project_id: project.into(),
                ..Scope::default()
            },
            filters: Filters::default(),
            query: None,
            target: None,
            direction: None,
            relations: Vec::new(),
            max_depth: None,
            sort: None,
            limit: None,
            cursor: None,
            budget_bytes: None,
            purpose: None,
            range: None,
            context_lines: None,
            from: None,
            to: None,
            since_checkpoint: None,
            code_mapping: None,
        }
    }
}
