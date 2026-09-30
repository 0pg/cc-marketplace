//! Korean synthetic conversations with explicit annotations; no NLP or live-service claims.
//! Each journey crosses SQLite persistence, Crepe retention, reopen, and bounded retrieval.

#[path = "support/raw_dialogue.rs"]
mod raw_dialogue;

use std::{collections::BTreeSet, error::Error};

use raw_dialogue::*;
use work_context::{
    Error as StoreError, Store,
    compaction::Policy,
    ingest,
    model::*,
    query::{self as retrieval, QueryError, QueryResponse},
    security::RedactionPolicy,
};

fn target(id: &str) -> Target {
    Target::Record { id: id.into() }
}

fn artifact(record: &Record) -> Target {
    Target::Artifact {
        record_id: record.id.clone(),
        revision: record.revision.clone(),
        range: None,
    }
}

fn trace(corpus: &Corpus, target: Target) -> Result<QueryResponse, QueryError> {
    let mut q = query(Operation::Trace);
    q.target = Some(target);
    q.direction = Some(Direction::Both);
    retrieval::execute(corpus, &q)
}

async fn reopen(dialogue: &mut Dialogue) -> Result<Corpus, Box<dyn Error>> {
    dialogue.store = Store::open(&dialogue.path, RedactionPolicy::default()).await?;
    Ok(dialogue.store.load().await?)
}

fn policy(max_entries: usize) -> Policy {
    Policy {
        max_entries,
        max_payload_bytes: 262_144,
        recent_entries: 0,
    }
}

async fn research_journey(has_old_excerpt: bool) -> TestResult {
    // A month after a document-only investigation, the user revisits a rejected product.
    let mut dialogue = Dialogue::new().await?;
    let request = message(
        "U13",
        ActorKind::Human,
        RecordKind::Request,
        "저장소 후보 A를 조사해줘. 외부 서버 없이 로컬에서 실행돼야 해. 보고서만 작성하고 코드는 바꾸지 마.",
    );
    let mut old = message(
        "R13",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "확인 시점: 2026-08-01\n조사 URL: https://example.invalid/a/deployment\n당시 발췌: A는 호스팅 서비스 연결이 필요합니다.",
    );
    old.paths = vec!["docs/storage-evaluation.md".into()];
    let mut decision = message(
        "D13",
        ActorKind::Agent,
        RecordKind::Decision,
        "당시 자료의 호스팅 연결 조건이 로컬 실행 요구와 맞지 않아 A를 보류합니다. 독립 실행 조건이 달라지면 다시 검토해야 합니다.",
    );
    decision.decision_status = Some(DecisionStatus::Rejected);
    let mut citation = evidence(&old);
    citation.locator = "https://example.invalid/a/deployment#checked-2026-08-01".into();
    citation.range = Some(TextRange {
        start_line: 3,
        end_line: 3,
    });
    if !has_old_excerpt {
        citation.availability = Availability::Missing;
    }
    decision.evidence = vec![evidence(&request), citation];
    let mut current = message(
        "R13",
        ActorKind::Agent,
        RecordKind::Finding,
        "한 달 뒤 페이지가 변경됐습니다. 현재 배포 사양을 다시 확인해야 하며, 이 문서는 당시 발췌를 대신하지 않습니다.",
    );
    current.revision = "v2".into();
    current.paths = old.paths.clone();
    let mut question = message(
        "Q13",
        ActorKind::Human,
        RecordKind::Request,
        "A를 왜 제외했고 다시 검토하려면 어떤 조건이 달라져야 해? 당시 자료와 지금 자료를 구분해서 보여줘.",
    );
    question.evidence = vec![evidence(&current)];
    let mut entities = vec![Entity::Record(request.clone())];
    if has_old_excerpt {
        entities.push(Entity::Record(old.clone()));
    }
    entities.extend([
        Entity::Record(current),
        Entity::Record(decision),
        Entity::Record(question),
        Entity::Relation(link(
            "U13-D13",
            target("U13"),
            target("D13"),
            RelationKind::Supports,
        )),
        Entity::Relation(link(
            "R13-D13",
            artifact(&old),
            target("D13"),
            RelationKind::Supports,
        )),
    ]);
    dialogue.append(entities).await?;
    let corpus = dialogue.compact_and_reopen().await?;

    let found = trace(&corpus, target("D13"))?;
    assert!(ids(&found).is_superset(&BTreeSet::from(["U13".into(), "D13".into()])));
    let decision = record(&found, "D13")?;
    assert_eq!(decision.nature, Nature::Reported);
    assert!(decision.body.contains("로컬 실행 요구"));
    let citation = decision
        .evidence
        .iter()
        .find(|e| e.record_id.as_deref() == Some("R13"))
        .ok_or("missing historical research citation")?;
    assert_eq!(citation.revision, "v1");
    assert!(citation.locator.contains("checked-2026-08-01"));
    let mut old_query = query(Operation::Read);
    old_query.target = Some(artifact(&old));
    let historical = retrieval::execute(&corpus, &old_query)?;
    if has_old_excerpt {
        assert_eq!(record(&historical, "R13")?.body, old.body);
        assert_eq!(citation.availability, Availability::Available);
    } else {
        assert!(historical.items.is_empty());
        assert_eq!(citation.availability, Availability::Missing);
        assert!(!serde_json::to_string(&found)?.contains("당시 발췌: A는"));
    }
    let current = read(&corpus, "R13")?;
    assert_eq!(record(&current, "R13")?.revision, "v2");
    assert!(
        record(&current, "R13")?
            .body
            .contains("현재 배포 사양을 다시 확인")
    );
    assert!(
        corpus
            .entries
            .iter()
            .all(|entry| !matches!(entry.entity, Entity::Commit(_) | Entity::CodeState(_)))
    );
    Ok(())
}

