use std::error::Error;
use std::fs;

use memento::adapters::{
    EvidenceLevel, ExecutionState, ImportError, ImportFormat, ImportGapCode, ImportedKind,
    ImportedNature, import_file, import_text,
};

// Synthetic, non-user fixture using the wire schemas in OpenAI's source:
// https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/history/src/rollout_payload.rs
// https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/protocol/src/models.rs
// https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/protocol/src/protocol.rs
const CODEX_TRANSCRIPT: &str = r#"{"timestamp":"2026-09-27T09:00:00Z","type":"session_meta","payload":{"id":"session-1","cwd":"/repo"}}
{"timestamp":"2026-09-27T09:00:01Z","type":"response_item","payload":{"type":"message","id":"request-1","role":"user","content":[{"type":"input_text","text":"429를 줄이되 정상 지연을 늘리지 말 것"}]}}
{"timestamp":"2026-09-27T09:00:02Z","type":"response_item","payload":{"type":"function_call","call_id":"exec-1","name":"exec_command","arguments":"{\"cmd\":\"cargo test\",\"workdir\":\"/repo/a\"}"}}
{"timestamp":"2026-09-27T09:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"exec-1","output":"normal-request latency increased; HTTP 429"}}
{"timestamp":"2026-09-27T09:00:04Z","type":"response_item","payload":{"type":"message","role":"assistant","phase":"final_answer","content":[{"type":"output_text","text":"고정 대기를 기각하고 오류 후 backoff만 적용한다."}]}}
{"timestamp":"2026-09-27T09:01:00Z","type":"event_msg","payload":{"type":"exec_command_begin","call_id":"exec-2","command":["cargo","test"],"cwd":"/repo/a"}}
{"timestamp":"2026-09-27T09:01:10Z","type":"event_msg","payload":{"type":"exec_command_end","call_id":"exec-2","command":["cargo","test"],"cwd":"/repo/a","stdout":"1 test passed","stderr":"","exit_code":0}}
"#;

#[test]
fn native_rollout_preserves_public_evidence_and_distinct_executions() -> Result<(), Box<dyn Error>>
{
    let report = import_text(CODEX_TRANSCRIPT, "selected.jsonl", ImportFormat::CodexJsonl)?;
    assert_eq!(report.records.len(), 6);
    assert_eq!(report.last_line, 7);
    let first = report
        .records
        .iter()
        .find(|record| record.source_record_id == "call:exec-1")
        .ok_or("missing invocation")?;
    assert_eq!(first.session_id.as_deref(), Some("session-1"));
    assert_eq!(first.execution_id.as_deref(), Some("exec-1"));
    assert_eq!(first.locator, "selected.jsonl#L3");
    assert_eq!(
        first
            .tool
            .as_ref()
            .and_then(|tool| tool.working_directory.as_deref()),
        Some("/repo/a")
    );
    let output = report
        .records
        .iter()
        .find(|record| record.source_record_id == "call_output:exec-1")
        .ok_or("missing output")?;
    assert!(output.text.contains("HTTP 429"));
    assert_eq!(output.kind, ImportedKind::ToolResult);
    // A generic tool response alone does not prove command completion.
    assert_eq!(
        output.tool.as_ref().map(|tool| tool.state),
        Some(ExecutionState::Unknown)
    );
    let second = report
        .records
        .iter()
        .find(|record| record.source_record_id == "exec_end:exec-2")
        .ok_or("missing second execution")?;
    assert_eq!(second.execution_id.as_deref(), Some("exec-2"));
    assert_eq!(
        second.tool.as_ref().and_then(|tool| tool.exit_code),
        Some(0)
    );
    assert_eq!(
        second.tool.as_ref().map(|tool| tool.state),
        Some(ExecutionState::Completed)
    );
    assert!(
        !report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::MissingExecutionResult)
    );
    let replay = import_text(CODEX_TRANSCRIPT, "selected.jsonl", ImportFormat::CodexJsonl)?;
    assert_eq!(report, replay);
    Ok(())
}

