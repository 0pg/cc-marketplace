use std::error::Error;

use memento::{
    Error as StoreError, Store,
    capture::{Decision, Event, EventKind, RecordRef, Request, Resolution, Scope},
    compaction::Policy,
    ingest,
    model::*,
    security::RedactionPolicy,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

struct Scenario {
    store: Store,
    scope: Scope,
    _directory: TempDir,
}

impl Scenario {
    async fn new(policy: RedactionPolicy) -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let scope = Scope {
            project_id: "upload-app".into(),
            repository: directory.path().to_string_lossy().into_owned(),
            work_id: "upload-429".into(),
            session_id: "codex-session".into(),
            turn_id: "turn-1".into(),
        };
        let mut store = Store::open(&directory.path().join("context.sqlite"), policy).await?;
        store
            .append(Entity::Source(ingest::source(
                "journal",
                &scope.project_id,
                SourceKind::Journal,
            )))
            .await?;
        Ok(Self {
            store,
            scope,
            _directory: directory,
        })
    }

    async fn open(
        &mut self,
        id: &str,
        kind: EventKind,
        body: &str,
    ) -> Result<Event, Box<dyn Error>> {
        let reply = self
            .store
            .checkpoint(Request::Open {
                scope: self.scope.clone(),
                event_id: id.into(),
                kind,
                detail: body.into(),
                commit_binding: None,
            })
            .await?;
        assert!(reply.durable);
        assert_eq!(reply.decision, Decision::Block);
        reply
            .events
            .into_iter()
            .find(|event| event.event_id == id)
            .ok_or_else(|| "missing opened event".into())
    }

    async fn write(&mut self, record: Record) -> Result<RecordRef, Box<dyn Error>> {
        let reference = RecordRef {
            source_id: record.source_id.clone(),
            record_id: record.id.clone(),
            revision: record.revision.clone(),
            sequence: self.store.append(Entity::Record(record)).await?.sequence,
        };
        Ok(reference)
    }

    async fn resolve(
        &mut self,
        event: &Event,
        references: Vec<RecordRef>,
    ) -> memento::Result<memento::capture::Reply> {
        self.store
            .checkpoint(Request::Resolve {
                scope: self.scope.clone(),
                event_id: event.event_id.clone(),
                resolution: Resolution::Records {
                    records: references,
                },
            })
            .await
    }
}

fn reference(origin: &RecordRef, purpose: EvidencePurpose) -> Evidence {
    Evidence {
        source_id: origin.source_id.clone(),
        record_id: Some(origin.record_id.clone()),
        revision: origin.revision.clone(),
        locator: format!("record:{}", origin.record_id),
        availability: Availability::Available,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
        purpose,
        span: None,
    }
}

fn claim(
    scope: &Scope,
    event: &Event,
    id: &str,
    kind: RecordKind,
    body: &str,
) -> Result<Record, Box<dyn Error>> {
    let mut record = Record::new(id, &scope.project_id, "journal", kind, body);
    record.representation = Representation::Claim;
    record.context_id = event.context_id.clone();
    record.derived = true;
    record.nature = Nature::Reported;
    record.fidelity = Fidelity::SummaryOnly;
    record.association = Association::Explicit;
    record.work_ids = vec![scope.work_id.clone()];
    record.session_id = Some(scope.session_id.clone());
    record.evidence = vec![reference(
        event.origin.as_ref().ok_or("missing native observation")?,
        EvidencePurpose::Origin,
    )];
    Ok(record)
}

fn raw(id: &str, body: &str, context: &str) -> Record {
    let mut record = Record::new(id, "upload-app", "journal", RecordKind::ToolResult, body);
    record.representation = Representation::Evidence;
    record.context_id = Some(context.into());
    record
}

fn manual_claim(original: &Record, id: &str) -> Record {
    let origin = RecordRef {
        source_id: original.source_id.clone(),
        record_id: original.id.clone(),
        revision: original.revision.clone(),
        sequence: 0,
    };
    let mut record = Record::new(
        id,
        "upload-app",
        "journal",
        RecordKind::Constraint,
        "처리량을 유지한다.",
    );
    record.representation = Representation::Claim;
    record.context_id = original.context_id.clone();
    record.derived = true;
    record.nature = Nature::Reported;
    record.fidelity = Fidelity::SummaryOnly;
    record.evidence = vec![reference(&origin, EvidencePurpose::Origin)];
    record
}

