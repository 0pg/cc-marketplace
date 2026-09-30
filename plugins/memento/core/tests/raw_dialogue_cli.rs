//! RAW-01: the user-visible upload conversation, through actual CLI processes.
//! The fixture is synthetic; its annotations are explicit capture input.
use std::{
    collections::BTreeMap,
    error::Error,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use memento::model::{Availability, Evidence};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    synthetic: bool,
    question: String,
    capture_contract: String,
    notes: Vec<Value>,
    retained: Vec<String>,
    removed: Vec<String>,
}

fn cli(store: &Path, arguments: &[&str], input: Option<&Value>) -> Result<Value, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memento"));
    command.args(arguments).arg("--store").arg(store);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    if let Some(input) = input {
        let mut stdin = child.stdin.take().ok_or("missing CLI stdin")?;
        stdin.write_all(&serde_json::to_vec(input)?)?;
    }
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "CLI {arguments:?} exited {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn read(store: &Path, id: &str) -> Result<Value, Box<dyn Error>> {
    cli(
        store,
        &["query"],
        Some(&json!({
            "operation":"read", "scope":{"project_id":"raw-cli","work_ids":["UPLOAD"]},
            "target":{"kind":"artifact","record_id":id,"revision":"v1"},
            "budget_bytes":16384
        })),
    )
}

#[test]
fn raw01_upload_dialogue_keeps_mistake_correction_failed_alternative_and_verification_limits()
-> Result<(), Box<dyn Error>> {
    let fixture: Scenario =
        serde_json::from_str(include_str!("fixtures/dialogues/upload-429.json"))?;
    assert_eq!(fixture.id, "RAW-01");
    assert!(
        fixture.synthetic && !fixture.question.is_empty() && !fixture.capture_contract.is_empty()
    );
    let directory = tempfile::tempdir()?;
    let store = directory.path().join("context.sqlite");
    cli(
        &store,
        &[
            "init",
            "--project",
            "raw-cli",
            "--work",
            "UPLOAD",
            "--session",
            "S1",
            "--title",
            "업로드 429 원인과 재시도 정책",
            "--goal",
            "실패한 대안과 사용자 정정, 검증 범위를 다음 작업에 전달한다",
        ],
        None,
    )?;
    let mut captured = BTreeMap::new();
    for note in &fixture.notes {
        let receipt = cli(
            &store,
            &[
                "note",
                "--project",
                "raw-cli",
                "--work",
                "UPLOAD",
                "--session",
                "S1",
            ],
            Some(note),
        )?;
        assert_eq!(receipt.pointer("/receipt/durable"), Some(&json!(true)));
        let replay = cli(
            &store,
            &[
                "note",
                "--project",
                "raw-cli",
                "--work",
                "UPLOAD",
                "--session",
                "S1",
            ],
            Some(note),
        )?;
        assert_eq!(replay.pointer("/receipt/duplicate"), Some(&json!(true)));
        assert_eq!(
            receipt.pointer("/receipt/sequence"),
            replay.pointer("/receipt/sequence")
        );
        let id = note
            .get("id")
            .and_then(Value::as_str)
            .ok_or("missing note ID")?;
        let original = read(&store, id)?;
        let data = original
            .pointer("/items/0/entity/data")
            .ok_or("missing captured original")?;
        for (field, expected) in note.as_object().ok_or("note is not an object")? {
            if field != "evidence" {
                assert_eq!(data.get(field), Some(expected), "{id}: {field}");
            }
        }
        let mut expected_evidence: Vec<Evidence> =
            serde_json::from_value(note.get("evidence").ok_or("missing evidence")?.clone())?;
        if expected_evidence.is_empty() {
            // The note interface adds an exact self locator to original messages.
            expected_evidence.push(Evidence {
                source_id: "journal".into(),
                record_id: Some(id.into()),
                revision: "v1".into(),
                locator: format!("record:{id}"),
                availability: Availability::Available,
                range: None,
            });
        }
        let actual_evidence: Vec<Evidence> = serde_json::from_value(
            data.get("evidence")
                .ok_or("missing saved evidence")?
                .clone(),
        )?;
        assert_eq!(actual_evidence, expected_evidence, "{id}: capture evidence");
        captured.insert(id.to_owned(), data.clone());
    }
    let policy = directory.path().join("retention.json");
    std::fs::write(
        &policy,
        serde_json::to_vec(
            &json!({"max_entries":10000,"max_payload_bytes":67108864,"recent_entries":0}),
        )?,
    )?;
    let policy = policy.to_str().ok_or("non-UTF8 fixture path")?;
    let args = ["compact", "--compaction-policy", policy];
    let preview = cli(&store, &args, None)?;
    assert_eq!(preview, cli(&store, &args, None)?);
    assert_eq!(preview.pointer("/report/removed_entries"), Some(&json!(3)));
    let applied = cli(
        &store,
        &["compact", "--compaction-policy", policy, "--apply", "true"],
        None,
    )?;
    assert_eq!(preview.get("report"), applied.get("report"));
    for id in &fixture.retained {
        let original = captured.get(id).ok_or("unknown expected note")?;
        let response = read(&store, id)?;
        let retained = response
            .pointer("/items/0/entity/data")
            .ok_or("missing original")?;
        assert_eq!(retained, original, "{id}: captured record changed");
    }
    for id in &fixture.removed {
        let response = read(&store, id)?;
        assert_eq!(response.get("items"), Some(&json!([])));
        assert_eq!(response.get("status"), Some(&json!("partial")));
        assert!(
            response
                .get("omitted")
                .and_then(Value::as_array)
                .ok_or("missing omissions")?
                .iter()
                .any(|v| v
                    .as_str()
                    .is_some_and(|s| s.starts_with("history_compacted:")))
        );
    }
    let report = cli(&store, &args, None)?;
    assert_eq!(report.pointer("/report/removed_entries"), Some(&json!(0)));
    assert_eq!(
        report.pointer("/report/before"),
        applied.pointer("/report/after")
    );
    Ok(())
}
