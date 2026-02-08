//! Build script for claude-md-core
//!
//! Reads schema rules from YAML (Single Source of Truth) and generates
//! Rust constants at compile time.

use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// UseCase metadata patterns in schema-rules.yaml
#[derive(Debug, Deserialize, Default)]
struct UsecaseMetadata {
    #[serde(default)]
    actor_pattern: String,
    #[serde(default)]
    id_pattern: String,
    #[serde(default)]
    include_pattern: String,
    #[serde(default)]
    extend_pattern: String,
    #[serde(default)]
    actor_ref_pattern: String,
}

/// V2 format for exports
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct V2Format {
    #[serde(default)]
    heading_required: bool,
    #[serde(default)]
    heading_pattern: String,
    #[serde(default)]
    description_follows_signature: bool,
}

/// Section definition in schema-rules.yaml
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SectionDef {
    name: String,
    required: bool,
    #[serde(default)]
    condition: String,
    #[serde(default)]
    allow_none: bool,
    #[serde(default)]
    usecase_metadata: Option<UsecaseMetadata>,
    #[serde(default)]
    indexable: Option<bool>,
    #[serde(default)]
    v2_format: Option<V2Format>,
}

/// Schema version configuration
#[derive(Debug, Deserialize, Default)]
struct SchemaVersionDef {
    #[serde(default)]
    marker_pattern: String,
    #[serde(default)]
    current: String,
    #[serde(default)]
    supported: Vec<String>,
}

/// Cross reference configuration
#[derive(Debug, Deserialize, Default)]
struct CrossReferenceDef {
    #[serde(default)]
    format: String,
    #[allow(dead_code)]
    #[serde(default)]
    anchor_source: String,
}

/// Schema rules structure
#[derive(Debug, Deserialize)]
struct SchemaRules {
    #[allow(dead_code)]
    version: String,
    sections: HashMap<String, SectionDef>,
    #[serde(default)]
    implements_sections: HashMap<String, SectionDef>,
    #[serde(default)]
    schema_version: Option<SchemaVersionDef>,
    #[serde(default)]
    cross_reference: Option<CrossReferenceDef>,
}

