use memento::{
    Error as StoreError, Store, compaction::Policy, ingest, model::*, query,
    security::RedactionPolicy,
};
use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};

type TestResult = Result<(), Box<dyn Error>>;

fn record(id: &str, kind: RecordKind, body: &str) -> Record {
    Record::new(id, "p", "journal", kind, body)
}

async fn opened(path: &std::path::Path) -> Result<Store, Box<dyn Error>> {
    let mut store = Store::open(path, RedactionPolicy::default()).await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            "p",
            SourceKind::Journal,
        )))
        .await?;
    Ok(store)
}

fn policy(entries: usize, bytes: usize) -> Policy {
    Policy {
        max_entries: entries,
        max_payload_bytes: bytes,
        recent_entries: 2,
    }
}

#[tokio::test]
async fn repeated_writes_are_bounded_and_policy_survives_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("context.sqlite");
    let mut store = opened(&path).await?;
    store.compact(Some(policy(12, 12_000)), true).await?;
    let mut collections = 0;
    for i in 0..75 {
        let receipt = store
            .append(Entity::Record(record(
                &format!("log-{i}"),
                RecordKind::ToolResult,
                &"x".repeat(700),
            )))
            .await?;
        collections += usize::from(receipt.compaction.is_some());
        let report = store.compact(None, false).await?;
        assert!(report.before.entries <= 12);
        assert!(report.before.payload_bytes <= 12_000);
        assert!(receipt.durable);
    }
    assert!(collections > 2);
    let corpus = store.load().await?;
    assert!(corpus.compaction.generation > 2);
    assert!(corpus.compaction.removed_entries > 50);
    drop(store);
    let mut store = Store::open(&path, RedactionPolicy::default()).await?;
    assert_eq!(store.compact(None, false).await?.policy.max_entries, 12);
    let latest = store
        .append(Entity::Record(record(
            "after-reopen",
            RecordKind::ToolResult,
            "last output",
        )))
        .await?;
    assert!(latest.sequence > corpus.entries.iter().map(|e| e.sequence).max().unwrap_or(0));
    assert!(store.compact(None, false).await?.before.entries <= 12);
    Ok(())
}

#[tokio::test]
async fn protected_capacity_failure_rolls_back_incoming_record_and_policy() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = opened(&dir.path().join("context.sqlite")).await?;
    store.compact(Some(policy(5, 32_000)), true).await?;
    for i in 0..3 {
        store
            .append(Entity::Record(record(
                &format!("D{i}"),
                RecordKind::Decision,
                "accepted reason",
            )))
            .await?;
    }
    let before = serde_json::to_value(store.load().await?)?;
    assert!(matches!(
        store
            .append(Entity::Record(record(
                "D4",
                RecordKind::Decision,
                "must not silently discard another reason"
            )))
            .await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(before, serde_json::to_value(store.load().await?)?);
    let smaller = policy(3, 32_000);
    assert!(!store.compact(Some(smaller.clone()), false).await?.fits);
    assert!(matches!(
        store.compact(Some(smaller), true).await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(store.compact(None, false).await?.policy.max_entries, 5);
    Ok(())
}

#[tokio::test]
async fn compaction_preserves_exact_evidence_and_latest_revision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = opened(&dir.path().join("context.sqlite")).await?;
    let original = record("output", RecordKind::ToolResult, "observed 429, not 401");
    store.append(Entity::Record(original.clone())).await?;
    store
        .append(Entity::Record(record(
            "output",
            RecordKind::ToolResult,
            "later corrected output",
        )))
        .await?;
    let mut decision = record(
        "D",
        RecordKind::Decision,
        "limit concurrency because of the original response",
    );
    decision.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some(original.id.clone()),
        revision: original.revision.clone(),
        locator: "record:output".into(),
        availability: Availability::Available,
        range: None,
    });
    store.append(Entity::Record(decision)).await?;
    for i in 0..8 {
        store
            .append(Entity::Record(record(
                &format!("noise-{i}"),
                RecordKind::ToolResult,
                "unrelated verbose output",
            )))
            .await?;
    }
    let before = store.load().await?;
    let preview = store.compact(Some(policy(30, 32_000)), false).await?;
    assert!(preview.removed_entries >= 6);
    assert_eq!(
        serde_json::to_value(&before)?,
        serde_json::to_value(store.load().await?)?
    );
    let applied = store.compact(Some(policy(30, 32_000)), true).await?;
    assert_eq!(applied.removed_entries, preview.removed_entries);
    let after = store.load().await?;
    let revisions: Vec<_> = after
        .entries
        .iter()
        .filter_map(|e| match &e.entity {
            Entity::Record(r) if r.id == "output" => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(revisions.len(), 2);
    let mut q = Query::new(Operation::Read, "p");
    q.target = Some(Target::Artifact {
        record_id: original.id,
        revision: original.revision,
        range: None,
    });
    let response = query::execute(&after, &q)?;
    assert!(serde_json::to_string(&response)?.contains("observed 429, not 401"));
    Ok(())
}

#[tokio::test]
async fn deletion_revocation_and_replay_contracts_survive_compaction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = opened(&dir.path().join("context.sqlite")).await?;
    let original = record("private", RecordKind::Finding, "private original");
    let mut derived = record("derived", RecordKind::Decision, "private derivative");
    derived.derived = true;
    derived.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some(original.id.clone()),
        revision: original.revision.clone(),
        locator: "record:private".into(),
        availability: Availability::Available,
        range: None,
    });
    store
        .append_all([Entity::Record(original.clone()), Entity::Record(derived)])
        .await?;
    store
        .compact(
            Some(Policy {
                recent_entries: 0,
                ..policy(20, 32_000)
            }),
            true,
        )
        .await?;
    let mut deleted = original.clone();
    deleted.availability = Availability::Deleted;
    store.append(Entity::Record(deleted)).await?;
    store.compact(None, true).await?;
    assert!(store.append(Entity::Record(original)).await.is_err());
    let raw = serde_json::to_string(&store.load().await?)?;
    assert!(!raw.contains("private original"));
    assert!(!raw.contains("private derivative"));
    let mut source = ingest::source("journal", "p", SourceKind::Journal);
    source.authorized = false;
    store.append(Entity::Source(source)).await?;
    store.compact(None, true).await?;
    assert!(
        store
            .append(Entity::Record(record("new", RecordKind::Finding, "denied")))
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn oversized_single_record_cannot_bypass_byte_cap() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = opened(&dir.path().join("context.sqlite")).await?;
    store.compact(Some(policy(20, 4096)), true).await?;
    let before = serde_json::to_value(store.load().await?)?;
    let result = store
        .append(Entity::Record(record(
            "huge",
            RecordKind::ToolResult,
            &"x".repeat(8192),
        )))
        .await;
    assert!(matches!(result, Err(StoreError::Capacity { .. })));
    assert_eq!(before, serde_json::to_value(store.load().await?)?);
    Ok(())
}

