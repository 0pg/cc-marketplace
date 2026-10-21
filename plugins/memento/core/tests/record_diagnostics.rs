use std::{error::Error, process::Command};

use memento::{
    Error as StoreError, Store, ingest,
    model::*,
    record_input::{self, Stage},
    security::RedactionPolicy,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn raw(id: &str) -> Record {
    let mut record = Record::new(id, "p", "journal", RecordKind::Feedback, "retained source");
    record.representation = Representation::Evidence;
    record.context_id = Some("diagnostic-context".into());
    record
}

fn claim(original: &Record) -> Record {
    let mut record = Record::new(
        "claim",
        "p",
        "journal",
        RecordKind::Finding,
        "retained fact",
    );
    record.representation = Representation::Claim;
    record.context_id = original.context_id.clone();
    record.derived = true;
    record.fidelity = Fidelity::SummaryOnly;
    record.evidence = vec![Evidence {
        source_id: original.source_id.clone(),
        record_id: Some(original.id.clone()),
        revision: original.revision.clone(),
        locator: format!("record:{}", original.id),
        availability: Availability::Available,
        range: Some(TextRange {
            start_line: 1,
            end_line: 1,
        }),
        purpose: EvidencePurpose::Origin,
        span: None,
    }];
    record
}

fn diagnostic(error: StoreError) -> Result<record_input::Diagnostic, Box<dyn Error>> {
    match error {
        StoreError::RecordInput(diagnostic) => Ok(*diagnostic),
        other => Err(format!("expected record diagnostic, got {other}").into()),
    }
}

#[test]
fn syntax_errors_do_not_invent_record_identity() -> TestResult {
    let error = record_input::parse_records("[\n")
        .err()
        .ok_or("expected syntax failure")?;
    let error = diagnostic(error)?;
    assert_eq!(error.code, "invalid_record_json");
    assert_eq!(error.stage, Stage::Syntax);
    assert_eq!(error.record_id, None);
    assert_eq!(error.record_index, None);
    assert!(error.line.is_some_and(|line| line > 0));
    assert!(error.hint.contains("syntax"));
    Ok(())
}

#[test]
fn batch_schema_failure_identifies_record_and_missing_title() -> TestResult {
    let first = serde_json::to_value(Entity::Record(raw("first")))?;
    let mut second = serde_json::to_value(Entity::Record(raw("second")))?;
    second
        .pointer_mut("/data")
        .and_then(Value::as_object_mut)
        .ok_or("missing data object")?
        .remove("title");
    let error = record_input::parse_records_value(json!([first, second]))
        .err()
        .ok_or("expected schema failure")?;
    let error = diagnostic(error)?;
    assert_eq!(error.code, "invalid_record_schema");
    assert_eq!(error.stage, Stage::Schema);
    assert_eq!(error.record_index, Some(1));
    assert_eq!(error.record_id.as_deref(), Some("second"));
    assert_eq!(error.field_path, "/1/data/title");
    assert!(error.hint.contains("\"title\":"));
    Ok(())
}

#[test]
fn missing_id_and_nested_schema_paths_are_not_guessed() -> TestResult {
    let mut value = serde_json::to_value(Entity::Record(raw("missing-id")))?;
    value
        .pointer_mut("/data")
        .and_then(Value::as_object_mut)
        .ok_or("missing data object")?
        .remove("id");
    let error = diagnostic(
        record_input::parse_records_value(value)
            .err()
            .ok_or("expected missing id")?,
    )?;
    assert_eq!(error.record_id, None);
    assert_eq!(error.field_path, "/data/id");

    let original = raw("origin");
    let mut value = serde_json::to_value(Entity::Record(claim(&original)))?;
    value
        .pointer_mut("/data/evidence/0/range")
        .and_then(Value::as_object_mut)
        .ok_or("missing range object")?
        .remove("start_line");
    let error = diagnostic(
        record_input::parse_records_value(value)
            .err()
            .ok_or("expected nested schema failure")?,
    )?;
    assert_eq!(error.field_path, "/data");
    assert!(error.message.contains("start_line"));
    Ok(())
}

#[tokio::test]
async fn truncated_origin_explains_partial_and_rolls_back_the_batch() -> TestResult {
    for source_truncated in [false, true] {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(
            &directory.path().join("context.sqlite"),
            RedactionPolicy::default(),
        )
        .await?;
        store
            .append(Entity::Source(ingest::source(
                "journal",
                "p",
                SourceKind::Journal,
            )))
            .await?;
        let mut original = raw("origin");
        if source_truncated {
            original.fidelity = Fidelity::SourceTruncated;
        } else {
            original.partial = true;
        }
        let mut claim = claim(&original);
        let error = store
            .append_all(vec![
                Entity::Record(original.clone()),
                Entity::Record(claim.clone()),
            ])
            .await
            .err()
            .ok_or("expected truncated origin rejection")?;
        let error = diagnostic(error)?;
        assert_eq!(error.code, "invalid_record_claim");
        assert_eq!(error.stage, Stage::Validation);
        assert_eq!(error.record_id.as_deref(), Some("claim"));
        assert_eq!(error.field_path, "/data/partial");
        assert_eq!(error.evidence_index, Some(0));
        assert_eq!(error.evidence_record_id.as_deref(), Some("origin"));
        assert_eq!(error.evidence_source_id.as_deref(), Some("journal"));
        assert!(error.hint.contains("partial=true"));
        assert!(error.hint.contains("source_truncated"));
        assert!(error.hint.contains("complete original"));
        assert_eq!(store.load().await?.entries.len(), 1);

        claim.partial = true;
        let receipts = store
            .append_all(vec![Entity::Record(original), Entity::Record(claim)])
            .await?;
        assert_eq!(receipts.len(), 2);
        assert!(receipts.iter().all(|receipt| receipt.durable));
    }
    Ok(())
}

#[tokio::test]
async fn invalid_evidence_range_identifies_the_exact_reference_field() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "p",
            SourceKind::Journal,
        )))
        .await?;
    let original = raw("origin");
    let mut claim = claim(&original);
    claim.evidence.first_mut().ok_or("missing evidence")?.range = Some(TextRange {
        start_line: 1,
        end_line: 2,
    });
    let error = store
        .append_all(vec![Entity::Record(original), Entity::Record(claim)])
        .await
        .err()
        .ok_or("expected range failure")?;
    let error = diagnostic(error)?;
    assert_eq!(error.field_path, "/data/evidence/0/range");
    assert_eq!(error.evidence_index, Some(0));
    assert_eq!(error.record_id.as_deref(), Some("claim"));
    assert_eq!(store.load().await?.entries.len(), 1);
    Ok(())
}

