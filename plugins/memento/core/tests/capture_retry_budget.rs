use memento::{
    Store,
    capture::{Decision, EventKind, Request, Resolution, Scope},
    security::RedactionPolicy,
};

#[tokio::test]
async fn recovered_requests_during_continuations_preserve_stop_retry_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let mut scope = Scope {
        project_id: "upload-app".into(),
        repository: directory.path().to_string_lossy().into_owned(),
        work_id: "upload-fix".into(),
        session_id: "codex-session".into(),
        turn_id: "original".into(),
    };
    store
        .checkpoint(Request::Open {
            scope: scope.clone(),
            event_id: "verification".into(),
            kind: EventKind::Verification,
            detail: "Verification still requires capture.".into(),
            commit_binding: None,
        })
        .await?;
    assert_eq!(
        store
            .checkpoint(Request::Stop {
                scope: scope.clone(),
            })
            .await?
            .stop_attempts,
        1
    );
    for index in 0..3 {
        scope.turn_id = format!("continuation-{index}");
        let event_id = format!("recover-{index}");
        let recovered = store
            .checkpoint(Request::RecoverRequest {
                scope: scope.clone(),
                event_id: event_id.clone(),
                detail: "Current request was not observed; original prompt text is unavailable."
                    .into(),
            })
            .await?;
        assert!(recovered.current_turn_has_user_prompt);
        assert_eq!(recovered.stop_attempts, if index == 0 { 1 } else { 2 });
        store
            .checkpoint(Request::Resolve {
                scope: scope.clone(),
                event_id,
                resolution: Resolution::NoNewContext {
                    reason: "Checkpoint continuation only; the original request is unchanged."
                        .into(),
                },
            })
            .await?;
        let stopped = store
            .checkpoint(Request::Stop {
                scope: scope.clone(),
            })
            .await?;
        assert_eq!(stopped.stop_attempts, 2);
        assert_eq!(
            stopped.decision,
            if index == 0 {
                Decision::Block
            } else {
                Decision::CaptureIncomplete
            }
        );
    }
    Ok(())
}

#[tokio::test]
async fn tool_events_during_stop_continuations_cannot_reset_the_retry_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let mut scope = Scope {
        project_id: "upload-app".into(),
        repository: directory.path().to_string_lossy().into_owned(),
        work_id: "upload-fix".into(),
        session_id: "codex-session".into(),
        turn_id: "original".into(),
    };
    store
        .checkpoint(Request::Open {
            scope: scope.clone(),
            event_id: "correction".into(),
            kind: EventKind::UserPrompt,
            detail: "토큰 갱신은 수정하지 않는다".into(),
            commit_binding: None,
        })
        .await?;
    assert_eq!(
        store
            .checkpoint(Request::Stop {
                scope: scope.clone()
            })
            .await?
            .stop_attempts,
        1
    );
    for (index, (turn, event, kind)) in [
        ("continuation-1", "failed-save", EventKind::ToolFailure),
        ("continuation-2", "recheck", EventKind::Verification),
        ("continuation-3", "investigation", EventKind::Investigation),
    ]
    .into_iter()
    .enumerate()
    {
        scope.turn_id = turn.into();
        store
            .checkpoint(Request::Open {
                scope: scope.clone(),
                event_id: event.into(),
                kind,
                detail: "종료를 위한 기록 작성 중 추가 실행 결과가 관측됨".into(),
                commit_binding: None,
            })
            .await?;
        let reply = store
            .checkpoint(Request::Stop {
                scope: scope.clone(),
            })
            .await?;
        assert_eq!(reply.stop_attempts, 2);
        if turn == "continuation-1" {
            assert_eq!(reply.decision, Decision::Block);
        } else {
            assert_eq!(reply.decision, Decision::CaptureIncomplete);
            assert_eq!(reply.capture_incomplete_event_ids.len(), index + 2);
        }
    }
    Ok(())
}
