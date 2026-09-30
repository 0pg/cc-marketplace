//! SC-01–SC-06: synthetic user/agent/tool dialogue, captured typed links, and
//! retrieval after real SQLite compaction/reopen. Actual Git execution/capture
//! is covered separately by git_scenarios.rs and runtime_scenarios.rs; these
//! transcripts do not claim that the fixture commands ran or that prose was
//! automatically classified by the Datalog engine.

use std::error::Error;

use memento::{
    ingest,
    model::*,
    query::{self, QueryResponse, ResponseStatus},
};

#[path = "support/raw_dialogue.rs"]
mod raw_dialogue;
use raw_dialogue::{
    Dialogue, PROJECT, TestResult, evidence, ids, link, message, query as request, read, record,
};

fn target(id: &str) -> Target {
    Target::Record { id: id.into() }
}

fn commit_target(sha: &str) -> Target {
    Target::Commit {
        repository_id: "repo".into(),
        commit_sha: sha.into(),
    }
}

fn commit(sha: &str, tree: &str) -> Commit {
    Commit {
        id: sha.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        sha: sha.into(),
        tree: tree.into(),
        parents: vec![],
        paths: vec!["src/retry.rs".into()],
        message: "fix retry".into(),
        occurred_at: None,
        origin_worktree: None,
        observed_worktree: Some("observed-worktree".into()),
    }
}

fn state(id: &str, worktree: &str, sha: Option<&str>, content: &str) -> CodeState {
    CodeState {
        id: id.into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        repository_id: "repo".into(),
        worktree_id: Some(worktree.into()),
        commit_sha: sha.map(str::to_owned),
        observed_at: "2026-09-29T09:00:00Z".into(),
        changed_during_observation: false,
        files: vec![FileState {
            path: "src/retry.rs".into(),
            working_kind: WorkingFileKind::File,
            working_content: Some(content.into()),
            ..FileState::default()
        }],
    }
}

fn code(id: &str) -> CodeRef {
    CodeRef {
        state_id: id.into(),
        path: "src/retry.rs".into(),
        range: None,
    }
}

fn execution(id: &str, before: Option<&str>, complete: bool) -> Execution {
    Execution {
        id: id.into(),
        command: "cargo test retry".into(),
        tool_name: Some("shell".into()),
        tool_input: None,
        cwd: Some("/fixture/repo".into()),
        started_at: Some("2026-09-28T18:42:00Z".into()),
        ended_at: complete.then(|| "2026-09-28T18:43:00Z".into()),
        exit_code: complete.then_some(0),
        last_observed_state: if complete { "exited" } else { "running" }.into(),
        observed_at: Some("2026-09-28T18:42:00Z".into()),
        liveness: Liveness::Unknown,
        before_state: before.map(str::to_owned),
        after_state: None,
        scope: vec!["retry unit tests".into()],
        environment: Environment::default(),
    }
}

fn trace(corpus: &Corpus, target: Target) -> Result<QueryResponse, Box<dyn Error>> {
    let mut q = request(Operation::Trace);
    q.target = Some(target);
    q.max_depth = Some(8);
    Ok(query::execute(corpus, &q)?)
}

fn get_state(corpus: &Corpus, id: &str) -> Result<CodeState, Box<dyn Error>> {
    let mut q = request(Operation::Read);
    q.target = Some(Target::Code {
        state_id: id.into(),
        path: "src/retry.rs".into(),
        range: None,
    });
    query::execute(corpus, &q)?
        .items
        .into_iter()
        .find_map(|item| match item.entity {
            Entity::CodeState(state) if state.id == id => Some(state),
            _ => None,
        })
        .ok_or_else(|| format!("missing code state {id}").into())
}

