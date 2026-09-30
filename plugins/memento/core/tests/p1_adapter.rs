use std::{error::Error, fs};

use work_context::{
    Store,
    adapters::ImportFormat,
    ingest,
    model::*,
    query::{ResponseStatus, execute},
    security::RedactionPolicy,
};

type TestResult = Result<(), Box<dyn Error>>;

#[path = "support/evaluation.rs"]
mod evaluation;

fn record<'a>(entities: &'a [Entity], id: &str) -> Result<&'a Record, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Record(record) if record.id == id => Some(record),
            _ => None,
        })
        .ok_or_else(|| format!("missing record {id}").into())
}

fn source<'a>(entities: &'a [Entity], id: &str) -> Result<&'a Source, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Source(source) if source.id == id => Some(source),
            _ => None,
        })
        .ok_or_else(|| format!("missing source {id}").into())
}

#[tokio::test]
async fn partial_and_delta_inputs_preserve_absent_history_across_restart() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("selected.jsonl");
    let database = directory.path().join("context.sqlite");
    fs::write(
        &input,
        concat!(
            "{\"id\":\"D1\",\"kind\":\"decision\",\"text\":\"Keep retries bounded at five\"}\n",
            "{\"id\":\"D2\",\"kind\":\"decision\",\"text\":\"Avoid fixed delay on successful requests\"}\n"
        ),
    )?;
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    ingest::import(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    let mut evaluation_query = Query::new(Operation::Brief, "p");
    evaluation_query.purpose = Some(BriefPurpose::Explain);
    evaluation::capture(
        "P1-A03",
        "A",
        "이전 재시도 결정이 삭제됐어? 지금 확인할 수 있는 내용과 입력의 한계를 알려줘.",
        &store.load().await?,
        &[evaluation_query.clone()],
    )?;
    let revision = record(&store.latest().await?, "selected:D1")?
        .revision
        .clone();
    fs::write(
        &input,
        "{\"id\":\"D1\",\"kind\":\"decision\",\"text\":\"Keep retries bounded at three\"}\n",
    )?;
    ingest::import_with_completeness(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
        ImportCompleteness::Partial,
    )
    .await?;
    let latest = store.latest().await?;
    assert_ne!(record(&latest, "selected:D1")?.revision, revision);
    assert_eq!(
        record(&latest, "selected:D2")?.availability,
        Availability::Available
    );
    assert_eq!(
        source(&latest, "selected")?.import_completeness,
        ImportCompleteness::Partial
    );
    let versions = store
        .load()
        .await?
        .entries
        .iter()
        .filter(
            |entry| matches!(&entry.entity, Entity::Record(record) if record.id == "selected:D1"),
        )
        .count();
    ingest::import_with_completeness(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
        ImportCompleteness::Partial,
    )
    .await?;
    assert_eq!(store.load().await?.entries.iter().filter(|entry|
        matches!(&entry.entity, Entity::Record(record) if record.id == "selected:D1")).count(), versions);
    drop(store);
    let mut store = Store::open(&database, RedactionPolicy::default()).await?;
    let mut query = Query::new(Operation::Search, "p");
    query.query = Some(TextQuery {
        text: "fixed delay".into(),
        mode: SearchMode::Literal,
    });
    let result = execute(&store.load().await?, &query)?;
    assert_eq!(result.status, ResponseStatus::Partial);
    assert!(
        result
            .items
            .iter()
            .any(|item| item.entity.id() == "selected:D2")
    );
    evaluation::capture(
        "P1-A03",
        "B",
        "이전 재시도 결정이 삭제됐어? 지금 확인할 수 있는 내용과 입력의 한계를 알려줘.",
        &store.load().await?,
        &[evaluation_query],
    )?;
    fs::write(
        &input,
        "{\"id\":\"F3\",\"kind\":\"feedback\",\"text\":\"Retry-After remains unfinished\"}\n",
    )?;
    ingest::import_with_completeness(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
        ImportCompleteness::Delta,
    )
    .await?;
    let latest = store.latest().await?;
    assert_eq!(
        source(&latest, "selected")?.import_completeness,
        ImportCompleteness::Delta
    );
    assert_eq!(
        record(&latest, "selected:D1")?.body,
        "Keep retries bounded at three"
    );
    assert_eq!(
        record(&latest, "selected:D2")?.availability,
        Availability::Available
    );
    // Only an explicitly complete selected replacement can establish absence.
    ingest::import(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    assert_eq!(
        record(&store.latest().await?, "selected:D2")?.availability,
        Availability::Missing
    );
    Ok(())
}

#[tokio::test]
async fn partial_result_links_only_to_retained_matching_source_execution() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("selected.jsonl");
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    fs::write(
        &input,
        concat!(
            "{\"id\":\"A1\",\"kind\":\"attempt\",\"text\":\"first attempt\",\"execution_id\":\"X1\"}\n",
            "{\"id\":\"A2\",\"kind\":\"attempt\",\"text\":\"same command retry\",\"execution_id\":\"X2\"}\n"
        ),
    )?;
    ingest::import(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    fs::write(
        &input,
        "{\"id\":\"E2\",\"kind\":\"tool_result\",\"text\":\"retry output\",\"execution_id\":\"X2\"}\n",
    )?;
    ingest::import_with_completeness(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::JournalJsonl,
        None,
        ImportCompleteness::Delta,
    )
    .await?;
    let mut query = Query::new(Operation::Trace, "p");
    query.target = Some(Target::Record {
        id: "selected:E2".into(),
    });
    query.direction = Some(Direction::Outgoing);
    let result = execute(&store.load().await?, &query)?;
    assert!(
        result
            .items
            .iter()
            .any(|item| item.entity.id() == "selected:A2")
    );
    assert!(
        !result
            .items
            .iter()
            .any(|item| item.entity.id() == "selected:A1")
    );
    assert!(
        !result
            .relations
            .iter()
            .any(|edge| edge.kind == RelationKind::Verifies)
    );
    Ok(())
}

#[tokio::test]
async fn source_provided_session_and_tool_fields_survive_masking_and_restart() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("selected.jsonl");
    let database = directory.path().join("context.sqlite");
    fs::write(
        &input,
        concat!(
            "{\"timestamp\":\"2026-09-29T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"S1\",\"timestamp\":\"2026-09-28T09:00:00Z\",\"cwd\":\"/repo/TOP_SECRET\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"X1\",\"name\":\"exec_command\",\"arguments\":\"{\\\"cmd\\\":\\\"printf TOP_SECRET\\\",\\\"workdir\\\":\\\"/repo/TOP_SECRET\\\"}\"}}\n"
        ),
    )?;
    let policy = RedactionPolicy {
        literal_secrets: vec!["TOP_SECRET".into()],
    };
    let mut store = Store::open(&database, policy.clone()).await?;
    ingest::import(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    drop(store);
    let mut store = Store::open(&database, policy).await?;
    let latest = store.latest().await?;
    let session = latest
        .iter()
        .find_map(|entity| match entity {
            Entity::Session(session) => Some(session),
            _ => None,
        })
        .ok_or("missing session")?;
    assert_eq!(
        session.started_at.as_deref(),
        Some("2026-09-28T09:00:00+00:00")
    );
    assert_eq!(
        session.working_directory.as_deref(),
        Some("/repo/[REDACTED]")
    );
    assert!(session.worktree_id.is_none());
    let execution = record(&latest, "selected:call:X1")?
        .execution
        .as_ref()
        .ok_or("missing execution")?;
    assert_eq!(execution.tool_name.as_deref(), Some("exec_command"));
    assert!(
        execution
            .tool_input
            .as_deref()
            .is_some_and(|input| input.contains("[REDACTED]"))
    );
    assert_eq!(
        execution.command,
        execution.tool_input.as_deref().ok_or("missing input")?
    );
    assert!(!serde_json::to_string(&store.load().await?)?.contains("TOP_SECRET"));
    // Existing persisted JSON without these optional fields remains readable.
    let mut legacy = serde_json::to_value(execution)?;
    legacy
        .as_object_mut()
        .ok_or("execution not object")?
        .remove("tool_name");
    legacy
        .as_object_mut()
        .ok_or("execution not object")?
        .remove("tool_input");
    let legacy: Execution = serde_json::from_value(legacy)?;
    assert!(legacy.tool_name.is_none() && legacy.tool_input.is_none());
    let mut legacy = serde_json::to_value(source(&latest, "selected")?)?;
    legacy
        .as_object_mut()
        .ok_or("source not object")?
        .remove("import_completeness");
    let legacy: Source = serde_json::from_value(legacy)?;
    assert_eq!(legacy.import_completeness, ImportCompleteness::FullSnapshot);
    Ok(())
}

#[tokio::test]
async fn native_parent_id_is_not_resolved_through_another_source_kind() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = directory.path().join("journal.jsonl");
    let child = directory.path().join("child.jsonl");
    fs::write(
        &journal,
        "{\"id\":\"E1\",\"kind\":\"finding\",\"text\":\"other product export\",\"session_id\":\"parent\"}\n",
    )?;
    fs::write(
        &child,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"child\",\"parent_thread_id\":\"parent\"}}\n",
    )?;
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "journal",
        &journal,
        ImportFormat::JournalJsonl,
        None,
    )
    .await?;
    ingest::import(
        &mut store,
        "p",
        "codex",
        &child,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    let latest = store.latest().await?;
    assert!(latest.iter().any(|entity| matches!(entity,
        Entity::Session(session) if session.id == "codex:child" && session.parent_id.as_deref() == Some("codex:parent"))));
    assert!(
        source(&latest, "codex")?
            .gaps
            .iter()
            .any(|gap| gap.contains("session_parent_unresolved"))
    );
    Ok(())
}

