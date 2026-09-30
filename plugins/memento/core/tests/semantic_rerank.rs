//! Mechanical lifecycle contracts; these providers do not measure retrieval quality.
use std::{collections::BTreeMap, error::Error, sync::Mutex, time::Duration};

use work_context::{
    ingest,
    model::*,
    query::{self, QueryError},
    security::{RedactionPolicy, hash},
    semantic::{
        self, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, PreparedSearch, RerankConfig,
        RerankProvider, RerankRequest, RerankResponse, SemanticConfig, SemanticError,
    },
};

type TestResult = Result<(), Box<dyn Error>>;
const FEEDBACK: &str = "\n\nRetrieved candidate context (unverified):\n";

#[derive(Default)]
struct Embeddings {
    calls: Mutex<Vec<EmbeddingRequest>>,
}

impl EmbeddingProvider for Embeddings {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        _: u64,
    ) -> Result<EmbeddingResponse, SemanticError> {
        self.calls
            .lock()
            .map_err(|_| SemanticError::InvalidResponse("test lock".into()))?
            .push(request.clone());
        Ok(EmbeddingResponse {
            protocol: 1,
            model_id: request.model_id.clone(),
            model_revision: request.model_revision.clone(),
            query: vec![1.0, 0.0],
            documents: request
                .documents
                .iter()
                .map(|text| {
                    if text.contains("NEGATIVE_COSINE") {
                        vec![-1.0, 0.0]
                    } else {
                        vec![1.0, 0.0]
                    }
                })
                .collect(),
            metrics: BTreeMap::new(),
        })
    }
}

#[derive(Clone, Copy)]
enum Scores {
    Low,
    Tied,
    SeedThenOthers,
}

#[derive(Clone, Copy)]
enum Invalid {
    Protocol,
    Model,
    Revision,
    Count,
    Nan,
    Infinity,
    Negative,
    AboveOne,
}

struct Reranker {
    calls: Mutex<Vec<(RerankRequest, u64)>>,
    scores: Scores,
    invalid: Option<(usize, Invalid)>,
    delay: Duration,
}

impl Reranker {
    fn new(scores: Scores) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            scores,
            invalid: None,
            delay: Duration::ZERO,
        }
    }
}

impl RerankProvider for Reranker {
    fn score(
        &self,
        request: &RerankRequest,
        timeout_ms: u64,
    ) -> Result<RerankResponse, SemanticError> {
        let stage = {
            let mut calls = self
                .calls
                .lock()
                .map_err(|_| SemanticError::InvalidResponse("test lock".into()))?;
            calls.push((request.clone(), timeout_ms));
            calls.len()
        };
        std::thread::sleep(self.delay);
        let mut response = RerankResponse {
            protocol: 1,
            model_id: request.model_id.clone(),
            model_revision: request.model_revision.clone(),
            scores: request
                .documents
                .iter()
                .map(|text| match self.scores {
                    Scores::Low => 0.001,
                    Scores::Tied => 0.75,
                    Scores::SeedThenOthers if stage == 1 && text.contains("SEED") => 0.95,
                    Scores::SeedThenOthers if stage == 1 => 0.05,
                    Scores::SeedThenOthers if text.contains("SEED") => 0.001,
                    Scores::SeedThenOthers if text.contains("FIRST") => 0.9,
                    Scores::SeedThenOthers if text.contains("SECOND") => 0.8,
                    Scores::SeedThenOthers => 0.7,
                })
                .collect(),
            metrics: BTreeMap::new(),
        };
        if let Some((invalid_stage, invalid)) = self.invalid
            && invalid_stage == stage
        {
            match invalid {
                Invalid::Protocol => response.protocol = 2,
                Invalid::Model => response.model_id = "unrequested-model".into(),
                Invalid::Revision => response.model_revision = "unrequested-revision".into(),
                Invalid::Count => {
                    response.scores.pop();
                }
                invalid => {
                    if let Some(score) = response.scores.first_mut() {
                        *score = match invalid {
                            Invalid::Nan => f32::NAN,
                            Invalid::Infinity => f32::INFINITY,
                            Invalid::Negative => -0.1,
                            _ => 1.1,
                        };
                    }
                }
            }
        }
        Ok(response)
    }
}