#[tokio::test]
async fn sc13_a_document_only_reason_and_exact_historical_excerpt_survive() -> TestResult {
    research_journey(true).await
}

#[tokio::test]
async fn sc13_b_current_document_does_not_replace_unavailable_old_excerpt() -> TestResult {
    research_journey(false).await
}

async fn long_log_journey(revoke_after_first_page: bool) -> TestResult {
    // The user needs a local neighborhood, not every page preceding an error.
    let mut dialogue = Dialogue::new().await?;
    let mut lines: Vec<_> = (1..=500)
        .map(|n| format!("출력 {n}: 업로드 진행 중"))
        .collect();
    *lines.get_mut(248).ok_or("missing fixture line")? =
        "인증 요청 시작 password=synthetic-password-value".into();
    *lines.get_mut(249).ok_or("missing fixture line")? =
        "HTTP_401 인증 실패 Bearer synthetic-token-value".into();
    *lines.get_mut(250).ok_or("missing fixture line")? = "인증 실패로 업로드 중단".into();
    let output = message(
        "E14",
        ActorKind::Tool,
        RecordKind::ToolResult,
        &lines.join("\n"),
    );
    let mut summary = message(
        "S14",
        ActorKind::Agent,
        RecordKind::Finding,
        "인증 오류 주변 로그를 보존했습니다. Bearer synthetic-token-value",
    );
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.evidence = vec![evidence(&output)];
    let mut question = message(
        "Q14",
        ActorKind::Human,
        RecordKind::Request,
        "HTTP_401 오류 바로 앞뒤에서 무엇을 실행했어? 비밀값은 가리고 보여줘. 다음 페이지 전에 접근이 회수되면 계속 읽지 마.",
    );
    question.evidence = vec![evidence(&summary)];
    dialogue
        .append(vec![
            Entity::Record(output),
            Entity::Record(summary),
            Entity::Record(question),
        ])
        .await?;
    let corpus = dialogue.compact_and_reopen().await?;
    let mut search = query(Operation::Search);
    search.query = Some(TextQuery {
        text: "HTTP_401".into(),
        mode: SearchMode::Literal,
    });
    search.filters.record_kinds = vec![RecordKind::ToolResult];
    let found = retrieval::execute(&corpus, &search)?;
    let hit = found
        .items
        .iter()
        .find(|item| item.entity.id() == "E14")
        .ok_or("missing error hit")?;
    let range = hit
        .match_ranges
        .iter()
        .find(|range| range.field == "body")
        .ok_or("missing body locator")?
        .range
        .clone();
    assert_eq!(
        range,
        TextRange {
            start_line: 250,
            end_line: 250
        }
    );
    let mut nearby = query(Operation::Read);
    nearby.target = Some(Target::Artifact {
        record_id: "E14".into(),
        revision: "v1".into(),
        range: None,
    });
    nearby.range = Some(range);
    nearby.context_lines = Some(1);
    let neighborhood = retrieval::execute(&corpus, &nearby)?;
    let text = &record(&neighborhood, "E14")?.body;
    assert!(text.contains("인증 요청 시작") && text.contains("인증 실패로 업로드 중단"));
    assert_eq!(text.lines().count(), 3);
    assert_eq!(
        neighborhood
            .items
            .first()
            .and_then(|item| item.rendered_revision.as_ref()),
        hit.rendered_revision.as_ref()
    );
    for response in [&found, &neighborhood, &read(&corpus, "S14")?] {
        let rendered = serde_json::to_string(response)?;
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("synthetic-token-value"));
        assert!(!rendered.contains("synthetic-password-value"));
    }
    if revoke_after_first_page {
        let mut paged = nearby;
        paged.range = None;
        paged.context_lines = None;
        paged.budget_bytes = Some(8192);
        let first = retrieval::execute(&corpus, &paged)?;
        paged.cursor = Some(first.next_cursor.ok_or("long output should continue")?);
        let mut revoked = ingest::source("journal", PROJECT, SourceKind::Journal);
        revoked.authorized = false;
        dialogue.append(vec![Entity::Source(revoked)]).await?;
        let revoked = reopen(&mut dialogue).await?;
        assert!(matches!(
            retrieval::execute(&revoked, &paged),
            Err(QueryError::StaleCursor | QueryError::InvalidScope(_))
        ));
        assert!(matches!(
            read(&revoked, "E14"),
            Err(QueryError::InvalidScope(_))
        ));
        let serialized = serde_json::to_string(&revoked)?;
        assert!(!serialized.contains("인증 요청 시작"));
        assert!(!serialized.contains("인증 오류 주변 로그를 보존"));
    }
    Ok(())
}

