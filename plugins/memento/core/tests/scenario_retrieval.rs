//! Scenario-level retrieval checks using concrete records, real imports, and storage.

use std::error::Error;
use std::fs;

use memento::adapters::ImportFormat;
use memento::model::*;
use memento::query::{self, BriefClaim, QueryError, QueryResponse, ResponseStatus};
use memento::security::{RedactionPolicy, hash};
use memento::{Store, ingest};
use serde_json::json;

#[path = "support/evaluation.rs"]
mod evaluation;

type TestResult = Result<(), Box<dyn Error>>;
const PROJECT: &str = "scenario-retrieval";

fn add(corpus: &mut Corpus, entity: Entity) {
    let sequence = corpus
        .entries
        .last()
        .map_or(1, |entry| entry.sequence.saturating_add(1));
    corpus.entries.push(Entry {
        sequence,
        captured_at: "2026-09-28T00:00:00Z".into(),
        entity,
    });
}

fn corpus() -> Corpus {
    let mut corpus = Corpus::default();
    add(
        &mut corpus,
        Entity::Source(ingest::source("journal", PROJECT, SourceKind::Journal)),
    );
    corpus
}

fn record(id: &str, work: &str, kind: RecordKind, body: &str) -> Record {
    let mut record = Record::new(id, PROJECT, "journal", kind, body);
    record.work_ids = vec![work.into()];
    record.association = Association::Explicit;
    record.nature = Nature::Reported;
    record.occurred_at = Some("2026-09-27T09:00:00Z".into());
    record
}

fn evidence(record: &Record) -> Evidence {
    Evidence {
        source_id: record.source_id.clone(),
        record_id: Some(record.id.clone()),
        revision: record.revision.clone(),
        locator: format!("record:{}", record.id),
        availability: record.availability,
        range: None,
    }
}

fn relation(id: &str, from: &Record, to: &Record, kind: RelationKind) -> Relation {
    Relation {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        from: Target::Record {
            id: from.id.clone(),
        },
        to: Target::Record { id: to.id.clone() },
        kind,
        nature: Nature::Reported,
        evidence: vec![evidence(from)],
        applies_to: Vec::new(),
    }
}

fn claims(response: &QueryResponse) -> Result<Vec<&BriefClaim>, Box<dyn Error>> {
    Ok(response
        .brief
        .as_ref()
        .ok_or("missing brief")?
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .collect())
}

fn claim<'a>(response: &'a QueryResponse, id: &str) -> Result<&'a BriefClaim, Box<dyn Error>> {
    claims(response)?
        .into_iter()
        .find(|claim| claim.record_id == id)
        .ok_or_else(|| format!("missing brief claim {id}").into())
}

fn item_record<'a>(response: &'a QueryResponse, id: &str) -> Result<&'a Record, Box<dyn Error>> {
    response
        .items
        .iter()
        .find_map(|item| match &item.entity {
            Entity::Record(record) if record.id == id => Some(record),
            _ => None,
        })
        .ok_or_else(|| format!("missing result record {id}").into())
}

fn query(operation: Operation, work: &str) -> Query {
    let mut query = Query::new(operation, PROJECT);
    query.scope.work_ids = vec![work.into()];
    query.limit = Some(100);
    query
}

fn record_query(operation: Operation, work: &str, id: &str) -> Query {
    let mut request = query(operation, work);
    request.target = Some(Target::Record { id: id.into() });
    if matches!(operation, Operation::Trace) {
        request.direction = Some(Direction::Both);
        request.max_depth = Some(3);
    }
    request
}

fn capture_pagination(variant: &str, data: &Corpus) -> TestResult {
    evaluation::capture(
        "SC08",
        variant,
        "페이지당 항목 수를 지금 어떻게 구현해야 하나요? 관리 화면과 일반 화면, 응답 필드, 새 의존성에 대해 현재 유효한 요구와 아직 결정할 부분을 설명해주세요.",
        data,
        &[
            record_query(Operation::Read, "pagination", "U2"),
            record_query(Operation::Read, "pagination", "U3"),
            record_query(Operation::Read, "pagination", "D6"),
            record_query(Operation::Trace, "pagination", "U3"),
            query(Operation::Brief, "pagination"),
        ],
    )
}

fn page_records(ambiguous: bool) -> Corpus {
    let mut data = corpus();
    let mut original = record(
        "U2",
        "pagination",
        RecordKind::Constraint,
        "페이지당 50개, 기존 응답 필드 유지, 새 의존성 금지. 관리 화면과 일반 화면 모두 적용.",
    );
    original.applies_to = vec![
        "admin-page-size".into(),
        "public-page-size".into(),
        "response-fields".into(),
        "dependencies".into(),
    ];
    let mut update = record(
        "U3",
        "pagination",
        RecordKind::Feedback,
        if ambiguous {
            "이번만 200개로 늘려줘."
        } else {
            "관리 화면만 200개로 늘려줘."
        },
    );
    update.occurred_at = Some("2026-09-27T10:00:00Z".into());
    update.applies_to = if ambiguous {
        vec!["unknown-page-scope".into()]
    } else {
        vec!["admin-page-size".into()]
    };
    let mut proposal = record(
        "D6",
        "pagination",
        RecordKind::Decision,
        "새 pagination 라이브러리 도입을 제안한다. 사용자 승인은 아직 없다.",
    );
    proposal.decision_status = Some(DecisionStatus::Proposed);
    let mut connection = relation(
        "page-count-change",
        &update,
        &original,
        if ambiguous {
            RelationKind::Contradicts
        } else {
            RelationKind::Supersedes
        },
    );
    connection.applies_to = update.applies_to.clone();
    if ambiguous {
        connection.nature = Nature::Inferred;
    }
    for record in [original, update, proposal] {
        add(&mut data, Entity::Record(record));
    }
    add(&mut data, Entity::Relation(connection));
    data
}

