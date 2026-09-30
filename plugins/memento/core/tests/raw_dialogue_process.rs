//! SC-07–SC-12: synthetic user/agent/tool utterances, durable storage, real
//! Crepe compaction, reopen, then bounded retrieval. Collection annotations are
//! explicit fixture inputs; these tests do not claim natural-language inference.

use std::error::Error;

use work_context::mapping::MappingRequest;
use work_context::model::*;
use work_context::query::{
    self, BriefClaim, LocationStatus, QueryError, QueryResponse, ResponseStatus,
};
use work_context::{ingest, security};

#[path = "support/raw_dialogue.rs"]
mod raw_dialogue;
use raw_dialogue::{
    Dialogue, PROJECT, TestResult, evidence, ids, link, message, query, read, record,
};

fn target(id: &str) -> Target {
    Target::Record { id: id.into() }
}

fn code_target(state: &str, path: &str) -> Target {
    Target::Code {
        state_id: state.into(),
        path: path.into(),
        range: None,
    }
}

fn state(id: &str, worktree: &str, files: &[(&str, &str)], changed: bool) -> CodeState {
    CodeState {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        repository_id: PROJECT.into(),
        worktree_id: Some(worktree.into()),
        commit_sha: None,
        observed_at: "2026-09-28T11:00:00Z".into(),
        changed_during_observation: changed,
        files: files
            .iter()
            .map(|(path, content)| FileState {
                path: (*path).into(),
                working_hash: Some(security::hash(content.as_bytes())),
                working_content: Some((*content).into()),
                working_kind: WorkingFileKind::File,
                ..FileState::default()
            })
            .collect(),
    }
}

fn execution(id: &str, passed: bool) -> Execution {
    Execution {
        id: id.into(),
        command: "cargo test --lib retry".into(),
        tool_name: Some("shell".into()),
        tool_input: None,
        cwd: Some("/workspace/WA".into()),
        started_at: Some("2026-09-28T10:00:00Z".into()),
        ended_at: Some("2026-09-28T10:01:00Z".into()),
        exit_code: Some(if passed { 0 } else { 101 }),
        last_observed_state: if passed { "succeeded" } else { "failed" }.into(),
        observed_at: Some("2026-09-28T10:01:00Z".into()),
        liveness: Liveness::Stopped,
        before_state: None,
        after_state: None,
        scope: vec!["unit:retry".into(), "default-features".into()],
        environment: Environment {
            os: Some("macos".into()),
            toolchain: Some("rust-1.93".into()),
            profile: Some("local-unit".into()),
            dependencies: Vec::new(),
        },
    }
}

fn claim<'a>(response: &'a QueryResponse, id: &str) -> Result<&'a BriefClaim, Box<dyn Error>> {
    response
        .brief
        .as_ref()
        .ok_or("missing brief")?
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .find(|claim| claim.record_id == id)
        .ok_or_else(|| format!("missing claim {id}").into())
}