#[tokio::test]
async fn sc14_a_masked_error_locator_reads_only_its_neighborhood() -> TestResult {
    long_log_journey(false).await
}

#[tokio::test]
async fn sc14_b_revocation_stops_saved_cursor_and_scrubs_derived_text() -> TestResult {
    long_log_journey(true).await
}

async fn summary_journey(has_correction: bool, has_original: bool) -> TestResult {
    let mut dialogue = Dialogue::new().await?;
    let mut original = message(
        "original-15",
        ActorKind::Agent,
        RecordKind::Finding,
        "당시 공개 설명 전문\n후보 A는 같은 입력의 처리 시간이 배포 조건을 초과해 제외했습니다.\n이 판단은 당시 실험 범위에 한정합니다.",
    );
    original.revision = "v0".into();
    let mut summary = message(
        "B15",
        ActorKind::Agent,
        RecordKind::Finding,
        "남은 요약: 후보 A는 성능 때문에 제외했습니다.",
    );
    summary.derived = true;
    summary.fidelity = Fidelity::SummaryOnly;
    let mut original_reference = evidence(&original);
    if !has_original {
        original_reference.availability = Availability::Missing;
    }
    summary.evidence = vec![original_reference];
    let mut repeated = message(
        "B16",
        ActorKind::Agent,
        RecordKind::Finding,
        "재개 요약: 앞선 요약에 따르면 후보 A는 성능 때문에 제외했습니다.",
    );
    repeated.derived = true;
    repeated.fidelity = Fidelity::SummaryOnly;
    repeated.evidence = vec![evidence(&summary)];
    let mut output = message(
        "E15",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "테스트 시작\n1번 검사 진행 중\n여기까지만 소스에 저장됐습니다.",
    );
    output.fidelity = Fidelity::SourceTruncated;
    let mut question = message(
        "Q15",
        ActorKind::Human,
        RecordKind::Request,
        "A를 안 쓴 이유와 당시 테스트 출력을 보여줘. 요약이 두 개면 별도 근거 두 개야?",
    );
    question.evidence = vec![evidence(&output), evidence(&repeated)];
    let mut entities = Vec::new();
    if has_original {
        entities.push(Entity::Record(original.clone()));
    }
    entities.extend([
        Entity::Record(summary),
        Entity::Record(repeated),
        Entity::Record(output),
        Entity::Record(question),
        Entity::Relation(link(
            "B16-B15",
            target("B16"),
            target("B15"),
            RelationKind::DerivedFrom,
        )),
    ]);
    if has_correction {
        let correction = message(
            "U15",
            ActorKind::Human,
            RecordKind::Feedback,
            "성능 때문이 아니야. 당시 라이선스 조건 때문에 A를 제외했어.",
        );
        let mut relation = link(
            "U15-B15",
            target("U15"),
            target("B15"),
            RelationKind::Supersedes,
        );
        relation.applies_to = vec!["제외 이유".into()];
        entities.extend([Entity::Record(correction), Entity::Relation(relation)]);
    }
    dialogue.append(entities).await?;
    let corpus = dialogue.compact_and_reopen().await?;
    let mut brief_query = query(Operation::Brief);
    brief_query.purpose = Some(BriefPurpose::Explain);
    let response = retrieval::execute(&corpus, &brief_query)?;
    let brief = response
        .brief
        .as_ref()
        .ok_or("missing explanation package")?;
    for id in ["B15", "B16"] {
        let claim = brief
            .sections
            .iter()
            .flat_map(|section| &section.claims)
            .find(|claim| claim.record_id == id)
            .ok_or("missing summary claim")?;
        assert_eq!(claim.fidelity, Fidelity::SummaryOnly);
        assert_eq!(claim.nature, Nature::Reported);
        assert!(
            claim
                .warnings
                .iter()
                .any(|warning| warning.contains("do not count as independent corroboration"))
        );
    }
    let lineage = trace(&corpus, target("B16"))?;
    assert!(ids(&lineage).contains("B15"));
    assert_eq!(
        record(&lineage, "B16")?
            .evidence
            .first()
            .and_then(|e| e.record_id.as_deref()),
        Some("B15")
    );
    let mut original_query = query(Operation::Read);
    original_query.target = Some(artifact(&original));
    let retained_original = retrieval::execute(&corpus, &original_query)?;
    let summary_response = read(&corpus, "B15")?;
    let cited = record(&summary_response, "B15")?
        .evidence
        .first()
        .ok_or("missing summary's original reference")?;
    assert_eq!(cited.revision, "v0");
    if has_original {
        let retained = record(&retained_original, "original-15")?;
        assert_eq!(retained.body, original.body);
        assert_eq!(retained.revision, "v0");
        assert_eq!(retained.fidelity, Fidelity::Original);
        assert_eq!(retained.availability, Availability::Available);
        assert_eq!(cited.availability, Availability::Available);
    } else {
        assert!(retained_original.items.is_empty());
        assert_eq!(cited.availability, Availability::Missing);
    }
    let output = read(&corpus, "E15")?;
    assert_eq!(record(&output, "E15")?.fidelity, Fidelity::SourceTruncated);
    assert!(output.items.iter().any(|item| {
        item.warnings
            .iter()
            .any(|warning| warning == "source_truncated")
    }));
    assert!(output.next_cursor.is_none());
    if has_correction {
        assert_eq!(
            record(&lineage, "U15")?.body,
            "성능 때문이 아니야. 당시 라이선스 조건 때문에 A를 제외했어."
        );
        assert!(
            lineage
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Supersedes && r.applies_to == ["제외 이유"])
        );
    } else {
        assert!(!ids(&lineage).contains("U15"));
        assert!(!serde_json::to_string(&response)?.contains("라이선스"));
    }
    Ok(())
}

