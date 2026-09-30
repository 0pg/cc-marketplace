use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Mutex;
use work_context::model::{Availability, Record, RecordKind};
use work_context::security::RedactionPolicy;
use work_context::semantic::{
    self, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, SemanticConfig, SemanticError,
};

/// Mechanical test provider; real multilingual model quality is measured separately.
struct CaptureProvider {
    requests: Mutex<Vec<EmbeddingRequest>>,
}
impl CaptureProvider {
    fn new() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
        }
    }
}
impl EmbeddingProvider for CaptureProvider {
    fn embed(
        &self,
        request: &EmbeddingRequest,
        _timeout_ms: u64,
    ) -> Result<EmbeddingResponse, SemanticError> {
        self.requests
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
                    if text.contains("TARGET") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect(),
            metrics: BTreeMap::new(),
        })
    }
}
fn config() -> SemanticConfig {
    SemanticConfig {
        model_id: "test-mechanical".into(),
        model_revision: "revision-1".into(),
        min_score: 0.5,
        ..Default::default()
    }
}
fn record(id: &str, text: &str) -> Record {
    Record::new(id, "p", "s", RecordKind::ToolResult, text)
}

#[test]
fn projection_tracks_title_command_body_and_cannot_replay_prepared_after_correction()
-> Result<(), Box<dyn Error>> {
    let provider = CaptureProvider::new();
    let policy = RedactionPolicy::default();
    let records = vec![record("r", "TARGET result")];
    let prepared =
        semantic::prepare_with_provider(&records, "question", &config(), &policy, &provider)?;
    prepared.validate(&records, "question", &policy)?;
    let mut edited = records.clone();
    let first = edited.first_mut().ok_or("record absent")?;
    first.title = "changed without changing original source revision".into();
    assert_ne!(
        semantic::projection_revision(records.first().ok_or("record absent")?)?,
        semantic::projection_revision(first)?
    );
    assert!(matches!(
        prepared.validate(&edited, "question", &policy),
        Err(SemanticError::Stale)
    ));
    assert!(matches!(
        prepared.validate(&records, "another question", &policy),
        Err(SemanticError::Stale)
    ));
    let mut tampered = prepared.clone();
    tampered
        .candidates
        .first_mut()
        .ok_or("candidate absent")?
        .score = 0.6;
    assert!(matches!(
        tampered.validate(&records, "question", &policy),
        Err(SemanticError::Stale)
    ));
    Ok(())
}

#[test]
fn query_and_every_projection_field_are_masked_before_embedding() -> Result<(), Box<dyn Error>> {
    let provider = CaptureProvider::new();
    let policy = RedactionPolicy {
        literal_secrets: vec!["private-example".into()],
    };
    let mut r = record("r", "TARGET private-example password=secret-password");
    r.title = "private-example".into();
    semantic::prepare_with_provider(
        &[r],
        "private-example Bearer abcsecret",
        &config(),
        &policy,
        &provider,
    )?;
    let requests = provider.requests.lock().map_err(|_| "poisoned")?;
    let encoded = serde_json::to_string(&*requests)?;
    assert!(!encoded.contains("private-example"));
    assert!(!encoded.contains("abcsecret"));
    assert!(!encoded.contains("secret-password"));
    assert!(encoded.contains("[REDACTED]"));
    Ok(())
}

#[test]
fn long_twenty_mib_output_keeps_middle_range_and_deduplicates_inputs() -> Result<(), Box<dyn Error>>
{
    let provider = CaptureProvider::new();
    let repeated = "ordinary output line\n".repeat(524_288);
    let body = format!("{repeated}TARGET: database connection acquisition delayed\n{repeated}");
    assert!(body.len() > 20 * 1024 * 1024);
    let records = vec![record("long", &body)];
    let prepared = semantic::prepare_with_provider(
        &records,
        "why slow",
        &config(),
        &RedactionPolicy::default(),
        &provider,
    )?;
    assert!(prepared.omitted.is_empty(), "{:?}", prepared.omitted);
    assert!(prepared.metrics.unique_chunks < 1000);
    assert!(prepared.metrics.chunks > 50_000);
    let candidate = prepared
        .candidates
        .first()
        .ok_or("middle evidence missing")?;
    assert!(candidate.chunks.len() <= 3);
    let needle_line = 524_289;
    assert!(
        candidate
            .chunks
            .iter()
            .any(|c| c.range.start_line <= needle_line && c.range.end_line >= needle_line)
    );
    for chunk in &candidate.chunks {
        assert!(
            body.get(chunk.byte_start..chunk.byte_end)
                .is_some_and(|text| text.contains("TARGET"))
        );
    }
    Ok(())
}