#[tokio::test]
async fn batch_admission_is_atomic_and_cannot_collect_its_own_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = opened(&dir.path().join("context.sqlite")).await?;
    store.compact(Some(policy(5, 32_000)), true).await?;
    store
        .append(Entity::Record(record(
            "old",
            RecordKind::ToolResult,
            "replaceable log",
        )))
        .await?;
    let before = serde_json::to_value(store.load().await?)?;
    let too_many = (0..4).map(|i| {
        Entity::Record(record(
            &format!("batch-{i}"),
            RecordKind::ToolResult,
            "new captured output",
        ))
    });
    assert!(matches!(
        store.append_all(too_many).await,
        Err(StoreError::Capacity { .. })
    ));
    assert_eq!(before, serde_json::to_value(store.load().await?)?);
    let output = record(
        "output",
        RecordKind::ToolResult,
        "captured before its decision",
    );
    let mut decision = record(
        "D",
        RecordKind::Decision,
        "decision cites the same batch output",
    );
    decision.evidence.push(Evidence {
        source_id: "journal".into(),
        record_id: Some(output.id.clone()),
        revision: output.revision.clone(),
        locator: "record:output".into(),
        availability: Availability::Available,
        range: None,
    });
    store
        .append_all([Entity::Record(output), Entity::Record(decision)])
        .await?;
    store
        .compact(
            Some(Policy {
                recent_entries: 0,
                ..policy(5, 32_000)
            }),
            true,
        )
        .await?;
    let corpus = store.load().await?;
    assert!(corpus.entries.iter().any(|e| e.entity.id() == "output"));
    assert!(corpus.entries.iter().any(|e| e.entity.id() == "D"));
    Ok(())
}

fn cli(args: &[&str], input: Option<&str>) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_memento"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or("missing stdin")?
            .write_all(input.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(String::from_utf8(output.stderr)?.into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn cli_previews_before_apply_and_explains_history_loss() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("context.sqlite");
    let file = path.to_str().ok_or("non UTF-8 test path")?;
    cli(&["init", "--store", file, "--project", "p"], None)?;
    for i in 0..5 {
        cli(&["note", "--store", file, "--project", "p"], Some(&serde_json::json!({"id": format!("log-{i}"), "kind":"tool_result", "body":"transient output"}).to_string()))?;
    }
    let policy_path = dir.path().join("policy.json");
    std::fs::write(&policy_path, serde_json::to_vec(&policy(10, 32_000))?)?;
    let policy_file = policy_path.to_str().ok_or("non UTF-8 policy path")?;
    let preview = cli(
        &[
            "compact",
            "--store",
            file,
            "--compaction-policy",
            policy_file,
        ],
        None,
    )?;
    assert_eq!(preview.get("applied"), Some(&serde_json::json!(false)));
    let applied = cli(
        &[
            "compact",
            "--store",
            file,
            "--compaction-policy",
            policy_file,
            "--apply",
            "true",
        ],
        None,
    )?;
    assert_eq!(applied.get("applied"), Some(&serde_json::json!(true)));
    let response = cli(
        &["query", "--store", file],
        Some(&serde_json::json!({"operation":"search", "scope":{"project_id":"p"}}).to_string()),
    )?;
    assert!(
        response
            .get("omitted")
            .and_then(|v| v.as_array())
            .is_some_and(|rows| rows
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s.contains("history_compacted"))))
    );
    Ok(())
}