#[tokio::test]
async fn sc15_a_user_correction_does_not_turn_summaries_into_originals() -> TestResult {
    summary_journey(true, false).await
}

#[tokio::test]
async fn sc15_b_missing_correction_distinguishes_absent_and_available_originals() -> TestResult {
    // Independent stores distinguish genuine source loss from an available full original.
    summary_journey(false, false).await?;
    summary_journey(false, true).await
}

fn commit(sha: &str, body: &str) -> Entity {
    Entity::Commit(Commit {
        id: format!("commit:{sha}"),
        project_id: PROJECT.into(),
        source_id: "git".into(),
        repository_id: "repository".into(),
        sha: sha.into(),
        tree: format!("tree:{sha}"),
        parents: Vec::new(),
        paths: vec!["src/timeout.rs".into()],
        message: body.into(),
        occurred_at: None,
        origin_worktree: None,
        observed_worktree: Some("new-clone".into()),
    })
}

fn commit_target(sha: &str) -> Target {
    Target::Commit {
        repository_id: "repository".into(),
        commit_sha: sha.into(),
    }
}

async fn cold_git_journey(has_new_context: bool) -> TestResult {
    // These are supplied Git observations, not a test of hooks or actual Git execution.
    let mut dialogue = Dialogue::new().await?;
    let mut question = message(
        "Q16",
        ActorKind::Human,
        RecordKind::Request,
        "오래된 저장소를 가져왔어. C16은 왜 timeout을 30초로 골랐고 C17에서는 왜 60초로 바뀌었어? clone에 예전 대화도 들어 있어?",
    );
    question.commit_shas = vec!["C16".into(), "C17".into()];
    let mut entities = vec![
        Entity::Source(ingest::source("git", PROJECT, SourceKind::Git)),
        commit("C16", "fix timeout: 30 seconds"),
        commit("C17", "allow queue delay: 60 seconds"),
        Entity::Record(question),
    ];
    if has_new_context {
        let output = message(
            "E16",
            ActorKind::Tool,
            RecordKind::ToolResult,
            "로컬 대기열 실험: 대기 42초 뒤 응답\n운영 피크=미검증",
        );
        let mut reason = message(
            "D16",
            ActorKind::Agent,
            RecordKind::Decision,
            "관측한 대기열 지연을 수용하려고 60초로 늘렸습니다. 운영 피크 검증은 남았습니다. 예전 30초의 선정 이유는 기록에 없습니다.",
        );
        reason.evidence = vec![evidence(&output)];
        reason.decision_status = Some(DecisionStatus::Accepted);
        entities.extend([
            Entity::Record(output),
            Entity::Record(reason),
            Entity::Relation(link(
                "D16-C17",
                target("D16"),
                commit_target("C17"),
                RelationKind::Supports,
            )),
        ]);
    }
    dialogue.append(entities).await?;
    let corpus = dialogue.compact_and_reopen().await?;
    let old = trace(&corpus, commit_target("C16"))?;
    assert!(ids(&old).contains("commit:C16"));
    assert!(
        old.omitted
            .iter()
            .any(|warning| warning.contains("reason_not_recorded"))
    );
    assert!(!ids(&old).contains("D16"));
    let new = trace(&corpus, commit_target("C17"))?;
    if has_new_context {
        assert!(
            record(&new, "D16")?
                .body
                .contains("운영 피크 검증은 남았습니다")
        );
        assert_eq!(record(&new, "D16")?.nature, Nature::Reported);
        assert_eq!(
            record(&read(&corpus, "E16")?, "E16")?.nature,
            Nature::Observed
        );
    } else {
        assert!(!ids(&new).contains("D16"));
        assert!(
            new.omitted
                .iter()
                .any(|warning| warning.contains("reason_not_recorded"))
        );
        assert!(!serde_json::to_string(&new)?.contains("관측한 대기열 지연"));
    }
    let mut sources_query = query(Operation::Sources);
    if !has_new_context {
        // Only the Git source from the clone is selected; no past journal was imported.
        sources_query.scope.source_ids = vec!["git".into()];
    }
    let sources = retrieval::execute(&corpus, &sources_query)?;
    assert!(
        sources
            .coverage
            .iter()
            .any(|source| source.source_id == "git" && source.result_only)
    );
    if has_new_context {
        assert!(
            sources
                .coverage
                .iter()
                .any(|source| source.source_id == "journal" && !source.result_only)
        );
    } else {
        assert_eq!(sources.coverage.len(), 1);
        assert!(sources.coverage.iter().all(|source| source.result_only));
    }
    Ok(())
}