#[tokio::test(flavor = "current_thread")]
async fn sc07_parallel_edits_preserve_authorship_and_test_observation_limits() -> TestResult {
    const QUESTION: &str = "누가 바꿨고, 현재 작업에도 반영돼서 테스트됐어?";
    for has_patch_observations in [true, false] {
        let mut dialogue = Dialogue::new().await?;
        let request = message(
            "P01",
            ActorKind::Human,
            RecordKind::Request,
            "A는 WA에서 재시도를 고쳐줘. B는 WB에서 로그 형식을 고쳐줘.",
        );
        let mut child = message(
            "P02",
            ActorKind::Agent,
            RecordKind::Status,
            "WB의 로그 형식 변경은 끝났습니다. 아직 WA로 옮기지는 않았습니다.",
        );
        child.actor.as_mut().ok_or("missing actor")?.name = Some("B".into());
        child.work_ids = vec!["W2".into()];
        child.worktree_id = Some("WB".into());
        let child_work = Work {
            id: "W2".into(),
            project_id: PROJECT.into(),
            source_id: "journal".into(),
            title: "로그 형식 수정".into(),
            goal: "WB의 로그 형식 변경".into(),
            status: WorkStatus::Completed,
            observed_at: None,
            evidence: vec![evidence(&child)],
            completion_conditions: vec!["WB의 로그 형식 변경".into()],
        };
        let mut current_diff = message(
            "P03",
            ActorKind::Unknown,
            RecordKind::Change,
            "diff -- config.toml\n-workers = 16\n+workers = 4",
        );
        current_diff.worktree_id = Some("WA".into());
        current_diff.paths = vec!["config.toml".into()];
        current_diff.nature = Nature::Observed;
        current_diff.association = Association::Candidate;
        let mut output = message(
            "P04",
            ActorKind::Tool,
            RecordKind::Verification,
            "test retry::backoff ... ok\ntest result: ok. 7 passed; 0 failed\nexit_code=0",
        );
        output.worktree_id = Some("WA".into());
        output.verification_outcome = Some(VerificationOutcome::Passed);
        output.execution = Some(execution("T6", true));
        output.evidence = vec![evidence(&current_diff)];
        let mut entities = vec![
            Entity::Record(request),
            Entity::Record(child.clone()),
            Entity::Record(current_diff.clone()),
            Entity::Work(child_work),
        ];
        if has_patch_observations {
            let mut edit = message(
                "P05",
                ActorKind::Agent,
                RecordKind::Change,
                "apply_patch src/retry.rs\n- retry_now();\n+ retry_after_error();",
            );
            edit.actor.as_mut().ok_or("missing actor")?.name = Some("A".into());
            edit.nature = Nature::Observed;
            edit.worktree_id = Some("WA".into());
            edit.paths = vec!["src/retry.rs".into()];
            output.evidence.push(evidence(&edit));
            let observed = output.execution.as_mut().ok_or("missing execution")?;
            observed.before_state = Some("wa-before".into());
            observed.after_state = Some("wa-changing".into());
            entities.extend([
                Entity::Record(edit),
                Entity::CodeState(state(
                    "wa-before",
                    "WA",
                    &[("src/retry.rs", "fn retry() { retry_now(); }")],
                    false,
                )),
                Entity::CodeState(state(
                    "wa-changing",
                    "WA",
                    &[("src/retry.rs", "fn retry() { retry_after_error(); }")],
                    true,
                )),
            ]);
        }
        entities.push(Entity::Record(output));
        dialogue.append(entities).await?;
        let corpus = dialogue.compact_and_reopen().await?;
        let mut request = query(Operation::Brief);
        request.scope.worktree_ids = vec!["WA".into()];
        let response = query::execute(&corpus, &request)?;
        assert!(
            !ids(&response).contains("P02"),
            "{QUESTION}: WB completion must not become WA completion"
        );
        let diff = record(&response, "P03")?;
        assert_eq!(
            diff.actor.as_ref().ok_or("missing actor")?.kind,
            ActorKind::Unknown
        );
        assert!(diff.actor.as_ref().ok_or("missing actor")?.name.is_none());
        let passed = record(&response, "P04")?
            .execution
            .as_ref()
            .ok_or("missing test execution")?;
        assert_eq!(passed.exit_code, Some(0));
        if has_patch_observations {
            assert_eq!(
                record(&response, "P05")?
                    .actor
                    .as_ref()
                    .and_then(|actor| actor.name.as_deref()),
                Some("A")
            );
            let mut compare = query(Operation::Compare);
            compare.from = Some(HistoryPoint {
                code_state_id: passed.before_state.clone(),
                ..HistoryPoint::default()
            });
            compare.to = Some(HistoryPoint {
                code_state_id: passed.after_state.clone(),
                ..HistoryPoint::default()
            });
            let compared = query::execute(&corpus, &compare)?;
            assert_eq!(
                compared
                    .comparison
                    .as_ref()
                    .and_then(|c| c.code.as_ref())
                    .ok_or("missing code comparison")?
                    .status,
                "changed_during_observation"
            );
        } else {
            assert!(
                passed.before_state.is_none() && passed.after_state.is_none(),
                "{QUESTION}: output alone does not identify tested code"
            );
            assert!(read(&corpus, "P05")?.items.is_empty());
        }
        let child_result = read(&corpus, "P02")?;
        assert_eq!(record(&child_result, "P02")?.body, child.body);
        assert!(corpus.entries.iter().any(|entry| matches!(&entry.entity,
            Entity::Work(work) if work.id == "W1" && work.status == WorkStatus::Active)));
        assert!(
            response
                .relations
                .iter()
                .all(|r| r.kind != RelationKind::IntegratedInto)
        );
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc08_partial_requirement_change_and_unapproved_proposal_survive_compaction() -> TestResult
{
    const QUESTION: &str = "지금 구현해야 할 페이지 조건이 뭐야?";
    for ambiguous in [false, true] {
        let mut dialogue = Dialogue::new().await?;
        let mut first = message(
            "R01",
            ActorKind::Human,
            RecordKind::Constraint,
            "관리 화면과 일반 화면 모두 페이지당 50개로 해줘. 기존 응답 필드는 유지하고 새 의존성은 넣지 마.",
        );
        first.applies_to = vec![
            "admin-page-size".into(),
            "public-page-size".into(),
            "response-fields".into(),
            "dependencies".into(),
        ];
        let mut update = message(
            "R02",
            ActorKind::Human,
            RecordKind::Feedback,
            if ambiguous {
                "이번만 200개로 늘려줘."
            } else {
                "관리 화면만 200개로 늘려줘."
            },
        );
        update.applies_to = vec![
            if ambiguous {
                "unknown-page-scope"
            } else {
                "admin-page-size"
            }
            .into(),
        ];
        let mut proposal = message(
            "R03",
            ActorKind::Agent,
            RecordKind::Decision,
            "페이지 계산을 새 pagination 라이브러리에 맡겨도 될까요?",
        );
        proposal.decision_status = Some(DecisionStatus::Proposed);
        let mut change = link(
            "partial-page-change",
            target("R02"),
            target("R01"),
            if ambiguous {
                RelationKind::Contradicts
            } else {
                RelationKind::Supersedes
            },
        );
        change.applies_to = update.applies_to.clone();
        change.nature = if ambiguous {
            Nature::Inferred
        } else {
            Nature::Reported
        };
        change.evidence = vec![evidence(&update)];
        dialogue
            .append(vec![
                Entity::Record(first.clone()),
                Entity::Record(update.clone()),
                Entity::Record(proposal.clone()),
                Entity::Relation(change),
            ])
            .await?;
        let corpus = dialogue.compact_and_reopen().await?;
        let response = query::execute(&corpus, &query(Operation::Brief))?;
        assert_eq!(record(&read(&corpus, "R01")?, "R01")?.body, first.body);
        let base = claim(&response, "R01")?;
        for scope in ["public-page-size", "response-fields", "dependencies"] {
            assert!(
                base.applies_to.iter().any(|value| value == scope),
                "{QUESTION}: unrelated clause {scope} survives"
            );
        }
        assert_eq!(
            base.applies_to
                .iter()
                .any(|value| value == "admin-page-size"),
            ambiguous
        );
        assert_eq!(
            claim(&response, "R03")?.decision_status,
            Some(DecisionStatus::Proposed)
        );
        assert_eq!(claim(&response, "R03")?.text, proposal.body);
        let brief = response.brief.as_ref().ok_or("missing brief")?;
        assert_eq!(brief.conflicts.len(), usize::from(ambiguous));
        if ambiguous {
            let conflict = brief.conflicts.first().ok_or("missing scoped conflict")?;
            assert_eq!(conflict.applies_to, vec!["unknown-page-scope"]);
            assert_eq!(conflict.nature, Nature::Inferred);
        } else {
            assert_eq!(claim(&response, "R02")?.applies_to, vec!["admin-page-size"]);
            assert!(
                base.warnings
                    .iter()
                    .any(|warning| warning.contains("superseded only"))
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc09_same_command_retry_keeps_distinct_executions_and_environment_uncertainty()
-> TestResult {
    const QUESTION: &str = "같은 방식이 전에 실패했는데 지금은 CI까지 검증된 거야?";
    for has_configuration_change in [true, false] {
        let mut dialogue = Dialogue::new().await?;
        let request = message(
            "X01",
            ActorKind::Human,
            RecordKind::Request,
            "재시도 패치를 적용해서 retry 단위 테스트부터 돌려줘.",
        );
        let mut first = message(
            "X02",
            ActorKind::Agent,
            RecordKind::Attempt,
            "P10을 적용했습니다. cargo test --lib retry를 실행하겠습니다.",
        );
        first.attempt_id = Some(first.id.clone());
        first.attempt_outcome = Some(AttemptOutcome::Failed);
        first.execution = Some(execution("execution-first", false));
        if let Some(execution) = &mut first.execution {
            execution.before_state = Some("p10-applied".into());
            execution.after_state = Some("p10-applied".into());
        }
        let mut failure = message(
            "X03",
            ActorKind::Tool,
            RecordKind::ToolResult,
            "phase=database-connect\nconnection refused\ntest body was not entered\nexit_code=101",
        );
        failure.attempt_id = first.attempt_id.clone();
        let mut restore = message(
            "X04",
            ActorKind::Tool,
            RecordKind::Change,
            "$ git restore src/retry.rs\nexit_code=0",
        );
        restore.code_refs = vec![CodeRef {
            state_id: "p10-restored".into(),
            path: "src/retry.rs".into(),
            range: None,
        }];
        let mut second = message(
            "X05",
            ActorKind::Agent,
            RecordKind::Attempt,
            "동일한 P10을 다시 적용했습니다. cargo test --lib retry를 다시 실행하겠습니다.",
        );
        second.attempt_id = Some(second.id.clone());
        second.attempt_outcome = Some(AttemptOutcome::Succeeded);
        second.execution = Some(execution("execution-second", true));
        if let Some(execution) = &mut second.execution {
            execution.before_state = Some("p10-applied".into());
            execution.after_state = Some("p10-applied".into());
        }
        let mut verified = message(
            "X06",
            ActorKind::Tool,
            RecordKind::Verification,
            "test retry::backoff ... ok\n7 passed; 0 failed\nexit_code=0",
        );
        verified.verification_outcome = Some(VerificationOutcome::Passed);
        verified.execution = second.execution.clone();
        verified.evidence = vec![evidence(&second), evidence(&restore)];
        let mut ci = message(
            "X07",
            ActorKind::Human,
            RecordKind::Request,
            "내일 CI는 Linux, Rust 1.94에서 --all-features와 DB 통합 테스트까지 확인해줘.",
        );
        ci.evidence = vec![evidence(&verified)];
        let mut entities = vec![
            Entity::Record(request),
            Entity::Record(first.clone()),
            Entity::Record(failure.clone()),
            Entity::Record(restore),
            Entity::Record(second.clone()),
            Entity::CodeState(state(
                "p10-applied",
                "WA",
                &[("src/retry.rs", "fn retry() { retry_after_error(); }")],
                false,
            )),
            Entity::CodeState(state(
                "p10-restored",
                "WA",
                &[("src/retry.rs", "fn retry() { retry_now(); }")],
                false,
            )),
        ];
        if has_configuration_change {
            let config = message(
                "X08",
                ActorKind::Tool,
                RecordKind::Change,
                "diff -- test-config.toml\n-db_host = \"127.0.0.2\"\n+db_host = \"127.0.0.1\"",
            );
            verified.evidence.push(evidence(&config));
            entities.push(Entity::Record(config));
        }
        entities.extend([Entity::Record(verified), Entity::Record(ci)]);
        dialogue.append(entities).await?;
        // Re-delivery of each execution is idempotent; identical command text
        // must not coalesce the separate real executions.
        let replay = dialogue
            .store
            .append_all(vec![Entity::Record(first), Entity::Record(second)])
            .await?;
        assert!(
            replay
                .iter()
                .all(|receipt| receipt.duplicate && receipt.durable)
        );
        let corpus = dialogue.compact_and_reopen().await?;
        let response = query::execute(&corpus, &query(Operation::Timeline))?;
        let failed = record(&response, "X02")?
            .execution
            .as_ref()
            .ok_or("missing first execution")?;
        let passed = record(&response, "X05")?
            .execution
            .as_ref()
            .ok_or("missing second execution")?;
        assert_ne!(failed.id, passed.id);
        assert_eq!(failed.command, passed.command);
        assert_eq!(failed.before_state.as_deref(), Some("p10-applied"));
        assert_eq!(failed.before_state, passed.before_state);
        assert_eq!(failed.exit_code, Some(101));
        assert_eq!(passed.exit_code, Some(0));
        assert_eq!(record(&response, "X03")?.body, failure.body);
        assert_eq!(record(&response, "X03")?.nature, Nature::Observed);
        assert_eq!(passed.scope, vec!["unit:retry", "default-features"]);
        assert_eq!(passed.environment.os.as_deref(), Some("macos"));
        assert_eq!(passed.environment.toolchain.as_deref(), Some("rust-1.93"));
        assert!(record(&response, "X07")?.body.contains("--all-features"));
        assert!(
            response
                .items
                .iter()
                .filter_map(|item| match &item.entity {
                    Entity::Record(record)
                        if record.verification_outcome == Some(VerificationOutcome::Passed) =>
                        record.execution.as_ref(),
                    _ => None,
                })
                .all(|execution| !execution.scope.iter().any(|scope| scope == "all-features")),
            "{QUESTION}: future CI request cannot certify local result"
        );
        assert_eq!(
            !read(&corpus, "X08")?.items.is_empty(),
            has_configuration_change
        );
        if !has_configuration_change {
            assert!(
                failed.environment.dependencies.is_empty()
                    && passed.environment.dependencies.is_empty(),
                "{QUESTION}: success does not fabricate configuration evidence"
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc10_wording_rewrite_recovers_output_report_or_bounded_unknown() -> TestResult {
    const QUESTION: &str = "전에 동시 요청 때문에 막혔던 방식이 뭐였지?";
    for variant in ["A", "B", "B2"] {
        let mut dialogue = Dialogue::new().await?;
        let mut other = message(
            "S401",
            ActorKind::Tool,
            RecordKind::Verification,
            "HTTP 401 authentication required\nexit_code=1",
        );
        other.work_ids = vec!["W401".into()];
        other.verification_outcome = Some(VerificationOutcome::Failed);
        let mut entities = vec![Entity::Record(other)];
        if variant != "B2" {
            let mut attempt = message(
                "S01",
                ActorKind::Agent,
                RecordKind::Attempt,
                "parallel upload 실험: 업로드 작업자 16개로 파일 40개를 전송하겠습니다.",
            );
            attempt.attempt_outcome = Some(AttemptOutcome::Failed);
            let mut output = message(
                "S02",
                ActorKind::Tool,
                RecordKind::ToolResult,
                "HTTP 429 concurrent upload limit exceeded\nfailed=9\nexit_code=1",
            );
            if variant == "B" {
                output.body.clear();
                output.availability = Availability::Missing;
            }
            let mut decision = message(
                "S03",
                ActorKind::Agent,
                RecordKind::Decision,
                "요청 제한 오류가 났습니다. 동시 실행 수를 4개로 낮추겠습니다.",
            );
            decision.decision_status = Some(DecisionStatus::Accepted);
            entities.extend([
                Entity::Relation(link(
                    "upload-output",
                    target("S02"),
                    target("S01"),
                    RelationKind::RespondsTo,
                )),
                Entity::Relation(link(
                    "upload-decision",
                    target("S03"),
                    target("S01"),
                    RelationKind::RespondsTo,
                )),
                Entity::Record(attempt),
                Entity::Record(output),
                Entity::Record(decision),
            ]);
        }
        dialogue.append(entities).await?;
        let corpus = dialogue.compact_and_reopen().await?;
        let mut search = query(Operation::Search);
        search.scope.work_ids = vec!["W1".into()];
        search.query = Some(TextQuery {
            text: "동시 요청 때문에 막혔던 방식".into(),
            mode: SearchMode::Tokens,
        });
        let initial = query::execute(&corpus, &search)?;
        assert!(initial.items.is_empty());
        assert!(matches!(
            initial.status,
            ResponseStatus::NoMatches | ResponseStatus::Partial
        ));
        assert!(
            initial
                .omitted
                .iter()
                .any(|warning| warning.contains("history_compacted"))
        );
        search.query = Some(TextQuery {
            text: "HTTP 429".into(),
            mode: SearchMode::Literal,
        });
        search.filters.record_kinds = vec![RecordKind::Attempt];
        assert!(query::execute(&corpus, &search)?.items.is_empty());
        search.filters = Filters::default();
        let output_match = query::execute(&corpus, &search)?;
        if variant == "A" {
            assert_eq!(
                ids(&output_match),
                ["S02".to_string()].into_iter().collect()
            );
            assert!(
                output_match
                    .items
                    .first()
                    .ok_or("missing match")?
                    .match_locations
                    .contains(&"body".into())
            );
            assert!(
                record(&read(&corpus, "S02")?, "S02")?
                    .body
                    .contains("HTTP 429")
            );
        } else {
            assert!(output_match.items.is_empty());
            assert!(matches!(
                output_match.status,
                ResponseStatus::NoMatches | ResponseStatus::Partial
            ));
            assert!(
                output_match
                    .omitted
                    .iter()
                    .any(|warning| warning.contains("history_compacted")),
                "{QUESTION}: this output search must disclose its own retention limit"
            );
        }
        search.query = Some(TextQuery {
            text: "parallel upload".into(),
            mode: SearchMode::Literal,
        });
        let attempt_match = query::execute(&corpus, &search)?;
        if variant == "B2" {
            assert!(attempt_match.items.is_empty());
            assert!(matches!(
                attempt_match.status,
                ResponseStatus::NoMatches | ResponseStatus::Partial
            ));
            assert_eq!(attempt_match.scope.work_ids, vec!["W1"]);
            assert!(!attempt_match.coverage.is_empty());
            assert!(
                attempt_match
                    .omitted
                    .iter()
                    .any(|warning| warning.contains("history_compacted")),
                "{QUESTION}: bounded absence must disclose compaction"
            );
            continue;
        }
        assert!(ids(&attempt_match).contains("S01"));
        let mut trace = query(Operation::Trace);
        trace.scope.work_ids = vec!["W1".into()];
        trace.target = Some(target("S01"));
        trace.direction = Some(Direction::Both);
        trace.max_depth = Some(3);
        let traced = query::execute(&corpus, &trace)?;
        assert!(ids(&traced).contains("S03"));
        assert!(
            !ids(&traced).contains("S401"),
            "{QUESTION}: authentication incident is a different work"
        );
        assert_eq!(record(&traced, "S03")?.nature, Nature::Reported);
        assert!(record(&traced, "S03")?.body.contains("4개"));
        assert!(record(&traced, "S03")?.verification_outcome.is_none());
        if variant == "B" {
            let missing = record(&read(&corpus, "S02")?, "S02")?.clone();
            assert!(missing.body.is_empty());
            assert_eq!(missing.availability, Availability::Missing);
            assert!(
                !record(&traced, "S03")?.body.contains("429"),
                "{QUESTION}: surviving report does not establish exact HTTP code"
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc11_late_dialogue_revisions_pages_and_compaction_require_correct_checkpoint() -> TestResult
{
    const QUESTION: &str = "월요일에 본 이후 달라진 게 있어?";
    for checkpoint_available in [true, false] {
        let mut dialogue = Dialogue::new().await?;
        let mut original = message(
            "L01",
            ActorKind::Agent,
            RecordKind::Decision,
            "현재 확보한 출력에서는 실패를 찾지 못했습니다. 재시도 상한은 5회로 진행하겠습니다.",
        );
        original.occurred_at = Some("2026-09-28T09:00:00Z".into());
        original.decision_status = Some(DecisionStatus::Accepted);
        dialogue
            .append(vec![Entity::Record(original.clone())])
            .await?;
        let initial = dialogue.store.load().await?;
        let before = query::execute(&initial, &query(Operation::Brief))?;
        let checkpoint = before.checkpoint.ok_or("missing initial checkpoint")?;
        let mut failure = message(
            "L02",
            ActorKind::Tool,
            RecordKind::Verification,
            "run=X13\nHTTP 429\n9 failed\nexit_code=1",
        );
        failure.occurred_at = Some("2026-09-27T08:00:00Z".into());
        failure.verification_outcome = Some(VerificationOutcome::Failed);
        let mut correction = message(
            "L03",
            ActorKind::Human,
            RecordKind::Feedback,
            "이 실패가 일요일 로그에 있어. 재시도 상한은 3회였고 5회가 아니야.",
        );
        correction.occurred_at = Some("2026-09-27T08:10:00Z".into());
        correction.evidence = vec![evidence(&failure)];
        let mut revised = original;
        revised.revision = "v2".into();
        revised.body =
            "늦게 받은 일요일 로그에 실패가 있습니다. 재시도 상한을 3회로 정정합니다.".into();
        revised.evidence = vec![evidence(&correction)];
        dialogue
            .append(vec![
                Entity::Record(failure.clone()),
                Entity::Record(correction),
                Entity::Record(revised),
            ])
            .await?;
        let updated = dialogue.store.load().await?;
        let mut delta = query(Operation::Compare);
        delta.since_checkpoint = checkpoint_available.then(|| checkpoint.clone());
        delta.limit = Some(1);
        if checkpoint_available {
            let first = query::execute(&updated, &delta)?;
            assert!(first.next_cursor.is_some());
            assert!(
                first.checkpoint.is_none(),
                "{QUESTION}: an incomplete page cannot advance checkpoint"
            );
            // Abandon and restart from the old checkpoint: at-least-once delivery
            // is allowed, omission of the late Sunday record is not.
            let replay = query::execute(&updated, &delta)?;
            assert_eq!(ids(&first), ids(&replay));
            let mut delivered = std::collections::BTreeSet::new();
            let mut complete = false;
            for _ in 0..10 {
                let page = query::execute(&updated, &delta)?;
                delivered.extend(ids(&page));
                assert!(page.items.iter().all(|item| {
                    item.warnings
                        .iter()
                        .any(|warning| warning.contains("newly captured or revised"))
                }));
                delta.cursor = page.next_cursor;
                if delta.cursor.is_none() {
                    assert!(page.checkpoint.is_some());
                    complete = true;
                    break;
                }
            }
            assert!(
                complete
                    && ["L01", "L02", "L03"]
                        .iter()
                        .all(|id| delivered.contains(*id))
            );
        } else {
            assert!(matches!(
                query::execute(&updated, &delta),
                Err(QueryError::RescanRequired)
            ));
        }
        let corpus = dialogue.compact_and_reopen().await?;
        let mut obsolete = query(Operation::Compare);
        obsolete.since_checkpoint = Some(checkpoint);
        assert!(
            matches!(
                query::execute(&corpus, &obsolete),
                Err(QueryError::RescanRequired)
            ),
            "{QUESTION}: compaction invalidates the old checkpoint"
        );
        let rescanned = query::execute(&corpus, &query(Operation::Brief))?;
        assert!(rescanned.checkpoint.is_some());
        assert_eq!(record(&rescanned, "L01")?.revision, "v2");
        assert!(!record(&rescanned, "L01")?.body.contains("5회로 진행"));
        assert_eq!(record(&rescanned, "L02")?.occurred_at, failure.occurred_at);
        let captured = corpus
            .entries
            .iter()
            .find(|entry| entry.entity.id() == "L02")
            .ok_or("late failure absent after reopen")?;
        assert_ne!(
            Some(captured.captured_at.as_str()),
            failure.occurred_at.as_deref()
        );
        assert!(ids(&rescanned).contains("L03"));
        if !checkpoint_available {
            let mut delayed = ingest::source("other-agent", PROJECT, SourceKind::Journal);
            delayed.available = false;
            delayed.gaps = vec!["화요일 동기화 실패: 이 소스의 최신 상태 미확인".into()];
            dialogue.append(vec![Entity::Source(delayed)]).await?;
            let partial = query::execute(&dialogue.store.load().await?, &query(Operation::Brief))?;
            assert!(
                partial
                    .freshness
                    .iter()
                    .any(|fresh| fresh.source_id == "other-agent" && !fresh.available)
            );
            assert!(
                ids(&partial).contains("L02"),
                "{QUESTION}: one stale source must not hide collected facts"
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc12_checkout_selection_and_ambiguous_moves_keep_historical_rationale_separate()
-> TestResult {
    const QUESTION: &str = "retry에서 왜 backoff를 썼어?";
    let fixed = "fn retry(attempt: u32) { let delay = 100; sleep(delay); }\n";
    let backoff =
        "fn retry(attempt: u32) { let delay = 100 * 2_u64.pow(attempt); sleep(delay); }\n";
    for unspecified_checkout in [false, true] {
        let mut dialogue = Dialogue::new().await?;
        let request = message(
            "C01",
            ActorKind::Human,
            RecordKind::Request,
            "main의 기존 재시도 동작은 유지하고, WX에서 오류 이후 backoff를 실험해줘.",
        );
        let mut main = message(
            "C02",
            ActorKind::Agent,
            RecordKind::Decision,
            "main은 기존 호출자의 동작을 유지하도록 100ms 고정 재시도를 유지하겠습니다.",
        );
        main.decision_status = Some(DecisionStatus::Accepted);
        main.worktree_id = Some("main".into());
        let mut experiment = message(
            "C03",
            ActorKind::Agent,
            RecordKind::Decision,
            "WX에서는 오류가 난 뒤에만 backoff를 적용하겠습니다. 정상 요청에 대기를 넣지 않기 위해서입니다.",
        );
        experiment.decision_status = Some(DecisionStatus::Accepted);
        experiment.worktree_id = Some("WX".into());
        let mut move_report = message(
            "C04",
            ActorKind::Tool,
            RecordKind::Verification,
            "git diff --stat\nsrc/retry.rs => src/network/retry.rs\nstatus=observed",
        );
        move_report.verification_outcome = Some(VerificationOutcome::Unknown);
        move_report.code_refs = vec![CodeRef {
            state_id: "wx-moved".into(),
            path: "src/network/retry.rs".into(),
            range: None,
        }];
        let moved = if unspecified_checkout {
            state(
                "wx-moved",
                "WX",
                &[
                    ("src/network/retry.rs", backoff),
                    ("src/backup/retry.rs", backoff),
                ],
                false,
            )
        } else {
            let mut moved = state(
                "wx-moved",
                "WX",
                &[("src/network/retry.rs", backoff)],
                false,
            );
            for file in &mut moved.files {
                file.working_content = None;
            }
            moved
        };
        dialogue
            .append(vec![
                Entity::Record(request),
                Entity::Record(main.clone()),
                Entity::Record(experiment.clone()),
                Entity::Record(move_report),
                Entity::CodeState(state(
                    "main-original",
                    "main",
                    &[("src/retry.rs", fixed)],
                    false,
                )),
                Entity::CodeState(state(
                    "wx-original",
                    "WX",
                    &[("src/retry.rs", backoff)],
                    false,
                )),
                Entity::CodeState(moved),
                Entity::Relation(link(
                    "main-reason",
                    target("C02"),
                    code_target("main-original", "src/retry.rs"),
                    RelationKind::Supports,
                )),
                Entity::Relation(link(
                    "wx-reason",
                    target("C03"),
                    code_target("wx-original", "src/retry.rs"),
                    RelationKind::Supports,
                )),
            ])
            .await?;
        let corpus = dialogue.compact_and_reopen().await?;
        let mut trace = query(Operation::Trace);
        trace.direction = Some(Direction::Both);
        trace.target = Some(code_target("", "src/retry.rs"));
        if unspecified_checkout {
            let unresolved = query::execute(&corpus, &trace)?;
            let location = unresolved.location.as_ref().ok_or("missing candidates")?;
            assert_eq!(location.status, LocationStatus::Ambiguous);
            assert_eq!(location.candidates.len(), 2);
            assert!(
                unresolved.items.is_empty() && unresolved.relations.is_empty(),
                "{QUESTION}: same name does not select latest rationale"
            );
        } else {
            trace.scope.worktree_ids = vec!["WX".into()];
            let selected = query::execute(&corpus, &trace)?;
            assert_eq!(
                selected
                    .location
                    .as_ref()
                    .ok_or("missing resolution")?
                    .status,
                LocationStatus::Exact
            );
            assert!(ids(&selected).contains("C03"));
            assert!(!ids(&selected).contains("C02"));
        }
        trace.scope.worktree_ids.clear();
        for (state_id, expected, unrelated, text) in [
            ("main-original", "C02", "C03", fixed),
            ("wx-original", "C03", "C02", backoff),
        ] {
            trace.target = Some(code_target(state_id, "src/retry.rs"));
            let traced = query::execute(&corpus, &trace)?;
            assert!(ids(&traced).contains(expected));
            assert!(!ids(&traced).contains(unrelated));
            let mut historical = trace.clone();
            historical.operation = Operation::Read;
            let read_code = query::execute(&corpus, &historical)?;
            let retained = read_code
                .items
                .iter()
                .find_map(|item| match &item.entity {
                    Entity::CodeState(state) => Some(state),
                    _ => None,
                })
                .ok_or("historical code missing")?;
            assert_eq!(
                retained
                    .files
                    .first()
                    .and_then(|file| file.working_content.as_deref()),
                Some(text)
            );
        }
        trace.target = Some(code_target("wx-original", "src/retry.rs"));
        trace.code_mapping = Some(MappingRequest {
            target_state_id: "wx-moved".into(),
            target_source_id: Some("journal".into()),
            paths: if unspecified_checkout {
                vec!["src/network/retry.rs".into(), "src/backup/retry.rs".into()]
            } else {
                vec!["src/network/retry.rs".into()]
            },
        });
        let mapped = query::execute(&corpus, &trace)?;
        let location = mapped.location.as_ref().ok_or("missing mapping")?;
        assert_eq!(
            location.status,
            LocationStatus::Exact,
            "{QUESTION}: destination uncertainty cannot erase historical source"
        );
        assert_eq!(
            location.mapping_status,
            Some(if unspecified_checkout {
                LocationStatus::Ambiguous
            } else {
                LocationStatus::Unavailable
            })
        );
        assert!(ids(&mapped).contains("C03"));
        assert!(!ids(&mapped).contains("C02"));
    }
    Ok(())
}