#[test]
fn sc08_a_partial_supersession_preserves_other_constraints_and_unapproved_proposal() -> TestResult {
    let data = page_records(false);
    capture_pagination("A", &data)?;
    let response = query::execute(&data, &query(Operation::Brief, "pagination"))?;
    let original = claim(&response, "U2")?;
    assert!(!original.applies_to.contains(&"admin-page-size".into()));
    assert!(original.applies_to.contains(&"public-page-size".into()));
    assert!(original.applies_to.contains(&"response-fields".into()));
    assert!(original.applies_to.contains(&"dependencies".into()));
    assert!(
        original
            .warnings
            .iter()
            .any(|warning| warning.contains("superseded only"))
    );
    assert_eq!(claim(&response, "U3")?.applies_to, vec!["admin-page-size"]);
    assert_eq!(
        claim(&response, "D6")?.decision_status,
        Some(DecisionStatus::Proposed)
    );
    assert!(
        response
            .brief
            .as_ref()
            .ok_or("missing brief")?
            .conflicts
            .is_empty()
    );
    Ok(())
}

#[test]
fn sc08_b_ambiguous_change_stays_an_unresolved_conflict() -> TestResult {
    let data = page_records(true);
    capture_pagination("B", &data)?;
    let response = query::execute(&data, &query(Operation::Brief, "pagination"))?;
    let conflicts = &response.brief.as_ref().ok_or("missing brief")?.conflicts;
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts.first().ok_or("missing conflict")?.kind,
        RelationKind::Contradicts
    );
    assert_eq!(
        conflicts.first().ok_or("missing conflict")?.nature,
        Nature::Inferred
    );
    assert_eq!(
        conflicts.first().ok_or("missing conflict")?.applies_to,
        vec!["unknown-page-scope"]
    );
    let original = claim(&response, "U2")?;
    assert!(original.applies_to.contains(&"admin-page-size".into()));
    assert!(original.applies_to.contains(&"public-page-size".into()));
    assert!(original.applies_to.contains(&"response-fields".into()));
    assert!(original.applies_to.contains(&"dependencies".into()));
    assert!(
        !original
            .warnings
            .iter()
            .any(|warning| warning.contains("superseded"))
    );
    assert_eq!(claim(&response, "U3")?.text, "이번만 200개로 늘려줘.");
    assert_eq!(
        claim(&response, "D6")?.decision_status,
        Some(DecisionStatus::Proposed)
    );
    Ok(())
}

fn upload_records(missing_output: bool, missing_context: bool) -> Corpus {
    let mut data = corpus();
    let work = Work {
        id: "W11".into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        title: "parallel upload experiment".into(),
        goal: "이미지 일괄 업로드".into(),
        status: WorkStatus::Active,
        observed_at: Some("2026-09-27T10:00:00Z".into()),
        evidence: Vec::new(),
        completion_conditions: Vec::new(),
    };
    add(&mut data, Entity::Work(work));
    let unrelated = record(
        "E12",
        "W12",
        RecordKind::ToolResult,
        "HTTP 401 authentication required",
    );
    add(&mut data, Entity::Record(unrelated));
    if missing_context {
        return data;
    }
    let mut attempt = record(
        "A11",
        "W11",
        RecordKind::Attempt,
        "parallel upload experiment: 동시 실행 수 16으로 파일 업로드를 시도함.",
    );
    attempt.attempt_outcome = Some(AttemptOutcome::Failed);
    let mut output = record(
        "E11",
        "W11",
        RecordKind::ToolResult,
        "provider status=HTTP 429; retryable=true; exit_code=1",
    );
    output.nature = Nature::Observed;
    if missing_output {
        output.availability = Availability::Missing;
        output.body.clear();
    }
    let mut decision = record(
        "D11",
        "W11",
        RecordKind::Decision,
        "당시 429가 발생했다고 보고되어 동시 실행 수를 4로 제한하기로 했다. 수정 후 성공 여부는 별도 확인이 필요하다.",
    );
    decision.decision_status = Some(DecisionStatus::Accepted);
    let observed = relation(
        "output-of-attempt",
        &output,
        &attempt,
        RelationKind::RespondsTo,
    );
    let successor = relation(
        "decision-after-attempt",
        &decision,
        &attempt,
        RelationKind::RespondsTo,
    );
    let belongs = Relation {
        id: "attempt-of-work".into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        from: Target::Record {
            id: attempt.id.clone(),
        },
        to: Target::Work { id: "W11".into() },
        kind: RelationKind::AttemptOf,
        nature: Nature::Reported,
        evidence: vec![evidence(&attempt)],
        applies_to: Vec::new(),
    };
    for record in [attempt, output, decision] {
        add(&mut data, Entity::Record(record));
    }
    for relation in [observed, successor, belongs] {
        add(&mut data, Entity::Relation(relation));
    }
    data
}

