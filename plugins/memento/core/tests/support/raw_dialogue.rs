//! Synthetic user-visible dialogue captured with explicit typed annotations.
//! This is an end-to-end persistence/retention/query harness, not an NLP annotator.
#![allow(dead_code)]

use std::{collections::BTreeSet, error::Error, path::PathBuf};

use memento::{
    Store,
    compaction::Policy,
    ingest,
    model::*,
    query::{self as retrieval, QueryError, QueryResponse},
    security::RedactionPolicy,
};
use tempfile::TempDir;

pub const PROJECT: &str = "raw-scenarios";
pub type TestResult = Result<(), Box<dyn Error>>;

pub struct Dialogue {
    pub store: Store,
    pub path: PathBuf,
    _directory: TempDir,
}

impl Dialogue {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("dialogue.sqlite");
        let store = Store::open(&path, RedactionPolicy::default()).await?;
        let mut dialogue = Self {
            store,
            path,
            _directory: directory,
        };
        dialogue
            .append(vec![
                Entity::Source(ingest::source("journal", PROJECT, SourceKind::Journal)),
                Entity::Work(Work {
                    id: "W1".into(),
                    project_id: PROJECT.into(),
                    source_id: "journal".into(),
                    title: "사용자 작업".into(),
                    goal: "기록된 요청과 근거로 작업을 이어간다".into(),
                    status: WorkStatus::Active,
                    observed_at: None,
                    evidence: Vec::new(),
                    completion_conditions: Vec::new(),
                }),
                Entity::Session(Session {
                    id: "S1".into(),
                    project_id: PROJECT.into(),
                    source_id: "journal".into(),
                    work_ids: vec!["W1".into()],
                    status: SessionStatus::Active,
                    started_at: None,
                    ended_at: None,
                    worktree_id: None,
                    parent_id: None,
                    working_directory: None,
                }),
                Entity::Record(message(
                    "__noise__",
                    ActorKind::Agent,
                    RecordKind::Status,
                    "잠시만요. 파일 목록을 읽고 있습니다.",
                )),
            ])
            .await?;
        Ok(dialogue)
    }

    pub async fn append(&mut self, entities: Vec<Entity>) -> TestResult {
        let receipts = self.store.append_all(entities).await?;
        assert!(receipts.iter().all(|receipt| receipt.durable));
        Ok(())
    }

    /// Age out the recent window explicitly. Separate automatic-retention
    /// journeys exercise writes against configured store limits.
    pub async fn compact_and_reopen(&mut self) -> Result<Corpus, Box<dyn Error>> {
        // Move the sequence frontier to source metadata so the final dialogue
        // message cannot survive merely because it was captured last.
        let mut source = self
            .store
            .latest()
            .await?
            .into_iter()
            .find_map(|entity| match entity {
                Entity::Source(source) if source.id == "journal" => Some(source),
                _ => None,
            })
            .ok_or("missing dialogue source")?;
        source.content_revision = Some("fixture-capture-complete".into());
        self.append(vec![Entity::Source(source)]).await?;
        let policy = Policy {
            recent_entries: 0,
            ..Policy::default()
        };
        let before = self.store.load().await?;
        let preview = self.store.compact(Some(policy.clone()), false).await?;
        assert_eq!(self.store.load().await?.entries.len(), before.entries.len());
        let applied = self.store.compact(Some(policy), true).await?;
        assert_eq!(preview, applied);
        assert!(applied.removed_entries > 0);
        self.store = Store::open(&self.path, RedactionPolicy::default()).await?;
        let corpus = self.store.load().await?;
        assert!(corpus.compaction.generation > 0);
        assert!(!corpus.entries.iter().any(|e| e.entity.id() == "__noise__"));
        Ok(corpus)
    }
}

pub fn message(id: &str, actor: ActorKind, kind: RecordKind, body: &str) -> Record {
    let mut record = Record::new(id, PROJECT, "journal", kind, body);
    record.revision = "v1".into();
    record.actor = Some(Actor {
        kind: actor,
        name: None,
    });
    record.nature = if actor == ActorKind::Tool {
        Nature::Observed
    } else {
        Nature::Reported
    };
    record.work_ids = vec!["W1".into()];
    record.session_id = Some("S1".into());
    record.association = Association::Explicit;
    record
}

pub fn evidence(record: &Record) -> Evidence {
    Evidence {
        source_id: record.source_id.clone(),
        record_id: Some(record.id.clone()),
        revision: record.revision.clone(),
        locator: format!("message:{}", record.id),
        availability: record.availability,
        range: None,
        purpose: memento::model::EvidencePurpose::Unspecified,
        span: None,
    }
}

pub fn link(id: &str, from: Target, to: Target, kind: RelationKind) -> Relation {
    Relation {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        from,
        to,
        kind,
        nature: Nature::Reported,
        evidence: Vec::new(),
        applies_to: Vec::new(),
    }
}

pub fn query(operation: Operation) -> Query {
    let mut query = Query::new(operation, PROJECT);
    query.limit = Some(100);
    query.budget_bytes = Some(262_144);
    query
}

pub fn read(corpus: &Corpus, id: &str) -> Result<QueryResponse, QueryError> {
    let mut query = query(Operation::Read);
    query.target = Some(Target::Record { id: id.into() });
    retrieval::execute(corpus, &query)
}

pub fn record<'a>(response: &'a QueryResponse, id: &str) -> Result<&'a Record, Box<dyn Error>> {
    response
        .items
        .iter()
        .find_map(|item| match &item.entity {
            Entity::Record(record) if record.id == id => Some(record),
            _ => None,
        })
        .ok_or_else(|| format!("original record {id} is missing from the response").into())
}

pub fn ids(response: &QueryResponse) -> BTreeSet<String> {
    response
        .items
        .iter()
        .map(|item| item.entity.id().to_owned())
        .collect()
}