fn main() {
    // Path to schema rules YAML (relative to core/ directory)
    let schema_path = "../skills/schema-validate/references/schema-rules.yaml";

    // Tell Cargo to rerun if the schema file changes
    println!("cargo:rerun-if-changed={}", schema_path);

    // Read and parse the YAML file
    let yaml_content = fs::read_to_string(schema_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read schema-rules.yaml at '{}': {}. \
             Make sure the file exists at plugins/claude-md-plugin/skills/schema-validate/references/schema-rules.yaml",
            schema_path, e
        )
    });

    let rules: SchemaRules = serde_yaml::from_str(&yaml_content).unwrap_or_else(|e| {
        panic!("Failed to parse schema-rules.yaml: {}", e)
    });

    // Extract required sections (where required=true and condition="always")
    let mut required_sections: Vec<&str> = rules
        .sections
        .values()
        .filter(|s| s.required && s.condition == "always")
        .map(|s| s.name.as_str())
        .collect();

    // Sort for consistent ordering
    required_sections.sort();

    // Extract sections that allow "None" as valid content
    let mut allow_none_sections: Vec<&str> = rules
        .sections
        .values()
        .filter(|s| s.allow_none)
        .map(|s| s.name.as_str())
        .collect();

    allow_none_sections.sort();

    // Extract IMPLEMENTS.md required sections
    let mut impl_required_sections: Vec<&str> = rules
        .implements_sections
        .values()
        .filter(|s| s.required && s.condition == "always")
        .map(|s| s.name.as_str())
        .collect();
    impl_required_sections.sort();

    let mut impl_allow_none_sections: Vec<&str> = rules
        .implements_sections
        .values()
        .filter(|s| s.allow_none)
        .map(|s| s.name.as_str())
        .collect();
    impl_allow_none_sections.sort();

    // Extract schema version info
    let schema_ver = rules.schema_version.unwrap_or_default();
    let schema_version_current = schema_ver.current;
    let schema_version_marker_pattern = schema_ver.marker_pattern;
    let schema_version_supported: Vec<&str> = schema_ver.supported.iter().map(|s| s.as_str()).collect();

    // Extract usecase metadata patterns from behavior section
    let behavior_section = rules.sections.get("behavior");
    let uc_meta = behavior_section.and_then(|s| s.usecase_metadata.as_ref());
    let usecase_id_pattern = uc_meta.map(|m| m.id_pattern.as_str()).unwrap_or("");
    let actor_pattern = uc_meta.map(|m| m.actor_pattern.as_str()).unwrap_or("");
    let include_pattern = uc_meta.map(|m| m.include_pattern.as_str()).unwrap_or("");
    let extend_pattern = uc_meta.map(|m| m.extend_pattern.as_str()).unwrap_or("");
    let actor_ref_pattern = uc_meta.map(|m| m.actor_ref_pattern.as_str()).unwrap_or("");

    // Extract v2 export format
    let exports_section = rules.sections.get("exports");
    let v2_fmt = exports_section.and_then(|s| s.v2_format.as_ref());
    let export_heading_pattern = v2_fmt.map(|f| f.heading_pattern.as_str()).unwrap_or("");

    // Extract cross reference format
    let cross_ref = rules.cross_reference.unwrap_or_default();
    let cross_ref_format = cross_ref.format;

    // Helper to escape a string for Rust source code (double backslashes, etc.)
    fn rust_str_literal(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }

    // Generate Rust code by building string manually to avoid raw-string nesting issues
    let mut code = String::new();
    code.push_str("// Auto-generated by build.rs from schema-rules.yaml\n");
    code.push_str("// DO NOT EDIT MANUALLY - Edit skills/schema-validate/references/schema-rules.yaml instead\n\n");

    code.push_str(&format!(
        "/// Required sections in CLAUDE.md (must always be present)\npub const REQUIRED_SECTIONS: &[&str] = &{:?};\n\n",
        required_sections
    ));

    code.push_str(&format!(
        "/// Sections that allow \"None\" as valid content\npub const ALLOW_NONE_SECTIONS: &[&str] = &{:?};\n\n",
        allow_none_sections
    ));

    code.push_str(&format!(
        "/// Required sections in IMPLEMENTS.md (must always be present)\npub const IMPLEMENTS_REQUIRED_SECTIONS: &[&str] = &{:?};\n\n",
        impl_required_sections
    ));

    code.push_str(&format!(
        "/// IMPLEMENTS.md sections that allow \"None\" as valid content\npub const IMPLEMENTS_ALLOW_NONE_SECTIONS: &[&str] = &{:?};\n\n",
        impl_allow_none_sections
    ));

    code.push_str(&format!(
        "/// Current schema version\npub const SCHEMA_VERSION_CURRENT: &str = \"{}\";\n\n",
        rust_str_literal(&schema_version_current)
    ));

    code.push_str(&format!(
        "/// Schema version marker regex pattern\npub const SCHEMA_VERSION_MARKER_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(&schema_version_marker_pattern)
    ));

    code.push_str(&format!(
        "/// Supported schema versions\npub const SCHEMA_VERSION_SUPPORTED: &[&str] = &{:?};\n\n",
        schema_version_supported
    ));

    code.push_str(&format!(
        "/// UseCase ID pattern (e.g. \"### UC-1: Name\")\npub const USECASE_ID_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(usecase_id_pattern)
    ));

    code.push_str(&format!(
        "/// Actor pattern (e.g. \"- ActorName: description\")\npub const ACTOR_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(actor_pattern)
    ));

    code.push_str(&format!(
        "/// Include pattern (e.g. \"- Includes: UC-3\")\npub const INCLUDE_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(include_pattern)
    ));

    code.push_str(&format!(
        "/// Extend pattern (e.g. \"- Extends: UC-1\")\npub const EXTEND_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(extend_pattern)
    ));

    code.push_str(&format!(
        "/// Actor reference pattern (e.g. \"- Actor: User\")\npub const ACTOR_REF_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(actor_ref_pattern)
    ));

    code.push_str(&format!(
        "/// Export heading pattern for v2 (e.g. \"#### symbolName\")\npub const EXPORT_HEADING_PATTERN: &str = \"{}\";\n\n",
        rust_str_literal(export_heading_pattern)
    ));

    code.push_str(&format!(
        "/// Cross-reference format string\npub const CROSS_REFERENCE_FORMAT: &str = \"{}\";\n",
        rust_str_literal(&cross_ref_format)
    ));

    // Write to OUT_DIR
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("schema_rules.rs");

    fs::write(&dest_path, code).unwrap_or_else(|e| {
        panic!("Failed to write generated code to {:?}: {}", dest_path, e)
    });

    println!("cargo:warning=Generated schema_rules.rs with {} required sections", required_sections.len());
}