#[test]
fn private_reasoning_and_unknown_payloads_never_enter_imported_text() -> Result<(), Box<dyn Error>>
{
    let content = r#"{"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"PRIVATE_A"}],"encrypted_content":"PRIVATE_B"}}
{"type":"response_item","payload":{"type":"message","role":"assistant","channel":"analysis","content":[{"type":"output_text","text":"PRIVATE_C"}]}}
{"type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"PRIVATE_D"}]}}
{"type":"event_msg","payload":{"type":"agent_reasoning","text":"PRIVATE_E"}}
{"type":"future_event","payload":{"text":"PRIVATE_F"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"public explanation"}}
{"type":"event_msg","payload":{"type":"user_message","message":"public correction"}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"visible caption"},{"type":"encrypted_content","encrypted_content":"PRIVATE_G"}]}}
{"type":"event_msg","payload":{"type":"user_message","message":123,"SECRET":"PRIVATE_H"}}
"#;
    let report = import_text(content, "selected.jsonl", ImportFormat::CodexJsonl)?;
    assert_eq!(report.ignored_private_records, 4);
    assert_eq!(report.records.len(), 3);
    let encoded = serde_json::to_string(&report)?;
    assert!(!encoded.contains("PRIVATE_"));
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::UnsupportedRecord)
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::MalformedRecord)
    );
    assert!(
        report
            .records
            .iter()
            .any(|record| record.text == "visible caption" && record.partial)
    );
    Ok(())
}

#[test]
fn interrupted_execution_retains_missing_result_and_partial_tail() -> Result<(), Box<dyn Error>> {
    let content = concat!(
        "{\"timestamp\":\"2026-09-27T18:42:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"exec_command_begin\",\"call_id\":\"X7\",\"command\":[\"cargo\",\"test\"],\"cwd\":\"/repo\"}}\n",
        "{\"type\":\"event_msg\",\"payload\":{"
    );
    let report = import_text(content, "interrupted.jsonl", ImportFormat::CodexJsonl)?;
    let execution = report.records.first().ok_or("missing execution start")?;
    assert!(execution.partial);
    assert_eq!(execution.execution_id.as_deref(), Some("X7"));
    assert_eq!(
        execution.tool.as_ref().map(|tool| tool.state),
        Some(ExecutionState::Running)
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::MissingExecutionResult)
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::PartialTail)
    );
    assert_eq!(report.last_line, 2);
    Ok(())
}

#[test]
fn native_ordinals_survive_reformatting_and_missing_call_ids_do_not_invent_links()
-> Result<(), Box<dyn Error>> {
    let event = r#"{"ordinal":42,"timestamp":"unknown","type":"event_msg","payload":{"type":"agent_message","message":"a public finding"}}"#;
    let first = import_text(event, "selected.jsonl", ImportFormat::CodexJsonl)?;
    let shifted = import_text(
        &format!("\n{event}"),
        "selected.jsonl",
        ImportFormat::CodexJsonl,
    )?;
    let first = first.records.first().ok_or("missing ordinal record")?;
    let shifted = shifted.records.first().ok_or("missing shifted record")?;
    assert_eq!(first.source_record_id, "ordinal:42");
    assert_eq!(first.source_record_id, shifted.source_record_id);
    assert_eq!(first.revision, shifted.revision);
    assert!(first.occurred_at.is_none());
    let result = import_text(
        r#"{"type":"response_item","payload":{"type":"function_call_output","id":"out-1","output":[{"type":"input_text","text":"result"}]}}"#,
        "partial.jsonl",
        ImportFormat::CodexJsonl,
    )?;
    let output = result.records.first().ok_or("missing unpaired output")?;
    assert!(output.execution_id.is_none());
    assert_eq!(output.source_record_id, "output:out-1");
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::MissingExecutionIdentity)
    );
    Ok(())
}

#[test]
fn compaction_and_native_truncation_remain_evidence_limitations() -> Result<(), Box<dyn Error>> {
    let content = r#"{"type":"compacted","payload":{"message":"Earlier trial was unsuccessful.","replacement_history":[{"type":"reasoning","encrypted_content":"PRIVATE"}]}}
{"type":"response_item","payload":{"type":"function_call_output","call_id":"X7","output":"Warning: truncated output (original token count: 20000)\nTotal output lines: 500\n\nexcerpt"}}
"#;
    let report = import_text(content, "partial.jsonl", ImportFormat::CodexJsonl)?;
    let summary = report.records.first().ok_or("missing summary")?;
    assert_eq!(summary.evidence_level, EvidenceLevel::SummaryOnly);
    assert!(summary.derived);
    assert_eq!(summary.nature, ImportedNature::Reported);
    assert!(report.records.iter().any(|record| record.source_truncated));
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::SummaryOnly)
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::SourceTruncated)
    );
    assert!(!serde_json::to_string(&report)?.contains("PRIVATE"));
    Ok(())
}