#[test]
fn sc10_a_wording_rewrite_finds_output_then_attempt_and_successor_decision() -> TestResult {
    let data = upload_records(false, false);
    let mut request = query(Operation::Search, "W11");
    request.query = Some(TextQuery {
        text: "동시 요청 때문에 막혔던 방식".into(),
        mode: SearchMode::Tokens,
    });
    let mut evaluation_queries = vec![request.clone()];
    assert_eq!(
        query::execute(&data, &request)?.status,
        ResponseStatus::NoMatches
    );
    request.query = Some(TextQuery {
        text: "HTTP 429".into(),
        mode: SearchMode::Literal,
    });
    request.filters.attempt_outcomes = vec![AttemptOutcome::Failed];
    evaluation_queries.push(request.clone());
    assert!(query::execute(&data, &request)?.items.is_empty());
    request.filters = Filters::default();
    evaluation_queries.push(request.clone());
    let found = query::execute(&data, &request)?;
    assert_eq!(found.items.len(), 1);
    let original = item_record(&found, "E11")?;
    assert_eq!(original.nature, Nature::Observed);
    assert!(
        found
            .items
            .first()
            .ok_or("missing output match")?
            .match_locations
            .contains(&"body".into())
    );
    let mut read = query(Operation::Read, "W11");
    read.target = Some(Target::Record {
        id: original.id.clone(),
    });
    assert!(
        item_record(&query::execute(&data, &read)?, "E11")?
            .body
            .contains("exit_code=1")
    );
    let mut trace = query(Operation::Trace, "W11");
    trace.target = read.target.clone();
    trace.direction = Some(Direction::Both);
    trace.max_depth = Some(3);
    let path = query::execute(&data, &trace)?;
    assert_eq!(
        item_record(&path, "A11")?.attempt_outcome,
        Some(AttemptOutcome::Failed)
    );
    assert!(item_record(&path, "D11")?.body.contains("4로 제한"));
    assert!(
        path.items
            .iter()
            .any(|item| matches!(&item.entity, Entity::Work(work) if work.id == "W11"))
    );
    assert!(!path.items.iter().any(|item| item.entity.id() == "E12"));
    let resumed = query::execute(&data, &query(Operation::Brief, "W11"))?;
    assert_eq!(claim(&resumed, "D11")?.nature, Nature::Reported);
    assert!(
        claim(&resumed, "D11")?
            .text
            .contains("성공 여부는 별도 확인")
    );
    evaluation_queries.extend([read, trace, query(Operation::Brief, "W11")]);
    evaluation::capture(
        "SC10",
        "A",
        "이미지 업로드에서 동시 요청 때문에 막혔던 방식이 무엇이었고, 왜 포기했나요? 그 뒤 어떤 방식으로 바꿨고 지금 성공한 상태인지 기록을 찾아 설명해주세요.",
        &data,
        &evaluation_queries,
    )?;
    Ok(())
}