async fn assert_native_event_requires_records_or_capture_gap(
    kind: EventKind,
    body: &str,
) -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene.open("observed-event", kind, body).await?;
    assert!(event.context_id.is_some());
    assert!(
        scene
            .store
            .checkpoint(Request::Resolve {
                scope: scene.scope.clone(),
                event_id: event.event_id.clone(),
                resolution: Resolution::NoNewContext {
                    reason: "추가 맥락이 없다.".into(),
                },
            })
            .await
            .is_err()
    );
    let status = scene
        .store
        .checkpoint(Request::Status {
            scope: scene.scope.clone(),
        })
        .await?;
    assert_eq!(status.decision, Decision::Block);
    assert_eq!(status.pending_event_ids, vec![event.event_id.clone()]);
    assert!(matches!(
        status
            .events
            .first()
            .ok_or("missing event after rejection")?
            .resolution,
        Resolution::Pending
    ));
    let incomplete = scene
        .store
        .checkpoint(Request::Resolve {
            scope: scene.scope.clone(),
            event_id: event.event_id.clone(),
            resolution: Resolution::CaptureIncomplete {
                reason: "관측한 사건의 의미 레코드를 아직 저장하지 못했다.".into(),
            },
        })
        .await?;
    assert_eq!(incomplete.decision, Decision::CaptureIncomplete);
    assert_eq!(
        incomplete.capture_incomplete_event_ids,
        vec![event.event_id]
    );
    Ok(())
}

#[tokio::test]
async fn native_failure_cannot_resolve_without_context_records() -> TestResult {
    assert_native_event_requires_records_or_capture_gap(
        EventKind::ToolFailure,
        "업로드가 HTTP 429로 실패했다.",
    )
    .await
}

#[tokio::test]
async fn native_verification_cannot_resolve_without_context_records() -> TestResult {
    assert_native_event_requires_records_or_capture_gap(
        EventKind::Verification,
        "429 재시도 검증이 통과했다.",
    )
    .await
}

#[tokio::test]
async fn native_mutation_cannot_resolve_without_context_records() -> TestResult {
    assert_native_event_requires_records_or_capture_gap(
        EventKind::Mutation,
        "429 응답에만 지수 백오프를 추가했다.",
    )
    .await
}

#[tokio::test]
async fn user_prompt_can_explicitly_report_no_new_context() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open("status", EventKind::UserPrompt, "현재 상태를 알려줘.")
        .await?;
    assert!(event.context_id.is_some());
    let reply = scene
        .store
        .checkpoint(Request::Resolve {
            scope: scene.scope.clone(),
            event_id: event.event_id,
            resolution: Resolution::NoNewContext {
                reason: "상태 조회 요청만 있어 새 작업 맥락이 없다.".into(),
            },
        })
        .await?;
    assert_eq!(reply.decision, Decision::Allow);
    assert!(reply.pending_event_ids.is_empty());
    assert!(reply.capture_incomplete_event_ids.is_empty());
    Ok(())
}

#[tokio::test]
async fn native_observation_supports_separate_atomic_constraints_and_exact_receipts() -> TestResult
{
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open(
            "correction",
            EventKind::UserPrompt,
            "토큰 갱신은 수정하지 마. 처리량은 유지해.",
        )
        .await?;
    let context = event.context_id.as_ref().ok_or("missing context")?;
    let origin = event.origin.as_ref().ok_or("missing origin")?;
    let corpus = scene.store.load().await?;
    let original = corpus
        .entries
        .iter()
        .find(|entry| entry.sequence == origin.sequence)
        .ok_or("missing native observation")?;
    let Entity::Record(original) = &original.entity else {
        return Err("origin is not a record".into());
    };
    assert_eq!(original.representation, Representation::Evidence);
    assert_eq!(original.context_id.as_ref(), Some(context));
    let no_refresh = claim(
        &scene.scope,
        &event,
        "no-refresh",
        RecordKind::Constraint,
        "토큰 갱신 로직을 수정하지 않는다.",
    )?;
    let throughput = claim(
        &scene.scope,
        &event,
        "throughput",
        RecordKind::Constraint,
        "평소 처리량을 유지한다.",
    )?;
    let references = vec![
        scene.write(no_refresh).await?,
        scene.write(throughput).await?,
    ];
    assert_eq!(
        scene.resolve(&event, references).await?.decision,
        Decision::Allow
    );
    Ok(())
}

