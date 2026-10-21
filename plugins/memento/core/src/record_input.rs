//! Actionable diagnostics for record input without guessing unavailable identities.
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    Result,
    model::{Entity, Evidence, Record},
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Syntax,
    Schema,
    Validation,
}

#[derive(Debug, Clone, Serialize, Error)]
#[error("{message} at {field_path}; {hint}")]
pub struct Diagnostic {
    pub code: &'static str,
    pub stage: Stage,
    pub message: String,
    pub field_path: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_record_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

impl Diagnostic {
    fn syntax(error: serde_json::Error) -> Self {
        Self {
            code: "invalid_record_json",
            stage: Stage::Syntax,
            message: error.to_string(),
            field_path: String::new(),
            hint:
                "Correct the JSON syntax at the reported line and column before schema validation."
                    .into(),
            record_index: None,
            record_id: None,
            evidence_index: None,
            evidence_record_id: None,
            evidence_source_id: None,
            line: Some(error.line()),
            column: Some(error.column()),
        }
    }

    pub(crate) fn claim(record: &Record, field_path: &str, message: &str, hint: &str) -> Self {
        Self {
            code: "invalid_record_claim",
            stage: Stage::Validation,
            message: message.into(),
            field_path: field_path.into(),
            hint: hint.into(),
            record_index: None,
            record_id: Some(record.id.clone()),
            evidence_index: None,
            evidence_record_id: None,
            evidence_source_id: None,
            line: None,
            column: None,
        }
    }

    pub(crate) fn with_evidence(mut self, index: usize, evidence: &Evidence) -> Self {
        self.evidence_index = Some(index);
        self.evidence_record_id = evidence.record_id.clone();
        self.evidence_source_id = Some(evidence.source_id.clone());
        self
    }

    pub(crate) fn with_record_index(mut self, index: usize) -> Self {
        self.record_index = Some(index);
        self.field_path = format!("/{index}{}", self.field_path);
        self
    }

    /// Prefix a diagnostic when records were nested in a command envelope.
    pub fn prepend_path(&mut self, prefix: &str) {
        self.field_path.insert_str(0, prefix);
    }
}

impl From<Diagnostic> for crate::Error {
    fn from(error: Diagnostic) -> Self {
        Self::RecordInput(Box::new(error))
    }
}

/// Parse JSON before envelope/schema validation without losing syntax diagnostics.
pub fn parse_value(input: &str) -> Result<Value> {
    serde_json::from_str(input).map_err(|error| Diagnostic::syntax(error).into())
}

/// Parse a single Entity or an Entity array before attempting any store writes.
pub fn parse_records(input: &str) -> Result<Vec<Entity>> {
    parse_records_value(parse_value(input)?)
}

/// Paths are JSON pointers relative to this value; array indices are zero-based.
pub fn parse_records_value(value: Value) -> Result<Vec<Entity>> {
    match value {
        Value::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| parse_entity(value, Some(index)))
            .collect(),
        value => Ok(vec![parse_entity(value, None)?]),
    }
}

fn parse_entity(value: Value, index: Option<usize>) -> Result<Entity> {
    let record_id = value
        .pointer("/data/id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let base = index.map_or_else(String::new, |index| format!("/{index}"));
    serde_json::from_value(value.clone()).map_err(|error| {
        let message = error.to_string();
        let container = if value.get("data").is_some() {
            format!("{base}/data")
        } else {
            base.clone()
        };
        let mut diagnostic = Diagnostic {
            code: "invalid_record_schema",
            stage: Stage::Schema,
            message: message.clone(),
            field_path: container,
            hint: "Use the Entity schema at field_path; nested locations that cannot be established are not guessed.".into(),
            record_index: index,
            record_id,
            evidence_index: None,
            evidence_record_id: None,
            evidence_source_id: None,
            line: None,
            column: None,
        };
        if let Some(field) = missing_field(&message) {
            if matches!(field, "entity" | "data") && value.get(field).is_none() {
                diagnostic.field_path = format!("{base}/{field}");
            } else if value.get("entity").and_then(Value::as_str) == Some("record")
                && required_record_field(field)
                && value.pointer("/data").is_some_and(Value::is_object)
                && value.pointer("/data").and_then(|data| data.get(field)).is_none()
            {
                diagnostic.field_path = format!("{base}/data/{field}");
            }
            diagnostic.hint = if field == "title" && diagnostic.field_path.ends_with("/title") {
                "Add a string title, for example \"title\": \"Brief description\".".into()
            } else {
                format!("Add the required field {field:?} in the indicated container using the Entity schema.")
            };
        }
        diagnostic.into()
    })
}

fn missing_field(message: &str) -> Option<&str> {
    message
        .strip_prefix("missing field \u{60}")?
        .split('\u{60}')
        .next()
}

fn required_record_field(field: &str) -> bool {
    matches!(
        field,
        "id" | "project_id"
            | "source_id"
            | "revision"
            | "kind"
            | "nature"
            | "fidelity"
            | "availability"
            | "title"
            | "body"
            | "association"
    )
}
