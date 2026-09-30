use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    EvidenceLevel, ImportError, ImportGapCode, ImportReport, ImportedKind, ImportedNature,
    ImportedReference, ImportedTool, digest, line_number, record,
};

/// Portable, explicit context journal. One JSON object per line.
///
/// IDs identify events, not content. Repeated executions must have distinct IDs
/// even when command, output, and timestamp happen to be equal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    pub id: String,
    pub kind: ImportedKind,
    pub text: String,
    #[serde(default)]
    pub nature: ImportedNature,
    #[serde(default)]
    pub occurred_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub tool: Option<ImportedTool>,
    #[serde(default)]
    pub evidence_level: EvidenceLevel,
    #[serde(default)]
    pub derived: bool,
    #[serde(default)]
    pub partial: bool,
    #[serde(default)]
    pub source_truncated: bool,
    #[serde(default)]
    pub references: Vec<ImportedReference>,
}

pub(super) fn parse(content: &str, locator: &str) -> Result<ImportReport, ImportError> {
    let mut report = ImportReport::new(content);
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut lines = content.lines().enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        let number = line_number(index)?;
        report.last_line = number;
        if line.trim().is_empty() {
            continue;
        }
        let entry = match serde_json::from_str::<JournalEntry>(line) {
            Ok(entry) if !entry.id.trim().is_empty() => entry,
            result => {
                let partial_tail = result
                    .err()
                    .is_some_and(|error| error.is_eof() && lines.peek().is_none());
                report.gap(
                    Some(number),
                    if partial_tail {
                        ImportGapCode::PartialTail
                    } else {
                        ImportGapCode::MalformedRecord
                    },
                    "Invalid journal entry: a supported schema and nonempty event ID are required.",
                );
                continue;
            }
        };
        // Canonical typed serialization tolerates formatting differences when
        // the same event is delivered more than once.
        let canonical = match serde_json::to_string(&entry) {
            Ok(canonical) => canonical,
            Err(_) => {
                report.gap(
                    Some(number),
                    ImportGapCode::MalformedRecord,
                    "Journal entry could not be normalized.",
                );
                continue;
            }
        };
        let revision = digest(&canonical);
        if let Some(previous) = seen.get(&entry.id) {
            if previous != &revision {
                report.gap(
                    Some(number),
                    ImportGapCode::ConflictingIdentity,
                    "An event ID appears with conflicting contents in the same journal; the later copy was omitted.",
                );
            }
            continue;
        }
        seen.insert(entry.id.clone(), revision.clone());
        let mut item = record(entry.id, locator, number, &entry.text);
        item.revision = revision;
        item.kind = entry.kind;
        item.nature = entry.nature;
        item.occurred_at = entry.occurred_at;
        item.actor = entry.actor;
        item.session_id = entry.session_id;
        item.work_id = entry.work_id;
        item.execution_id = entry.execution_id;
        item.tool = entry.tool;
        item.evidence_level = entry.evidence_level;
        item.derived = entry.derived || entry.evidence_level == EvidenceLevel::SummaryOnly;
        item.partial = entry.partial;
        item.source_truncated = entry.source_truncated;
        item.references = entry.references;
        if item.evidence_level == EvidenceLevel::SummaryOnly {
            report.gap(
                Some(number),
                ImportGapCode::SummaryOnly,
                "Only a derived summary is supplied for this event; referenced originals are not fetched.",
            );
        }
        if item.source_truncated {
            report.gap(
                Some(number),
                ImportGapCode::SourceTruncated,
                "The supplied original is truncated; additional paging cannot recover missing text.",
            );
        }
        report.records.push(item);
    }
    Ok(report)
}