#[tokio::test]
async fn sc16_a_new_context_explains_new_commit_without_inventing_old_reason() -> TestResult {
    cold_git_journey(true).await
}

#[tokio::test]
async fn sc16_b_git_only_clone_cannot_supply_absent_dialogue() -> TestResult {
    cold_git_journey(false).await
}

#[tokio::test]
async fn cc01_automatic_capacity_collection_and_restart_keep_the_cited_failure() -> TestResult {
    // User returns after many noisy uploads and an application restart.
    let mut dialogue = Dialogue::new().await?;
    let question = message(
        "CC1-Q",
        ActorKind::Human,
        RecordKind::Request,
        "로그가 계속 쌓여 저장 한도가 찼어. 앱을 다시 열어도 업로드 작업자를 줄인 이유와 실패 원문을 찾을 수 있어?",
    );
    let output = message(
        "CC1-E",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "동시 업로드 16개: 파일 40개 중 9개가 HTTP 429로 실패했습니다.",
    );
    let mut decision = message(
        "CC1-D",
        ActorKind::Agent,
        RecordKind::Decision,
        "동시 요청 초과를 줄이기 위해 업로드 작업자를 4개로 제한합니다.",
    );
    decision.evidence = vec![evidence(&output)];
    dialogue
        .append(vec![
            Entity::Record(question),
            Entity::Record(output.clone()),
            Entity::Record(decision),
        ])
        .await?;
    dialogue.compact_and_reopen().await?;
    let initial = dialogue.store.compact(None, false).await?;
    let cap = initial
        .before
        .entries
        .checked_add(3)
        .ok_or("entry cap overflow")?;
    dialogue.store.compact(Some(policy(cap)), true).await?;
    let mut automatic_collections = 0;
    for index in 0..24 {
        let body = format!(
            "업로드 진행 출력 {index}: {}",
            "아직 처리 중입니다. ".repeat(30)
        );
        let receipt = dialogue
            .store
            .append(Entity::Record(message(
                &format!("CC1-log-{index}"),
                ActorKind::Tool,
                RecordKind::ToolResult,
                &body,
            )))
            .await?;
        automatic_collections += usize::from(receipt.compaction.is_some());
        assert!(receipt.durable);
        let usage = dialogue.store.compact(None, false).await?;
        assert!(
            usage.before.entries <= cap
                && usage.before.payload_bytes <= usage.policy.max_payload_bytes
        );
    }
    assert!(automatic_collections > 1);
    let corpus = reopen(&mut dialogue).await?;
    assert_eq!(
        dialogue
            .store
            .compact(None, false)
            .await?
            .policy
            .max_entries,
        cap
    );
    assert_eq!(record(&read(&corpus, "CC1-E")?, "CC1-E")?.body, output.body);
    assert!(
        record(&read(&corpus, "CC1-D")?, "CC1-D")?
            .body
            .contains("4개")
    );
    let response = read(&corpus, "CC1-log-0")?;
    assert!(response.items.is_empty());
    assert!(
        response
            .omitted
            .iter()
            .any(|warning| warning.starts_with("history_compacted:"))
    );
    Ok(())
}