#[test]
fn sc10_b_missing_output_keeps_surviving_report_and_b2_is_bounded_no_matches() -> TestResult {
    let data = upload_records(true, false);
    let mut search = query(Operation::Search, "W11");
    search.query = Some(TextQuery {
        text: "parallel upload".into(),
        mode: SearchMode::Literal,
    });
    let found = query::execute(&data, &search)?;
    assert_eq!(found.items.len(), 1);
    assert_eq!(
        found.items.first().ok_or("missing attempt")?.entity.id(),
        "A11"
    );
    let mut trace = query(Operation::Trace, "W11");
    trace.target = Some(Target::Record { id: "A11".into() });
    trace.direction = Some(Direction::Both);
    let path = query::execute(&data, &trace)?;
    let missing = item_record(&path, "E11")?;
    assert!(missing.body.is_empty());
    assert_eq!(missing.availability, Availability::Missing);
    assert_eq!(item_record(&path, "D11")?.nature, Nature::Reported);
    assert!(
        path.items
            .iter()
            .any(|item| matches!(&item.entity, Entity::Work(work) if work.id == "W11"))
    );
    let resumed = query::execute(&data, &query(Operation::Brief, "W11"))?;
    assert!(claim(&resumed, "E11")?.text.is_empty());
    assert!(
        claim(&resumed, "E11")?
            .warnings
            .iter()
            .any(|warning| warning.contains("body unavailable"))
    );
    assert!(claim(&resumed, "D11")?.text.contains("보고되어"));
    assert_eq!(claim(&resumed, "D11")?.nature, Nature::Reported);
    evaluation::capture(
        "SC10",
        "B",
        "이미지 업로드에서 동시 요청 때문에 막혔던 방식이 무엇이었고, 왜 포기했나요? 그 뒤 어떤 방식으로 바꿨고 지금 성공한 상태인지 기록을 찾아 설명해주세요.",
        &data,
        &[
            search.clone(),
            trace,
            record_query(Operation::Read, "W11", "E11"),
            record_query(Operation::Read, "W11", "D11"),
            query(Operation::Brief, "W11"),
        ],
    )?;
    let no_context = upload_records(true, true);
    search.query = Some(TextQuery {
        text: "HTTP 429".into(),
        mode: SearchMode::Literal,
    });
    let absent = query::execute(&no_context, &search)?;
    assert_eq!(absent.status, ResponseStatus::NoMatches);
    assert_eq!(absent.scope.work_ids, vec!["W11"]);
    assert!(absent.items.is_empty());
    assert!(!absent.coverage.is_empty());
    evaluation::capture(
        "SC10",
        "B2",
        "이미지 업로드에서 동시 요청 때문에 막혔던 방식이 무엇이었고, 왜 포기했나요? 그 뒤 어떤 방식으로 바꿨고 지금 성공한 상태인지 기록을 찾아 설명해주세요.",
        &no_context,
        &[
            search,
            record_query(Operation::Read, "W11", "A11"),
            record_query(Operation::Trace, "W11", "A11"),
            query(Operation::Brief, "W11"),
        ],
    )?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc11_real_import_checkpoint_recovers_late_event_and_revised_brief() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = directory.path().join("selected.jsonl");
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let first = json!({"id":"request","kind":"request","text":"최대 재시도 5회","work_id":"W11","session_id":"session","nature":"reported","occurred_at":"2026-09-27T12:00:00Z"});
    let status = json!({"id":"status","kind":"status","text":"5회 구현 완료; 부하 검증은 아직 남음","work_id":"W11","session_id":"session","nature":"reported","occurred_at":"2026-09-27T13:00:00Z"});
    fs::write(&journal, format!("{first}\n{status}\n"))?;
    ingest::import(
        &mut store,
        PROJECT,
        "export",
        &journal,
        ImportFormat::JournalJsonl,
        Some("W11"),
    )
    .await?;
    let initial = store.load().await?;
    let before = query::execute(&initial, &query(Operation::Brief, "W11"))?;
    assert!(
        claim(&before, "export:status")?
            .text
            .contains("5회 구현 완료")
    );
    let checkpoint = before.checkpoint.ok_or("missing complete checkpoint")?;
    let revised = json!({"id":"status","kind":"status","text":"정정: 최대 재시도 3회로 변경해야 함. 이전 완료 보고는 현재 요구 충족이 아님.","work_id":"W11","session_id":"session","nature":"reported","occurred_at":"2026-09-27T13:00:00Z"});
    let late = json!({"id":"old-failure","kind":"attempt","text":"오래된 실패: 16개 동시 전송에서 provider 429; 아직 회수되지 않았던 출력 참조","work_id":"W11","session_id":"session","nature":"reported","occurred_at":"2026-09-25T08:00:00Z"});
    fs::write(&journal, format!("{first}\n{revised}\n{late}\n"))?;
    ingest::import(
        &mut store,
        PROJECT,
        "export",
        &journal,
        ImportFormat::JournalJsonl,
        Some("W11"),
    )
    .await?;
    let updated = store.load().await?;
    let mut delta = query(Operation::Compare, "W11");
    delta.since_checkpoint = Some(checkpoint.clone());
    delta.limit = Some(1);
    let first_page = query::execute(&updated, &delta)?;
    assert!(first_page.next_cursor.is_some());
    assert!(first_page.checkpoint.is_none());
    let mut collected = Vec::new();
    let mut final_checkpoint = None;
    for _ in 0..10 {
        let page = query::execute(&updated, &delta)?;
        collected.extend(page.items);
        delta.cursor = page.next_cursor;
        if delta.cursor.is_none() {
            final_checkpoint = page.checkpoint;
            break;
        }
    }
    assert!(final_checkpoint.is_some());
    let late = collected
        .iter()
        .find_map(|item| match &item.entity {
            Entity::Record(record) if record.id == "export:old-failure" => Some(record),
            _ => None,
        })
        .ok_or("late historical event was not recovered")?;
    assert_eq!(
        late.occurred_at.as_deref(),
        Some("2026-09-25T08:00:00+00:00")
    );
    assert!(collected.iter().any(|item| matches!(&item.entity, Entity::Record(record) if record.id == "export:status" && record.body.contains("3회"))));
    assert!(collected.iter().all(|item| {
        item.warnings
            .iter()
            .any(|warning| warning.contains("newly captured or revised"))
    }));
    let after = query::execute(&updated, &query(Operation::Brief, "W11"))?;
    assert!(claim(&after, "export:status")?.text.contains("3회"));
    assert!(
        !claims(&after)?
            .iter()
            .any(|claim| claim.text.contains("5회 구현 완료"))
    );
    assert!(
        claim(&after, "export:old-failure")?
            .text
            .contains("오래된 실패")
    );
    let mut complete_delta = delta.clone();
    complete_delta.cursor = None;
    complete_delta.limit = Some(100);
    evaluation::capture(
        "SC11",
        "A",
        "지난 조회 이후 새로 알게 된 변경만 설명해주세요. 오래전에 발생한 기록이 지금 들어왔는지, 이전 완료 보고가 아직 유효한지, 오늘 이어갈 작업이 무엇인지 확인해주세요.",
        &updated,
        &[
            complete_delta,
            record_query(Operation::Read, "W11", "export:old-failure"),
            record_query(Operation::Read, "W11", "export:status"),
            query(Operation::Brief, "W11"),
        ],
    )?;
    let mut invalid = query(Operation::Compare, "W11");
    invalid.since_checkpoint = Some("not-an-opaque-checkpoint".into());
    assert!(matches!(
        query::execute(&updated, &invalid),
        Err(QueryError::RescanRequired)
    ));
    evaluation::capture(
        "SC11",
        "B2",
        "이전에 저장한 checkpoint로 지난 조회 이후 변경만 설명해주세요. 이전 완료 보고가 아직 유효한지, 오늘 이어갈 작업이 무엇인지 확인해주세요.",
        &updated,
        &[
            invalid.clone(),
            record_query(Operation::Read, "W11", "export:status"),
            query(Operation::Brief, "W11"),
        ],
    )?;
    invalid.since_checkpoint = first_page.checkpoint;
    assert!(matches!(
        query::execute(&updated, &invalid),
        Err(QueryError::RescanRequired)
    ));
    evaluation::capture(
        "SC11",
        "B",
        "지난 조회의 checkpoint를 찾을 수 없어요. 그 이후의 변경과 현재 이어갈 작업을 설명해주세요. 어느 범위까지 확인할 수 있는지도 알려주세요.",
        &updated,
        &[
            invalid.clone(),
            record_query(Operation::Read, "W11", "export:old-failure"),
            record_query(Operation::Read, "W11", "export:status"),
            query(Operation::Brief, "W11"),
        ],
    )?;
    // Missing progress cannot be fabricated. A fresh bounded query recovers context.
    let rescanned = query::execute(&updated, &query(Operation::Brief, "W11"))?;
    assert!(rescanned.checkpoint.is_some());
    assert!(
        claim(&rescanned, "export:old-failure")?
            .text
            .contains("429")
    );
    let mut wrong_scope = query(Operation::Compare, "unrelated-work");
    wrong_scope.since_checkpoint = Some(checkpoint);
    assert!(matches!(
        query::execute(&updated, &wrong_scope),
        Err(QueryError::RescanRequired)
    ));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc14_twenty_mib_masked_log_middle_window_and_revocation_invalidate_pages() -> TestResult {
    let directory = tempfile::tempdir()?;
    let secret = "SC14_LITERAL_PRIVATE_VALUE_8c1d6f";
    let credential = "SC14_PASSWORD_VALUE_42e9";
    let policy = RedactionPolicy {
        literal_secrets: vec![secret.into()],
    };
    let mut store = Store::open(&directory.path().join("context.sqlite"), policy).await?;
    let source = ingest::source("logs", PROJECT, SourceKind::Journal);
    store.append(Entity::Source(source.clone())).await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            PROJECT,
            SourceKind::Journal,
        )))
        .await?;
    let line = format!("{}\n", "x".repeat(1023));
    let half = line.repeat(10 * 1024);
    let body = format!("{half}SC14_MATCH HTTP 429 password={credential} literal={secret}\n{half}");
    assert!(body.len() > 20 * 1024 * 1024);
    let mut large = Record::new(
        "large-output",
        PROJECT,
        "logs",
        RecordKind::ToolResult,
        &body,
    );
    large.work_ids = vec!["W14".into()];
    large.association = Association::Explicit;
    let original_revision = large.revision.clone();
    store.append(Entity::Record(large)).await?;
    let mut summary = record(
        "log-summary",
        "W14",
        RecordKind::Status,
        "SC14_MATCH was reported in the retained run; only an evidence-derived summary.",
    );
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.evidence = vec![Evidence {
        source_id: "logs".into(),
        record_id: Some("large-output".into()),
        revision: original_revision.clone(),
        locator: "record:large-output".into(),
        availability: Availability::Available,
        range: None,
    }];
    store.append(Entity::Record(summary)).await?;
    let corpus = store.load().await?;
    let retained = corpus
        .entries
        .iter()
        .find_map(|entry| match &entry.entity {
            Entity::Record(record) if record.id == "large-output" => Some(record),
            _ => None,
        })
        .ok_or("missing persisted log")?;
    assert!(!retained.body.contains(secret));
    assert!(!retained.body.contains(credential));
    assert_eq!(retained.availability, Availability::Redacted);
    let rendered_revision = hash(retained.body.as_bytes());
    let mut search = query(Operation::Search, "W14");
    search.scope.source_ids = vec!["logs".into()];
    search.query = Some(TextQuery {
        text: "SC14_MATCH".into(),
        mode: SearchMode::Literal,
    });
    search.budget_bytes = Some(16 * 1024);
    let found = query::execute(&corpus, &search)?;
    let match_item = found.items.first().ok_or("middle match was not found")?;
    assert!(
        match_item
            .excerpt
            .as_ref()
            .is_some_and(|text| text.contains("SC14_MATCH") && text.contains("[REDACTED]"))
    );
    assert_eq!(
        match_item.rendered_revision.as_deref(),
        Some(rendered_revision.as_str())
    );
    assert!(serde_json::to_vec(&found)?.len() <= 16 * 1024);
    let mut read = query(Operation::Read, "W14");
    read.scope.source_ids = vec!["logs".into()];
    read.target = Some(Target::Artifact {
        record_id: "large-output".into(),
        revision: original_revision,
        range: None,
    });
    read.range = Some(
        match_item
            .match_ranges
            .iter()
            .find(|matched| matched.field == "body")
            .ok_or("search did not return a body match locator")?
            .range
            .clone(),
    );
    read.context_lines = Some(1);
    read.budget_bytes = Some(8192);
    let window = query::execute(&corpus, &read)?;
    let window_item = window.items.first().ok_or("missing middle window")?;
    assert_eq!(
        window_item.rendered_revision.as_deref(),
        Some(rendered_revision.as_str())
    );
    assert!(
        item_record(&window, "large-output")?
            .body
            .contains("SC14_MATCH")
    );
    assert!(
        item_record(&window, "large-output")?
            .body
            .contains("[REDACTED]")
    );
    assert_eq!(
        item_record(&window, "large-output")?.body.lines().count(),
        3
    );
    assert!(!serde_json::to_string(&window)?.contains(secret));
    assert!(!serde_json::to_string(&window)?.contains(credential));
    evaluation::capture(
        "SC14",
        "A",
        "W14 실행 로그에서 SC14_MATCH가 나온 지점의 원문과 앞뒤 문맥을 확인해 무슨 일이 있었는지 설명해주세요. 검색 요약과 원문이 같은 보존본인지도 확인해주세요.",
        &corpus,
        &[search.clone(), read.clone(), query(Operation::Brief, "W14")],
    )?;
    read.range = None;
    read.context_lines = None;
    let page = query::execute(&corpus, &read)?;
    assert!(page.next_cursor.is_some());
    assert!(page.checkpoint.is_none());
    assert_eq!(page.status, ResponseStatus::Partial);
    assert!(serde_json::to_vec(&page)?.len() <= 8192);
    read.cursor = page.next_cursor;
    let mut revoked = source;
    revoked.authorized = false;
    store.append(Entity::Source(revoked)).await?;
    let scrubbed = store.load().await?;
    assert!(matches!(
        query::execute(&scrubbed, &read),
        Err(QueryError::StaleCursor)
    ));
    let serialized = serde_json::to_string(&scrubbed)?;
    assert!(!serialized.contains(secret));
    assert!(!serialized.contains(credential));
    assert!(!serialized.contains("SC14_MATCH"));
    let surviving = query::execute(&scrubbed, &query(Operation::Brief, "W14"))?;
    assert!(claim(&surviving, "log-summary")?.text.is_empty());
    assert!(
        surviving
            .items
            .iter()
            .all(|item| item.entity.source_id() != "logs")
    );
    evaluation::capture(
        "SC14",
        "B",
        "W14 실행 로그의 다음 페이지를 이어 읽고 SC14_MATCH 원문을 확인해 무슨 일이 있었는지 설명해주세요. 현재 접근 가능한 근거 범위도 알려주세요.",
        &scrubbed,
        &[search, read, query(Operation::Brief, "W14")],
    )?;
    Ok(())
}

