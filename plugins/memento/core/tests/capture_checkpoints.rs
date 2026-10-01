use std::{error::Error, path::Path};

use memento::{
    Store,
    capture::{Decision, EventKind, RecordRef, Request, Resolution, Scope},
    compaction::Policy,
    git::IndexBinding,
    ingest,
    model::*,
    security::RedactionPolicy,
};

type TestResult = Result<(), Box<dyn Error>>;

fn scope(repository: &Path, project: &str, turn: &str) -> Scope {
    Scope {
        project_id: project.into(),
        repository: repository.to_string_lossy().into_owned(),
        work_id: "upload-fix".into(),
        session_id: "codex-session".into(),
        turn_id: turn.into(),
    }
}

async fn source(store: &mut Store, project: &str) -> TestResult {
    store
        .append(Entity::Source(ingest::source(
            "journal",
            project,
            SourceKind::Journal,
        )))
        .await?;
    Ok(())
}

async fn open(store: &mut Store, scope: &Scope, id: &str, kind: EventKind) -> TestResult {
    let result = store
        .checkpoint(Request::Open {
            scope: scope.clone(),
            event_id: id.into(),
            kind,
            detail: "사용자 정정: 토큰 갱신은 수정하지 말고 처리량을 유지한다.".into(),
            commit_binding: None,
        })
        .await?;
    assert!(result.durable);
    Ok(())
}

async fn context(
    store: &mut Store,
    scope: &Scope,
    id: &str,
    kind: RecordKind,
) -> Result<RecordRef, Box<dyn Error>> {
    let mut record = Record::new(
        id,
        &scope.project_id,
        "journal",
        kind,
        "작업자 4개로 로컬 파일 40개 성공. 운영 피크와 기존 대비 처리량은 미검증.",
    );
    record.work_ids = vec![scope.work_id.clone()];
    record.session_id = Some(scope.session_id.clone());
    record.association = Association::Explicit;
    let revision = record.revision.clone();
    let receipt = store.append(Entity::Record(record)).await?;
    assert!(receipt.durable);
    Ok(RecordRef {
        source_id: "journal".into(),
        record_id: id.into(),
        revision,
        sequence: receipt.sequence,
    })
}

#[tokio::test]
async fn correction_capture_survives_reopen_and_a_new_continuation_turn() -> TestResult {
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("context.sqlite");
    let original = scope(dir.path(), "upload-app", "turn-1");
    let continuation = scope(dir.path(), "upload-app", "turn-2");
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    source(&mut store, "upload-app").await?;
    open(
        &mut store,
        &original,
        "user-correction",
        EventKind::UserPrompt,
    )
    .await?;
    drop(store);
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    let blocked = store
        .checkpoint(Request::Stop {
            scope: continuation.clone(),
        })
        .await?;
    assert_eq!(blocked.decision, Decision::Block);
    assert_eq!(blocked.pending_event_ids, ["user-correction"]);
    assert_eq!(
        blocked
            .events
            .first()
            .ok_or("missing event")?
            .original_turn_id,
        "turn-1"
    );
    let reference = context(
        &mut store,
        &continuation,
        "verified-with-limits",
        RecordKind::Verification,
    )
    .await?;
    let resolved = store
        .checkpoint(Request::Resolve {
            scope: continuation.clone(),
            event_id: "user-correction".into(),
            resolution: Resolution::Records {
                records: vec![reference],
            },
        })
        .await?;
    assert_eq!(resolved.decision, Decision::Allow);
    assert_eq!(
        store
            .checkpoint(Request::Stop {
                scope: continuation
            })
            .await?
            .decision,
        Decision::Allow
    );
    Ok(())
}

