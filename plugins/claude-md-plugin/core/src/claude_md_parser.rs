use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub use crate::bracket_utils::split_respecting_brackets;
use crate::bracket_utils::{find_matching_bracket, extract_parenthesized};

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

/// Error types for CLAUDE.md parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Cannot read file '{path}': {source}")]
    FileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Missing required section: {section}")]
    MissingRequiredSection { section: String },
    #[error("Invalid section format in '{section}': {details}")]
    InvalidSectionFormat { section: String, details: String },
}

/// Complete specification parsed from CLAUDE.md
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeMdSpec {
    /// Module name (from H1 header)
    pub name: String,
    /// Purpose description
    pub purpose: String,
    /// Exported symbols
    pub exports: ExportsSpec,
    /// Dependencies
    pub dependencies: DependenciesSpec,
    /// Behavioral scenarios
    pub behaviors: Vec<BehaviorSpec>,
    /// Function contracts
    pub contracts: Vec<ContractSpec>,
    /// Protocol (state machine, lifecycle)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ProtocolSpec>,
    /// Domain Context (decision rationale, constraints, compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_context: Option<DomainContextSpec>,
    /// Directory structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure: Option<StructureSpec>,
    /// Validation warnings (non-fatal issues found during parsing)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Exports specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportsSpec {
    pub functions: Vec<FunctionExport>,
    pub types: Vec<TypeExport>,
    pub classes: Vec<ClassExport>,
    pub enums: Vec<EnumExport>,
    pub variables: Vec<VariableExport>,
}

/// Function export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExport {
    pub name: String,
    pub signature: String,
    #[serde(default)]
    pub is_async: bool,
}

/// Type export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeExport {
    pub name: String,
    pub definition: String,
    #[serde(default)]
    pub kind: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    #[default]
    Interface,
    TypeAlias,
    Struct,
    DataClass,
    Record,
}

/// Class export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassExport {
    pub name: String,
    pub constructor_signature: String,
}

/// Enum export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumExport {
    pub name: String,
    pub variants: Vec<String>,
}

/// Variable export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableExport {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// A parsed internal dependency from CLAUDE.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalDepSpec {
    /// CLAUDE.md path or raw import path
    pub path: String,
    /// Imported symbols (if specified after colon)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    /// Whether the path points to a CLAUDE.md file (new format)
    pub is_claude_md_ref: bool,
}

/// Dependencies specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependenciesSpec {
    pub external: Vec<String>,
    pub internal: Vec<InternalDepSpec>,
}

/// Behavioral scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSpec {
    pub input: String,
    pub output: String,
    pub category: BehaviorCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorCategory {
    #[default]
    Success,
    Error,
}

/// Contract specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSpec {
    pub function_name: String,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub throws: Vec<String>,
    pub invariants: Vec<String>,
}

/// Domain Context specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainContextSpec {
    pub decision_rationale: Vec<String>,
    pub constraints: Vec<String>,
    pub compatibility: Vec<String>,
}

/// Protocol specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolSpec {
    pub states: Vec<String>,
    pub transitions: Vec<TransitionSpec>,
    pub lifecycle: Vec<LifecycleMethod>,
}

/// State transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub from: String,
    pub trigger: String,
    pub to: String,
}

/// Lifecycle method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleMethod {
    pub order: u32,
    pub method: String,
    pub description: String,
}

/// Directory structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructureSpec {
    pub subdirs: Vec<StructureEntry>,
    pub files: Vec<StructureEntry>,
}

/// Structure entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureEntry {
    pub name: String,
    pub description: String,
}

/// Deduplicate items by name, keeping the first occurrence.
fn dedup_by_name<T, F>(items: &mut Vec<T>, name_fn: F)
where
    F: Fn(&T) -> &String,
{
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| seen.insert(name_fn(item).clone()));
}

/// CLAUDE.md Parser
pub struct ClaudeMdParser {
    section_pattern: Regex,
    behavior_pattern: Regex,
    function_pattern: Regex,
    type_pattern: Regex,
    class_pattern: Regex,
    dependency_pattern: Regex,
    transition_pattern: Regex,
    lifecycle_pattern: Regex,
    structure_pattern: Regex,
    data_class_pattern: Regex,
}