fn config() -> SemanticConfig {
    SemanticConfig {
        model_id: "mechanical-embedding".into(),
        model_revision: "embedding-v1".into(),
        min_score: 0.0,
        rerank: Some(RerankConfig {
            model_id: "mechanical-reranker".into(),
            model_revision: "reranker-v1".into(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn record(id: &str, text: &str) -> Record {
    Record::new(id, "project", "journal", RecordKind::ToolResult, text)
}

fn append(corpus: &mut Corpus, entity: Entity) {
    corpus.entries.push(Entry {
        sequence: corpus
            .entries
            .last()
            .map_or(1, |entry| entry.sequence.saturating_add(1)),
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}

fn corpus(records: &[Record]) -> Corpus {
    let mut corpus = Corpus::default();
    for source in ["journal", "seed-source"] {
        append(
            &mut corpus,
            Entity::Source(ingest::source(source, "project", SourceKind::Journal)),
        );
    }
    for record in records {
        append(&mut corpus, Entity::Record(record.clone()));
    }
    corpus
}

fn request() -> Query {
    let mut query = Query::new(Operation::Search, "project");
    query.query = Some(TextQuery {
        text: "계정 전환 뒤 화면이 잘못 표시된 이유와 조치는?".into(),
        mode: SearchMode::Semantic,
    });
    query
}

fn prepare(
    corpus: &Corpus,
    query: &Query,
    config: &SemanticConfig,
    embeddings: &Embeddings,
    reranker: &Reranker,
) -> Result<PreparedSearch, Box<dyn Error>> {
    let records = query::prepare_semantic_records(corpus, query)?;
    Ok(semantic::prepare_with_backends(
        &records,
        &query.query.as_ref().ok_or("query absent")?.text,
        config,
        &RedactionPolicy::default(),
        embeddings,
        reranker,
    )?)
}

#[test]
fn unauthorized_and_unavailable_text_never_reaches_either_rerank_stage() -> TestResult {
    let mut allowed = record("allowed", "SEED private-literal password=hidden-secret");
    allowed.title = "private-literal title".into();
    allowed.work_ids = vec!["allowed-work".into()];
    let mut unavailable = record("missing", "UNAVAILABLE_CONTENT");
    unavailable.availability = Availability::Missing;
    unavailable.work_ids = allowed.work_ids.clone();
    let mut secret = record("secret", "UNAUTHORIZED_CONTENT");
    secret.source_id = "private".into();
    secret.work_ids = allowed.work_ids.clone();
    let mut outside_work = record("other-work", "OUTSIDE_WORK_CONTENT");
    outside_work.work_ids = vec!["another-work".into()];
    let mut data = corpus(&[allowed, unavailable, secret, outside_work]);
    let mut revoked = ingest::source("private", "project", SourceKind::Journal);
    revoked.authorized = false;
    append(&mut data, Entity::Source(revoked));
    let mut query = request();
    query.scope.work_ids = vec!["allowed-work".into()];
    query.query.as_mut().ok_or("query absent")?.text = "private-literal Bearer hidden-token".into();
    let records = query::prepare_semantic_records(&data, &query)?;
    let policy = RedactionPolicy {
        literal_secrets: vec!["private-literal".into()],
    };
    let embeddings = Embeddings::default();
    let reranker = Reranker::new(Scores::Tied);
    let prepared = semantic::prepare_with_backends(
        &records,
        &query.query.as_ref().ok_or("query absent")?.text,
        &config(),
        &policy,
        &embeddings,
        &reranker,
    )?;
    let calls = reranker.calls.lock().map_err(|_| "test lock")?;
    assert_eq!(calls.len(), 2);
    let encoded = format!(
        "{}{}{}",
        serde_json::to_string(&*calls)?,
        serde_json::to_string(&*embeddings.calls.lock().map_err(|_| "test lock")?)?,
        serde_json::to_string(&prepared)?
    );
    for forbidden in [
        "private-literal",
        "hidden-token",
        "hidden-secret",
        "UNAUTHORIZED_CONTENT",
        "UNAVAILABLE_CONTENT",
        "OUTSIDE_WORK_CONTENT",
    ] {
        assert!(!encoded.contains(forbidden), "exposed {forbidden}");
    }
    assert!(encoded.contains("[REDACTED]"));
    Ok(())
}

#[test]
fn feedback_has_real_unicode_ranges_and_hash_after_its_own_truncation() -> TestResult {
    let mut original = record("unicode", &"한글🙂 원문 근거\n".repeat(30));
    original.title = "검토 제목".into();
    let records = vec![original.clone()];
    let mut config = config();
    config.chunk_chars = 32;
    config.overlap_chars = 8;
    let rerank = config.rerank.as_mut().ok_or("rerank config absent")?;
    rerank.context_chars = 96;
    rerank.feedback_chars = 48;
    let embeddings = Embeddings::default();
    let reranker = Reranker::new(Scores::Tied);
    let prepared = semantic::prepare_with_backends(
        &records,
        "원문 질문",
        &config,
        &RedactionPolicy::default(),
        &embeddings,
        &reranker,
    )?;
    let info = prepared.reranking.as_ref().ok_or("rerank info absent")?;
    let seed = info.seed.as_ref().ok_or("seed absent")?;
    let calls = reranker.calls.lock().map_err(|_| "test lock")?;
    let first = &calls.first().ok_or("initial call absent")?.0;
    let feedback = &calls.get(1).ok_or("feedback call absent")?.0;
    let supplied = feedback
        .query
        .strip_prefix(&format!("원문 질문{FEEDBACK}"))
        .ok_or("feedback did not preserve the original query")?;
    assert_eq!(supplied.chars().count(), 48);
    assert!(supplied.contains(&original.title));
    assert_eq!(seed.text_sha256, hash(supplied.as_bytes()));
    assert_eq!(seed.context.field, "body");
    assert_eq!(seed.scored_context.byte_start, 0);
    let source = original
        .body
        .get(seed.context.byte_start..seed.context.byte_end)
        .ok_or("seed metadata split a Unicode code point")?;
    assert!(!source.is_empty());
    assert!(supplied.ends_with(source));
    assert!(seed.context.context_fields.contains(&"title".into()));
    assert_eq!(seed.revision, original.revision);
    assert_eq!(
        seed.projection_revision,
        semantic::projection_revision(&original)?
    );
    assert_eq!(first.documents, feedback.documents);
    let candidate = prepared.candidates.first().ok_or("candidate absent")?;
    let expanded = candidate.rerank.as_ref().ok_or("match absent")?;
    assert!(expanded.chunks.iter().all(|chunk| {
        original
            .body
            .get(chunk.byte_start..chunk.byte_end)
            .is_some_and(|text| text.chars().count() <= 96)
    }));
    assert!(expanded.chunks.iter().any(|chunk| {
        original
            .body
            .get(chunk.byte_start..chunk.byte_end)
            .is_some_and(|text| text.chars().count() > 32)
    }));
    assert_eq!(info.stages, 2);
    Ok(())
}

#[test]
fn feedback_ending_inside_a_title_does_not_claim_to_have_used_the_body() -> TestResult {
    let mut original = record("title-seed", "Body selected for initial relevance scoring.");
    original.title = "첫🙂제목 나머지".into();
    let mut config = config();
    config
        .rerank
        .as_mut()
        .ok_or("rerank config absent")?
        .feedback_chars = 2;
    let reranker = Reranker::new(Scores::Tied);
    let prepared = semantic::prepare_with_backends(
        std::slice::from_ref(&original),
        "question",
        &config,
        &RedactionPolicy::default(),
        &Embeddings::default(),
        &reranker,
    )?;
    let seed = prepared
        .reranking
        .as_ref()
        .and_then(|info| info.seed.as_ref())
        .ok_or("seed absent")?;
    assert_eq!(seed.scored_context.field, "body");
    assert_eq!(seed.context.field, "title");
    assert_eq!(seed.context.byte_start, 0);
    assert_eq!(seed.context.byte_end, "첫🙂".len());
    assert!(seed.context.context_fields.is_empty());
    let calls = reranker.calls.lock().map_err(|_| "test lock")?;
    let feedback = &calls.get(1).ok_or("feedback absent")?.0;
    assert_eq!(feedback.query, format!("question{FEEDBACK}첫🙂"));
    assert_eq!(seed.text_sha256, hash("첫🙂".as_bytes()));
    Ok(())
}

#[test]
fn low_scores_skip_feedback_without_filtering_out_bounded_candidates() -> TestResult {
    let records = vec![
        record("positive", "plain result"),
        record("negative", "NEGATIVE_COSINE"),
    ];
    let embeddings = Embeddings::default();
    let reranker = Reranker::new(Scores::Low);
    let prepared = semantic::prepare_with_backends(
        &records,
        "unrelated question",
        &config(),
        &RedactionPolicy::default(),
        &embeddings,
        &reranker,
    )?;
    assert_eq!(reranker.calls.lock().map_err(|_| "test lock")?.len(), 1);
    let info = prepared.reranking.as_ref().ok_or("rerank info absent")?;
    assert_eq!(info.stages, 1);
    assert!(info.seed.is_none());
    assert_eq!(prepared.candidates.len(), 2);
    assert!(
        prepared
            .candidates
            .iter()
            .any(|candidate| candidate.score < 0.0)
    );
    assert!(prepared.candidates.iter().all(|candidate| {
        candidate
            .rerank
            .as_ref()
            .is_some_and(|matched| matched.score == 0.001)
    }));
    Ok(())
}

#[test]
fn a_wrong_seed_can_leave_final_top_three_without_creating_evidence_relations() -> TestResult {
    let mut seed = record("seed", "SEED avatar cache is a separate incident");
    seed.source_id = "seed-source".into();
    let records = vec![
        seed.clone(),
        record("first", "FIRST dashboard account switch"),
        record("second", "SECOND identity-scoped cache"),
        record("third", "THIRD account-switch regression"),
    ];
    let data = corpus(&records);
    let before = serde_json::to_string(&data)?;
    let mut config = config();
    config.max_candidates = 3;
    let mut query = request();
    query.limit = Some(1);
    let prepared = prepare(
        &data,
        &query,
        &config,
        &Embeddings::default(),
        &Reranker::new(Scores::SeedThenOthers),
    )?;
    let info = prepared.reranking.as_ref().ok_or("rerank info absent")?;
    assert_eq!(
        info.seed.as_ref().map(|seed| seed.record_id.as_str()),
        Some("seed")
    );
    assert_eq!(prepared.candidates.len(), 3);
    assert!(
        prepared
            .candidates
            .iter()
            .all(|candidate| candidate.record_id != "seed")
    );
    let page = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(
        page.items.first().map(|item| item.entity.id()),
        Some("first")
    );
    assert!(page.relations.is_empty());
    assert!(page.brief.is_none());
    assert_eq!(before, serde_json::to_string(&data)?);
    let mut next = query.clone();
    next.cursor = Some(page.next_cursor.ok_or("cursor absent")?);
    for mutation in ["title", "deleted", "revoked"] {
        let mut fresh = data.clone();
        if mutation == "revoked" {
            let mut source = ingest::source("seed-source", "project", SourceKind::Journal);
            source.authorized = false;
            append(&mut fresh, Entity::Source(source));
        } else {
            let mut updated = seed.clone();
            if mutation == "title" {
                updated.title = "corrected title with unchanged body and source revision".into();
            } else {
                updated.availability = Availability::Deleted;
            }
            append(&mut fresh, Entity::Record(updated));
        }
        for request in [&query, &next] {
            assert!(
                matches!(
                    query::execute_semantic(
                        &fresh,
                        request,
                        &prepared,
                        &RedactionPolicy::default()
                    ),
                    Err(QueryError::StaleCursor | QueryError::InvalidScope(_))
                ),
                "accepted changed seed: {mutation}"
            );
        }
    }
    Ok(())
}

#[test]
fn seed_and_reranker_generation_are_part_of_prepared_and_cursor_identity() -> TestResult {
    let records = vec![record("b", "same context"), record("a", "same context")];
    let data = corpus(&records);
    let mut query = request();
    query.limit = Some(1);
    let config = config();
    let prepared = prepare(
        &data,
        &query,
        &config,
        &Embeddings::default(),
        &Reranker::new(Scores::Tied),
    )?;
    assert_eq!(
        prepared
            .reranking
            .as_ref()
            .and_then(|info| info.seed.as_ref())
            .map(|seed| seed.record_id.as_str()),
        Some("a")
    );
    let page = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    query.cursor = Some(page.next_cursor.ok_or("cursor absent")?);
    for mutation in ["revision", "feedback-budget"] {
        let mut changed = config.clone();
        let rerank = changed.rerank.as_mut().ok_or("rerank config absent")?;
        if mutation == "revision" {
            rerank.model_revision = "reranker-v2".into();
        } else {
            rerank.feedback_chars = 384;
        }
        let newer = prepare(
            &data,
            &query,
            &changed,
            &Embeddings::default(),
            &Reranker::new(Scores::Tied),
        )?;
        assert!(matches!(
            query::execute_semantic(&data, &query, &newer, &RedactionPolicy::default()),
            Err(QueryError::StaleCursor)
        ));
    }
    let mut tampered = prepared.clone();
    tampered
        .reranking
        .as_mut()
        .and_then(|info| info.seed.as_mut())
        .ok_or("seed absent")?
        .text_sha256 = "different unverified input".into();
    assert!(matches!(
        tampered.validate(
            &records,
            &query.query.as_ref().ok_or("query absent")?.text,
            &RedactionPolicy::default()
        ),
        Err(SemanticError::Stale)
    ));
    Ok(())
}

#[test]
fn invalid_reranker_responses_fail_in_either_stage_instead_of_becoming_no_matches() -> TestResult {
    for stage in [1, 2] {
        for invalid in [
            Invalid::Protocol,
            Invalid::Model,
            Invalid::Revision,
            Invalid::Count,
            Invalid::Nan,
            Invalid::Infinity,
            Invalid::Negative,
            Invalid::AboveOne,
        ] {
            let mut reranker = Reranker::new(Scores::Tied);
            reranker.invalid = Some((stage, invalid));
            assert!(matches!(
                semantic::prepare_with_backends(
                    &[record("r", "retained text")],
                    "query",
                    &config(),
                    &RedactionPolicy::default(),
                    &Embeddings::default(),
                    &reranker
                ),
                Err(SemanticError::InvalidResponse(_))
            ));
            assert_eq!(reranker.calls.lock().map_err(|_| "test lock")?.len(), stage);
        }
    }
    Ok(())
}

#[test]
fn all_rerank_stages_share_the_preparation_deadline() -> TestResult {
    let records = vec![record("r", "retained text")];
    let mut reranker = Reranker::new(Scores::Tied);
    reranker.delay = Duration::from_millis(20);
    let mut bounded = config();
    bounded.timeout_ms = 1000;
    semantic::prepare_with_backends(
        &records,
        "query",
        &bounded,
        &RedactionPolicy::default(),
        &Embeddings::default(),
        &reranker,
    )?;
    let calls = reranker.calls.lock().map_err(|_| "test lock")?;
    let first = calls.first().ok_or("initial absent")?.1;
    let second = calls.get(1).ok_or("feedback absent")?.1;
    assert!(first <= bounded.timeout_ms && second < first);
    drop(calls);
    let mut expired = Reranker::new(Scores::Tied);
    expired.delay = Duration::from_millis(60);
    bounded.timeout_ms = 25;
    assert!(matches!(
        semantic::prepare_with_backends(
            &records,
            "query",
            &bounded,
            &RedactionPolicy::default(),
            &Embeddings::default(),
            &expired
        ),
        Err(SemanticError::BudgetExceeded(_))
    ));
    assert_eq!(expired.calls.lock().map_err(|_| "test lock")?.len(), 1);
    Ok(())
}

#[test]
fn invalid_limits_and_cosine_cutoffs_are_rejected_before_calling_backends() -> TestResult {
    for invalid in [
        "pool",
        "context",
        "feedback",
        "seed-nan",
        "seed-range",
        "cosine-cutoff",
    ] {
        let mut config = config();
        let rerank = config.rerank.as_mut().ok_or("rerank config absent")?;
        match invalid {
            "pool" => rerank.pool_limit = 51,
            "context" => rerank.context_chars = 769,
            "feedback" => rerank.feedback_chars = 769,
            "seed-nan" => rerank.seed_min_score = f64::NAN,
            "seed-range" => rerank.seed_min_score = 1.1,
            _ => config.min_score = 0.1,
        }
        let embeddings = Embeddings::default();
        let reranker = Reranker::new(Scores::Tied);
        assert!(matches!(
            semantic::prepare_with_backends(
                &[record("r", "retained text")],
                "query",
                &config,
                &RedactionPolicy::default(),
                &embeddings,
                &reranker
            ),
            Err(SemanticError::InvalidConfig(_))
        ));
        assert!(embeddings.calls.lock().map_err(|_| "test lock")?.is_empty());
        assert!(reranker.calls.lock().map_err(|_| "test lock")?.is_empty());
    }
    Ok(())
}

#[test]
fn derived_text_with_inaccessible_evidence_does_not_reach_reranking() -> TestResult {
    let mut original = record("original", "original evidence in another source");
    original.source_id = "seed-source".into();
    let mut summary = record("summary", "SEED DERIVED_CONTENT_OUTSIDE_SCOPE");
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.title = "DERIVED_TITLE_OUTSIDE_SCOPE".into();
    summary.execution = Some(serde_json::from_value(serde_json::json!({
        "id": "summary-execution",
        "command": "DERIVED_COMMAND_OUTSIDE_SCOPE",
        "tool_name": "DERIVED_TOOL_OUTSIDE_SCOPE",
        "tool_input": "DERIVED_INPUT_OUTSIDE_SCOPE",
        "last_observed_state": "completed",
        "liveness": "stopped"
    }))?);
    summary.evidence.push(Evidence {
        source_id: original.source_id.clone(),
        record_id: Some(original.id.clone()),
        revision: original.revision.clone(),
        locator: "record:original".into(),
        availability: Availability::Available,
        range: None,
    });
    let data = corpus(&[
        original,
        summary,
        record("safe", "ordinary accessible context"),
    ]);
    let mut query = request();
    query.scope.source_ids = vec!["journal".into()];
    let embeddings = Embeddings::default();
    let reranker = Reranker::new(Scores::Tied);
    let prepared = prepare(&data, &query, &config(), &embeddings, &reranker)?;
    let response = query::execute_semantic(&data, &query, &prepared, &RedactionPolicy::default())?;
    assert_eq!(reranker.calls.lock().map_err(|_| "test lock")?.len(), 2);
    assert_eq!(
        prepared
            .reranking
            .as_ref()
            .and_then(|info| info.seed.as_ref())
            .map(|seed| seed.record_id.as_str()),
        Some("safe")
    );
    assert!(
        prepared
            .candidates
            .iter()
            .all(|candidate| candidate.record_id != "summary")
    );
    let model_inputs = format!(
        "{}{}",
        serde_json::to_string(&*embeddings.calls.lock().map_err(|_| "test lock")?)?,
        serde_json::to_string(&*reranker.calls.lock().map_err(|_| "test lock")?)?
    );
    let mut read = Query::new(Operation::Read, "project");
    read.scope = query.scope.clone();
    read.target = Some(Target::Record {
        id: "summary".into(),
    });
    let direct = query::execute(&data, &read)?;
    let returned = format!(
        "{}{}{}",
        serde_json::to_string(&prepared)?,
        serde_json::to_string(&response)?,
        serde_json::to_string(&direct)?
    );
    for forbidden in [
        "DERIVED_CONTENT_OUTSIDE_SCOPE",
        "DERIVED_TITLE_OUTSIDE_SCOPE",
        "DERIVED_COMMAND_OUTSIDE_SCOPE",
        "DERIVED_TOOL_OUTSIDE_SCOPE",
        "DERIVED_INPUT_OUTSIDE_SCOPE",
    ] {
        assert!(
            !model_inputs.contains(forbidden),
            "inaccessible derived field reached the model: {forbidden}"
        );
        assert!(
            !returned.contains(forbidden),
            "inaccessible derived field reached a response: {forbidden}"
        );
    }
    Ok(())
}

#[test]
fn ordinary_words_ending_in_key_prefixes_keep_their_search_identity() -> TestResult {
    let policy = RedactionPolicy::default();
    let mut records = Vec::new();
    for id in [
        "umask-verification",
        "disk-verification",
        "task-verification",
    ] {
        let mut original = record(id, &format!("The {id} check completed successfully."));
        original.title = format!("Result for {id}");
        original.evidence.push(Evidence {
            source_id: "journal".into(),
            record_id: Some(id.into()),
            revision: original.revision.clone(),
            locator: format!("fixture:{id}"),
            availability: Availability::Available,
            range: None,
        });
        let entity = Entity::Record(original.clone());
        assert_eq!(
            serde_json::to_value(policy.entity(&entity)?)?,
            serde_json::to_value(entity)?,
            "ordinary identifier was mistaken for a provider key: {id}"
        );
        records.push(original);
    }
    let data = corpus(&records);
    let query = request();
    let embeddings = Embeddings::default();
    let reranker = Reranker::new(Scores::Tied);
    let prepared = prepare(&data, &query, &config(), &embeddings, &reranker)?;
    let response = query::execute_semantic(&data, &query, &prepared, &policy)?;
    let returned = serde_json::to_string(&response)?;
    for original in &records {
        assert!(
            prepared
                .candidates
                .iter()
                .any(|candidate| candidate.record_id == original.id),
            "prepared search lost the original identifier: {}",
            original.id
        );
        assert!(returned.contains(&original.id));
    }
    assert!(!returned.contains("[REDACTED]"));
    Ok(())
}

#[test]
fn standalone_provider_keys_and_literal_secrets_still_mask_every_text_surface() -> TestResult {
    let literal = "private-literal-fragment";
    let policy = RedactionPolicy {
        literal_secrets: vec![literal.into()],
    };
    for (secret, prefix, suffix) in [
        ("sk-fixture0123456789", "", ""),
        ("ghp_fixture0123456789", "", ""),
        ("github_pat_fixture0123456789", "", ""),
        ("AKIAABCDEFGHIJKLMNOP", "", ""),
        (literal, "embedded", "suffix"),
    ] {
        let token = format!("{prefix}{secret}{suffix}");
        let masked = format!("{prefix}[REDACTED]{suffix}");
        let mut original = record(&token, &format!("before {token} after"));
        original.title = format!("Value: {token}");
        original.evidence.push(Evidence {
            source_id: "journal".into(),
            record_id: Some(token.clone()),
            revision: original.revision.clone(),
            locator: format!("fixture:{token}"),
            availability: Availability::Available,
            range: None,
        });
        let sanitized = policy.entity(&Entity::Record(original))?;
        let Entity::Record(sanitized) = sanitized else {
            return Err("redaction changed the entity type".into());
        };
        assert_eq!(sanitized.id, masked);
        assert_eq!(sanitized.body, format!("before {masked} after"));
        assert_eq!(sanitized.title, format!("Value: {masked}"));
        assert_eq!(sanitized.availability, Availability::Redacted);
        let evidence = sanitized.evidence.first().ok_or("evidence absent")?;
        assert_eq!(evidence.record_id.as_deref(), Some(masked.as_str()));
        assert_eq!(evidence.locator, format!("fixture:{masked}"));
        assert!(!serde_json::to_string(&sanitized)?.contains(secret));
    }
    Ok(())
}