#[tokio::test]
async fn fragments_without_stable_record_identity_are_rejected_before_storage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("selected.jsonl");
    let mut store = Store::open(
        &directory.path().join("context.sqlite"),
        RedactionPolicy::default(),
    )
    .await?;
    fs::write(
        &input,
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"agent_message\",\"message\":\"original line identity\"}}\n",
    )?;
    ingest::import(
        &mut store,
        "p",
        "selected",
        &input,
        ImportFormat::CodexJsonl,
        None,
    )
    .await?;
    let before = serde_json::to_string(&store.load().await?)?;
    fs::write(
        &input,
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"agent_message\",\"message\":\"unrelated page with the same line number\"}}\n",
    )?;
    assert!(
        ingest::import_with_completeness(
            &mut store,
            "p",
            "selected",
            &input,
            ImportFormat::CodexJsonl,
            None,
            ImportCompleteness::Partial
        )
        .await
        .is_err()
    );
    assert_eq!(serde_json::to_string(&store.load().await?)?, before);
    fs::write(&input, "document fragment")?;
    assert!(
        ingest::import_with_completeness(
            &mut store,
            "p",
            "fragment",
            &input,
            ImportFormat::Document,
            None,
            ImportCompleteness::Delta
        )
        .await
        .is_err()
    );
    assert_eq!(serde_json::to_string(&store.load().await?)?, before);
    Ok(())
}
