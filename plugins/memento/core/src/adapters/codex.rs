//! Conservative support for the explicitly selected Codex rollout JSONL subset.
//! Unknown variants remain coverage gaps; this is not an app-server client or
//! the distinct `codex exec --json` item-event protocol.
//!
//! Wire-format references (reviewed 2026-09-28), pinned to OpenAI's source:
//! https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/history/src/lib.rs
//! https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/history/src/rollout_payload.rs
//! https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/protocol/src/models.rs
//! https://github.com/openai/codex/blob/88235f881d4e222cf779df785e747d8c8b935768/codex-rs/protocol/src/protocol.rs

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::{
    EvidenceLevel, ExecutionState, ImportError, ImportGapCode, ImportReport, ImportedKind,
    ImportedNature, ImportedRecord, ImportedSession, ImportedTool, digest, line_number, record,
    timestamp,
};

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    ordinal: Option<u64>,
    #[serde(flatten)]
    event: Event,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    SessionMeta {
        payload: SessionMeta,
    },
    ResponseItem {
        payload: ResponseItem,
    },
    #[serde(rename = "event_msg")]
    Message {
        payload: EventMessage,
    },
    Compacted {
        payload: Compacted,
    },
    TurnContext,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct SessionMeta {
    id: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    parent_thread_id: Option<String>,
    #[serde(default)]
    forked_from_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResponseItem {
    Message {
        #[serde(default)]
        id: Option<String>,
        role: String,
        #[serde(default)]
        channel: Option<String>,
        #[serde(default)]
        phase: Option<String>,
        #[serde(default)]
        status: Option<String>,
        content: Vec<TextContent>,
    },
    FunctionCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    FunctionCallOutput {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        call_id: Option<String>,
        output: ToolOutput,
    },
    Reasoning,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ToolOutput {
    Text(String),
    Parts(Vec<TextContent>),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TextContent {
    InputText {
        text: String,
    },
    OutputText {
        text: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EventMessage {
    UserMessage {
        message: String,
    },
    AgentMessage {
        message: String,
        #[serde(default)]
        phase: Option<String>,
    },
    ExecCommandBegin {
        call_id: String,
        command: Vec<String>,
        cwd: String,
    },
    ExecCommandEnd {
        call_id: String,
        command: Vec<String>,
        cwd: String,
        stdout: String,
        stderr: String,
        #[serde(default)]
        aggregated_output: String,
        exit_code: i32,
    },
    AgentReasoning,
    AgentReasoningRawContent,
    TokenCount,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct Compacted {
    message: String,
}

#[derive(Deserialize)]
struct CommandArguments {
    #[serde(default)]
    workdir: Option<String>,
}

#[derive(Default)]
struct ParseState {
    session_id: Option<String>,
    cwd: Option<String>,
    calls: HashMap<String, ImportedTool>,
    results: HashSet<String>,
    started_executions: HashSet<String>,
    completed_executions: HashSet<String>,
    used_line_identity: bool,
    current_ordinal: Option<u64>,
}

pub(super) fn parse(content: &str, locator: &str) -> Result<ImportReport, ImportError> {
    let mut report = ImportReport::new(content);
    let mut state = ParseState::default();
    let mut lines = content.lines().enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        let number = line_number(index)?;
        report.last_line = number;
        if line.trim().is_empty() {
            continue;
        }
        let envelope = match serde_json::from_str::<Envelope>(line) {
            Ok(envelope) => envelope,
            Err(error) => {
                let partial_tail = error.is_eof() && lines.peek().is_none();
                report.gap(
                    Some(number),
                    if partial_tail {
                        ImportGapCode::PartialTail
                    } else {
                        ImportGapCode::MalformedRecord
                    },
                    "Could not decode this source line; its contents were not imported.",
                );
                continue;
            }
        };
        let occurrence = timestamp(envelope.timestamp.as_deref(), number, &mut report);
        state.current_ordinal = envelope.ordinal;
        let mut imported = match envelope.event {
            Event::SessionMeta { payload: metadata } => {
                let started_at = timestamp(metadata.timestamp.as_deref(), number, &mut report);
                report.sessions.push(ImportedSession {
                    id: metadata.id.clone(),
                    parent_id: metadata.parent_thread_id,
                    forked_from_id: metadata.forked_from_id,
                    locator: format!("{locator}#L{number}"),
                    revision: digest(line),
                    started_at,
                    working_directory: metadata.cwd.clone(),
                });
                state.session_id = Some(metadata.id);
                state.cwd = metadata.cwd;
                None
            }
            Event::ResponseItem { payload: item } => {
                response(item, number, locator, &mut state, &mut report)
            }
            Event::Message { payload: event } => {
                event_message(event, number, locator, &mut state, &mut report)
            }
            Event::Compacted { payload: summary } => {
                let mut item = line_record(number, locator, &summary.message, &mut state);
                item.evidence_level = EvidenceLevel::SummaryOnly;
                item.nature = ImportedNature::Reported;
                item.derived = true;
                report.gap(
                    Some(number),
                    ImportGapCode::SummaryOnly,
                    "Compaction summary is derived evidence; its original coverage is not established.",
                );
                Some(item)
            }
            Event::TurnContext => None,
            Event::Unknown => {
                unsupported(number, &mut report);
                None
            }
        };
        if let Some(item) = imported.as_mut() {
            item.occurred_at = occurrence;
            item.session_id = state.session_id.clone();
            item.source_order = envelope.ordinal.unwrap_or(number);
            // Preserve changes to supplied metadata as well as to the body.
            item.revision = digest(line);
        }
        report.records.extend(imported);
    }
    for (call_id, tool) in &state.calls {
        let missing_result = if state.started_executions.contains(call_id) {
            !state.completed_executions.contains(call_id)
        } else {
            !state.results.contains(call_id)
        };
        if missing_result {
            for item in report.records.iter_mut().filter(|item| {
                item.execution_id.as_deref() == Some(call_id) && item.kind == ImportedKind::Attempt
            }) {
                item.partial = true;
                item.tool = Some(tool.clone());
            }
            report.gap(
                None,
                ImportGapCode::MissingExecutionResult,
                "A recorded invocation has no result in the supplied file; current liveness is unknown.",
            );
        }
    }
    if state.used_line_identity {
        report.gap(
            None,
            ImportGapCode::LineIdentity,
            "Some events have no native ID; line identities are stable only for append-only exports.",
        );
    }
    if state.session_id.is_none() && !report.records.is_empty() {
        report.gap(
            None,
            ImportGapCode::MissingSessionIdentity,
            "The supplied file has no session metadata; no session or work identity was inferred.",
        );
    }
    Ok(report)
}

fn response(
    response: ResponseItem,
    line: u64,
    locator: &str,
    state: &mut ParseState,
    report: &mut ImportReport,
) -> Option<ImportedRecord> {
    match response {
        ResponseItem::Message {
            id,
            role,
            channel,
            phase,
            status,
            content,
        } => {
            if !matches!(role.as_str(), "user" | "assistant")
                || channel
                    .as_deref()
                    .is_some_and(|channel| !matches!(channel, "final" | "commentary"))
                || phase
                    .as_deref()
                    .is_some_and(|phase| !matches!(phase, "final_answer" | "commentary"))
            {
                report.ignored_private_records += 1;
                return None;
            }
            let gaps_before = report.gaps.len();
            let text = text_parts(content, line, report);
            if text.is_empty() {
                return None;
            }
            let mut item = match id.filter(|id| !id.is_empty()) {
                Some(id) => record(format!("message:{id}"), locator, line, &text),
                None => line_record(line, locator, &text, state),
            };
            item.kind = if role == "user" {
                ImportedKind::Request
            } else {
                ImportedKind::Finding
            };
            item.nature = ImportedNature::Reported;
            item.actor = Some(role);
            item.partial = matches!(status.as_deref(), Some("in_progress" | "incomplete"))
                || report.gaps.len() != gaps_before;
            Some(item)
        }
        ResponseItem::FunctionCall {
            call_id,
            name,
            arguments,
        } => {
            let working_directory = if matches!(name.as_str(), "exec_command" | "shell_command") {
                serde_json::from_str::<CommandArguments>(&arguments)
                    .ok()
                    .and_then(|arguments| arguments.workdir)
                    .or_else(|| state.cwd.clone())
            } else {
                state.cwd.clone()
            };
            let tool = ImportedTool {
                name: name.clone(),
                input: Some(arguments.clone()),
                working_directory,
                exit_code: None,
                state: ExecutionState::Unknown,
            };
            state.calls.insert(call_id.clone(), tool.clone());
            let mut item = record(format!("call:{call_id}"), locator, line, &arguments);
            item.kind = ImportedKind::Attempt;
            item.actor = Some("assistant".to_owned());
            item.execution_id = Some(call_id);
            item.tool = Some(tool);
            Some(item)
        }
        ResponseItem::FunctionCallOutput {
            id,
            call_id,
            output,
        } => {
            let gaps_before = report.gaps.len();
            let text = match output {
                ToolOutput::Text(text) => text,
                ToolOutput::Parts(parts) => text_parts(parts, line, report),
            };
            let mut item = if let Some(call_id) = &call_id {
                state.results.insert(call_id.clone());
                record(format!("call_output:{call_id}"), locator, line, &text)
            } else if let Some(id) = id {
                record(format!("output:{id}"), locator, line, &text)
            } else {
                line_record(line, locator, &text, state)
            };
            item.kind = ImportedKind::ToolResult;
            item.partial = report.gaps.len() != gaps_before;
            item.tool = call_id.as_ref().and_then(|id| state.calls.get(id)).cloned();
            if call_id.is_none() {
                report.gap(
                    Some(line),
                    ImportGapCode::MissingExecutionIdentity,
                    "Tool output has no invocation ID; it was not joined to a preceding call.",
                );
            }
            item.execution_id = call_id;
            mark_truncation(&mut item, line, report);
            // A tool response may describe a still-running process. It is not
            // proof of successful command completion or a passing test.
            Some(item)
        }
        ResponseItem::Reasoning => {
            report.ignored_private_records += 1;
            None
        }
        ResponseItem::Unknown => {
            unsupported(line, report);
            None
        }
    }
}

fn event_message(
    event: EventMessage,
    line: u64,
    locator: &str,
    state: &mut ParseState,
    report: &mut ImportReport,
) -> Option<ImportedRecord> {
    let (text, role, kind) = match event {
        EventMessage::UserMessage { message } => (message, "user", ImportedKind::Request),
        EventMessage::AgentMessage { message, phase } => {
            if phase
                .as_deref()
                .is_some_and(|phase| !matches!(phase, "final_answer" | "commentary"))
            {
                report.ignored_private_records += 1;
                return None;
            }
            (message, "assistant", ImportedKind::Finding)
        }
        EventMessage::ExecCommandBegin {
            call_id,
            command,
            cwd,
        } => {
            state.started_executions.insert(call_id.clone());
            let input = command_input(&command);
            let tool = ImportedTool {
                name: "exec_command".to_owned(),
                input: Some(input.clone()),
                working_directory: Some(cwd),
                exit_code: None,
                state: ExecutionState::Running,
            };
            state.calls.insert(call_id.clone(), tool.clone());
            let mut item = record(format!("exec_begin:{call_id}"), locator, line, &input);
            item.kind = ImportedKind::Attempt;
            item.execution_id = Some(call_id);
            item.tool = Some(tool);
            return Some(item);
        }
        EventMessage::ExecCommandEnd {
            call_id,
            command,
            cwd,
            stdout,
            stderr,
            aggregated_output,
            exit_code,
        } => {
            let text = if aggregated_output.is_empty() {
                format!("stdout:\n{stdout}\nstderr:\n{stderr}")
            } else {
                aggregated_output
            };
            let mut item = record(format!("exec_end:{call_id}"), locator, line, &text);
            item.kind = ImportedKind::ToolResult;
            state.results.insert(call_id.clone());
            state.completed_executions.insert(call_id.clone());
            item.execution_id = Some(call_id);
            item.tool = Some(ImportedTool {
                name: "exec_command".to_owned(),
                input: Some(command_input(&command)),
                working_directory: Some(cwd),
                exit_code: Some(exit_code),
                state: if exit_code == 0 {
                    ExecutionState::Completed
                } else {
                    ExecutionState::Failed
                },
            });
            mark_truncation(&mut item, line, report);
            return Some(item);
        }
        EventMessage::AgentReasoning | EventMessage::AgentReasoningRawContent => {
            report.ignored_private_records += 1;
            return None;
        }
        EventMessage::TokenCount => return None,
        EventMessage::Unknown => {
            unsupported(line, report);
            return None;
        }
    };
    let mut item = line_record(line, locator, &text, state);
    item.actor = Some(role.to_owned());
    item.kind = kind;
    item.nature = ImportedNature::Reported;
    Some(item)
}

fn text_parts(parts: Vec<TextContent>, line: u64, report: &mut ImportReport) -> String {
    let mut texts = Vec::new();
    for part in parts {
        match part {
            TextContent::InputText { text } | TextContent::OutputText { text } => texts.push(text),
            TextContent::Unknown => report.gap(
                Some(line),
                ImportGapCode::UnsupportedContent,
                "A non-text or unsupported content part was omitted.",
            ),
        }
    }
    texts.join("\n")
}

fn line_record(line: u64, locator: &str, text: &str, state: &mut ParseState) -> ImportedRecord {
    let id = if let Some(ordinal) = state.current_ordinal {
        format!("ordinal:{ordinal}")
    } else {
        state.used_line_identity = true;
        format!("line:{line}")
    };
    record(id, locator, line, text)
}

fn command_input(command: &[String]) -> String {
    // This is a representation of argv, never a shell command to execute.
    command
        .iter()
        .map(|argument| format!("{argument:?}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn mark_truncation(item: &mut ImportedRecord, line: u64, report: &mut ImportReport) {
    // Explicit formatter marker from codex-rs/utils/output-truncation/src/lib.rs.
    if item
        .text
        .lines()
        .any(|line| line.starts_with("Warning: truncated output (original token count: "))
    {
        item.source_truncated = true;
        report.gap(
            Some(line),
            ImportGapCode::SourceTruncated,
            "The source output contains Codex's truncation marker; omitted output is not recoverable here.",
        );
    }
}

fn unsupported(line: u64, report: &mut ImportReport) {
    report.gap(
        Some(line),
        ImportGapCode::UnsupportedRecord,
        "This source event variant is not supported and was not imported.",
    );
}
