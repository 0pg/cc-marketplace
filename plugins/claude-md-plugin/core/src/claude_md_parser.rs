use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

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
    /// Summary - brief 1-2 sentence overview of role/responsibility/features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
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

/// Dependencies specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependenciesSpec {
    pub external: Vec<String>,
    pub internal: Vec<String>,
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
}

impl ClaudeMdParser {
    pub fn new() -> Self {
        Self {
            // Match markdown headers: ## Purpose, ### Functions
            section_pattern: Regex::new(r"^(#{1,4})\s+(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match behavior: input → output or input -> output
            behavior_pattern: Regex::new(r"^[-*]?\s*(.+?)\s*[→\->]+\s*(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match function signature: `funcName(params): ReturnType` or Name(params): Type
            function_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*[:\s]*(.+?)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match type definition: `TypeName { fields }` or TypeName { fields }
            type_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match class: `ClassName(params)` or ClassName(params)
            class_pattern: Regex::new(r"^[-*]?\s*`?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match dependency: external: pkg or internal: path
            dependency_pattern: Regex::new(r"^[-*]?\s*(external|internal):\s*(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match transition: `State` + `trigger` → `NewState`
            transition_pattern: Regex::new(r"^[-*]?\s*`?([^`]+)`?\s*\+\s*`?([^`]+)`?\s*[→\->]+\s*`?([^`]+)`?\s*$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match lifecycle: N. `method` - description
            lifecycle_pattern: Regex::new(r"^(\d+)\.\s*`?([A-Za-z_][A-Za-z0-9_]*(?:\(\))?)`?\s*[-–]\s*(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Match structure: name/: description or name.ext: description
            structure_pattern: Regex::new(r"^[-*]?\s*([A-Za-z0-9_.-]+/?)\s*[:\s]+(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
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

        // Parse Summary section
        if let Some(summary_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Summary")) {
            let summary_text = summary_section.content.join(" ").trim().to_string();
            if !summary_text.is_empty() && !self.is_none_marker(summary_section) {
                spec.summary = Some(summary_text);
            }
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

        // Parse Structure section (optional - not in REQUIRED_SECTIONS)
        if let Some(structure_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Structure")) {
            spec.structure = Some(self.parse_structure(&structure_section.content));
        }

        Ok(spec)
    }

    /// Check if a section contains only a "None" marker (None, N/A, etc.)
    fn is_none_marker(&self, section: &Section) -> bool {
        let non_empty_lines: Vec<&str> = section
            .content
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        // If section has only one non-empty line and it's a none marker
        if non_empty_lines.len() == 1 {
            let line = non_empty_lines[0].to_lowercase();
            return line == "none" || line == "n/a";
        }

        false
    }

    fn extract_sections(&self, content: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut current_section: Option<Section> = None;

        for line in content.lines() {
            if let Some(caps) = self.section_pattern.captures(line) {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let level = caps.get(1).map(|m| m.as_str().len()).unwrap_or(1);
                let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

                current_section = Some(Section {
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

    fn parse_exports(&self, sections: &[Section], spec: &mut ClaudeMdSpec) {
        // Note: existence of Exports section is checked in parse_content (fail-fast)
        let exports_section = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Exports"));

        if exports_section.is_none() {
            return;
        }

        // Find subsections under Exports
        let mut in_functions = false;
        let mut in_types = false;
        let mut in_classes = false;
        let mut in_methods = false;
        let mut in_structs = false;
        let mut in_data_classes = false;

        for section in sections {
            let name_lower = section.name.to_lowercase();

            // Track subsection context
            if name_lower == "functions" || name_lower == "methods" {
                in_functions = true;
                in_types = false;
                in_classes = false;
                in_methods = name_lower == "methods";
                in_structs = false;
                in_data_classes = false;
            } else if name_lower == "types" || name_lower == "structs" {
                in_functions = false;
                in_types = true;
                in_classes = false;
                in_structs = name_lower == "structs";
                in_data_classes = false;
            } else if name_lower == "classes" {
                in_functions = false;
                in_types = false;
                in_classes = true;
                in_structs = false;
                in_data_classes = false;
            } else if name_lower == "data classes" {
                in_functions = false;
                in_types = true;
                in_classes = false;
                in_structs = false;
                in_data_classes = true;
            } else if section.level <= 2 {
                // Reset context on new major section
                in_functions = false;
                in_types = false;
                in_classes = false;
                in_structs = false;
                in_data_classes = false;
                continue;
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
                }
            }
        }

        // If no subsections found, try parsing Exports content directly
        if spec.exports.functions.is_empty()
            && spec.exports.types.is_empty()
            && spec.exports.classes.is_empty()
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
            let generic_end = bracket_utils::find_matching_bracket(name_part, generic_start, '<', '>')?;
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
        let (params, rest) = bracket_utils::extract_parenthesized(cleaned)?;

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
        let (params, rest) = bracket_utils::extract_parenthesized(cleaned)?;

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
        let (params, return_part) = bracket_utils::extract_parenthesized(rest)?;

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

        let (params, rest) = bracket_utils::extract_parenthesized(cleaned)?;

        // Return type in Go can be simple or tuple
        let return_type = rest.trim();
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

        // Python names are snake_case
        if name.chars().any(|c| c.is_uppercase()) && !name.contains('_') {
            return None;
        }

        let (params, rest) = bracket_utils::extract_parenthesized(cleaned)?;

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
        let data_class_pattern = Regex::new(r"^data\s+class\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)$").ok()?;
        if let Some(caps) = data_class_pattern.captures(cleaned) {
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

    fn parse_dependencies(&self, content: &[String]) -> DependenciesSpec {
        let mut deps = DependenciesSpec::default();

        for line in content {
            let trimmed = line.trim();
            if let Some(caps) = self.dependency_pattern.captures(trimmed) {
                let dep_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let dep_value = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                match dep_type {
                    "external" => deps.external.push(dep_value),
                    "internal" => deps.internal.push(dep_value),
                    _ => {}
                }
            }
        }

        deps
    }

    fn parse_behaviors(&self, sections: &[Section], spec: &mut ClaudeMdSpec) {
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
            // Stop if we hit another level 2 section (different major section)
            if section.level <= 2 && !section.name.eq_ignore_ascii_case("Behavior") {
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

    fn parse_contracts(&self, sections: &[Section], spec: &mut ClaudeMdSpec) {
        let contract_section = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Contract"));
        if contract_section.is_none() {
            return;
        }

        let mut current_function = String::new();
        let mut current_contract = ContractSpec {
            function_name: String::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            throws: Vec::new(),
            invariants: Vec::new(),
        };

        for section in sections {
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

    fn parse_protocol(&self, sections: &[Section], spec: &mut ClaudeMdSpec) {
        let protocol_section = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Protocol"));
        if protocol_section.is_none() {
            return;
        }

        let mut protocol = ProtocolSpec::default();
        let mut in_state_machine = false;
        let mut in_lifecycle = false;

        for section in sections {
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

/// Internal section representation
struct Section {
    name: String,
    level: usize,
    content: Vec<String>,
}

/// Utility functions for parsing complex types with balanced brackets
mod bracket_utils {
    /// Find the index of a closing bracket that matches the opening bracket at start_idx.
    /// Handles nested brackets of the same type.
    /// Returns None if no matching bracket is found.
    pub fn find_matching_bracket(s: &str, start_idx: usize, open: char, close: char) -> Option<usize> {
        let chars: Vec<char> = s.chars().collect();
        if start_idx >= chars.len() || chars[start_idx] != open {
            return None;
        }

        let mut depth = 0;
        for (i, c) in chars.iter().enumerate().skip(start_idx) {
            if *c == open {
                depth += 1;
            } else if *c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Extract content between parentheses, respecting nested brackets.
    /// Returns (params_content, rest_of_string) or None if malformed.
    pub fn extract_parenthesized(s: &str) -> Option<(String, String)> {
        let paren_start = s.find('(')?;
        let paren_end = find_matching_bracket(s, paren_start, '(', ')')?;

        let params = s[paren_start + 1..paren_end].to_string();
        let rest = s[paren_end + 1..].trim().to_string();

        Some((params, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: Returns minimal required sections with Contract/Protocol/Domain Context as None
    fn with_required_sections(base: &str) -> String {
        let mut content = base.to_string();
        // Add Summary right after Purpose if not present
        if !content.contains("## Summary") {
            // Insert Summary after Purpose section
            if let Some(pos) = content.find("## Exports") {
                content.insert_str(pos, "## Summary\nTest module summary.\n\n");
            } else {
                content.push_str("\n## Summary\nTest module summary.\n");
            }
        }
        if !content.contains("## Contract") {
            content.push_str("\n## Contract\nNone\n");
        }
        if !content.contains("## Protocol") {
            content.push_str("\n## Protocol\nNone\n");
        }
        if !content.contains("## Domain Context") {
            content.push_str("\n## Domain Context\nNone\n");
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
        // Missing Contract section (has Summary, Protocol, Domain Context)
        let content = r#"# test

## Purpose
Test module.

## Summary
Test module summary.

## Exports
- `validate(): void`

## Behavior
- input → output

## Protocol
None

## Domain Context
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
        // Missing Protocol section (has Summary, Contract, Domain Context)
        let content = r#"# test

## Purpose
Test module.

## Summary
Test module summary.

## Exports
- `validate(): void`

## Behavior
- input → output

## Contract
None

## Domain Context
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
}