#[tokio::test]
async fn legacy_evidence_and_another_context_finding_cannot_satisfy_a_new_failure_checkpoint()
-> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open(
            "failed",
            EventKind::ToolFailure,
            "업로드가 HTTP 429로 실패했다.",
        )
        .await?;
    for representation in [
        Representation::Legacy,
        Representation::Evidence,
        Representation::Claim,
    ] {
        let mut record = claim(
            &scene.scope,
            &event,
            &format!("finding-{representation:?}"),
            RecordKind::Finding,
            "동시 요청 수가 증가했다.",
        )?;
        record.representation = representation;
        if representation != Representation::Claim {
            record.derived = false;
        } else {
            // Semantic relevance is not inferred from prose. The checkable
            // failure here is that this finding belongs to another context.
            record.context_id = Some("another-investigation".into());
        }
        let saved = scene.write(record).await?;
        assert!(scene.resolve(&event, vec![saved]).await.is_err());
    }
    let mut failed = claim(
        &scene.scope,
        &event,
        "failure",
        RecordKind::Attempt,
        "동시 업로드 16개 실험이 실패했다.",
    )?;
    failed.attempt_outcome = Some(AttemptOutcome::Failed);
    let saved = scene.write(failed).await?;
    assert_eq!(
        scene.resolve(&event, vec![saved]).await?.decision,
        Decision::Allow
    );
    Ok(())
}

#[tokio::test]
async fn native_checkpoint_requires_the_claim_source_to_remain_available() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open(
            "failed",
            EventKind::ToolFailure,
            "업로드가 HTTP 429로 실패했다.",
        )
        .await?;
    let mut failed = claim(
        &scene.scope,
        &event,
        "failure",
        RecordKind::Attempt,
        "동시 업로드 16개 실험이 실패했다.",
    )?;
    failed.attempt_outcome = Some(AttemptOutcome::Failed);
    let saved = scene.write(failed).await?;
    let mut source = ingest::source("journal", &scene.scope.project_id, SourceKind::Journal);
    source.available = false;
    scene.store.append(Entity::Source(source.clone())).await?;
    assert!(scene.resolve(&event, vec![saved.clone()]).await.is_err());
    let status = scene
        .store
        .checkpoint(Request::Status {
            scope: scene.scope.clone(),
        })
        .await?;
    assert_eq!(status.decision, Decision::Block);
    source.available = true;
    scene.store.append(Entity::Source(source.clone())).await?;
    assert_eq!(
        scene.resolve(&event, vec![saved]).await?.decision,
        Decision::Allow
    );
    source.available = false;
    scene.store.append(Entity::Source(source)).await?;
    let unavailable = scene
        .store
        .checkpoint(Request::Status {
            scope: scene.scope.clone(),
        })
        .await?;
    assert_eq!(unavailable.decision, Decision::CaptureIncomplete);
    assert_eq!(
        unavailable.capture_incomplete_event_ids,
        vec![event.event_id]
    );
    Ok(())
}

#[tokio::test]
async fn claim_from_an_unavailable_own_source_is_rejected_without_mutation() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open("request", EventKind::UserPrompt, "처리량을 유지해.")
        .await?;
    let mut source = ingest::source("journal", &scene.scope.project_id, SourceKind::Journal);
    source.available = false;
    scene.store.append(Entity::Source(source)).await?;
    let before = serde_json::to_value(scene.store.load().await?)?;
    let constraint = claim(
        &scene.scope,
        &event,
        "throughput",
        RecordKind::Constraint,
        "평소 처리량을 유지한다.",
    )?;
    assert!(
        scene
            .store
            .append(Entity::Record(constraint))
            .await
            .is_err()
    );
    assert_eq!(before, serde_json::to_value(scene.store.load().await?)?);
    let status = scene
        .store
        .checkpoint(Request::Status {
            scope: scene.scope.clone(),
        })
        .await?;
    assert_eq!(status.decision, Decision::Block);
    assert_eq!(status.pending_event_ids, vec![event.event_id]);
    Ok(())
}