#[tokio::test]
async fn old_cross_project_status_and_wrong_revision_receipts_do_not_complete_capture() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    let billing = scope(dir.path(), "billing-api", "turn-1");
    source(&mut store, "upload-app").await?;
    source(&mut store, "billing-api").await?;
    let old = context(
        &mut store,
        &upload,
        "old-verification",
        RecordKind::Verification,
    )
    .await?;
    open(&mut store, &upload, "correction", EventKind::UserPrompt).await?;
    let other = context(
        &mut store,
        &billing,
        "same-looking-decision",
        RecordKind::Decision,
    )
    .await?;
    let status = context(&mut store, &upload, "filler", RecordKind::Status).await?;
    let valid = context(
        &mut store,
        &upload,
        "actual-correction",
        RecordKind::Feedback,
    )
    .await?;
    let mut wrong_revision = valid.clone();
    wrong_revision.revision = "invented-revision".into();
    for reference in [old, other, status, wrong_revision] {
        assert!(
            store
                .checkpoint(Request::Resolve {
                    scope: upload.clone(),
                    event_id: "correction".into(),
                    resolution: Resolution::Records {
                        records: vec![reference]
                    }
                })
                .await
                .is_err()
        );
    }
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: billing })
            .await?
            .decision,
        Decision::Allow
    );
    assert_eq!(
        store
            .checkpoint(Request::Status {
                scope: upload.clone()
            })
            .await?
            .pending_event_ids,
        ["correction"]
    );
    store
        .checkpoint(Request::Resolve {
            scope: upload,
            event_id: "correction".into(),
            resolution: Resolution::Records {
                records: vec![valid],
            },
        })
        .await?;
    Ok(())
}

#[tokio::test]
async fn unassigned_or_other_session_records_and_revoked_sources_are_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    source(&mut store, "upload-app").await?;
    open(&mut store, &upload, "failure", EventKind::ToolFailure).await?;
    for (id, association, session) in [
        ("unassigned", Association::Unassigned, "codex-session"),
        ("other-session", Association::Explicit, "other-session"),
    ] {
        let mut record = Record::new(
            id,
            "upload-app",
            "journal",
            RecordKind::Finding,
            "Failure reason",
        );
        record.association = association;
        record.session_id = Some(session.into());
        record.work_ids = vec!["upload-fix".into()];
        let revision = record.revision.clone();
        let receipt = store.append(Entity::Record(record)).await?;
        assert!(
            store
                .checkpoint(Request::Resolve {
                    scope: upload.clone(),
                    event_id: "failure".into(),
                    resolution: Resolution::Records {
                        records: vec![RecordRef {
                            source_id: "journal".into(),
                            record_id: id.into(),
                            revision,
                            sequence: receipt.sequence
                        }]
                    }
                })
                .await
                .is_err()
        );
    }
    let valid = context(&mut store, &upload, "failure-reason", RecordKind::Finding).await?;
    store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "failure".into(),
            resolution: Resolution::Records {
                records: vec![valid],
            },
        })
        .await?;
    let mut revoked = ingest::source("journal", "upload-app", SourceKind::Journal);
    revoked.authorized = false;
    store.append(Entity::Source(revoked)).await?;
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: upload })
            .await?
            .decision,
        Decision::CaptureIncomplete
    );
    Ok(())
}

#[tokio::test]
async fn stop_is_bounded_and_reports_unsaved_context_without_approving_a_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    open(&mut store, &upload, "unrecorded", EventKind::UserPrompt).await?;
    for expected in [1, 2] {
        let reply = store
            .checkpoint(Request::Stop {
                scope: upload.clone(),
            })
            .await?;
        assert_eq!(reply.decision, Decision::Block);
        assert_eq!(reply.stop_attempts, expected);
    }
    for _ in 0..3 {
        let reply = store
            .checkpoint(Request::Stop {
                scope: upload.clone(),
            })
            .await?;
        assert_eq!(reply.decision, Decision::CaptureIncomplete);
        assert_eq!(reply.stop_attempts, 2);
    }
    open(&mut store, &upload, "new-request", EventKind::UserPrompt).await?;
    assert_eq!(
        store
            .checkpoint(Request::Stop { scope: upload })
            .await?
            .decision,
        Decision::Block
    );
    Ok(())
}