#[test]
fn portable_journal_preserves_provenance_and_does_not_merge_repeated_runs()
-> Result<(), Box<dyn Error>> {
    let content = r#"{"id":"E1","kind":"tool_result","text":"same command failed","execution_id":"X1","source_truncated":true,"partial":true}
{"id":"E1","kind":"tool_result","text":"same command failed","execution_id":"X1","source_truncated":true,"partial":true}
{"id":"E2","kind":"tool_result","text":"same command failed","execution_id":"X2"}
{"id":"E2","kind":"tool_result","text":"contradicting copy","execution_id":"X2"}
{"id":"S1","kind":"finding","text":"Earlier work summary","evidence_level":"summary_only","references":[{"locator":"not-fetched.jsonl#L10","revision":"old-revision"}]}
"#;
    let report = import_text(content, "journal.jsonl", ImportFormat::JournalJsonl)?;
    assert_eq!(report.records.len(), 3);
    assert!(
        report
            .records
            .iter()
            .any(|record| record.execution_id.as_deref() == Some("X1"))
    );
    assert!(
        report
            .records
            .iter()
            .any(|record| record.execution_id.as_deref() == Some("X2"))
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::ConflictingIdentity)
    );
    let summary = report
        .records
        .iter()
        .find(|record| record.source_record_id == "S1")
        .ok_or("missing summary")?;
    assert!(summary.derived);
    assert_eq!(
        summary
            .references
            .first()
            .and_then(|reference| reference.revision.as_deref()),
        Some("old-revision")
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.code == ImportGapCode::SourceTruncated)
    );
    Ok(())
}

#[test]
fn document_revisions_are_stable_without_inventing_code_or_time() -> Result<(), Box<dyn Error>> {
    let before = import_text(
        "# Decision\nExclude A for licensing.",
        "design.md",
        ImportFormat::Document,
    )?;
    let after = import_text(
        "# Decision\nA can now be reconsidered.",
        "design.md",
        ImportFormat::Document,
    )?;
    let before = before.records.first().ok_or("missing original document")?;
    let after = after.records.first().ok_or("missing current document")?;
    assert_eq!(before.source_record_id, after.source_record_id);
    assert_ne!(before.revision, after.revision);
    assert_eq!(before.locator, "design.md#L1");
    assert!(before.occurred_at.is_none());
    assert!(before.work_id.is_none());
    assert_eq!(before.nature, ImportedNature::Reported);
    Ok(())
}

#[test]
fn file_import_reads_only_explicitly_selected_regular_file() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let selected = directory.path().join("selected.md");
    let other = directory.path().join("private.md");
    fs::write(&selected, "allowed document")?;
    fs::write(&other, "NOT_REQUESTED")?;
    let report = import_file(&selected, ImportFormat::Document)?;
    assert_eq!(report.records.len(), 1);
    assert!(!serde_json::to_string(&report)?.contains("NOT_REQUESTED"));
    assert!(matches!(
        import_file(directory.path(), ImportFormat::Document),
        Err(ImportError::NotAFile)
    ));
    assert_eq!(fs::read_to_string(&other)?, "NOT_REQUESTED");
    Ok(())
}

#[test]
fn native_session_ancestry_preserves_explicit_parent_and_fork_ids() -> Result<(), Box<dyn Error>> {
    let report = import_text(
        r#"{"type":"session_meta","payload":{"id":"child","cwd":"/repo","parent_thread_id":"parent","forked_from_id":"fork-origin"}}"#,
        "child.jsonl",
        ImportFormat::CodexJsonl,
    )?;
    let session = report
        .sessions
        .first()
        .ok_or("missing native session metadata")?;
    assert_eq!(session.id, "child");
    assert_eq!(session.parent_id.as_deref(), Some("parent"));
    assert_eq!(session.forked_from_id.as_deref(), Some("fork-origin"));
    assert_eq!(session.locator, "child.jsonl#L1");
    assert!(report.records.is_empty());
    Ok(())
}