#[tokio::test(flavor = "current_thread")]
async fn sc01_handoff_retains_rejected_attempt_and_does_not_invent_missing_measurement()
-> TestResult {
    for original_output in [true, false] {
        let mut d = Dialogue::new().await?;
        let user = message(
            "u1",
            ActorKind::Human,
            RecordKind::Request,
            "동시 요청에서 429가 자꾸 나. 줄여줘. 대신 정상 요청 지연은 늘리지 마.",
        );
        let mut attempt = message(
            "a1",
            ActorKind::Agent,
            RecordKind::Attempt,
            "모든 요청 앞에 500ms를 기다리게 바꾸고 정상 요청 지연을 재겠습니다.",
        );
        attempt.attempt_outcome = Some(AttemptOutcome::Failed);
        let output = message(
            "e1",
            ActorKind::Tool,
            RecordKind::ToolResult,
            "normal_request_p95_ms: before=42 after=543\nHTTP_429=0",
        );
        let mut decision = message(
            "d1",
            ActorKind::Agent,
            RecordKind::Decision,
            "정상 요청까지 느려졌습니다. 고정 500ms 대기는 버리고 오류 응답 뒤에만 지수 backoff를 적용하겠습니다.",
        );
        decision.decision_status = Some(DecisionStatus::Accepted);
        decision.evidence = vec![evidence(&user), evidence(&attempt), evidence(&output)];
        if !original_output && let Some(reference) = decision.evidence.last_mut() {
            reference.availability = Availability::Missing;
        }
        decision.code_refs = vec![code("K2")];
        decision.commit_shas = vec!["C1".into()];
        let mut patch = message(
            "P2",
            ActorKind::Agent,
            RecordKind::Change,
            "src/retry.rs에서 정상 응답은 바로 반환하고 오류 응답 분기에서만 backoff를 호출하도록 바꿨습니다.",
        );
        patch.code_refs = vec![code("K2")];
        patch.commit_shas = vec!["C1".into()];
        let mut verification = message(
            "e2",
            ActorKind::Tool,
            RecordKind::Verification,
            "cargo test retry\ntest result: ok. 8 passed; 0 failed\nload_test: NOT_RUN",
        );
        verification.verification_outcome = Some(VerificationOutcome::Passed);
        verification.execution = Some(execution("T2", Some("K2"), true));
        let mut remaining = message(
            "n1",
            ActorKind::Agent,
            RecordKind::Status,
            "C1을 만들었습니다. 실서비스 부하는 아직 못 돌렸으니 다음 작업에서 확인해야 합니다.",
        );
        remaining.commit_shas = vec!["C1".into()];
        let mut entries = vec![
            Entity::Record(user),
            Entity::Record(attempt),
            Entity::Record(decision),
            Entity::Record(patch),
            Entity::Record(verification),
            Entity::Record(remaining),
            Entity::CodeState(state("K2", "main", Some("C1"), "backoff_after_error();")),
            Entity::Commit(commit("C1", "tree-backoff")),
            Entity::Relation(link(
                "chosen-patch",
                target("d1"),
                target("P2"),
                RelationKind::Changes,
            )),
            Entity::Relation(link(
                "commit-reason",
                target("d1"),
                commit_target("C1"),
                RelationKind::Changes,
            )),
            Entity::Relation(link(
                "remaining-reason",
                target("n1"),
                target("d1"),
                RelationKind::RespondsTo,
            )),
            Entity::Relation(link(
                "verified-decision",
                target("e2"),
                target("d1"),
                RelationKind::Verifies,
            )),
        ];
        if original_output {
            entries.push(Entity::Record(output.clone()));
        }
        d.append(entries).await?;
        let corpus = d.compact_and_reopen().await?;
        let response = trace(&corpus, commit_target("C1"))?;
        assert!(ids(&response).contains("d1"));
        assert!(ids(&response).contains("P2"));
        assert_eq!(
            record(&read(&corpus, "u1")?, "u1")?.body,
            "동시 요청에서 429가 자꾸 나. 줄여줘. 대신 정상 요청 지연은 늘리지 마."
        );
        assert_eq!(
            record(&read(&corpus, "d1")?, "d1")?.nature,
            Nature::Reported
        );
        assert_eq!(
            record(&read(&corpus, "a1")?, "a1")?.attempt_outcome,
            Some(AttemptOutcome::Failed)
        );
        assert_eq!(
            record(&read(&corpus, "n1")?, "n1")?.body,
            "C1을 만들었습니다. 실서비스 부하는 아직 못 돌렸으니 다음 작업에서 확인해야 합니다."
        );
        assert_eq!(
            record(&read(&corpus, "e2")?, "e2")?
                .execution
                .as_ref()
                .and_then(|e| e.before_state.as_deref()),
            Some("K2")
        );
        let measured = read(&corpus, "e1")?;
        if original_output {
            assert_eq!(record(&measured, "e1")?.body, output.body);
            assert_eq!(record(&measured, "e1")?.nature, Nature::Observed);
        } else {
            assert!(measured.items.is_empty());
            assert_eq!(
                record(&read(&corpus, "d1")?, "d1")?
                    .evidence
                    .last()
                    .map(|e| e.availability),
                Some(Availability::Missing)
            );
            assert!(!serde_json::to_string(&response)?.contains("543"));
        }
        let mut list = request(Operation::ListWork);
        list.target = Some(Target::Work { id: "W1".into() });
        assert!(query::execute(&corpus, &list)?.items.iter().any(|item| matches!(&item.entity, Entity::Work(w) if w.id=="W1" && w.status==WorkStatus::Active)));
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc02_crashed_session_keeps_running_observation_distinct_from_liveness() -> TestResult {
    for originals in [true, false] {
        let mut d = Dialogue::new().await?;
        let user = message(
            "u2",
            ActorKind::Human,
            RecordKind::Request,
            "어제 앱이 꺼졌는데 어디까지 했어? 테스트 끝난 거면 바로 이어서 하자.",
        );
        let mut summary = message(
            "summary",
            ActorKind::Agent,
            RecordKind::Decision,
            "재개 메모: 요청 취소 시 backoff도 중단하도록 바꿨고 retry 테스트를 시작했다.",
        );
        summary.fidelity = Fidelity::SummaryOnly;
        summary.decision_status = Some(DecisionStatus::Accepted);
        let mut observation = message(
            "now",
            ActorKind::Tool,
            RecordKind::Verification,
            "observe src/retry.rs\nworking tree: modified\nprocess lookup: unsupported",
        );
        observation.verification_outcome = Some(VerificationOutcome::Unknown);
        observation.code_refs = vec![code("K3")];
        let mut entries = vec![
            Entity::Record(user),
            Entity::Record(summary),
            Entity::Record(observation),
            Entity::CodeState(state("K3", "main", None, "cancel_stops_backoff();")),
        ];
        if originals {
            let mut patch = message(
                "P3",
                ActorKind::Agent,
                RecordKind::Change,
                "취소 신호를 backoff 루프에 연결했습니다. 아직 커밋하지 않았습니다.",
            );
            patch.code_refs = vec![code("K3")];
            let mut start = message(
                "X7",
                ActorKind::Agent,
                RecordKind::Attempt,
                "이제 cargo test retry를 실행하겠습니다.",
            );
            start.attempt_outcome = Some(AttemptOutcome::Running);
            start.execution = Some(execution("X7", Some("K3"), false));
            start.evidence = vec![evidence(&patch)];
            let mut partial = message(
                "E7",
                ActorKind::Tool,
                RecordKind::ToolResult,
                "running 8 tests\ntest retry::cancel ...",
            );
            partial.execution = Some(execution("X7", Some("K3"), false));
            partial.partial = true;
            let saved = message(
                "R42",
                ActorKind::Tool,
                RecordKind::ToolResult,
                "{\"durable\":true,\"record_id\":\"P3\",\"sequence\":42}",
            );
            start.evidence.push(evidence(&saved));
            let mut decision = message(
                "D2",
                ActorKind::Agent,
                RecordKind::Decision,
                "취소된 요청이 다시 전송되지 않도록 취소 신호를 대기 루프에도 전달하겠습니다.",
            );
            decision.decision_status = Some(DecisionStatus::Accepted);
            decision.evidence = vec![evidence(&patch), evidence(&saved)];
            entries.extend([
                Entity::Record(decision),
                Entity::Record(patch),
                Entity::Record(start),
                Entity::Record(partial),
                Entity::Record(saved),
            ]);
        }
        d.append(entries).await?;
        let corpus = d.compact_and_reopen().await?;
        assert_eq!(
            get_state(&corpus, "K3")?
                .files
                .first()
                .and_then(|f| f.working_content.as_deref()),
            Some("cancel_stops_backoff();")
        );
        let response = read(&corpus, "X7")?;
        if originals {
            let exec = record(&response, "X7")?
                .execution
                .as_ref()
                .ok_or("execution absent")?;
            assert_eq!(exec.last_observed_state, "running");
            assert_eq!(exec.liveness, Liveness::Unknown);
            assert_eq!(exec.exit_code, None);
            assert_eq!(exec.ended_at, None);
            assert_eq!(
                record(&read(&corpus, "D2")?, "D2")?.fidelity,
                Fidelity::Original
            );
            assert!(record(&read(&corpus, "E7")?, "E7")?.partial);
            assert_eq!(
                record(&read(&corpus, "R42")?, "R42")?.body,
                "{\"durable\":true,\"record_id\":\"P3\",\"sequence\":42}"
            );
        } else {
            assert!(response.items.is_empty());
            assert!(read(&corpus, "P3")?.items.is_empty());
            let summary = read(&corpus, "summary")?;
            assert_eq!(record(&summary, "summary")?.fidelity, Fidelity::SummaryOnly);
            assert_eq!(record(&summary, "summary")?.nature, Nature::Reported);
            assert!(!summary.items.iter().any(|item| matches!(&item.entity, Entity::Record(r) if r.verification_outcome == Some(VerificationOutcome::Passed))));
        }
        let before = corpus.entries.len();
        let mut resume = request(Operation::Brief);
        resume.purpose = Some(BriefPurpose::Resume);
        query::execute(&corpus, &resume)?;
        assert_eq!(
            d.store.load().await?.entries.len(),
            before,
            "resuming reads cannot restart a command or capture a new result"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc03_partial_commit_never_inherits_dirty_tree_verification_or_hunk_ownership() -> TestResult
{
    for hunks_recorded in [true, false] {
        let mut d = Dialogue::new().await?;
        let mut request_retry = message(
            "retry-request",
            ActorKind::Human,
            RecordKind::Request,
            "retry 대기 계산을 바꿔줘. 로그 형식 수정은 별도 작업이야.",
        );
        request_retry.work_ids = vec!["W3".into()];
        let mut request_log = message(
            "log-request",
            ActorKind::Human,
            RecordKind::Request,
            "같은 파일의 로그도 JSON으로 바꿔줘. 이건 W4로 남겨.",
        );
        request_log.work_ids = vec!["W4".into()];
        let mut test = message(
            "T3",
            ActorKind::Tool,
            RecordKind::Verification,
            "cargo test retry\ntest result: ok. 8 passed; 0 failed",
        );
        test.verification_outcome = Some(VerificationOutcome::Passed);
        test.execution = Some(execution("T3", Some("K4"), true));
        let mut partial = message(
            "partial-commit",
            ActorKind::Tool,
            RecordKind::GitEvent,
            "git show C2:src/retry.rs\nbackoff();\ntext_log();\ngit diff\n-text_log();\n+json_log();",
        );
        partial.commit_shas = vec!["C2".into()];
        partial.code_refs = vec![code("K4")];
        let mut decision = message(
            "stage-request",
            ActorKind::Human,
            RecordKind::Decision,
            "지금은 retry 쪽만 stage해서 커밋해. 로그 수정은 작업 트리에 남겨 둬.",
        );
        decision.decision_status = Some(DecisionStatus::Accepted);
        decision.evidence = vec![evidence(&partial)];
        let mut entries = vec![
            Entity::Record(request_retry),
            Entity::Record(request_log),
            Entity::Record(test),
            Entity::Record(partial),
            Entity::Record(decision),
            Entity::CodeState(state("K4", "main", None, "backoff();\njson_log();")),
            Entity::Commit(commit("C2", "tree-H1-only")),
        ];
        for (id, title) in [("W3", "retry"), ("W4", "logging")] {
            entries.push(Entity::Work(Work {
                id: id.into(),
                project_id: PROJECT.into(),
                source_id: "journal".into(),
                title: title.into(),
                goal: title.into(),
                status: WorkStatus::Active,
                observed_at: None,
                evidence: vec![],
                completion_conditions: vec![],
            }));
        }
        if hunks_recorded {
            let mut h1 = message(
                "H1",
                ActorKind::Agent,
                RecordKind::Change,
                "대기 계산만 backoff()로 바꿨습니다.",
            );
            h1.work_ids = vec!["W3".into()];
            h1.commit_shas = vec!["C2".into()];
            let mut h2 = message(
                "H2",
                ActorKind::Agent,
                RecordKind::Change,
                "로그 호출만 json_log()로 바꿨습니다. 아직 커밋하지 않았습니다.",
            );
            h2.work_ids = vec!["W4".into()];
            h2.code_refs = vec![code("K4")];
            let mut h3 = message(
                "H3",
                ActorKind::Agent,
                RecordKind::Change,
                "retry 설명을 문서에 추가하고 C3으로 커밋했습니다.",
            );
            h3.work_ids = vec!["W3".into()];
            h3.commit_shas = vec!["C3".into()];
            let mut combined = message(
                "combined-branch",
                ActorKind::Human,
                RecordKind::Decision,
                "별도 검토 브랜치에서는 대기 계산과 JSON 로그를 함께 C4로 묶었어. 이 커밋은 W3와 W4 양쪽 작업에 연결해 둬.",
            );
            combined.decision_status = Some(DecisionStatus::Accepted);
            combined.commit_shas = vec!["C4".into()];
            combined.work_ids = vec!["W3".into(), "W4".into()];
            entries.extend([
                Entity::Record(h1),
                Entity::Record(h2),
                Entity::Record(h3),
                Entity::Record(combined),
                Entity::Commit(commit("C3", "tree-H3")),
                Entity::Commit(commit("C4", "tree-H1-H2")),
                Entity::Relation(link(
                    "C4-retry",
                    commit_target("C4"),
                    Target::Work { id: "W3".into() },
                    RelationKind::Changes,
                )),
                Entity::Relation(link(
                    "C4-log",
                    commit_target("C4"),
                    Target::Work { id: "W4".into() },
                    RelationKind::Changes,
                )),
                Entity::Relation(link(
                    "C2-work",
                    commit_target("C2"),
                    Target::Work { id: "W3".into() },
                    RelationKind::Changes,
                )),
                Entity::Relation(link(
                    "H1-stage",
                    target("stage-request"),
                    target("H1"),
                    RelationKind::RespondsTo,
                )),
                Entity::Relation(link(
                    "H2-stage",
                    target("stage-request"),
                    target("H2"),
                    RelationKind::RespondsTo,
                )),
                Entity::Relation(link(
                    "H3-doc",
                    target("retry-request"),
                    target("H3"),
                    RelationKind::RespondsTo,
                )),
            ]);
        } else {
            let mut candidate = message(
                "file-candidate",
                ActorKind::Agent,
                RecordKind::Change,
                "W3와 W4 모두 src/retry.rs를 수정했습니다.",
            );
            candidate.association = Association::Candidate;
            candidate.work_ids = vec!["W3".into(), "W4".into()];
            candidate.paths = vec!["src/retry.rs".into()];
            entries.extend([
                Entity::Record(candidate),
                Entity::Relation(link(
                    "candidate-request",
                    target("stage-request"),
                    target("file-candidate"),
                    RelationKind::RespondsTo,
                )),
            ]);
        }
        d.append(entries).await?;
        let corpus = d.compact_and_reopen().await?;
        let response = trace(&corpus, commit_target("C2"))?;
        assert!(response.items.iter().any(
            |i| matches!(&i.entity,Entity::Commit(c) if c.sha=="C2" && c.tree=="tree-H1-only")
        ));
        let tested = read(&corpus, "T3")?;
        assert_eq!(
            record(&tested, "T3")?
                .execution
                .as_ref()
                .and_then(|e| e.before_state.as_deref()),
            Some("K4")
        );
        assert!(record(&tested, "T3")?.commit_shas.is_empty());
        assert!(
            !response
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Verifies
                    && (r.to == commit_target("C2") || r.from == commit_target("C2")))
        );
        assert!(
            get_state(&corpus, "K4")?
                .files
                .first()
                .and_then(|f| f.working_content.as_deref())
                .is_some_and(|s| s.contains("json_log"))
        );
        if hunks_recorded {
            assert_eq!(record(&read(&corpus, "H1")?, "H1")?.work_ids, vec!["W3"]);
            assert_eq!(record(&read(&corpus, "H2")?, "H2")?.work_ids, vec!["W4"]);
            assert!(record(&read(&corpus, "H2")?, "H2")?.commit_shas.is_empty());
            assert_eq!(record(&read(&corpus, "H3")?, "H3")?.commit_shas, vec!["C3"]);
            let combined = trace(&corpus, commit_target("C4"))?;
            assert!(ids(&combined).contains("W3"));
            assert!(ids(&combined).contains("W4"));
            assert_eq!(
                corpus
                    .entries
                    .iter()
                    .filter(|entry| entry.entity.id() == "T3")
                    .count(),
                1
            );
        } else {
            assert!(read(&corpus, "H1")?.items.is_empty());
            assert!(read(&corpus, "H2")?.items.is_empty());
            assert_eq!(
                record(&read(&corpus, "file-candidate")?, "file-candidate")?.association,
                Association::Candidate
            );
        }
        assert!(query::execute(&corpus,&request(Operation::ListWork))?.items.iter().any(|i|matches!(&i.entity,Entity::Work(w) if w.id=="W4" && w.status==WorkStatus::Active)));
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc04_rewrite_chain_preserves_old_reason_without_promoting_old_test() -> TestResult {
    for mappings in [true, false] {
        let mut d = Dialogue::new().await?;
        let mut decision = message(
            "D3",
            ActorKind::Agent,
            RecordKind::Decision,
            "이 응답은 재시도해도 바뀌지 않으니 영구 오류일 때 바로 반환하겠습니다.",
        );
        decision.decision_status = Some(DecisionStatus::Accepted);
        decision.commit_shas = vec!["C5".into()];
        let mut test = message(
            "T4",
            ActorKind::Tool,
            RecordKind::Verification,
            "HEAD=C5\ncargo test retry\ntest result: ok. 8 passed; 0 failed",
        );
        test.verification_outcome = Some(VerificationOutcome::Passed);
        test.commit_shas = vec!["C5".into()];
        test.execution = Some(execution("T4", Some("K5"), true));
        let mut user = message(
            "U4",
            ActorKind::Human,
            RecordKind::Request,
            "amend한 뒤 문서 커밋까지 squash했어. 지금 C7의 이유와 테스트 범위를 찾아줘.",
        );
        user.commit_shas = vec!["C7".into()];
        let mut entries = vec![
            Entity::Record(decision),
            Entity::Record(test),
            Entity::Record(user),
            Entity::Commit(commit("C7", "tree-amended-and-docs")),
            Entity::Relation(link(
                "C7-work",
                commit_target("C7"),
                Target::Work { id: "W1".into() },
                RelationKind::Changes,
            )),
        ];
        if mappings {
            let mut patch = message(
                "P5",
                ActorKind::Agent,
                RecordKind::Change,
                "영구 오류이면 재시도 루프에 들어가기 전에 바로 반환하도록 분기를 추가했습니다.",
            );
            patch.code_refs = vec![code("K5")];
            patch.commit_shas = vec!["C5".into()];
            entries.extend([
                Entity::Record(patch),
                Entity::Relation(link(
                    "original-change",
                    target("D3"),
                    target("P5"),
                    RelationKind::Changes,
                )),
                Entity::Commit(commit("C5", "tree-original")),
                Entity::Commit(commit("C5a", "tree-amended")),
                Entity::Commit(commit("C6", "tree-docs")),
                Entity::CodeState(state(
                    "K5",
                    "main",
                    Some("C5"),
                    "permanent_error_returns();",
                )),
            ]);
            for (id, new, old) in [
                ("amend", "C5a", "C5"),
                ("squash-code", "C7", "C5a"),
                ("squash-doc", "C7", "C6"),
            ] {
                let raw = message(
                    id,
                    ActorKind::Tool,
                    RecordKind::GitEvent,
                    &format!("post-rewrite\n{old} {new}"),
                );
                let mut relation = link(
                    &format!("map-{id}"),
                    commit_target(new),
                    commit_target(old),
                    RelationKind::DerivedFrom,
                );
                relation.evidence = vec![evidence(&raw)];
                relation.nature = Nature::Observed;
                entries.extend([Entity::Record(raw), Entity::Relation(relation)]);
            }
            entries.push(Entity::Relation(link(
                "original-reason",
                target("D3"),
                commit_target("C5"),
                RelationKind::Changes,
            )));
        }
        d.append(entries).await?;
        let corpus = d.compact_and_reopen().await?;
        let response = trace(&corpus, commit_target("C7"))?;
        let test = read(&corpus, "T4")?;
        assert_eq!(record(&test, "T4")?.commit_shas, vec!["C5"]);
        assert!(
            !response
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Verifies && r.to == commit_target("C7"))
        );
        if mappings {
            for id in ["C5", "C5a", "C6", "C7", "D3", "P5"] {
                assert!(ids(&response).contains(id), "missing {id}");
            }
            assert_eq!(
                response
                    .relations
                    .iter()
                    .filter(|r| r.kind == RelationKind::DerivedFrom)
                    .count(),
                3
            );
            assert_eq!(
                record(&read(&corpus, "amend")?, "amend")?.body,
                "post-rewrite\nC5 C5a"
            );
            assert_eq!(get_state(&corpus, "K5")?.commit_sha.as_deref(), Some("C5"));
        } else {
            assert!(
                response
                    .relations
                    .iter()
                    .all(|r| r.kind != RelationKind::DerivedFrom)
            );
            assert!(
                !ids(&response).contains("D3"),
                "same commit message cannot establish a rewrite lineage"
            );
            assert!(get_state(&corpus, "K5").is_err());
            let mut q = request(Operation::Read);
            q.target = Some(commit_target("C5"));
            assert!(query::execute(&corpus, &q)?.items.is_empty());
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc05_release_revert_does_not_cancel_main_or_invent_missing_reason() -> TestResult {
    for reasons in [true, false] {
        let mut d = Dialogue::new().await?;
        let mut original = message(
            "D4",
            ActorKind::Agent,
            RecordKind::Decision,
            "main의 신규 API는 429 응답 뒤에만 backoff를 넣겠습니다.",
        );
        original.decision_status = Some(DecisionStatus::Accepted);
        original.commit_shas = vec!["C8".into()];
        let mut user = message(
            "U5",
            ActorKind::Human,
            RecordKind::Request,
            "release에서 취소된 건 알겠는데 main도 빠진 거야? 지금 두 브랜치를 확인해줘.",
        );
        user.commit_shas = vec!["C9".into(), "R1".into()];
        let mut observation = message(
            "current",
            ActorKind::Tool,
            RecordKind::Verification,
            "git show main:src/retry.rs\nnew_api_backoff();\ngit show release:src/retry.rs\nlegacy_api();",
        );
        observation.verification_outcome = Some(VerificationOutcome::Unknown);
        observation.code_refs = vec![code("main-now"), code("release-now")];
        let mut passed = message(
            "T5",
            ActorKind::Tool,
            RecordKind::Verification,
            "ref=release HEAD=C9\ntest result: ok. 12 passed; 0 failed",
        );
        passed.verification_outcome = Some(VerificationOutcome::Passed);
        passed.commit_shas = vec!["C9".into()];
        let reverted = message(
            "revert-output",
            ActorKind::Tool,
            RecordKind::GitEvent,
            "git show R1\nRevert fix retry\nThis reverts commit C9.",
        );
        let mut reverts = link(
            "revert",
            commit_target("R1"),
            commit_target("C9"),
            RelationKind::Reverts,
        );
        reverts.nature = Nature::Observed;
        reverts.evidence = vec![evidence(&reverted)];
        let mut entries = vec![
            Entity::Record(original),
            Entity::Record(user),
            Entity::Record(observation),
            Entity::Record(passed),
            Entity::Record(reverted),
            Entity::Commit(commit("C8", "main-new-api")),
            Entity::Commit(commit("C9", "release-adapted")),
            Entity::Commit(commit("R1", "release-reverted")),
            Entity::CodeState(state("main-now", "main", Some("C8"), "new_api_backoff();")),
            Entity::CodeState(state("release-now", "release", Some("R1"), "legacy_api();")),
            Entity::Relation(link(
                "main-choice",
                target("D4"),
                commit_target("C8"),
                RelationKind::Changes,
            )),
            Entity::Relation(reverts),
        ];
        if reasons {
            let conflict = message(
                "E9",
                ActorKind::Agent,
                RecordKind::Change,
                "cherry-pick에서 충돌났습니다. release에는 retry_after 필드가 없어서 응답 헤더를 읽도록 바꿨습니다.",
            );
            let mut why = message(
                "D5",
                ActorKind::Human,
                RecordKind::Decision,
                "release의 구형 API와 호환 문제가 생겼어. 이 브랜치의 C9만 되돌려줘.",
            );
            why.decision_status = Some(DecisionStatus::Accepted);
            why.commit_shas = vec!["R1".into()];
            why.evidence = vec![evidence(&conflict)];
            entries.extend([
                Entity::Record(conflict),
                Entity::Record(why),
                Entity::Relation(link(
                    "release-choice",
                    target("D5"),
                    commit_target("R1"),
                    RelationKind::Changes,
                )),
                Entity::Relation(link(
                    "release-adaptation",
                    target("E9"),
                    commit_target("C9"),
                    RelationKind::Changes,
                )),
                Entity::Relation(link(
                    "cherry-pick",
                    commit_target("C9"),
                    commit_target("C8"),
                    RelationKind::DerivedFrom,
                )),
            ]);
        }
        d.append(entries).await?;
        let corpus = d.compact_and_reopen().await?;
        let response = trace(&corpus, commit_target("R1"))?;
        assert_eq!(
            record(&read(&corpus, "revert-output")?, "revert-output")?.body,
            "git show R1\nRevert fix retry\nThis reverts commit C9."
        );
        assert!(
            response
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Reverts && r.to == commit_target("C9"))
        );
        assert!(
            !response
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Reverts && r.to == commit_target("C8"))
        );
        assert_eq!(
            get_state(&corpus, "main-now")?
                .files
                .first()
                .and_then(|f| f.working_content.as_deref()),
            Some("new_api_backoff();")
        );
        assert_eq!(
            get_state(&corpus, "release-now")?
                .files
                .first()
                .and_then(|f| f.working_content.as_deref()),
            Some("legacy_api();")
        );
        assert_eq!(record(&read(&corpus, "T5")?, "T5")?.commit_shas, vec!["C9"]);
        assert_eq!(
            record(&read(&corpus, "D4")?, "D4")?.decision_status,
            Some(DecisionStatus::Accepted)
        );
        if reasons {
            for id in ["D4", "E9", "D5"] {
                assert!(ids(&response).contains(id));
            }
            assert!(
                record(&read(&corpus, "E9")?, "E9")?
                    .body
                    .contains("retry_after")
            );
            assert_eq!(
                record(&read(&corpus, "D5")?, "D5")?
                    .actor
                    .as_ref()
                    .map(|a| a.kind),
                Some(ActorKind::Human)
            );
        } else {
            assert!(read(&corpus, "E9")?.items.is_empty());
            assert!(read(&corpus, "D5")?.items.is_empty());
            assert!(!serde_json::to_string(&response)?.contains("retry_after"));
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sc06_hook_gap_recovers_only_retained_dialogue_and_deduplicates_redelivery() -> TestResult {
    for retained_process in [true, false] {
        let mut d = Dialogue::new().await?;
        let mut source = ingest::source("git-history", PROJECT, SourceKind::Git);
        source.result_only = true;
        source.gaps = vec!["commit result captured; process unavailable".into()];
        let mut old = commit("C10", "old-tree");
        old.source_id = "git-history".into();
        let mut recent = commit("C11", "new-tree");
        if !retained_process {
            recent.source_id = "git-history".into();
        }
        let mut user = message(
            "U6",
            ActorKind::Human,
            RecordKind::Request,
            "최근 커밋 과정이 다 남았어? 빠진 것만 복구해줘. 원래 hooksPath는 건드리지 마.",
        );
        user.commit_shas = vec!["C10".into(), "C11".into()];
        let mut entries = vec![
            Entity::Source(source),
            Entity::Commit(old),
            Entity::Commit(recent.clone()),
            Entity::Record(user),
        ];
        let mut redelivery = vec![Entity::Commit(recent)];
        if retained_process {
            let mut original = message(
                "original",
                ActorKind::Human,
                RecordKind::Decision,
                "정상 요청 지연은 그대로 두고 429일 때만 기다리도록 바꿔줘.",
            );
            original.decision_status = Some(DecisionStatus::Accepted);
            let warning = message(
                "hook-warning",
                ActorKind::Tool,
                RecordKind::ToolResult,
                "[main C11] fix retry\nwork-context: capture failed: database is read-only\nGit commit succeeded",
            );
            let mut recovered = message(
                "recovered",
                ActorKind::Agent,
                RecordKind::Verification,
                "C11과 남아 있던 대화 원문을 읽었습니다. 두 기록을 연결하겠습니다.",
            );
            recovered.verification_outcome = Some(VerificationOutcome::Unknown);
            recovered.evidence = vec![evidence(&original), evidence(&warning)];
            recovered.commit_shas = vec!["C11".into()];
            let relation = link(
                "recovered-link",
                target("original"),
                commit_target("C11"),
                RelationKind::Changes,
            );
            redelivery.extend([
                Entity::Record(recovered.clone()),
                Entity::Relation(relation.clone()),
            ]);
            entries.extend([
                Entity::Record(original),
                Entity::Record(warning),
                Entity::Record(recovered),
                Entity::Relation(relation),
            ]);
        }
        d.append(entries).await?;
        let receipts = d.store.append_all(redelivery.clone()).await?;
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.durable && receipt.duplicate)
        );
        let corpus = d.compact_and_reopen().await?;
        let receipts = d.store.append_all(redelivery).await?;
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.durable && receipt.duplicate)
        );
        assert_eq!(d.store.load().await?.entries.len(), corpus.entries.len());
        let mut q = request(Operation::Read);
        q.target = Some(commit_target("C11"));
        let response = query::execute(&corpus, &q)?;
        let commits: Vec<_> = response
            .items
            .iter()
            .filter_map(|i| match &i.entity {
                Entity::Commit(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(commits.len(), 1);
        let captured = commits.first().ok_or("missing C11")?;
        assert_eq!(captured.origin_worktree, None);
        assert_eq!(
            captured.observed_worktree.as_deref(),
            Some("observed-worktree")
        );
        let sources = query::execute(&corpus, &request(Operation::Sources))?;
        assert!(
            sources
                .coverage
                .iter()
                .any(|s| s.source_id == "git-history" && s.result_only)
        );
        let response = trace(&corpus, commit_target("C11"))?;
        if retained_process {
            assert!(ids(&response).contains("original"));
            assert_eq!(
                record(&read(&corpus, "hook-warning")?, "hook-warning")?.body,
                "[main C11] fix retry\nwork-context: capture failed: database is read-only\nGit commit succeeded"
            );
            assert_eq!(
                response
                    .relations
                    .iter()
                    .filter(|r| r.id == "recovered-link")
                    .count(),
                1
            );
        } else {
            assert!(read(&corpus, "original")?.items.is_empty());
            let missing = read(&corpus, "hook-warning")?;
            assert!(matches!(
                missing.status,
                ResponseStatus::NoMatches | ResponseStatus::Partial
            ));
            assert!(!serde_json::to_string(&response)?.contains("database is read-only"));
            assert!(!ids(&response).contains("recovered"));
        }
    }
    Ok(())
}