#[tokio::test]
async fn checkpoint_refs_survive_compaction_and_metadata_consumes_the_same_byte_budget()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    source(&mut store, "upload-app").await?;
    open(&mut store, &upload, "finding", EventKind::ToolFailure).await?;
    let reference = context(&mut store, &upload, "evidence-finding", RecordKind::Finding).await?;
    store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "finding".into(),
            resolution: Resolution::Records {
                records: vec![reference.clone()],
            },
        })
        .await?;
    for i in 0..5 {
        context(
            &mut store,
            &upload,
            &format!("noise-{i}"),
            RecordKind::Status,
        )
        .await?;
    }
    let report = store
        .compact(
            Some(Policy {
                max_entries: 100,
                max_payload_bytes: 100_000,
                recent_entries: 0,
            }),
            true,
        )
        .await?;
    assert!(report.removed_entries >= 4);
    assert!(
        store
            .load()
            .await?
            .entries
            .iter()
            .any(|e| e.sequence == reference.sequence)
    );
    let before = store.compact(None, false).await?;
    store
        .compact(
            Some(Policy {
                max_entries: 100,
                max_payload_bytes: before.after.payload_bytes + 200,
                recent_entries: 0,
            }),
            true,
        )
        .await?;
    let result = store
        .checkpoint(Request::Open {
            scope: upload.clone(),
            event_id: "too-large".into(),
            kind: EventKind::UserPrompt,
            detail: "x".repeat(2000),
            commit_binding: None,
        })
        .await;
    assert!(matches!(result, Err(memento::Error::Capacity { .. })));
    assert!(
        !store
            .checkpoint(Request::Status { scope: upload })
            .await?
            .events
            .iter()
            .any(|e| e.event_id == "too-large")
    );
    Ok(())
}

#[tokio::test]
async fn no_new_context_has_a_persisted_redacted_reason_and_commit_requires_semantic_records()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy {
            literal_secrets: vec!["private-note".into()],
        },
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    store
        .checkpoint(Request::Open {
            scope: upload.clone(),
            event_id: "status-question".into(),
            kind: EventKind::UserPrompt,
            detail: "private-note 진행 상태만 알려줘".into(),
            commit_binding: None,
        })
        .await?;
    let result = store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "status-question".into(),
            resolution: Resolution::NoNewContext {
                reason: "private-note 요청에 새 제약 없음".into(),
            },
        })
        .await?;
    assert_eq!(result.decision, Decision::Allow);
    assert!(!serde_json::to_string(&result)?.contains("private-note"));
    let binding = IndexBinding {
        parent_head: None,
        staged_tree: "abc".into(),
    };
    store
        .checkpoint(Request::Open {
            scope: upload.clone(),
            event_id: "commit".into(),
            kind: EventKind::Commit,
            detail: "업로드 동시성 수정 커밋".into(),
            commit_binding: Some(binding.clone()),
        })
        .await?;
    assert!(
        store
            .checkpoint(Request::Resolve {
                scope: upload.clone(),
                event_id: "commit".into(),
                resolution: Resolution::NoNewContext {
                    reason: "없음".into()
                }
            })
            .await
            .is_err()
    );
    assert!(
        store
            .check_commit("upload-app", dir.path(), &binding)
            .await
            .is_err()
    );
    source(&mut store, "upload-app").await?;
    let reference = context(&mut store, &upload, "commit-context", RecordKind::Decision).await?;
    store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "commit".into(),
            resolution: Resolution::Records {
                records: vec![reference],
            },
        })
        .await?;
    store
        .check_commit("upload-app", dir.path(), &binding)
        .await?;
    let stale = IndexBinding {
        parent_head: None,
        staged_tree: "different".into(),
    };
    assert!(
        store
            .check_commit("upload-app", dir.path(), &stale)
            .await
            .is_err()
    );
    open(
        &mut store,
        &upload,
        "post-commit-event",
        EventKind::Mutation,
    )
    .await?;
    store
        .link_commit("upload-app", dir.path(), &binding, "actual-commit-sha")
        .await?;
    let after = store.checkpoint(Request::Status { scope: upload }).await?;
    assert!(
        after
            .events
            .iter()
            .any(|e| e.event_id == "commit" && e.commit_shas == ["actual-commit-sha"])
    );
    Ok(())
}

#[tokio::test]
async fn too_many_unresolved_events_are_rejected_without_silently_dropping_pending_context()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    for i in 0..128 {
        open(
            &mut store,
            &upload,
            &format!("pending-{i}"),
            EventKind::UserPrompt,
        )
        .await?;
    }
    assert!(
        open(&mut store, &upload, "overflow", EventKind::UserPrompt)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: upload })
            .await?
            .pending_event_ids
            .len(),
        128
    );
    Ok(())
}