#[tokio::test]
async fn missing_exact_revision_and_other_project_origins_roll_back() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open("request", EventKind::UserPrompt, "처리량을 유지해.")
        .await?;
    let mut missing = claim(
        &scene.scope,
        &event,
        "missing",
        RecordKind::Constraint,
        "처리량을 유지한다.",
    )?;
    missing
        .evidence
        .first_mut()
        .ok_or("missing evidence")?
        .revision = "absent-v2".into();
    let before = serde_json::to_value(scene.store.load().await?)?;
    assert!(scene.store.append(Entity::Record(missing)).await.is_err());
    assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    scene
        .store
        .append(Entity::Source(ingest::source(
            "foreign",
            "other-project",
            SourceKind::Journal,
        )))
        .await?;
    let mut foreign = raw("same-id", "처리량을 유지해.", "foreign");
    foreign.project_id = "other-project".into();
    foreign.source_id = "foreign".into();
    scene.store.append(Entity::Record(foreign.clone())).await?;
    let mut cross = claim(
        &scene.scope,
        &event,
        "cross-project",
        RecordKind::Constraint,
        "처리량을 유지한다.",
    )?;
    cross.evidence = manual_claim(&foreign, "temporary").evidence;
    let before = serde_json::to_value(scene.store.load().await?)?;
    assert!(scene.store.append(Entity::Record(cross)).await.is_err());
    assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    Ok(())
}

#[tokio::test]
async fn another_context_or_unbound_native_event_cannot_complete_the_checkpoint() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let first = scene
        .open("first", EventKind::UserPrompt, "토큰 갱신은 수정하지 마.")
        .await?;
    let second = scene
        .open("second", EventKind::UserPrompt, "처리량은 유지해.")
        .await?;
    let wrong_context = claim(
        &scene.scope,
        &first,
        "wrong-context",
        RecordKind::Constraint,
        "토큰 갱신 로직을 수정하지 않는다.",
    )?;
    let saved = scene.write(wrong_context).await?;
    assert!(scene.resolve(&second, vec![saved]).await.is_err());
    let mut unbound = claim(
        &scene.scope,
        &first,
        "unbound",
        RecordKind::Constraint,
        "토큰 갱신 로직을 수정하지 않는다.",
    )?;
    unbound.context_id = second.context_id.clone();
    let saved = scene.write(unbound).await?;
    assert!(scene.resolve(&second, vec![saved]).await.is_err());
    Ok(())
}

#[tokio::test]
async fn an_earlier_origin_and_current_event_support_are_explicitly_distinct() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let first = scene
        .open(
            "request",
            EventKind::UserPrompt,
            "작업자 수를 4개로 제한해.",
        )
        .await?;
    let requested = claim(
        &scene.scope,
        &first,
        "request-claim",
        RecordKind::Request,
        "작업자 수를 4개로 제한한다.",
    )?;
    let saved = scene.write(requested).await?;
    scene.resolve(&first, vec![saved]).await?;
    let current = scene
        .open(
            "change",
            EventKind::Mutation,
            "업로드 작업자 설정을 수정했다.",
        )
        .await?;
    let mut change = claim(
        &scene.scope,
        &first,
        "change-claim",
        RecordKind::Change,
        "업로드 작업자 수를 4개로 변경했다.",
    )?;
    change.context_id = current.context_id.clone();
    change.evidence.push(reference(
        current
            .origin
            .as_ref()
            .ok_or("missing current observation")?,
        EvidencePurpose::Support,
    ));
    let saved = scene.write(change).await?;
    assert_eq!(
        scene.resolve(&current, vec![saved]).await?.decision,
        Decision::Allow
    );
    Ok(())
}

