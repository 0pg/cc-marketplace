use std::{collections::BTreeMap, error::Error};

use work_context::{
    ingest,
    model::*,
    query::{self, QueryError, ResponseStatus},
    security::RedactionPolicy,
    semantic::{self, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, SemanticError},
};

type TestResult = Result<(), Box<dyn Error>>;

fn record(id: &str, body: &str) -> Record {
    let mut record = Record::new(id, "project", "journal", RecordKind::Finding, body);
    record.occurred_at = Some("2026-09-01T10:00:00Z".into());
    record
}

fn append(corpus: &mut Corpus, entity: Entity) {
    let sequence = corpus
        .entries
        .last()
        .map_or(1, |entry| entry.sequence.saturating_add(1));
    corpus.entries.push(Entry {
        sequence,
        captured_at: "2026-09-29T10:00:00Z".into(),
        entity,
    });
}

fn corpus(records: impl IntoIterator<Item = Record>) -> Corpus {
    let mut corpus = Corpus::default();
    append(
        &mut corpus,
        Entity::Source(ingest::source("journal", "project", SourceKind::Journal)),
    );
    for record in records {
        append(&mut corpus, Entity::Record(record));
    }
    corpus
}

#[test]
fn legacy_corpus_without_compaction_metadata_still_loads() -> TestResult {
    let corpus: Corpus = serde_json::from_str(r#"{"entries":[]}"#)?;
    assert_eq!(corpus.compaction, CompactionState::default());
    Ok(())
}

#[test]
fn compaction_invalidates_old_tokens_and_new_tokens_work() -> TestResult {
    let mut corpus = corpus([record("a", "first retained record"), record("b", "second")]);
    let mut search = Query::new(Operation::Search, "project");
    let checkpoint = query::execute(&corpus, &search)?.checkpoint;
    search.limit = Some(1);
    let cursor = query::execute(&corpus, &search)?.next_cursor;
    assert!(cursor.is_some());

    // A compaction can remove history without changing any latest entity or max sequence.
    corpus.compaction = CompactionState {
        generation: 1,
        removed_entries: 1,
    };
    search.cursor = cursor;
    assert_eq!(
        query::execute(&corpus, &search).err(),
        Some(QueryError::StaleCursor)
    );
    let mut changes = Query::new(Operation::Compare, "project");
    changes.since_checkpoint = checkpoint;
    assert_eq!(
        query::execute(&corpus, &changes).err(),
        Some(QueryError::RescanRequired)
    );

    search.cursor = None;
    search.cursor = query::execute(&corpus, &search)?.next_cursor;
    let last = query::execute(&corpus, &search)?;
    assert_eq!(last.items.len(), 1);
    assert_eq!(last.status, ResponseStatus::Ok);
    assert_eq!(
        last.omitted
            .iter()
            .filter(|warning| warning.starts_with("history_compacted:"))
            .count(),
        1
    );
    changes.since_checkpoint = last.checkpoint;
    append(
        &mut corpus,
        Entity::Record(record("c", "new after compact")),
    );
    let changes = query::execute(&corpus, &changes)?;
    assert_eq!(changes.status, ResponseStatus::Ok);
    assert_eq!(changes.items.len(), 1);
    assert_eq!(
        changes.items.first().map(|item| item.entity.id()),
        Some("c")
    );
    Ok(())
}

#[test]
fn missing_compacted_revision_is_disclosed_without_replacing_it_with_current_text() -> TestResult {
    let original = record("doc", "historical explanation");
    let revision = original.revision.clone();
    let mut corpus = corpus([original, record("doc", "current explanation")]);
    let mut read = Query::new(Operation::Read, "project");
    read.target = Some(Target::Artifact {
        record_id: "doc".into(),
        revision: revision.clone(),
        range: None,
    });
    assert_eq!(query::execute(&corpus, &read)?.items.len(), 1);
    corpus.entries.retain(
        |entry| !matches!(&entry.entity, Entity::Record(record) if record.revision == revision),
    );
    corpus.compaction = CompactionState {
        generation: 1,
        removed_entries: 1,
    };
    let response = query::execute(&corpus, &read)?;
    assert!(response.items.is_empty());
    assert_eq!(response.status, ResponseStatus::Partial);
    assert!(response.omitted.iter().any(|warning| {
        warning.starts_with("history_compacted:")
            && warning.contains("no match does not establish absence")
    }));
    read.target = Some(Target::Record { id: "doc".into() });
    let current = query::execute(&corpus, &read)?;
    assert_eq!(current.status, ResponseStatus::Ok);
    assert!(current.items.iter().any(
        |item| matches!(&item.entity, Entity::Record(record) if record.body == "current explanation")
    ));
    Ok(())
}

#[test]
fn compaction_does_not_reinterpret_as_of_as_capture_time() -> TestResult {
    let mut corpus = corpus([record("late", "captured late, occurred earlier")]);
    corpus.compaction = CompactionState {
        generation: 2,
        removed_entries: 7,
    };
    let mut search = Query::new(Operation::Search, "project");
    search.filters.as_of = Some("2026-09-02T00:00:00Z".into());
    let response = query::execute(&corpus, &search)?;
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.status, ResponseStatus::Ok);
    Ok(())
}

struct FixedProvider;

impl EmbeddingProvider for FixedProvider {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        _: u64,
    ) -> Result<EmbeddingResponse, SemanticError> {
        Ok(EmbeddingResponse {
            protocol: 1,
            model_id: request.model_id.clone(),
            model_revision: request.model_revision.clone(),
            query: vec![1.0, 0.0],
            documents: request.documents.iter().map(|_| vec![1.0, 0.0]).collect(),
            metrics: BTreeMap::new(),
        })
    }
}

#[test]
fn prepared_semantic_results_are_bound_to_the_preparation_generation() -> TestResult {
    let mut corpus = corpus([record("kept", "retained candidate")]);
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "retained candidate".into(),
        mode: SearchMode::Semantic,
    });
    let config = semantic::SemanticConfig {
        model_id: "mechanical-compaction-contract".into(),
        model_revision: "fixed-1".into(),
        ..semantic::SemanticConfig::default()
    };
    let policy = RedactionPolicy::default();
    let prepare = |corpus: &Corpus| -> Result<_, Box<dyn Error>> {
        let records = query::prepare_semantic_records(corpus, &query)?;
        Ok(semantic::prepare_with_provider(
            &records,
            "retained candidate",
            &config,
            &policy,
            &FixedProvider,
        )?
        .with_compaction_generation(corpus.compaction.generation)?)
    };
    let before = prepare(&corpus)?;
    assert_eq!(
        query::execute_semantic(&corpus, &query, &before, &policy)?
            .items
            .len(),
        1
    );
    corpus.compaction = CompactionState {
        generation: 1,
        removed_entries: 1,
    };
    assert_eq!(
        query::execute_semantic(&corpus, &query, &before, &policy).err(),
        Some(QueryError::StaleCursor)
    );
    let after = prepare(&corpus)?;
    assert_ne!(before.fingerprint, after.fingerprint);
    assert_eq!(
        query::execute_semantic(&corpus, &query, &after, &policy)?
            .items
            .len(),
        1
    );
    corpus.compaction.generation = 2;
    assert_eq!(
        query::execute_semantic(&corpus, &query, &after, &policy).err(),
        Some(QueryError::StaleCursor)
    );
    Ok(())
}