impl ClaudeMdParser {
    pub fn new() -> Self {
        Self {
            // Match markdown headers: ## Purpose, ### Functions
            section_pattern: Regex::new(r"^(#{1,4})\s+(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match behavior: input → output or input -> output
            behavior_pattern: Regex::new(r"^[-*]?\s*(.+?)\s*(?:→|->)+\s*(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match function signature: `funcName(params): ReturnType` or Name(params): Type
            function_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*[:\s]*(.+?)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match type definition: `TypeName { fields }` or TypeName { fields }
            type_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match class: `ClassName(params)` or ClassName(params)
            class_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match dependency: external: pkg or internal: path (value is optional for list-style)
            dependency_pattern: Regex::new(r"^[-*]?\s*(external|internal):\s*(.*)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match transition: `State` + `trigger` → `NewState`
            transition_pattern: Regex::new(r"^[-*]?\s*`?([^`]+)`?\s*\+\s*`?([^`]+)`?\s*[→\->]+\s*`?([^`]+)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match lifecycle: N. `method` - description
            lifecycle_pattern: Regex::new(r"^(\d+)\.\s*`?([A-Za-z_][A-Za-z0-9_]*(?:\(\))?)`?\s*[-–]\s*(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match structure: name/: description or name.ext: description
            structure_pattern: Regex::new(r"^[-*]?\s*([A-Za-z0-9_.-]+/?)\s*[:\s]+(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            data_class_pattern: Regex::new(r"^data\s+class\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
        }
    }

    /// Parse a CLAUDE.md file
    pub fn parse(&self, file: &Path) -> Result<ClaudeMdSpec, ParseError> {
        let content = std::fs::read_to_string(file).map_err(|e| ParseError::FileReadError {
            path: file.to_string_lossy().to_string(),
            source: e,
        })?;

        self.parse_content(&content)
    }

    /// Parse CLAUDE.md content directly
    /// Returns Err immediately if any required section is missing.
    /// Required sections are defined in schema-rules.yaml (SSOT).
    pub fn parse_content(&self, content: &str) -> Result<ClaudeMdSpec, ParseError> {
        let mut spec = ClaudeMdSpec::default();
        let sections = self.extract_sections(content);

        // Extract module name from first H1 header
        for section in &sections {
            if section.level == 1 {
                spec.name = section.name.clone();
                break;
            }
        }

        // Check all required sections exist (from SSOT) - FAIL FAST
        for required in REQUIRED_SECTIONS {
            let section_found = sections.iter().find(|s| s.name.eq_ignore_ascii_case(required));

            match section_found {
                None => {
                    return Err(ParseError::MissingRequiredSection {
                        section: required.to_string(),
                    });
                }
                Some(section) => {
                    // For sections that allow "None", check if it's a valid None marker
                    let allows_none = ALLOW_NONE_SECTIONS
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(required));
                    let is_none_marker = self.is_none_marker(section);

                    // If section doesn't allow None but has None marker, that's an error
                    if !allows_none && is_none_marker {
                        return Err(ParseError::InvalidSectionFormat {
                            section: required.to_string(),
                            details: format!("Section '{}' does not allow 'None' as value", required),
                        });
                    }
                }
            }
        }

        // Parse Purpose section
        if let Some(purpose_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Purpose")) {
            spec.purpose = purpose_section.content.join("\n").trim().to_string();
        }

        // Parse Exports section
        self.parse_exports(&sections, &mut spec);

        // Parse Dependencies section (optional - not in REQUIRED_SECTIONS)
        if let Some(deps_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Dependencies")) {
            spec.dependencies = self.parse_dependencies(&deps_section.content);
        }

        // Parse Behavior section
        self.parse_behaviors(&sections, &mut spec);

        // Parse Contract section
        self.parse_contracts(&sections, &mut spec);

        // Parse Protocol section
        self.parse_protocol(&sections, &mut spec);

        // Parse Domain Context section
        self.parse_domain_context(&sections, &mut spec);

        // Parse Structure section (optional - not in REQUIRED_SECTIONS)
        if let Some(structure_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Structure")) {
            spec.structure = Some(self.parse_structure(&structure_section.content));
        }

        Ok(spec)
    }

    /// Check if a section contains only a "None" marker (None, N/A, etc.)
    fn is_none_marker(&self, section: &ParserSection) -> bool {
        let lines: Vec<&str> = section.content.iter().map(|s| s.as_str()).collect();
        crate::is_none_marker_content(&lines)
    }

    fn extract_sections(&self, content: &str) -> Vec<ParserSection> {
        let mut sections = Vec::new();
        let mut current_section: Option<ParserSection> = None;

        for line in content.lines() {
            if let Some(caps) = self.section_pattern.captures(line) {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let level = caps.get(1).map(|m| m.as_str().len()).unwrap_or(1);
                let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

                current_section = Some(ParserSection {
                    name,
                    level,
                    content: Vec::new(),
                });
            } else if let Some(ref mut section) = current_section {
                section.content.push(line.to_string());
            }
        }

        if let Some(section) = current_section {
            sections.push(section);
        }

        sections
    }

    fn parse_exports(&self, sections: &[ParserSection], spec: &mut ClaudeMdSpec) {
        // Note: existence of Exports section is checked in parse_content (fail-fast)
        let exports_section = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Exports"));

        if exports_section.is_none() {
            return;
        }

        // Find subsections under Exports
        // Track whether we're inside the Exports H2 section to avoid matching
        // same-named H3 sections under other H2 sections (e.g. ### Functions under ## Behavior)
        let mut in_exports_scope = false;
        let mut in_functions = false;
        let mut in_types = false;
        let mut in_classes = false;
        let mut in_methods = false;
        let mut in_structs = false;
        let mut in_data_classes = false;
        let mut in_enums = false;
        let mut in_variables = false;

        for section in sections {
            let name_lower = section.name.to_lowercase();

            // Track H2 section scope
            if section.level <= 2 {
                in_exports_scope = name_lower == "exports";
                in_functions = false;
                in_types = false;
                in_classes = false;
                in_methods = false;
                in_structs = false;
                in_data_classes = false;
                in_enums = false;
                in_variables = false;
                // Parse direct content of ## Exports (flat exports before any subsection)
                if in_exports_scope {
                    for line in &section.content {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                            continue;
                        }
                        if let Some(func) = self.parse_function_line(trimmed) {
                            spec.exports.functions.push(func);
                        } else if let Some(type_export) = self.parse_type_line(trimmed, false, false) {
                            spec.exports.types.push(type_export);
                        } else if let Some(class) = self.parse_class_line(trimmed) {
                            spec.exports.classes.push(class);
                        } else if let Some(enum_export) = self.parse_enum_line(trimmed) {
                            spec.exports.enums.push(enum_export);
                        } else if let Some(var) = self.parse_variable_line(trimmed) {
                            spec.exports.variables.push(var);
                        }
                    }
                }
                continue;
            }

            // Only process subsections within Exports scope
            if !in_exports_scope {
                continue;
            }

            // Track subsection context
            if name_lower == "functions" || name_lower == "methods" {
                in_functions = true;
                in_types = false;
                in_classes = false;
                in_methods = name_lower == "methods";
                in_structs = false;
                in_data_classes = false;
                in_enums = false;
                in_variables = false;
            } else if name_lower == "types" || name_lower == "structs" {
                in_functions = false;
                in_types = true;
                in_classes = false;
                in_structs = name_lower == "structs";
                in_data_classes = false;
                in_enums = false;
                in_variables = false;
            } else if name_lower == "classes" {
                in_functions = false;
                in_types = false;
                in_classes = true;
                in_structs = false;
                in_data_classes = false;
                in_enums = false;
                in_variables = false;
            } else if name_lower == "data classes" {
                in_functions = false;
                in_types = true;
                in_classes = false;
                in_structs = false;
                in_data_classes = true;
                in_enums = false;
                in_variables = false;
            } else if name_lower == "enums" {
                in_functions = false;
                in_types = false;
                in_classes = false;
                in_structs = false;
                in_data_classes = false;
                in_enums = true;
                in_variables = false;
            } else if name_lower == "variables" || name_lower == "constants" {
                in_functions = false;
                in_types = false;
                in_classes = false;
                in_structs = false;
                in_data_classes = false;
                in_enums = false;
                in_variables = true;
            }

            // Parse content based on context
            for line in &section.content {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                    continue;
                }

                if in_functions || in_methods {
                    if let Some(func) = self.parse_function_line(trimmed) {
                        spec.exports.functions.push(func);
                    }
                } else if in_types || in_structs || in_data_classes {
                    if let Some(type_export) = self.parse_type_line(trimmed, in_structs, in_data_classes) {
                        spec.exports.types.push(type_export);
                    }
                } else if in_classes {
                    if let Some(class) = self.parse_class_line(trimmed) {
                        spec.exports.classes.push(class);
                    }
                } else if in_enums {
                    if let Some(enum_export) = self.parse_enum_line(trimmed) {
                        spec.exports.enums.push(enum_export);
                    }
                } else if in_variables {
                    if let Some(var) = self.parse_variable_line(trimmed) {
                        spec.exports.variables.push(var);
                    }
                }
            }
        }

        // Deduplicate exports (mixed flat + subsection format may produce duplicates)
        dedup_by_name(&mut spec.exports.functions, |f| &f.name);
        dedup_by_name(&mut spec.exports.types, |t| &t.name);
        dedup_by_name(&mut spec.exports.classes, |c| &c.name);
        dedup_by_name(&mut spec.exports.enums, |e| &e.name);
        dedup_by_name(&mut spec.exports.variables, |v| &v.name);

        // If no subsections found, try parsing Exports content directly
        if spec.exports.functions.is_empty()
            && spec.exports.types.is_empty()
            && spec.exports.classes.is_empty()
            && spec.exports.enums.is_empty()
            && spec.exports.variables.is_empty()
        {
            if let Some(exports) = exports_section {
                for line in &exports.content {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                        continue;
                    }

                    // Try parsing as function first
                    if let Some(func) = self.parse_function_line(trimmed) {
                        spec.exports.functions.push(func);
                    } else if let Some(type_export) = self.parse_type_line(trimmed, false, false) {
                        spec.exports.types.push(type_export);
                    } else if let Some(class) = self.parse_class_line(trimmed) {
                        spec.exports.classes.push(class);
                    } else if let Some(enum_export) = self.parse_enum_line(trimmed) {
                        spec.exports.enums.push(enum_export);
                    } else if let Some(var) = self.parse_variable_line(trimmed) {
                        spec.exports.variables.push(var);
                    }
                }
            }
        }

        // Check for "None" in exports
        if let Some(exports) = exports_section {
            let content_lower = exports.content.join("\n").to_lowercase();
            if content_lower.contains("none") && spec.exports.functions.is_empty() {
                // Valid: explicitly no exports
                return;
            }
        }

        if spec.exports.functions.is_empty()
            && spec.exports.types.is_empty()
            && spec.exports.classes.is_empty()
            && spec.exports.enums.is_empty()
            && spec.exports.variables.is_empty()
        {
            spec.warnings.push("Exports section contains no valid exports".to_string());
        }
    }

    fn parse_function_line(&self, line: &str) -> Option<FunctionExport> {
        // Handle various function signature formats
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');

        // Check for async indicators
        let is_async = cleaned.contains("Promise<")
            || cleaned.contains("async ")
            || cleaned.contains("suspend ")
            || cleaned.contains("CompletableFuture<");

        // Use balanced bracket parsing for better generic type support
        if let Some(result) = self.parse_function_with_balanced_brackets(cleaned, is_async) {
            return Some(result);
        }

        // Fallback to regex patterns for simple cases
        // Try to extract function name and signature
        // Pattern: funcName(params): ReturnType or ReturnType funcName(params)
        if let Some(caps) = self.function_pattern.captures(cleaned) {
            let name = caps.get(1)?.as_str().to_string();
            let params = caps.get(2)?.as_str();
            let return_type = caps.get(3)?.as_str().trim();

            let signature = format!("{}({}): {}", name, params, return_type);
            return Some(FunctionExport {
                name,
                signature,
                is_async,
            });
        }

        // Handle Java-style: ReturnType funcName(params)
        if let Some(result) = self.parse_java_style_function(cleaned) {
            return Some(result);
        }

        // Handle Kotlin suspend fun style
        if let Some(result) = self.parse_kotlin_style_function(cleaned) {
            return Some(result);
        }

        // Handle Go-style: FuncName(params) (ReturnType, error)
        if let Some(result) = self.parse_go_style_function(cleaned) {
            return Some(result);
        }

        // Handle Python-style: func_name(params) -> ReturnType
        if let Some(result) = self.parse_python_style_function(cleaned, is_async) {
            return Some(result);
        }

        None
    }

    /// Parse function using balanced bracket matching for generic types
    fn parse_function_with_balanced_brackets(&self, s: &str, is_async: bool) -> Option<FunctionExport> {
        // Skip language keywords
        let cleaned = s
            .trim_start_matches("async ")
            .trim_start_matches("function ")
            .trim_start_matches("pub ")
            .trim_start_matches("fn ")
            .trim_start_matches("def ")
            .trim_start_matches("suspend ")
            .trim_start_matches("fun ")
            .trim();

        // Extract function name (may include generic params like func<T, U>)
        let name_end = cleaned.find('(')?;
        let name_part = cleaned[..name_end].trim();

        // Handle generic function names like transform<T, U>
        let (name, generic_params) = if let Some(generic_start) = name_part.find('<') {
            let generic_end = find_matching_bracket(name_part, generic_start, '<', '>')?;
            (
                name_part[..generic_start].to_string(),
                Some(name_part[generic_start..=generic_end].to_string()),
            )
        } else {
            (name_part.to_string(), None)
        };

        if name.is_empty() || !name.chars().next()?.is_alphabetic() {
            return None;
        }

        // Extract params using balanced bracket matching
        let (params, rest) = extract_parenthesized(cleaned)?;

        // Parse return type
        let return_type = rest
            .trim_start_matches(':')
            .trim_start_matches("->")
            .trim()
            .to_string();

        if return_type.is_empty() {
            return None;
        }

        // Build signature
        let signature = if let Some(generics) = generic_params {
            format!("{}{}({}): {}", name, generics, params, return_type)
        } else {
            format!("{}({}): {}", name, params, return_type)
        };

        Some(FunctionExport {
            name,
            signature,
            is_async,
        })
    }

    fn parse_java_style_function(&self, cleaned: &str) -> Option<FunctionExport> {
        // Handle Java-style: ReturnType funcName(params) or complex generics
        // Match pattern: Type<...> name(params) or Type name(params)

        // Find the opening paren
        let paren_idx = cleaned.find('(')?;

        // Find the function name (word before paren)
        let before_paren = cleaned[..paren_idx].trim();
        let parts: Vec<&str> = before_paren.split_whitespace().collect();

        if parts.len() < 2 {
            return None;
        }

        let name = parts.last()?.to_string();
        let return_type = parts[..parts.len() - 1].join(" ");

        // Check if this looks like Java style (return type has uppercase letter or is primitive)
        let first_char = return_type.chars().next()?;
        if !first_char.is_uppercase()
            && !["int", "long", "double", "float", "boolean", "void", "byte", "char", "short"]
                .contains(&return_type.to_lowercase().as_str())
        {
            return None;
        }

        // Extract params using balanced brackets
        let (params, rest) = extract_parenthesized(cleaned)?;

        let signature = format!("{} {}({}){}", return_type, name, params, rest);
        Some(FunctionExport {
            name,
            signature,
            is_async: return_type.contains("CompletableFuture"),
        })
    }

    fn parse_kotlin_style_function(&self, cleaned: &str) -> Option<FunctionExport> {
        let is_suspend = cleaned.starts_with("suspend ");
        let rest = cleaned
            .trim_start_matches("suspend ")
            .trim_start_matches("fun ")
            .trim();

        if !cleaned.contains("fun ") {
            return None;
        }

        // Extract name and generics
        let paren_idx = rest.find('(')?;
        let name = rest[..paren_idx].trim().to_string();

        // Extract params using balanced brackets
        let (params, return_part) = extract_parenthesized(rest)?;

        let return_type = return_part.trim_start_matches(':').trim();
        if return_type.is_empty() {
            return None;
        }

        let prefix = if is_suspend { "suspend fun " } else { "fun " };
        let signature = format!("{}{}({}): {}", prefix, name, params, return_type);

        Some(FunctionExport {
            name,
            signature,
            is_async: is_suspend,
        })
    }

    fn parse_go_style_function(&self, cleaned: &str) -> Option<FunctionExport> {
        // Go style: FuncName(params) ReturnType or FuncName(params) (ReturnType, error)
        let paren_idx = cleaned.find('(')?;
        let name = cleaned[..paren_idx].trim().to_string();

        // Name should be PascalCase for exported Go functions
        if !name.chars().next()?.is_uppercase() {
            return None;
        }

        let (params, rest) = extract_parenthesized(cleaned)?;

        // Reject if params contain ":" (TypeScript/JS typed params like `props: Props`)
        // Go params use space-separated types (e.g., `id string`) not colons
        if params.contains(':') {
            return None;
        }

        // Reject if return part starts with ":" or "=>" (TS/JS return type syntax)
        let return_type = rest.trim();
        if return_type.starts_with(':') || return_type.starts_with("=>") {
            return None;
        }

        if return_type.is_empty() {
            // void function
            let signature = format!("{}({})", name, params);
            return Some(FunctionExport {
                name,
                signature,
                is_async: false,
            });
        }

        let signature = format!("{}({}) {}", name, params, return_type);
        Some(FunctionExport {
            name,
            signature,
            is_async: false,
        })
    }

    fn parse_python_style_function(&self, cleaned: &str, is_async: bool) -> Option<FunctionExport> {
        // Python style: func_name(params) -> ReturnType
        if !cleaned.contains("->") {
            return None;
        }

        let paren_idx = cleaned.find('(')?;
        let name = cleaned[..paren_idx].trim().to_string();

        // Python names: allow snake_case, SCREAMING_SNAKE, PascalCase with ->
        // Only reject single all-uppercase words (likely type names, not functions)
        if name.chars().all(|c| c.is_uppercase()) && !name.contains('_') && name.len() > 1 {
            return None;
        }

        let (params, rest) = extract_parenthesized(cleaned)?;

        let return_type = rest.trim_start_matches("->").trim();
        if return_type.is_empty() {
            return None;
        }

        let signature = format!("{}({}) -> {}", name, params, return_type);
        Some(FunctionExport {
            name,
            signature,
            is_async,
        })
    }

    fn parse_type_line(&self, line: &str, is_struct: bool, is_data_class: bool) -> Option<TypeExport> {
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');

        if let Some(caps) = self.type_pattern.captures(cleaned) {
            let name = caps.get(1)?.as_str().to_string();
            let fields = caps.get(2)?.as_str();

            let definition = format!("{} {{ {} }}", name, fields);
            let kind = if is_struct {
                TypeKind::Struct
            } else if is_data_class {
                TypeKind::DataClass
            } else {
                TypeKind::Interface
            };

            return Some(TypeExport {
                name,
                definition,
                kind,
            });
        }

        // Handle Kotlin data class pattern
        if let Some(caps) = self.data_class_pattern.captures(cleaned) {
            let name = caps.get(1)?.as_str().to_string();
            let fields = caps.get(2)?.as_str();

            return Some(TypeExport {
                name: name.clone(),
                definition: format!("data class {}({})", name, fields),
                kind: TypeKind::DataClass,
            });
        }

        None
    }

    fn parse_class_line(&self, line: &str) -> Option<ClassExport> {
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');

        if let Some(caps) = self.class_pattern.captures(cleaned) {
            let name = caps.get(1)?.as_str().to_string();
            let params = caps.get(2)?.as_str();

            let constructor_signature = format!("{}({})", name, params);
            return Some(ClassExport {
                name,
                constructor_signature,
            });
        }

        None
    }

    fn parse_enum_line(&self, line: &str) -> Option<EnumExport> {
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');

        // Pattern: EnumName: Variant1 | Variant2 | ...
        // or: EnumName = Variant1 | Variant2
        let sep_pos = cleaned.find(':').or_else(|| cleaned.find('='))?;
        let name = cleaned[..sep_pos].trim().to_string();
        let variants_str = cleaned[sep_pos + 1..].trim();

        // Must have pipe-separated variants
        if !variants_str.contains('|') {
            return None;
        }

        let variants: Vec<String> = variants_str
            .split('|')
            .map(|v| v.trim().trim_matches('`').trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();

        if name.is_empty() || variants.is_empty() {
            return None;
        }

        Some(EnumExport { name, variants })
    }

    fn parse_variable_line(&self, line: &str) -> Option<VariableExport> {
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');

        // Pattern: CONST_NAME = value or CONST_NAME: Type
        // Must start with uppercase or contain underscore (constant-like)
        let first_char = cleaned.chars().next()?;
        if !first_char.is_uppercase() {
            return None;
        }

        if let Some(eq_pos) = cleaned.find('=') {
            let name = cleaned[..eq_pos].trim().to_string();
            let value = cleaned[eq_pos + 1..].trim().to_string();
            return Some(VariableExport {
                name,
                value: if value.is_empty() { None } else { Some(value) },
            });
        }

        if let Some(colon_pos) = cleaned.find(':') {
            let name = cleaned[..colon_pos].trim().to_string();
            let type_str = cleaned[colon_pos + 1..].trim().to_string();
            // Don't confuse with function signatures containing parentheses
            if cleaned.contains('(') {
                return None;
            }
            return Some(VariableExport {
                name,
                value: if type_str.is_empty() { None } else { Some(type_str) },
            });
        }

        None
    }

    fn parse_dependencies(&self, content: &[String]) -> DependenciesSpec {
        let mut deps = DependenciesSpec::default();
        let mut current_dep_type: Option<String> = None;

        for line in content {
            let trimmed = line.trim();

            // Check for top-level dep type markers: "- external:" or "- internal:"
            if let Some(caps) = self.dependency_pattern.captures(trimmed) {
                let dep_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let dep_value = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                if dep_value.is_empty() {
                    // List-style: "- internal:" followed by sub-items
                    current_dep_type = Some(dep_type.to_string());
                } else {
                    // Inline-style: "- internal: ./types"
                    match dep_type {
                        "external" => deps.external.push(dep_value),
                        "internal" => {
                            deps.internal.push(InternalDepSpec {
                                path: dep_value,
                                symbols: Vec::new(),
                                is_claude_md_ref: false,
                            });
                        }
                        _ => {}
                    }
                    current_dep_type = None;
                }
                continue;
            }

            // Check for sub-items under current dep type: "  - `path`: symbols"
            if let Some(ref dep_type) = current_dep_type {
                let sub_trimmed = trimmed.trim_start_matches('-').trim_start_matches('*').trim();
                if sub_trimmed.is_empty() {
                    continue;
                }

                // Parse: `path`: symbols  or  `path`
                if sub_trimmed.starts_with('`') {
                    if let Some(backtick_end) = sub_trimmed[1..].find('`') {
                        let path = sub_trimmed[1..=backtick_end].to_string();
                        let rest = sub_trimmed[backtick_end + 2..].trim();

                        // Extract symbols after colon
                        let symbols = if rest.starts_with(':') {
                            rest[1..]
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect()
                        } else {
                            Vec::new()
                        };

                        match dep_type.as_str() {
                            "external" => deps.external.push(
                                if symbols.is_empty() {
                                    path
                                } else {
                                    format!("{}: {}", path, symbols.join(", "))
                                }
                            ),
                            "internal" => {
                                let is_claude_md = path.ends_with("/CLAUDE.md");
                                deps.internal.push(InternalDepSpec {
                                    path,
                                    symbols,
                                    is_claude_md_ref: is_claude_md,
                                });
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Plain text sub-item (legacy format or simple text)
                    match dep_type.as_str() {
                        "external" => deps.external.push(sub_trimmed.to_string()),
                        "internal" => {
                            deps.internal.push(InternalDepSpec {
                                path: sub_trimmed.to_string(),
                                symbols: Vec::new(),
                                is_claude_md_ref: false,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        deps
    }

    fn parse_behaviors(&self, sections: &[ParserSection], spec: &mut ClaudeMdSpec) {
        // Note: existence of Behavior section is checked in parse_content (fail-fast)
        // Find the index of the Behavior section
        let behavior_idx = match sections.iter().position(|s| s.name.eq_ignore_ascii_case("Behavior")) {
            Some(idx) => idx,
            None => return,
        };
        let behavior_section = &sections[behavior_idx];

        let mut current_category = BehaviorCategory::Success;

        // First parse the main Behavior section content
        for line in &behavior_section.content {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                continue;
            }

            if let Some(caps) = self.behavior_pattern.captures(trimmed) {
                let input = caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                let output = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                let category = if output.contains("Error") || output.contains("Exception") || output.contains("Err") {
                    BehaviorCategory::Error
                } else {
                    BehaviorCategory::Success
                };

                spec.behaviors.push(BehaviorSpec {
                    input,
                    output,
                    category,
                });
            }
        }

        // Then parse subsections (level 3+) that come after Behavior
        for section in sections.iter().skip(behavior_idx + 1) {
            // Stop if we hit another H2 section (any H2 means Behavior scope ended)
            if section.level <= 2 {
                break;
            }

            let name_lower = section.name.to_lowercase();

            // Update category based on subsection name
            if name_lower.contains("success") || name_lower.contains("정상") {
                current_category = BehaviorCategory::Success;
            } else if name_lower.contains("error") || name_lower.contains("에러") || name_lower.contains("failure") {
                current_category = BehaviorCategory::Error;
            }

            // Parse behavior lines in subsection
            for line in &section.content {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                    continue;
                }

                if let Some(caps) = self.behavior_pattern.captures(trimmed) {
                    let input = caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                    let output = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                    // Infer category from output if it looks like an error
                    let category = if output.contains("Error") || output.contains("Exception") || output.contains("Err") {
                        BehaviorCategory::Error
                    } else {
                        current_category.clone()
                    };

                    spec.behaviors.push(BehaviorSpec {
                        input,
                        output,
                        category,
                    });
                }
            }
        }

        // Check for "None" in behavior
        let content_lower = behavior_section.content.join("\n").to_lowercase();
        if content_lower.contains("none") && spec.behaviors.is_empty() {
            return;
        }

        if spec.behaviors.is_empty() {
            spec.warnings.push("Behavior section contains no valid scenarios".to_string());
        }
    }

    fn parse_contracts(&self, sections: &[ParserSection], spec: &mut ClaudeMdSpec) {
        let contract_idx = sections.iter().position(|s| s.name.eq_ignore_ascii_case("Contract"));
        if contract_idx.is_none() {
            return;
        }
        let contract_start = contract_idx.unwrap();
        let contract_level = sections[contract_start].level;

        // Collect Contract section and its subsections (until next same-or-higher level section)
        let contract_sections: Vec<&ParserSection> = std::iter::once(&sections[contract_start])
            .chain(
                sections[contract_start + 1..]
                    .iter()
                    .take_while(|s| s.level > contract_level)
            )
            .collect();

        let mut current_function = String::new();
        let mut current_contract = ContractSpec {
            function_name: String::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            throws: Vec::new(),
            invariants: Vec::new(),
        };

        for section in &contract_sections {
            // Check if this is a function-specific contract section
            if section.level == 3 {
                // Save previous contract
                if !current_function.is_empty() && (!current_contract.preconditions.is_empty()
                    || !current_contract.postconditions.is_empty()
                    || !current_contract.throws.is_empty())
                {
                    spec.contracts.push(current_contract.clone());
                }

                current_function = section.name.clone();
                current_contract = ContractSpec {
                    function_name: current_function.clone(),
                    preconditions: Vec::new(),
                    postconditions: Vec::new(),
                    throws: Vec::new(),
                    invariants: Vec::new(),
                };
            }

            // Parse contract content
            for line in &section.content {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if trimmed.contains("**Preconditions**") || trimmed.contains("**Precondition**") {
                    let value = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if !value.is_empty() {
                        current_contract.preconditions.push(value.to_string());
                    }
                } else if trimmed.contains("**Postconditions**") || trimmed.contains("**Postcondition**") {
                    let value = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if !value.is_empty() {
                        current_contract.postconditions.push(value.to_string());
                    }
                } else if trimmed.contains("**Throws**") || trimmed.contains("**Throw**") {
                    let value = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if !value.is_empty() {
                        current_contract.throws.push(value.to_string());
                    }
                } else if trimmed.contains("**Invariants**") || trimmed.contains("**Invariant**") {
                    let value = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if !value.is_empty() {
                        current_contract.invariants.push(value.to_string());
                    }
                }
            }
        }

        // Save last contract
        if !current_function.is_empty() && (!current_contract.preconditions.is_empty()
            || !current_contract.postconditions.is_empty()
            || !current_contract.throws.is_empty())
        {
            spec.contracts.push(current_contract);
        }
    }

    fn parse_protocol(&self, sections: &[ParserSection], spec: &mut ClaudeMdSpec) {
        let protocol_idx = sections.iter().position(|s| s.name.eq_ignore_ascii_case("Protocol"));
        if protocol_idx.is_none() {
            return;
        }
        let protocol_start = protocol_idx.unwrap();
        let protocol_level = sections[protocol_start].level;

        // Collect Protocol section and its subsections (until next same-or-higher level section)
        let protocol_sections: Vec<&ParserSection> = std::iter::once(&sections[protocol_start])
            .chain(
                sections[protocol_start + 1..]
                    .iter()
                    .take_while(|s| s.level > protocol_level)
            )
            .collect();

        let mut protocol = ProtocolSpec::default();
        let mut in_state_machine = false;
        let mut in_lifecycle = false;

        for section in &protocol_sections {
            let name_lower = section.name.to_lowercase();

            if name_lower.contains("state") && name_lower.contains("machine") {
                in_state_machine = true;
                in_lifecycle = false;
            } else if name_lower.contains("lifecycle") {
                in_state_machine = false;
                in_lifecycle = true;
            }

            for line in &section.content {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Parse states line: States: `A` | `B` | `C`
                if trimmed.starts_with("States:") {
                    let states_str = trimmed.trim_start_matches("States:").trim();
                    for state in states_str.split('|') {
                        let state = state.trim().trim_matches('`').trim();
                        if !state.is_empty() {
                            protocol.states.push(state.to_string());
                        }
                    }
                }
                // Parse transitions
                else if in_state_machine {
                    if let Some(caps) = self.transition_pattern.captures(trimmed) {
                        let from = caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                        let trigger = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                        let to = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                        protocol.transitions.push(TransitionSpec { from, trigger, to });
                    }
                }
                // Parse lifecycle methods
                else if in_lifecycle {
                    if let Some(caps) = self.lifecycle_pattern.captures(trimmed) {
                        let order: u32 = caps.get(1).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                        let method = caps.get(2).map(|m| m.as_str().trim_end_matches("()").to_string()).unwrap_or_default();
                        let description = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                        protocol.lifecycle.push(LifecycleMethod {
                            order,
                            method,
                            description,
                        });
                    }
                }
            }
        }

        if !protocol.states.is_empty() || !protocol.lifecycle.is_empty() {
            spec.protocol = Some(protocol);
        }
    }

    fn parse_domain_context(&self, sections: &[ParserSection], spec: &mut ClaudeMdSpec) {
        let dc_idx = sections.iter().position(|s| s.name.eq_ignore_ascii_case("Domain Context"));
        if dc_idx.is_none() {
            return;
        }
        let dc_start = dc_idx.unwrap();
        let dc_section = &sections[dc_start];

        // Check for None marker
        if self.is_none_marker(dc_section) {
            return;
        }

        let dc_level = dc_section.level;

        // Collect Domain Context section and its subsections
        let dc_sections: Vec<&ParserSection> = std::iter::once(&sections[dc_start])
            .chain(
                sections[dc_start + 1..]
                    .iter()
                    .take_while(|s| s.level > dc_level)
            )
            .collect();

        let mut domain_context = DomainContextSpec::default();
        let mut current_sub = "";

        for section in &dc_sections {
            let name_lower = section.name.to_lowercase();

            if name_lower.contains("decision") || name_lower.contains("rationale") {
                current_sub = "decision_rationale";
            } else if name_lower.contains("constraint") {
                current_sub = "constraints";
            } else if name_lower.contains("compatibility") || name_lower.contains("호환") {
                current_sub = "compatibility";
            }

            for line in &section.content {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Parse bullet-point lines
                let is_bullet = trimmed.starts_with('-') || trimmed.starts_with('*');
                if is_bullet {
                    let value = trimmed.trim_start_matches('-').trim_start_matches('*').trim().to_string();
                    if value.is_empty() || value.eq_ignore_ascii_case("none") {
                        continue;
                    }
                    match current_sub {
                        "decision_rationale" => domain_context.decision_rationale.push(value),
                        "constraints" => domain_context.constraints.push(value),
                        "compatibility" => domain_context.compatibility.push(value),
                        _ => {
                            // If no subsection identified, treat as decision_rationale
                            domain_context.decision_rationale.push(value);
                        }
                    }
                }
            }
        }

        if !domain_context.decision_rationale.is_empty()
            || !domain_context.constraints.is_empty()
            || !domain_context.compatibility.is_empty()
        {
            spec.domain_context = Some(domain_context);
        }
    }

    fn parse_structure(&self, content: &[String]) -> StructureSpec {
        let mut structure = StructureSpec::default();

        for line in content {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(caps) = self.structure_pattern.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let description = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                let entry = StructureEntry {
                    name: name.trim_end_matches('/').to_string(),
                    description,
                };

                if name.ends_with('/') {
                    structure.subdirs.push(entry);
                } else {
                    structure.files.push(entry);
                }
            }
        }

        structure
    }
}

impl Default for ClaudeMdParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Section representation for CLAUDE.md parsing, storing heading level and raw content lines.
struct ParserSection {
    name: String,
    level: usize,
    content: Vec<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: Returns minimal required sections with Contract/Protocol as None
    fn with_required_sections(base: &str) -> String {
        let mut content = base.to_string();
        if !content.contains("## Domain Context") {
            content.push_str("\n## Domain Context\nNone\n");
        }
        if !content.contains("## Contract") {
            content.push_str("\n## Contract\nNone\n");
        }
        if !content.contains("## Protocol") {
            content.push_str("\n## Protocol\nNone\n");
        }
        content
    }

    #[test]
    fn test_parse_purpose() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test-module

## Purpose
Handles user authentication.

## Exports
- `validate(): void`

## Behavior
- input → output
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.purpose, "Handles user authentication.");
    }

    #[test]
    fn test_parse_typescript_function() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports

### Functions
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid → Claims
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.exports.functions.len(), 1);
        assert_eq!(spec.exports.functions[0].name, "validateToken");
        assert!(spec.exports.functions[0].is_async);
    }

    #[test]
    fn test_parse_dependencies() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Dependencies
- external: jsonwebtoken@9.0.0
- internal: ./types

## Exports
- `validate(): void`

## Behavior
- input → output
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.dependencies.external.len(), 1);
        assert_eq!(spec.dependencies.internal.len(), 1);
        assert_eq!(spec.dependencies.external[0], "jsonwebtoken@9.0.0");
        assert_eq!(spec.dependencies.internal[0].path, "./types");
        assert!(!spec.dependencies.internal[0].is_claude_md_ref);
    }

    #[test]
    fn test_parse_dependencies_list_style_with_claude_md_paths() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Dependencies

- external:
  - `jsonwebtoken@9.0.0`: sign, verify

- internal:
  - `core/domain/transaction/CLAUDE.md`: WithdrawalResultSynchronizer, TransferResultContext
  - `core/support/utils/CLAUDE.md`: RateLimitExecutor

## Exports
- `validate(): void`

## Behavior
- input → output

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let spec = parser.parse_content(content).unwrap();
        assert_eq!(spec.dependencies.external.len(), 1);
        assert_eq!(spec.dependencies.internal.len(), 2);

        // Check first internal dep
        assert_eq!(spec.dependencies.internal[0].path, "core/domain/transaction/CLAUDE.md");
        assert!(spec.dependencies.internal[0].is_claude_md_ref);
        assert_eq!(spec.dependencies.internal[0].symbols.len(), 2);
        assert_eq!(spec.dependencies.internal[0].symbols[0], "WithdrawalResultSynchronizer");

        // Check second internal dep
        assert_eq!(spec.dependencies.internal[1].path, "core/support/utils/CLAUDE.md");
        assert!(spec.dependencies.internal[1].is_claude_md_ref);
    }

    #[test]
    fn test_parse_behaviors() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior

### Success Cases
- valid token → Claims object

### Error Cases
- expired token → TokenExpiredError
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.behaviors.len(), 2);
        assert_eq!(spec.behaviors[0].category, BehaviorCategory::Success);
        assert_eq!(spec.behaviors[1].category, BehaviorCategory::Error);
    }

    #[test]
    fn test_fail_fast_missing_purpose() {
        let parser = ClaudeMdParser::new();
        // Note: Even with Contract/Protocol, missing Purpose should fail
        let content = with_required_sections(
            r#"# test

## Exports
- `validate(): void`

## Behavior
- input → output
"#,
        );
        let result = parser.parse_content(&content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Purpose"));
    }

    #[test]
    fn test_fail_fast_missing_exports() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Behavior
- input → output
"#,
        );
        let result = parser.parse_content(&content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Exports"));
    }

    #[test]
    fn test_fail_fast_missing_behavior() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`
"#,
        );
        let result = parser.parse_content(&content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Behavior"));
    }

    #[test]
    fn test_fail_fast_missing_contract() {
        let parser = ClaudeMdParser::new();
        // Missing Contract section (only Protocol)
        let content = r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output

## Protocol
None
"#;
        let result = parser.parse_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Contract"));
    }

    #[test]
    fn test_fail_fast_missing_protocol() {
        let parser = ClaudeMdParser::new();
        // Missing Protocol section (only Contract + Domain Context)
        let content = r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output

## Domain Context
None

## Contract
None
"#;
        let result = parser.parse_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Protocol"));
    }

    #[test]
    fn test_contract_allows_none() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output
"#,
        );
        // Should pass because Contract/Protocol allow None
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.purpose, "Test module.");
    }

    #[test]
    fn test_parse_generic_type_function() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports

### Functions
- `getCache(key: string): Map<string, List<CacheEntry>>`

## Behavior
- key → cached value
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.exports.functions.len(), 1);
        assert_eq!(spec.exports.functions[0].name, "getCache");
        assert!(spec
            .exports
            .functions[0]
            .signature
            .contains("Map<string, List<CacheEntry>>"));
    }

    #[test]
    fn test_parse_domain_context_full() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output

## Contract
None

## Protocol
None

## Domain Context

### Decision Rationale
- TOKEN_EXPIRY: 7일 (PCI-DSS compliance)
- MAX_RETRY: 3 (외부 API SLA 기반)

### Constraints
- 비밀번호 재설정 90일 제한
- 동시 세션 최대 5개

### Compatibility
- UUID v1 형식 지원 필요
"#;
        let spec = parser.parse_content(content).unwrap();
        let dc = spec.domain_context.expect("domain_context should be Some");
        assert_eq!(dc.decision_rationale.len(), 2);
        assert!(dc.decision_rationale[0].contains("TOKEN_EXPIRY"));
        assert_eq!(dc.constraints.len(), 2);
        assert!(dc.constraints[0].contains("90일"));
        assert_eq!(dc.compatibility.len(), 1);
        assert!(dc.compatibility[0].contains("UUID v1"));
    }

    #[test]
    fn test_parse_domain_context_none() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert!(spec.domain_context.is_none());
    }

    #[test]
    fn test_parse_domain_context_partial() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Exports
- `validate(): void`

## Behavior
- input → output

## Contract
None

## Protocol
None

## Domain Context

### Decision Rationale
- TIMEOUT: 2000ms (IdP SLA × 4)
"#;
        let spec = parser.parse_content(content).unwrap();
        let dc = spec.domain_context.expect("domain_context should be Some");
        assert_eq!(dc.decision_rationale.len(), 1);
        assert!(dc.constraints.is_empty());
        assert!(dc.compatibility.is_empty());
    }

    // --- parse_enum_line tests ---

    #[test]
    fn test_parse_enum_line_colon_style() {
        let parser = ClaudeMdParser::new();
        let result = parser.parse_enum_line("- `Status: Active | Inactive | Pending`");
        assert!(result.is_some());
        let e = result.unwrap();
        assert_eq!(e.name, "Status");
        assert_eq!(e.variants, vec!["Active", "Inactive", "Pending"]);
    }

    #[test]
    fn test_parse_enum_line_equals_style() {
        let parser = ClaudeMdParser::new();
        let result = parser.parse_enum_line("- `Role = Admin | User | Guest`");
        assert!(result.is_some());
        let e = result.unwrap();
        assert_eq!(e.name, "Role");
        assert_eq!(e.variants, vec!["Admin", "User", "Guest"]);
    }

    #[test]
    fn test_parse_enum_line_no_pipe_returns_none() {
        let parser = ClaudeMdParser::new();
        // No pipe-separated variants -> should return None
        let result = parser.parse_enum_line("- `Status: Active`");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_enum_line_empty_name_returns_none() {
        let parser = ClaudeMdParser::new();
        // Empty name before colon -> should return None
        let result = parser.parse_enum_line("- `: A | B`");
        assert!(result.is_none());
    }

    // --- parse_variable_line tests ---

    #[test]
    fn test_parse_variable_line_value_assignment() {
        let parser = ClaudeMdParser::new();
        let result = parser.parse_variable_line("- `MAX_RETRIES = 3`");
        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.name, "MAX_RETRIES");
        assert_eq!(v.value, Some("3".to_string()));
    }

    #[test]
    fn test_parse_variable_line_type_specification() {
        let parser = ClaudeMdParser::new();
        let result = parser.parse_variable_line("- `DEFAULT_TIMEOUT: Duration`");
        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.name, "DEFAULT_TIMEOUT");
        assert_eq!(v.value, Some("Duration".to_string()));
    }

    #[test]
    fn test_parse_variable_line_lowercase_rejected() {
        let parser = ClaudeMdParser::new();
        // Lowercase first char -> should return None (not a constant)
        let result = parser.parse_variable_line("- `maxRetries = 3`");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_variable_line_parenthesis_rejected() {
        let parser = ClaudeMdParser::new();
        // Contains parenthesis -> should return None (looks like function, not variable)
        let result = parser.parse_variable_line("- `Config(value: string): void`");
        assert!(result.is_none());
    }

    #[test]
    fn test_mixed_flat_and_subsection_exports_deduplicated() {
        let parser = ClaudeMdParser::new();
        // ## Exports has flat content AND ### Functions subsection with the same function
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.

## Exports
- `validateToken(token: string): Promise<Claims>`

### Functions
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid → Claims
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        // Should be deduplicated: only 1 function, not 2
        assert_eq!(
            spec.exports.functions.len(),
            1,
            "Expected 1 function after dedup, got {}",
            spec.exports.functions.len()
        );
        assert_eq!(spec.exports.functions[0].name, "validateToken");
    }
}