#[tokio::test]
async fn spans_require_utf8_boundaries_valid_lines_and_consistent_line_windows() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let original = raw("unicode", "가나\n다라", "unicode-context");
    scene.store.append(Entity::Record(original.clone())).await?;
    for (index, span, range) in [
        (0, Some(ByteSpan { start: 1, end: 3 }), None),
        (1, Some(ByteSpan { start: 0, end: 100 }), None),
        (
            2,
            None,
            Some(TextRange {
                start_line: 0,
                end_line: 1,
            }),
        ),
        (
            3,
            None,
            Some(TextRange {
                start_line: 1,
                end_line: 3,
            }),
        ),
        (
            4,
            Some(ByteSpan { start: 7, end: 10 }),
            Some(TextRange {
                start_line: 1,
                end_line: 1,
            }),
        ),
    ] {
        let mut record = manual_claim(&original, &format!("bad-range-{index}"));
        let evidence = record.evidence.first_mut().ok_or("missing evidence")?;
        evidence.span = span;
        evidence.range = range;
        let before = serde_json::to_value(scene.store.load().await?)?;
        assert!(scene.store.append(Entity::Record(record)).await.is_err());
        assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    }
    let mut valid = manual_claim(&original, "valid-span");
    valid.evidence.first_mut().ok_or("missing evidence")?.span =
        Some(ByteSpan { start: 0, end: 6 });
    assert!(scene.store.append(Entity::Record(valid)).await?.durable);
    let blanks = raw("blank-lines", "a\n\nb", "blank-context");
    scene.store.append(Entity::Record(blanks.clone())).await?;
    let mut crossing = manual_claim(&blanks, "crossing-blank-line");
    crossing
        .evidence
        .first_mut()
        .ok_or("missing evidence")?
        .span = Some(ByteSpan { start: 0, end: 3 });
    assert!(scene.store.append(Entity::Record(crossing)).await.is_err());
    Ok(())
}

#[tokio::test]
async fn span_coordinates_are_checked_against_the_masked_retained_body() -> TestResult {
    let secret = "confidential-value-".repeat(8);
    let mut scene = Scenario::new(RedactionPolicy {
        literal_secrets: vec![secret.clone()],
    })
    .await?;
    let original = raw("masked", &format!("{secret}suffix"), "masked-context");
    scene.store.append(Entity::Record(original.clone())).await?;
    let mut stale = manual_claim(&original, "stale-span");
    let evidence = stale.evidence.first_mut().ok_or("missing evidence")?;
    evidence.range = None;
    evidence.span = Some(ByteSpan {
        start: secret.len(),
        end: secret.len() + 6,
    });
    let before = serde_json::to_value(scene.store.load().await?)?;
    assert!(scene.store.append(Entity::Record(stale)).await.is_err());
    assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    let mut corrected = manual_claim(&original, "retained-span");
    let evidence = corrected.evidence.first_mut().ok_or("missing evidence")?;
    evidence.range = None;
    evidence.span = Some(ByteSpan {
        start: "[REDACTED]".len(),
        end: "[REDACTED]suffix".len(),
    });
    assert!(scene.store.append(Entity::Record(corrected)).await?.durable);
    Ok(())
}

#[tokio::test]
async fn masking_cannot_silently_shift_an_in_bounds_batch_span() -> TestResult {
    let secret = "SUPER_PRIVATE_TOKEN";
    let mut scene = Scenario::new(RedactionPolicy {
        literal_secrets: vec![secret.into()],
    })
    .await?;
    let prefix = format!("{secret}abcde");
    let body = format!("{prefix}SUFFIX1234567890abcdefghijk");
    let original = raw("shifted-origin", &body, "shifted-context");
    let mut derived = manual_claim(&original, "shifted-claim");
    let evidence = derived.evidence.first_mut().ok_or("missing evidence")?;
    evidence.range = None;
    evidence.span = Some(ByteSpan {
        start: prefix.len(),
        end: prefix.len() + "SUFFIX".len(),
    });
    let projected = scene.store.sanitize(&Entity::Record(original.clone()))?;
    let Entity::Record(projected) = projected else {
        return Err("projected origin is not a record".into());
    };
    let span = evidence.span.as_ref().ok_or("missing span")?;
    assert!(span.end <= projected.body.len());
    assert_ne!(
        body.get(span.start..span.end),
        projected.body.get(span.start..span.end)
    );
    let before = serde_json::to_value(scene.store.load().await?)?;
    assert!(
        scene
            .store
            .append_all(vec![Entity::Record(derived), Entity::Record(original)])
            .await
            .is_err()
    );
    assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    Ok(())
}