#[tokio::test]
async fn redacted_context_is_valid_and_hook_summary_keeps_complete_gate_decisions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    source(&mut store, "upload-app").await?;
    open(
        &mut store,
        &upload,
        "credentials-observed",
        EventKind::ToolFailure,
    )
    .await?;
    let mut record = Record::new(
        "credential-failure",
        "upload-app",
        "journal",
        RecordKind::Finding,
        "password=fixture-value failed authentication; credential value is irrelevant to the finding.",
    );
    record.work_ids = vec![upload.work_id.clone()];
    record.session_id = Some(upload.session_id.clone());
    record.association = Association::Explicit;
    let revision = record.revision.clone();
    let receipt = store.append(Entity::Record(record)).await?;
    store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "credentials-observed".into(),
            resolution: Resolution::Records {
                records: vec![RecordRef {
                    source_id: "journal".into(),
                    record_id: "credential-failure".into(),
                    revision,
                    sequence: receipt.sequence,
                }],
            },
        })
        .await?;
    for i in 0..20 {
        open(
            &mut store,
            &upload,
            &format!("user-{i}"),
            EventKind::UserPrompt,
        )
        .await?;
    }
    let summary = store
        .checkpoint(Request::Status { scope: upload })
        .await?
        .summarize();
    assert_eq!(summary.decision, Decision::Block);
    assert!(summary.pending_user_prompt);
    assert!(summary.events.is_empty());
    assert_eq!(summary.pending_event_ids.len(), 8);
    assert_eq!(summary.omitted_pending_event_ids, 12);
    assert!(serde_json::to_vec(&summary)?.len() < 16 * 1024);
    Ok(())
}

fn git(repository: &Path, args: &[&str]) -> TestResult {
    let result = std::process::Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
    }
    Ok(())
}

async fn prepare_and_resolve(store: &mut Store, scope: &Scope, event_id: &str) -> TestResult {
    store
        .checkpoint(Request::PrepareCommit {
            scope: scope.clone(),
            event_id: event_id.into(),
            detail: "커밋 목적과 검증 한계".into(),
        })
        .await?;
    let reference = context(
        store,
        scope,
        &format!("context-{event_id}"),
        RecordKind::Decision,
    )
    .await?;
    store
        .checkpoint(Request::Resolve {
            scope: scope.clone(),
            event_id: event_id.into(),
            resolution: Resolution::Records {
                records: vec![reference],
            },
        })
        .await?;
    Ok(())
}