fn code_state(id: &str, content: &str, changed_during_observation: bool) -> CodeState {
    CodeState {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        repository_id: PROJECT.into(),
        worktree_id: Some("shared-worktree".into()),
        commit_sha: Some("retained-commit".into()),
        observed_at: "2026-09-27T10:00:00Z".into(),
        changed_during_observation,
        files: vec![FileState {
            path: "src/retry.rs".into(),
            working_hash: Some(hash(content.as_bytes())),
            working_kind: WorkingFileKind::File,
            ..FileState::default()
        }],
    }
}

fn parallel_records(integrated: bool) -> Corpus {
    let mut data = corpus();
    for (id, status) in [
        ("parent", WorkStatus::Active),
        ("child", WorkStatus::Completed),
    ] {
        add(
            &mut data,
            Entity::Work(Work {
                id: id.into(),
                project_id: PROJECT.into(),
                source_id: "journal".into(),
                title: id.into(),
                goal: "retry 개선과 검증".into(),
                status,
                observed_at: Some("2026-09-27T10:00:00Z".into()),
                evidence: Vec::new(),
                completion_conditions: Vec::new(),
            }),
        );
    }
    let child = record(
        "child-done",
        "child",
        RecordKind::Status,
        "하위 작업 완료라고 보고함. 선택한 단위 테스트만 실행했고 원 작업 반영은 별개다.",
    );
    let mut known = record(
        "agent-edit",
        "parent",
        RecordKind::Change,
        "도구가 A의 src/retry.rs 변경 범위를 기록했다.",
    );
    known.nature = Nature::Observed;
    known.actor = Some(Actor {
        kind: ActorKind::Agent,
        name: Some("A".into()),
    });
    known.worktree_id = Some("shared-worktree".into());
    known.paths = vec!["src/retry.rs".into()];
    let mut unknown = record(
        "unknown-edit",
        "parent",
        RecordKind::Change,
        "같은 작업 공간에서 추가 변경을 관측했으나 작성자 로그는 없다.",
    );
    unknown.nature = Nature::Observed;
    unknown.actor = Some(Actor {
        kind: ActorKind::Unknown,
        name: None,
    });
    unknown.worktree_id = known.worktree_id.clone();
    unknown.paths = known.paths.clone();
    unknown.association = Association::Candidate;
    for record in [child.clone(), known, unknown] {
        add(&mut data, Entity::Record(record));
    }
    add(
        &mut data,
        Entity::Relation(Relation {
            id: "child-report".into(),
            project_id: PROJECT.into(),
            source_id: "journal".into(),
            from: Target::Record {
                id: child.id.clone(),
            },
            to: Target::Work { id: "child".into() },
            kind: RelationKind::RelatedTo,
            nature: Nature::Reported,
            evidence: vec![evidence(&child)],
            applies_to: Vec::new(),
        }),
    );
    if integrated {
        let integration = record(
            "integration-output",
            "parent",
            RecordKind::ToolResult,
            "하위 패치 반영을 확인함. 현재 전체 검증은 아직 실행하지 않았다.",
        );
        add(
            &mut data,
            Entity::Relation(Relation {
                id: "child-integrated".into(),
                project_id: PROJECT.into(),
                source_id: "journal".into(),
                from: Target::Work { id: "child".into() },
                to: Target::Work {
                    id: "parent".into(),
                },
                kind: RelationKind::IntegratedInto,
                nature: Nature::Observed,
                evidence: vec![evidence(&integration)],
                applies_to: vec!["src/retry.rs".into()],
            }),
        );
        add(&mut data, Entity::Record(integration));
    }
    add(
        &mut data,
        Entity::CodeState(code_state("before-edit", "before", false)),
    );
    add(
        &mut data,
        Entity::CodeState(code_state("changing-edit", "changed while observing", true)),
    );
    data
}