#[tokio::test]
async fn masking_another_line_preserves_a_batch_evidence_window() -> TestResult {
    let secret = "SUPER_PRIVATE_TOKEN";
    let mut scene = Scenario::new(RedactionPolicy {
        literal_secrets: vec![secret.into()],
    })
    .await?;
    let original = raw(
        "stable-origin",
        &format!("{secret}\n처리량을 유지해."),
        "stable-context",
    );
    let mut derived = manual_claim(&original, "stable-claim");
    derived
        .evidence
        .first_mut()
        .ok_or("missing evidence")?
        .range = Some(TextRange {
        start_line: 2,
        end_line: 2,
    });
    let receipts = scene
        .store
        .append_all(vec![Entity::Record(derived), Entity::Record(original)])
        .await?;
    assert_eq!(receipts.len(), 2);
    assert!(receipts.iter().all(|receipt| receipt.durable));
    assert!(!serde_json::to_string(&scene.store.load().await?)?.contains(secret));
    Ok(())
}

#[tokio::test]
async fn forward_evidence_references_are_validated_after_the_entire_batch() -> TestResult {
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let original = raw("batch-origin", "처리량을 유지해.", "batch-context");
    let derived = manual_claim(&original, "batch-claim");
    let receipts = scene
        .store
        .append_all(vec![Entity::Record(derived), Entity::Record(original)])
        .await?;
    assert_eq!(receipts.len(), 2);
    assert!(receipts.iter().all(|receipt| receipt.durable));
    let invalid_origin = raw("rollback-origin", "처리량을 유지해.", "rollback-context");
    let mut invalid = manual_claim(&invalid_origin, "rollback-claim");
    invalid
        .evidence
        .first_mut()
        .ok_or("missing evidence")?
        .revision = "missing-v2".into();
    let before = serde_json::to_value(scene.store.load().await?)?;
    assert!(
        scene
            .store
            .append_all(vec![
                Entity::Record(invalid),
                Entity::Record(invalid_origin)
            ])
            .await
            .is_err()
    );
    assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
    Ok(())
}

#[tokio::test]
async fn partial_origin_requires_disclosure_and_cannot_resolve_a_complete_checkpoint() -> TestResult
{
    let mut scene = Scenario::new(RedactionPolicy::default()).await?;
    let event = scene
        .open(
            "partial",
            EventKind::UserPrompt,
            "토큰 갱신은 수정하지 마.\n[prompt truncated; not complete user message]",
        )
        .await?;
    let complete = claim(
        &scene.scope,
        &event,
        "complete",
        RecordKind::Constraint,
        "토큰 갱신 로직을 수정하지 않는다.",
    )?;
    assert!(scene.store.append(Entity::Record(complete)).await.is_err());
    let mut partial = claim(
        &scene.scope,
        &event,
        "partial-claim",
        RecordKind::Constraint,
        "토큰 갱신 로직을 수정하지 않는다.",
    )?;
    partial.partial = true;
    let saved = scene.write(partial).await?;
    assert!(scene.resolve(&event, vec![saved]).await.is_err());
    let gap = scene
        .store
        .checkpoint(Request::Resolve {
            scope: scene.scope.clone(),
            event_id: event.event_id,
            resolution: Resolution::CaptureIncomplete {
                reason: "사용자 메시지 뒤쪽 원문을 확인하지 못했다.".into(),
            },
        })
        .await?;
    assert_eq!(gap.decision, Decision::CaptureIncomplete);
    Ok(())
}