#[tokio::test]
async fn native_commit_check_cannot_borrow_another_session_and_newer_obligations_block_reuse()
-> TestResult {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q"])?;
    std::fs::write(dir.path().join("upload.rs"), "const WORKERS: usize = 4;\n")?;
    git(dir.path(), &["add", "upload.rs"])?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    source(&mut store, "upload-app").await?;
    let first = scope(dir.path(), "upload-app", "turn-1");
    prepare_and_resolve(&mut store, &first, "first-commit").await?;
    store
        .checkpoint(Request::CheckCommit {
            scope: first.clone(),
        })
        .await?;
    let mut wrong_session = first.clone();
    wrong_session.session_id = "unrelated-session".into();
    assert_eq!(
        store
            .checkpoint(Request::CheckCommit {
                scope: wrong_session
            })
            .await?
            .decision,
        Decision::Block
    );
    let mut wrong_work = first.clone();
    wrong_work.work_id = "unrelated-work".into();
    assert_eq!(
        store
            .checkpoint(Request::CheckCommit { scope: wrong_work })
            .await?
            .decision,
        Decision::Block
    );
    let mut second = first.clone();
    second.session_id = "second-session".into();
    prepare_and_resolve(&mut store, &second, "second-commit").await?;
    let binding = memento::git::index_binding(dir.path())?;
    store
        .check_commit("upload-app", dir.path(), &binding)
        .await?;
    // Revisit the first-created session later. Its latest pending event must win globally.
    store
        .checkpoint(Request::PrepareCommit {
            scope: first.clone(),
            event_id: "latest-pending".into(),
            detail: "새로운 결정이 아직 기록되지 않음".into(),
        })
        .await?;
    assert!(
        store
            .check_commit("upload-app", dir.path(), &binding)
            .await
            .is_err()
    );
    store
        .checkpoint(Request::Resolve {
            scope: first,
            event_id: "latest-pending".into(),
            resolution: Resolution::CaptureIncomplete {
                reason: "기록을 저장하지 못함".into(),
            },
        })
        .await?;
    assert!(
        store
            .check_commit("upload-app", dir.path(), &binding)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn cli_summary_is_json_and_leaves_a_persisted_pending_event() -> TestResult {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("context.sqlite");
    let upload = scope(dir.path(), "upload-app", "turn-1");
    let request = serde_json::json!({"operation":"open","scope":upload,"event_id":"correction","kind":"user_prompt","detail":"토큰 갱신을 수정하지 않는다"});
    let mut child = Command::new(env!("CARGO_BIN_EXE_memento"))
        .args([
            "checkpoint",
            "--store",
            database.to_str().ok_or("path")?,
            "--input",
            "-",
            "--summary",
            "true",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(&serde_json::to_vec(&request)?)?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reply: memento::capture::Reply = serde_json::from_slice(&output.stdout)?;
    assert!(reply.events.is_empty());
    assert!(reply.pending_user_prompt);
    assert!(reply.durable);
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: upload })
            .await?
            .pending_event_ids,
        ["correction"]
    );
    Ok(())
}

#[tokio::test]
async fn unavailable_resolved_refs_are_not_silently_evicted_to_admit_more_events() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let upload = scope(dir.path(), "upload-app", "turn-1");
    source(&mut store, "upload-app").await?;
    open(
        &mut store,
        &upload,
        "important-finding",
        EventKind::ToolFailure,
    )
    .await?;
    let reference = context(&mut store, &upload, "original", RecordKind::Finding).await?;
    store
        .checkpoint(Request::Resolve {
            scope: upload.clone(),
            event_id: "important-finding".into(),
            resolution: Resolution::Records {
                records: vec![reference],
            },
        })
        .await?;
    let mut revoked = ingest::source("journal", "upload-app", SourceKind::Journal);
    revoked.authorized = false;
    store.append(Entity::Source(revoked)).await?;
    for i in 0..127 {
        open(
            &mut store,
            &upload,
            &format!("pending-{i}"),
            EventKind::UserPrompt,
        )
        .await?;
    }
    assert!(
        open(&mut store, &upload, "overflow", EventKind::UserPrompt)
            .await
            .is_err()
    );
    let reply = store.checkpoint(Request::Status { scope: upload }).await?;
    assert_eq!(reply.events.len(), 128);
    assert_eq!(reply.capture_incomplete_event_ids, ["important-finding"]);
    Ok(())
}

#[tokio::test]
async fn old_completed_sessions_are_bounded_without_dropping_unresolved_sessions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(
        &dir.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    let base = scope(dir.path(), "upload-app", "turn-1");
    let mut unresolved = base.clone();
    unresolved.session_id = "unfinished".into();
    open(
        &mut store,
        &unresolved,
        "still-needed",
        EventKind::ToolFailure,
    )
    .await?;
    for i in 0..128 {
        let mut completed = base.clone();
        completed.session_id = format!("completed-{i}");
        open(
            &mut store,
            &completed,
            "status-only-question",
            EventKind::UserPrompt,
        )
        .await?;
        store
            .checkpoint(Request::Resolve {
                scope: completed.clone(),
                event_id: "status-only-question".into(),
                resolution: Resolution::NoNewContext {
                    reason: "진행 상태 조회이며 새로운 결정이나 제약 없음".into(),
                },
            })
            .await?;
        store.checkpoint(Request::Stop { scope: completed }).await?;
    }
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: unresolved })
            .await?
            .pending_event_ids,
        ["still-needed"]
    );
    let mut oldest = base.clone();
    oldest.session_id = "completed-0".into();
    assert!(
        store
            .checkpoint(Request::Status { scope: oldest })
            .await?
            .events
            .is_empty()
    );
    let mut latest = base;
    latest.session_id = "completed-127".into();
    assert_eq!(
        store
            .checkpoint(Request::Status { scope: latest })
            .await?
            .decision,
        Decision::Allow
    );
    Ok(())
}
