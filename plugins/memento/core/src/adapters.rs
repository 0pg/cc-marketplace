//! Read explicitly selected source files without discovering or mutating them.
//!
//! Imported text is not yet safe to persist or send to a model: the caller must
//! apply its shared redaction policy to text, metadata, and source locators.

mod codex;
mod journal;

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use journal::JournalEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    CodexJsonl,
    Document,
    JournalJsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedKind {
    Request,
    Constraint,
    Finding,
    Decision,
    Attempt,
    ToolResult,
    Change,
    Verification,
    Feedback,
    Status,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedNature {
    #[default]
    Observed,
    Reported,
    Inferred,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    #[default]
    Original,
    SummaryOnly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Running,
    Completed,
    Failed,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedTool {
    pub name: String,
    pub input: Option<String>,
    pub working_directory: Option<String>,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub state: ExecutionState,
}

/// An explicit source reference, never an inferred causal relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedReference {
    pub locator: String,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedRecord {
    /// Identity within this source. File-line identities require append-only input.
    pub source_record_id: String,
    pub revision: String,
    pub locator: String,
    pub source_order: u64,
    pub kind: ImportedKind,
    pub nature: ImportedNature,
    pub text: String,
    pub occurred_at: Option<DateTime<Utc>>,
    pub actor: Option<String>,
    pub session_id: Option<String>,
    pub work_id: Option<String>,
    /// Original invocation ID, shared by its call and result; not a command hash.
    pub execution_id: Option<String>,
    pub tool: Option<ImportedTool>,
    pub evidence_level: EvidenceLevel,
    pub derived: bool,
    pub partial: bool,
    pub source_truncated: bool,
    pub references: Vec<ImportedReference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportGapCode {
    MalformedRecord,
    UnsupportedRecord,
    UnsupportedContent,
    InvalidTimestamp,
    PartialTail,
    SourceTruncated,
    SummaryOnly,
    MissingExecutionResult,
    MissingExecutionIdentity,
    MissingSessionIdentity,
    LineIdentity,
    ConflictingIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportGap {
    pub line: Option<u64>,
    pub code: ImportGapCode,
    /// Describes the limitation without echoing malformed source content.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSession {
    pub id: String,
    pub parent_id: Option<String>,
    pub forked_from_id: Option<String>,
    pub locator: String,
    pub revision: String,
    /// Source-provided start time; never inferred from import time.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// A source path is not a verified Git worktree identity.
    #[serde(default)]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub records: Vec<ImportedRecord>,
    #[serde(default)]
    pub sessions: Vec<ImportedSession>,
    pub gaps: Vec<ImportGap>,
    pub source_revision: String,
    /// Last examined line, not a durable storage acknowledgement.
    pub last_line: u64,
    pub ignored_private_records: u64,
}

impl ImportReport {
    fn new(content: &str) -> Self {
        Self {
            records: Vec::new(),
            sessions: Vec::new(),
            gaps: Vec::new(),
            source_revision: digest(content),
            last_line: 0,
            ignored_private_records: 0,
        }
    }

    fn gap(&mut self, line: Option<u64>, code: ImportGapCode, detail: &str) {
        self.gaps.push(ImportGap {
            line,
            code,
            detail: detail.to_owned(),
        });
    }
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("could not read explicitly selected source: {0}")]
    Read(#[from] std::io::Error),
    #[error("selected source is not a regular file")]
    NotAFile,
    #[error("source line count exceeds the supported range")]
    TooManyLines,
}

/// Reads only `path`; never scans parent directories or the user's home.
pub fn import_file(path: &Path, format: ImportFormat) -> Result<ImportReport, ImportError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(ImportError::NotAFile);
    }
    let content = fs::read_to_string(path)?;
    let locator = path.to_string_lossy();
    import_text(&content, &locator, format)
}

/// Parses an explicitly supplied export. Locators are labels, never fetched.
pub fn import_text(
    content: &str,
    locator: &str,
    format: ImportFormat,
) -> Result<ImportReport, ImportError> {
    match format {
        ImportFormat::CodexJsonl => codex::parse(content, locator),
        ImportFormat::JournalJsonl => journal::parse(content, locator),
        ImportFormat::Document => {
            let mut report = ImportReport::new(content);
            report.last_line =
                u64::try_from(content.lines().count()).map_err(|_| ImportError::TooManyLines)?;
            let mut record = record("document".to_owned(), locator, 1, content);
            record.kind = ImportedKind::Finding;
            // Reading a document proves what it says, not that its claims are true.
            record.nature = ImportedNature::Reported;
            report.records.push(record);
            Ok(report)
        }
    }
}

fn digest(content: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(content.as_bytes()))
}

fn record(id: String, locator: &str, line: u64, text: &str) -> ImportedRecord {
    ImportedRecord {
        source_record_id: id,
        revision: digest(text),
        locator: format!("{locator}#L{line}"),
        source_order: line,
        kind: ImportedKind::Finding,
        nature: ImportedNature::Observed,
        text: text.to_owned(),
        occurred_at: None,
        actor: None,
        session_id: None,
        work_id: None,
        execution_id: None,
        tool: None,
        evidence_level: EvidenceLevel::Original,
        derived: false,
        partial: false,
        source_truncated: false,
        references: Vec::new(),
    }
}

fn timestamp(value: Option<&str>, line: u64, report: &mut ImportReport) -> Option<DateTime<Utc>> {
    let value = value?;
    match DateTime::parse_from_rfc3339(value) {
        Ok(value) => Some(value.with_timezone(&Utc)),
        Err(_) => {
            report.gap(
                Some(line),
                ImportGapCode::InvalidTimestamp,
                "Occurrence time could not be parsed; capture time must not replace it.",
            );
            None
        }
    }
}

fn line_number(index: usize) -> Result<u64, ImportError> {
    u64::try_from(index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .ok_or(ImportError::TooManyLines)
}