#[test]
fn sc07_child_completion_integration_and_unknown_shared_edits_remain_distinct() -> TestResult {
    for integrated in [false, true] {
        let data = parallel_records(integrated);
        let parent = query::execute(&data, &query(Operation::Brief, "parent"))?;
        assert!(parent.items.iter().any(|item| matches!(&item.entity, Entity::Work(work) if work.id == "parent" && work.status == WorkStatus::Active)));
        assert!(
            !claims(&parent)?
                .iter()
                .any(|claim| claim.record_id == "child-done")
        );
        let agent = item_record(&parent, "agent-edit")?
            .actor
            .as_ref()
            .ok_or("missing known actor")?;
        let unknown = item_record(&parent, "unknown-edit")?
            .actor
            .as_ref()
            .ok_or("missing unknown actor")?;
        assert_eq!(agent.kind, ActorKind::Agent);
        assert_eq!(agent.name.as_deref(), Some("A"));
        assert_eq!(unknown.kind, ActorKind::Unknown);
        assert!(unknown.name.is_none());
        assert!(
            claim(&parent, "unknown-edit")?
                .warnings
                .iter()
                .any(|warning| warning.contains("candidate"))
        );
        let mut trace = Query::new(Operation::Trace, PROJECT);
        trace.target = Some(Target::Work {
            id: "parent".into(),
        });
        trace.direction = Some(Direction::Both);
        let traced = query::execute(&data, &trace)?;
        assert_eq!(
            traced
                .relations
                .iter()
                .any(|relation| relation.kind == RelationKind::IntegratedInto),
            integrated
        );
        assert_eq!(traced.items.iter().any(|item| matches!(&item.entity, Entity::Work(work) if work.id == "child" && work.status == WorkStatus::Completed)), integrated);
        let mut compare = Query::new(Operation::Compare, PROJECT);
        compare.from = Some(HistoryPoint {
            code_state_id: Some("before-edit".into()),
            ..HistoryPoint::default()
        });
        compare.to = Some(HistoryPoint {
            code_state_id: Some("changing-edit".into()),
            ..HistoryPoint::default()
        });
        let response = query::execute(&data, &compare)?;
        assert_eq!(
            response
                .comparison
                .as_ref()
                .and_then(|comparison| comparison.code.as_ref())
                .ok_or("missing code comparison")?
                .status,
            "changed_during_observation"
        );
        assert!(
            claims(&parent)?
                .iter()
                .all(|claim| claim.verification_outcome != Some(VerificationOutcome::Passed))
        );
        let mut evaluation_queries = vec![
            record_query(Operation::Read, "child", "child-done"),
            record_query(Operation::Read, "parent", "agent-edit"),
            record_query(Operation::Read, "parent", "unknown-edit"),
            trace,
            compare,
            query(Operation::Brief, "parent"),
            query(Operation::Brief, "child"),
        ];
        if integrated {
            evaluation_queries.push(record_query(
                Operation::Read,
                "parent",
                "integration-output",
            ));
        }
        evaluation::capture(
            "SC07",
            if integrated { "A" } else { "B" },
            "하위 에이전트 작업이 끝났다고 했는데 원 작업도 완료된 건가요? 하위 패치 반영 여부, 공유 작업 공간의 변경 작성자, 현재 검증 상태를 근거와 함께 설명해주세요.",
            &data,
            &evaluation_queries,
        )?;
    }
    Ok(())
}