#[test]
fn unavailable_records_never_reach_model_and_revocation_invalidates_prepared()
-> Result<(), Box<dyn Error>> {
    let provider = CaptureProvider::new();
    let policy = RedactionPolicy::default();
    let records = vec![record("r", "TARGET")];
    let prepared = semantic::prepare_with_provider(&records, "why", &config(), &policy, &provider)?;
    let mut missing = records.clone();
    missing.first_mut().ok_or("record")?.availability = Availability::Deleted;
    let after = semantic::prepare_with_provider(&missing, "why", &config(), &policy, &provider)?;
    assert!(after.candidates.is_empty());
    assert!(matches!(
        prepared.validate(&missing, "why", &policy),
        Err(SemanticError::Stale)
    ));
    assert!(matches!(
        prepared.validate(&[], "why", &policy),
        Err(SemanticError::Stale)
    ));
    assert_eq!(provider.requests.lock().map_err(|_| "poisoned")?.len(), 1);
    Ok(())
}

#[test]
fn processing_limits_report_uninspected_ranges_and_model_failures_do_not_become_no_matches()
-> Result<(), Box<dyn Error>> {
    let provider = CaptureProvider::new();
    let records = vec![record("r", &"ordinary text ".repeat(200))];
    let bounded = SemanticConfig {
        max_chunks: 2,
        ..config()
    };
    let prepared = semantic::prepare_with_provider(
        &records,
        "why",
        &bounded,
        &RedactionPolicy::default(),
        &provider,
    )?;
    assert!(!prepared.omitted.is_empty());
    assert_eq!(prepared.metrics.chunks, 2);
    assert!(matches!(
        semantic::prepare(&records, "why", &config(), &RedactionPolicy::default()),
        Err(SemanticError::Unavailable(_))
    ));
    let overlong = SemanticConfig {
        max_query_bytes: 3,
        ..config()
    };
    assert!(matches!(
        semantic::prepare_with_provider(
            &records,
            "long query",
            &overlong,
            &RedactionPolicy::default(),
            &provider
        ),
        Err(SemanticError::BudgetExceeded(_))
    ));
    Ok(())
}

#[test]
fn nonfinite_or_wrong_model_vectors_are_explicit_errors() -> Result<(), Box<dyn Error>> {
    struct Wrong;
    impl EmbeddingProvider for Wrong {
        fn embed(
            &self,
            request: &EmbeddingRequest,
            _: u64,
        ) -> Result<EmbeddingResponse, SemanticError> {
            Ok(EmbeddingResponse {
                protocol: 1,
                model_id: "other-model".into(),
                model_revision: request.model_revision.clone(),
                query: vec![f32::NAN],
                documents: vec![vec![1.0]; request.documents.len()],
                metrics: BTreeMap::new(),
            })
        }
    }
    assert!(matches!(
        semantic::prepare_with_provider(
            &[record("r", "text")],
            "why",
            &config(),
            &RedactionPolicy::default(),
            &Wrong
        ),
        Err(SemanticError::InvalidResponse(_))
    ));
    Ok(())
}

#[test]
fn local_executable_is_bounded_and_failed_backend_is_not_empty_success()
-> Result<(), Box<dyn Error>> {
    let records = vec![record("r", "TARGET")];
    let policy = RedactionPolicy::default();
    let failed = SemanticConfig {
        command: vec!["/bin/sh".into(), "-c".into(), "exit 7".into()],
        ..config()
    };
    assert!(matches!(
        semantic::prepare(&records, "why", &failed, &policy),
        Err(SemanticError::Unavailable(_))
    ));
    let timeout = SemanticConfig {
        command: vec!["/bin/sh".into(), "-c".into(), "exec sleep 5".into()],
        timeout_ms: 50,
        ..config()
    };
    let started = std::time::Instant::now();
    assert!(matches!(
        semantic::prepare(&records, "why", &timeout, &policy),
        Err(SemanticError::BudgetExceeded(_))
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    Ok(())
}

#[test]
fn command_and_native_tool_input_corrections_change_full_projection() -> Result<(), Box<dyn Error>>
{
    let mut record = record("r", "unchanged body");
    record.execution = Some(serde_json::from_value(
        serde_json::json!({"id":"exec","command":"old command","tool_name":"shell","tool_input":"old input","last_observed_state":"completed","liveness":"stopped"}),
    )?);
    let before = semantic::projection_revision(&record)?;
    record.execution.as_mut().ok_or("execution")?.command = "new command".into();
    let command = semantic::projection_revision(&record)?;
    assert_ne!(before, command);
    record.execution.as_mut().ok_or("execution")?.tool_input = Some("new input".into());
    let input = semantic::projection_revision(&record)?;
    assert_ne!(command, input);
    record.execution.as_mut().ok_or("execution")?.tool_name = Some("different tool".into());
    assert_ne!(input, semantic::projection_revision(&record)?);
    Ok(())
}