#[tokio::test]
async fn cc02_protected_capacity_rejection_never_claims_the_new_decision_was_saved() -> TestResult {
    let mut dialogue = Dialogue::new().await?;
    dialogue.append(vec![
        Entity::Record(message("CC2-Q", ActorKind::Human, RecordKind::Request, "중요한 결정만으로 저장 한도가 꽉 찼어. 새 결정을 기록하다 실패하면 기존 근거와 새 결정은 각각 어떻게 남아?")),
        Entity::Record(message("CC2-old", ActorKind::Agent, RecordKind::Decision, "기존 결정: 사용자 승인 없이는 토큰 갱신을 변경하지 않습니다.")),
    ]).await?;
    dialogue.compact_and_reopen().await?;
    let cap = dialogue.store.compact(None, false).await?.before.entries;
    dialogue.store.compact(Some(policy(cap)), true).await?;
    let before = serde_json::to_value(dialogue.store.load().await?)?;
    let incoming = message(
        "CC2-new",
        ActorKind::Agent,
        RecordKind::Decision,
        "새 결정: 운영 피크는 별도로 검증합니다.",
    );
    assert!(matches!(
        dialogue.store.append(Entity::Record(incoming)).await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(serde_json::to_value(dialogue.store.load().await?)?, before);
    let corpus = reopen(&mut dialogue).await?;
    assert_eq!(serde_json::to_value(&corpus)?, before);
    assert!(ids(&read(&corpus, "CC2-old")?).contains("CC2-old"));
    assert!(read(&corpus, "CC2-new")?.items.is_empty());
    assert_eq!(
        dialogue
            .store
            .compact(None, false)
            .await?
            .policy
            .max_entries,
        cap
    );
    Ok(())
}

#[tokio::test]
async fn cc03_mixed_duplicate_receipts_and_new_batch_evidence_are_atomic() -> TestResult {
    let mut dialogue = Dialogue::new().await?;
    dialogue.append(vec![Entity::Record(message("CC3-Q", ActorKind::Human, RecordKind::Request, "중복 로그와 새 실험 결과·결정을 한 번에 저장했어. 중복도 durable이라고 받은 원문은 재시작 뒤 남아 있고, 배치 실패 때 일부만 저장되지는 않아?"))]).await?;
    dialogue.compact_and_reopen().await?;
    let original = message(
        "CC3-old",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "이전 실험: 동시 16개에서 429가 9개 발생했습니다.",
    );
    let first = dialogue
        .store
        .append(Entity::Record(original.clone()))
        .await?;
    let cap = dialogue
        .store
        .compact(None, false)
        .await?
        .before
        .entries
        .checked_add(3)
        .ok_or("entry cap overflow")?;
    dialogue.store.compact(Some(policy(cap)), true).await?;
    let uncited = message(
        "CC3-uncited",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "전송 진행 출력: 요청을 보냈습니다. 선택 이유로 인용하지 않은 출력입니다.",
    );
    let uncited_first = dialogue
        .store
        .append(Entity::Record(uncited.clone()))
        .await?;
    for id in ["CC3-noise-a", "CC3-noise-b"] {
        dialogue
            .append(vec![Entity::Record(message(
                id,
                ActorKind::Tool,
                RecordKind::ToolResult,
                "저장해도 선택 이유에는 쓰이지 않는 진행 출력입니다.",
            ))])
            .await?;
    }
    let output = message(
        "CC3-new",
        ActorKind::Tool,
        RecordKind::ToolResult,
        "새 실험: 작업자 4개, 파일 40개 성공, 429 없음.",
    );
    let mut decision = message(
        "CC3-D",
        ActorKind::Agent,
        RecordKind::Decision,
        "두 실험을 근거로 업로드 작업자 4개를 선택합니다. 운영 피크 검증은 별개입니다.",
    );
    decision.evidence = vec![evidence(&original), evidence(&output)];
    let receipts = dialogue
        .store
        .append_all([
            Entity::Record(original),
            Entity::Record(uncited.clone()),
            Entity::Record(output),
            Entity::Record(decision),
        ])
        .await?;
    let duplicate = receipts.first().ok_or("missing duplicate receipt")?;
    assert!(duplicate.duplicate && duplicate.durable);
    assert_eq!(duplicate.sequence, first.sequence);
    let uncited_duplicate = receipts.get(1).ok_or("missing uncited duplicate receipt")?;
    assert!(uncited_duplicate.duplicate && uncited_duplicate.durable);
    assert_eq!(uncited_duplicate.sequence, uncited_first.sequence);
    assert!(receipts.iter().all(|receipt| receipt.durable));
    assert!(
        receipts
            .last()
            .is_some_and(|receipt| receipt.compaction.is_some())
    );
    let corpus = reopen(&mut dialogue).await?;
    for receipt in receipts {
        assert!(
            corpus
                .entries
                .iter()
                .any(|entry| entry.sequence == receipt.sequence)
        );
    }
    let response = read(&corpus, "CC3-D")?;
    assert_eq!(record(&response, "CC3-D")?.evidence.len(), 2);
    assert!(
        record(&response, "CC3-D")?
            .evidence
            .iter()
            .all(|evidence| { evidence.record_id.as_deref() != Some("CC3-uncited") })
    );
    assert_eq!(
        record(&read(&corpus, "CC3-uncited")?, "CC3-uncited")?.body,
        uncited.body
    );
    // Without admission's receipt pin this unreferenced output is collectible.
    assert!(
        dialogue
            .store
            .compact(None, false)
            .await?
            .removals
            .iter()
            .any(|removal| { removal.entity_id == "CC3-uncited" })
    );
    for id in ["CC3-old", "CC3-new"] {
        assert_eq!(record(&read(&corpus, id)?, id)?.revision, "v1");
    }
    let before = serde_json::to_value(&corpus)?;
    let rejected = ["CC3-rejected-a", "CC3-rejected-b"].map(|id| {
        Entity::Record(message(
            id,
            ActorKind::Agent,
            RecordKind::Decision,
            "보호 사실 상한을 넘는 신규 배치 결정입니다.",
        ))
    });
    assert!(matches!(
        dialogue.store.append_all(rejected).await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(serde_json::to_value(reopen(&mut dialogue).await?)?, before);
    Ok(())
}

#[tokio::test]
async fn cc04_exact_old_revision_and_current_correction_obey_deletion_and_revocation() -> TestResult
{
    for revoke_source in [false, true] {
        let mut dialogue = Dialogue::new().await?;
        let mut original = message(
            "CC4-E",
            ActorKind::Tool,
            RecordKind::ToolResult,
            "민감 원문 v1: 첫 측정은 31개 성공으로 기록됐습니다.",
        );
        original.source_id = "measurements".into();
        let mut corrected = original.clone();
        corrected.revision = "v2".into();
        corrected.body = "민감 원문 v2: 집계 오류를 정정해 성공은 30개입니다.".into();
        let mut decision = message(
            "CC4-D",
            ActorKind::Agent,
            RecordKind::Decision,
            "민감 파생 판단: 처음 측정을 근거로 동시 업로드를 줄였습니다.",
        );
        decision.derived = true;
        decision.evidence = vec![evidence(&original)];
        let mut summary = message(
            "CC4-summary",
            ActorKind::Agent,
            RecordKind::Finding,
            "민감 재요약: 이전 판단을 이어받습니다.",
        );
        summary.derived = true;
        summary.fidelity = Fidelity::SummaryOnly;
        summary.evidence = vec![evidence(&decision)];
        let mut question = message(
            "CC4-Q",
            ActorKind::Human,
            RecordKind::Request,
            "원문이 정정됐어. 당시 판단에 쓴 버전과 현재 버전을 따로 보여줘. 이후 원문 삭제나 접근 회수가 확인되면 재요약까지 숨겨줘.",
        );
        question.evidence = vec![evidence(&summary)];
        dialogue
            .append(vec![
                Entity::Source(ingest::source("measurements", PROJECT, SourceKind::Journal)),
                Entity::Record(original.clone()),
                Entity::Record(corrected.clone()),
                Entity::Record(decision),
                Entity::Record(summary),
                Entity::Record(question),
            ])
            .await?;
        let corpus = dialogue.compact_and_reopen().await?;
        let mut old_query = query(Operation::Read);
        old_query.target = Some(artifact(&original));
        assert_eq!(
            record(&retrieval::execute(&corpus, &old_query)?, "CC4-E")?.body,
            original.body
        );
        assert_eq!(
            record(&read(&corpus, "CC4-E")?, "CC4-E")?.body,
            corrected.body
        );
        assert_eq!(
            record(&read(&corpus, "CC4-D")?, "CC4-D")?
                .evidence
                .first()
                .map(|e| e.revision.as_str()),
            Some("v1")
        );
        if revoke_source {
            let mut source = ingest::source("measurements", PROJECT, SourceKind::Journal);
            source.authorized = false;
            dialogue.append(vec![Entity::Source(source)]).await?;
        } else {
            let mut deleted = corrected;
            deleted.availability = Availability::Deleted;
            dialogue.append(vec![Entity::Record(deleted)]).await?;
        }
        dialogue.store.compact(None, true).await?;
        let removed = reopen(&mut dialogue).await?;
        assert!(!serde_json::to_string(&removed)?.contains("민감"));
        for id in ["CC4-E", "CC4-D", "CC4-summary"] {
            let response = read(&removed, id)?;
            assert!(!serde_json::to_string(&response)?.contains("민감"));
        }
        assert!(
            !serde_json::to_string(&retrieval::execute(&removed, &old_query)?)?.contains("민감")
        );
        assert!(
            dialogue
                .store
                .append(Entity::Record(original))
                .await
                .is_err()
        );
    }
    Ok(())
}

#[tokio::test]
async fn cc05_mid_query_compaction_requires_new_cursor_and_checkpoint() -> TestResult {
    let mut dialogue = Dialogue::new().await?;
    dialogue.append(vec![
        Entity::Record(message("CC5-Q", ActorKind::Human, RecordKind::Request, "이전 결정 목록을 읽다가 기록 압축이 실행됐어. 예전 다음 페이지와 지난 조회 이후 변경 확인을 그대로 이어가도 돼?")),
        Entity::Record(message("CC5-D", ActorKind::Agent, RecordKind::Decision, "기존 결정: 공통 재시도는 3회를 유지합니다.")),
    ]).await?;
    let before = dialogue.store.load().await?;
    let mut paged = query(Operation::Search);
    paged.limit = Some(1);
    paged.filters.record_kinds = vec![
        RecordKind::Request,
        RecordKind::Decision,
        RecordKind::Status,
    ];
    let first = retrieval::execute(&before, &paged)?;
    let cursor = first
        .next_cursor
        .ok_or("fixture must have multiple pages")?;
    assert!(first.checkpoint.is_none());
    let mut complete = paged.clone();
    complete.limit = Some(100);
    let checkpoint = retrieval::execute(&before, &complete)?
        .checkpoint
        .ok_or("missing complete checkpoint")?;
    let corpus = dialogue.compact_and_reopen().await?;
    paged.cursor = Some(cursor);
    assert_eq!(
        retrieval::execute(&corpus, &paged).err(),
        Some(QueryError::StaleCursor)
    );
    let mut changes = query(Operation::Compare);
    changes.since_checkpoint = Some(checkpoint);
    assert_eq!(
        retrieval::execute(&corpus, &changes).err(),
        Some(QueryError::RescanRequired)
    );
    paged.cursor = None;
    let mut seen = BTreeSet::new();
    let new_checkpoint = loop {
        let page = retrieval::execute(&corpus, &paged)?;
        for id in ids(&page) {
            assert!(seen.insert(id));
        }
        if let Some(cursor) = page.next_cursor {
            assert!(page.checkpoint.is_none());
            paged.cursor = Some(cursor);
        } else {
            break page.checkpoint.ok_or("missing final-page checkpoint")?;
        }
    };
    assert_eq!(seen, BTreeSet::from(["CC5-Q".into(), "CC5-D".into()]));
    let mut late = message(
        "CC5-late",
        ActorKind::Human,
        RecordKind::Feedback,
        "이제 확보한 어제 정정: 운영 피크까지 통과한 것은 아니야.",
    );
    late.occurred_at = Some("2026-08-01T00:00:00Z".into());
    dialogue.append(vec![Entity::Record(late)]).await?;
    let current = reopen(&mut dialogue).await?;
    changes.since_checkpoint = Some(new_checkpoint);
    let added = retrieval::execute(&current, &changes)?;
    assert!(ids(&added).contains("CC5-late"));
    assert!(!ids(&added).contains("__noise__"));
    Ok(())
}