fn execution(id: &str, passed: bool, ci: bool) -> Execution {
    Execution {
        id: id.into(),
        tool_name: None,
        tool_input: None,
        command: if ci {
            "cargo test --all-features"
        } else {
            "cargo test --lib retry"
        }
        .into(),
        cwd: Some("selected-repository".into()),
        started_at: Some("2026-09-27T10:00:00Z".into()),
        ended_at: Some("2026-09-27T10:01:00Z".into()),
        exit_code: Some(if passed { 0 } else { 101 }),
        last_observed_state: if passed { "succeeded" } else { "failed" }.into(),
        observed_at: Some("2026-09-27T10:01:00Z".into()),
        liveness: Liveness::Stopped,
        before_state: Some("same-code".into()),
        after_state: Some("same-code".into()),
        scope: if ci {
            vec!["all-features".into(), "postgres integration".into()]
        } else {
            vec!["unit:retry".into(), "default-features".into()]
        },
        environment: Environment {
            os: Some(if ci { "linux" } else { "macos" }.into()),
            toolchain: Some(if ci { "rust-1.94" } else { "rust-1.93" }.into()),
            profile: Some(if ci { "ci" } else { "local-unit" }.into()),
            dependencies: vec![
                if passed {
                    "postgres available"
                } else {
                    "postgres condition needs evidence"
                }
                .into(),
            ],
        },
    }
}

