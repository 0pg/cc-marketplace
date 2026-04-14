//! Deterministic diff of the `## Data Schemas` section between two DEVELOPERS.md contents.
//!
//! Safety-boundary tool (not a judgment tool): returns only a boolean
//! indicating whether the Data Schemas section differs.

pub fn changed(before: &str, after: &str) -> bool {
    extract_section(before, "## Data Schemas") != extract_section(after, "## Data Schemas")
}

fn extract_section(doc: &str, heading: &str) -> String {
    let mut in_section = false;
    let mut out = String::new();
    for line in doc.lines() {
        if line.starts_with("## ") {
            in_section = line.trim_end() == heading;
            continue;
        }
        if in_section {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    out
}
