//! Deterministic audit of caller-declared preserved sections between two document bodies.
//!
//! The impl agent declares, in its rationale sidecar, which sections it copied
//! verbatim from the prior DEVELOPERS.md. This module verifies that claim by
//! comparing section bodies byte-for-byte. Semantic Remove/Keep/Merge judgment
//! stays in the Brain layer (impl); this Hands-layer tool only checks whether
//! declared preservation actually held.

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Drift {
    pub section: String,
    pub reason: String, // "removed" | "body_changed" | "absent_in_prior"
}

#[derive(Debug, Serialize)]
pub struct PreservationAudit {
    pub preserved: Vec<String>,
    pub drifted: Vec<Drift>,
}

pub fn audit(prior: &str, new: &str, sections: &[&str]) -> PreservationAudit {
    let mut preserved = Vec::new();
    let mut drifted = Vec::new();

    for section in sections {
        let heading = format!("## {}", section);
        let prior_body = extract_section(prior, &heading);
        let new_body = extract_section(new, &heading);

        match (prior_body, new_body) {
            (None, _) => drifted.push(Drift {
                section: (*section).to_string(),
                reason: "absent_in_prior".to_string(),
            }),
            (Some(_), None) => drifted.push(Drift {
                section: (*section).to_string(),
                reason: "removed".to_string(),
            }),
            (Some(p), Some(n)) if p == n => preserved.push((*section).to_string()),
            (Some(_), Some(_)) => drifted.push(Drift {
                section: (*section).to_string(),
                reason: "body_changed".to_string(),
            }),
        }
    }

    PreservationAudit { preserved, drifted }
}

/// Extract a section body by H2 heading. Returns `None` when the heading is absent.
/// Body spans from the line after the heading up to (but excluding) the next `## ` heading or EOF.
fn extract_section(doc: &str, heading: &str) -> Option<String> {
    let mut in_section = false;
    let mut found = false;
    let mut out = String::new();
    for line in doc.lines() {
        if line.starts_with("## ") {
            if in_section {
                break;
            }
            if line.trim_end() == heading {
                in_section = true;
                found = true;
            }
            continue;
        }
        if in_section {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    if found { Some(out) } else { None }
}