#[tokio::test]
async fn native_observation_deletion_or_revocation_scrubs_metadata_copies() -> TestResult {
    for revoke_source in [false, true] {
        let mut scene = Scenario::new(RedactionPolicy::default()).await?;
        let private = "내부기밀문구-검증용";
        let event = scene
            .open(
                "private",
                EventKind::UserPrompt,
                &format!("{private}를 참고해."),
            )
            .await?;
        scene
            .store
            .checkpoint(Request::Resolve {
                scope: scene.scope.clone(),
                event_id: event.event_id.clone(),
                resolution: Resolution::CaptureIncomplete {
                    reason: format!("{private} 원문이 추가로 필요하다."),
                },
            })
            .await?;
        let origin = event.origin.as_ref().ok_or("missing private origin")?;
        let corpus = scene.store.load().await?;
        if revoke_source {
            let mut source = corpus
                .entries
                .iter()
                .filter_map(|entry| match &entry.entity {
                    Entity::Source(source) if source.id == origin.source_id => {
                        Some((entry.sequence, source.clone()))
                    }
                    _ => None,
                })
                .max_by_key(|(sequence, _)| *sequence)
                .map(|(_, source)| source)
                .ok_or("missing source")?;
            source.authorized = false;
            scene.store.append(Entity::Source(source)).await?;
        } else {
            let mut original = corpus
                .entries
                .iter()
                .find_map(|entry| match &entry.entity {
                    Entity::Record(record) if entry.sequence == origin.sequence => {
                        Some(record.clone())
                    }
                    _ => None,
                })
                .ok_or("missing original")?;
            original.availability = Availability::Deleted;
            original.revision = "deleted-v1".into();
            scene.store.append(Entity::Record(original)).await?;
        }
        let status = scene
            .store
            .checkpoint(Request::Status {
                scope: scene.scope.clone(),
            })
            .await?;
        assert!(!serde_json::to_string(&status)?.contains(private));
        assert!(!serde_json::to_string(&scene.store.load().await?)?.contains(private));
        assert_eq!(status.decision, Decision::CaptureIncomplete);
    }
    Ok(())
}

#[tokio::test]
async fn native_open_and_claim_capacity_failures_roll_back_observations_and_metadata() -> TestResult
{
    for max_entries in [2, 4] {
        let mut scene = Scenario::new(RedactionPolicy::default()).await?;
        scene
            .store
            .compact(
                Some(Policy {
                    max_entries,
                    max_payload_bytes: 64 * 1024,
                    recent_entries: 0,
                }),
                true,
            )
            .await?;
        let before = serde_json::to_value(scene.store.load().await?)?;
        if max_entries == 2 {
            let result = scene
                .store
                .checkpoint(Request::Open {
                    scope: scene.scope.clone(),
                    event_id: "no-space".into(),
                    kind: EventKind::UserPrompt,
                    detail: "처리량을 유지해.".into(),
                    commit_binding: None,
                })
                .await;
            assert!(matches!(result, Err(StoreError::Capacity { .. })));
            assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
            let status = scene
                .store
                .checkpoint(Request::Status {
                    scope: scene.scope.clone(),
                })
                .await?;
            assert!(status.events.is_empty());
        } else {
            let event = scene
                .open("has-origin", EventKind::UserPrompt, "처리량을 유지해.")
                .await?;
            let before = serde_json::to_value(scene.store.load().await?)?;
            let record = claim(
                &scene.scope,
                &event,
                "no-space-claim",
                RecordKind::Constraint,
                "처리량을 유지한다.",
            )?;
            assert!(matches!(
                scene.store.append(Entity::Record(record)).await,
                Err(StoreError::Capacity { .. })
            ));
            assert_eq!(serde_json::to_value(scene.store.load().await?)?, before);
            let status = scene
                .store
                .checkpoint(Request::Status {
                    scope: scene.scope.clone(),
                })
                .await?;
            assert_eq!(status.decision, Decision::Block);
            assert_eq!(status.pending_event_ids, vec![event.event_id]);
        }
        assert_eq!(
            scene.store.compact(None, false).await?.policy.max_entries,
            max_entries
        );
    }
    Ok(())
}