fn retry_records(ci_available: bool) -> Corpus {
    let mut data = corpus();
    let mut ci = ingest::source("ci", PROJECT, SourceKind::Journal);
    ci.available = ci_available;
    if !ci_available {
        ci.gaps.push("current CI output unavailable".into());
    }
    add(&mut data, Entity::Source(ci));
    add(
        &mut data,
        Entity::CodeState(code_state(
            "same-code",
            "unchanged retry implementation",
            false,
        )),
    );
    let mut first = record(
        "X10",
        "W9",
        RecordKind::Attempt,
        "같은 patch를 적용하고 명령 실행; DB 접속 단계에서 실패했다.",
    );
    first.attempt_id = Some("attempt-first".into());
    first.attempt_outcome = Some(AttemptOutcome::Failed);
    first.execution = Some(execution("execution-first", false, false));
    let mut output = record(
        "E10",
        "W9",
        RecordKind::ToolResult,
        "phase=database-connect connection refused; test body was not entered; exit_code=101",
    );
    output.nature = Nature::Observed;
    let mut guess = record(
        "G10",
        "W9",
        RecordKind::Finding,
        "코드 버그일 수 있다는 당시 가설. 원인은 아직 확인하지 않았다.",
    );
    guess.nature = Nature::Inferred;
    let mut second = record(
        "X11",
        "W9",
        RecordKind::Attempt,
        "파일을 복원했다가 같은 patch를 다시 적용하고 DB 설정을 수정한 뒤 동일 명령을 재실행했다.",
    );
    second.attempt_id = Some("attempt-second".into());
    second.attempt_outcome = Some(AttemptOutcome::Succeeded);
    second.execution = Some(execution("execution-second", true, false));
    let mut verification = record(
        "V11",
        "W9",
        RecordKind::Verification,
        "local default-feature retry unit tests passed. 다른 feature/OS/integration 조합은 이 결과에 포함하지 않음.",
    );
    verification.nature = Nature::Observed;
    verification.verification_outcome = Some(VerificationOutcome::Passed);
    verification.execution = second.execution.clone();
    let mut ci_result = record(
        "V12",
        "W9",
        RecordKind::Verification,
        if ci_available {
            "CI all-feature postgres integration failed: schema-version mismatch; local unit pass cannot cover this scope."
        } else {
            ""
        },
    );
    ci_result.source_id = "ci".into();
    ci_result.nature = Nature::Observed;
    ci_result.verification_outcome = Some(if ci_available {
        VerificationOutcome::Failed
    } else {
        VerificationOutcome::Unknown
    });
    ci_result.execution = ci_available.then(|| execution("ci-execution", false, true));
    if !ci_available {
        ci_result.availability = Availability::Missing;
    }
    for relation in [
        relation("failure-output", &output, &first, RelationKind::RespondsTo),
        relation("failure-guess", &guess, &first, RelationKind::RelatedTo),
        relation(
            "retry-result",
            &verification,
            &second,
            RelationKind::Verifies,
        ),
    ] {
        add(&mut data, Entity::Relation(relation));
    }
    for record in [first, output, guess, second, verification, ci_result] {
        add(&mut data, Entity::Record(record));
    }
    data
}

#[test]
fn sc09_retry_phase_guess_and_local_ci_verification_scopes_stay_separate() -> TestResult {
    for ci_available in [true, false] {
        let data = retry_records(ci_available);
        let response = query::execute(&data, &query(Operation::Brief, "W9"))?;
        let first = item_record(&response, "X10")?
            .execution
            .as_ref()
            .ok_or("missing first execution")?;
        let retry = item_record(&response, "X11")?
            .execution
            .as_ref()
            .ok_or("missing retry execution")?;
        assert_ne!(first.id, retry.id);
        assert_eq!(first.command, retry.command);
        assert_eq!(first.before_state, retry.before_state);
        assert_eq!(first.exit_code, Some(101));
        assert_eq!(retry.exit_code, Some(0));
        assert_eq!(claim(&response, "E10")?.nature, Nature::Observed);
        assert!(
            claim(&response, "E10")?
                .text
                .contains("test body was not entered")
        );
        assert_eq!(claim(&response, "G10")?.nature, Nature::Inferred);
        assert!(claim(&response, "G10")?.text.contains("가설"));
        assert_eq!(
            claim(&response, "V11")?.verification_outcome,
            Some(VerificationOutcome::Passed)
        );
        let local = item_record(&response, "V11")?
            .execution
            .as_ref()
            .ok_or("missing local verification execution")?;
        assert_eq!(local.scope, vec!["unit:retry", "default-features"]);
        assert_eq!(local.environment.profile.as_deref(), Some("local-unit"));
        if ci_available {
            assert_eq!(
                claim(&response, "V12")?.verification_outcome,
                Some(VerificationOutcome::Failed)
            );
            let ci = item_record(&response, "V12")?
                .execution
                .as_ref()
                .ok_or("missing CI verification execution")?;
            assert_ne!(local.scope, ci.scope);
            assert_ne!(local.environment.os, ci.environment.os);
            assert_ne!(local.environment.toolchain, ci.environment.toolchain);
            assert_ne!(local.environment.profile, ci.environment.profile);
            assert!(
                claim(&response, "V12")?
                    .text
                    .contains("schema-version mismatch")
            );
        } else {
            assert_eq!(
                claim(&response, "V12")?.verification_outcome,
                Some(VerificationOutcome::Unknown)
            );
            assert!(claim(&response, "V12")?.text.is_empty());
            assert!(
                response
                    .freshness
                    .iter()
                    .any(|source| source.source_id == "ci" && !source.available)
            );
            assert!(
                response
                    .coverage
                    .iter()
                    .any(|source| source.source_id == "ci"
                        && source.gaps.iter().any(|gap| gap.contains("unavailable")))
            );
        }
        evaluation::capture(
            "SC09",
            if ci_available { "A" } else { "B" },
            "같은 패치와 같은 명령이 처음에는 실패하고 재실행에서는 통과한 이유를 설명해주세요. 실패한 단계와 원인 가설을 구분하고, 로컬 통과와 CI 결과를 바탕으로 전체 검증이 완료됐는지 알려주세요.",
            &data,
            &[
                record_query(Operation::Read, "W9", "E10"),
                record_query(Operation::Read, "W9", "G10"),
                record_query(Operation::Read, "W9", "V11"),
                record_query(Operation::Read, "W9", "V12"),
                record_query(Operation::Trace, "W9", "X10"),
                record_query(Operation::Trace, "W9", "X11"),
                query(Operation::Brief, "W9"),
            ],
        )?;
    }
    Ok(())
}