#[test]
fn cli_emits_structured_schema_diagnostics() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input_path = directory.path().join("input.json");
    let store_path = directory.path().join("context.sqlite");
    let mut value = serde_json::to_value(Entity::Record(raw("cli-record")))?;
    value
        .pointer_mut("/data")
        .and_then(Value::as_object_mut)
        .ok_or("missing data object")?
        .remove("title");
    std::fs::write(&input_path, serde_json::to_vec(&json!([value]))?)?;
    let output = Command::new(env!("CARGO_BIN_EXE_memento"))
        .arg("record")
        .arg("--store")
        .arg(&store_path)
        .arg("--input")
        .arg(&input_path)
        .output()?;
    assert!(!output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        response.pointer("/error/code"),
        Some(&json!("invalid_record_schema"))
    );
    assert_eq!(response.pointer("/error/record_index"), Some(&json!(0)));
    assert_eq!(
        response.pointer("/error/record_id"),
        Some(&json!("cli-record"))
    );
    assert_eq!(
        response.pointer("/error/field_path"),
        Some(&json!("/0/data/title"))
    );
    Ok(())
}

#[test]
fn cli_syntax_errors_keep_their_stage_and_location() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input_path = directory.path().join("invalid.json");
    let store_path = directory.path().join("context.sqlite");
    std::fs::write(&input_path, "[\\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_memento"))
        .arg("record")
        .arg("--store")
        .arg(&store_path)
        .arg("--input")
        .arg(&input_path)
        .output()?;
    assert!(!output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        response.pointer("/error/code"),
        Some(&json!("invalid_record_json"))
    );
    assert_eq!(response.pointer("/error/stage"), Some(&json!("syntax")));
    assert!(response.pointer("/error/line").is_some());
    assert!(response.pointer("/error/record_id").is_none());
    Ok(())
}
