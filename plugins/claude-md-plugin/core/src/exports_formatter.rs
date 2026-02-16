//! Deterministic formatter for converting `analyze-code` exports into CLAUDE.md Exports markdown.
//!
//! Converts `code_analyzer::Exports` into a deterministic markdown representation
//! suitable for direct insertion into a CLAUDE.md `## Exports` section.
//! Guarantees: same input → same output (sorted, fixed category order, deterministic format).

use crate::code_analyzer::{
    ExportedClass, ExportedEnum, ExportedFunction, ExportedType, ExportedVariable, Exports,
    ReExport,
};

/// Category order for subsection rendering (fixed).
const CATEGORY_ORDER: &[&str] = &[
    "Functions",
    "Types",
    "Classes",
    "Enums",
    "Variables",
    "Re-exports",
];

/// Formats an `Exports` struct into CLAUDE.md Exports section markdown.
///
/// # Rules
/// - 2+ non-empty categories → subsection headers (`### Functions`, etc.)
/// - 1 category → flat list (no subsection header)
/// - 0 items across all categories → `"None"`
/// - Items within each category sorted alphabetically by name
/// - Category order: Functions → Types → Classes → Enums → Variables → Re-exports
pub fn format_exports(exports: &Exports) -> String {
    let categories = build_categories(exports);
    let non_empty: Vec<&(&str, Vec<String>)> =
        categories.iter().filter(|(_, items)| !items.is_empty()).collect();

    if non_empty.is_empty() {
        return "None".to_string();
    }

    let mut lines = Vec::new();

    if non_empty.len() == 1 {
        // Flat list — no subsection header
        let (_, items) = non_empty[0];
        for item in items {
            lines.push(item.clone());
        }
    } else {
        // Multiple categories — use subsection headers
        for (i, (name, items)) in non_empty.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.push(format!("### {name}"));
            lines.push(String::new());
            for item in items.iter() {
                lines.push(item.clone());
            }
        }
    }

    lines.join("\n")
}

/// Builds category entries in fixed order, each with sorted formatted items.
fn build_categories(exports: &Exports) -> Vec<(&'static str, Vec<String>)> {
    CATEGORY_ORDER
        .iter()
        .map(|&name| {
            let items = match name {
                "Functions" => format_functions(&exports.functions),
                "Types" => format_types(&exports.types),
                "Classes" => format_classes(&exports.classes),
                "Enums" => format_enums(&exports.enums),
                "Variables" => format_variables(&exports.variables),
                "Re-exports" => format_re_exports(&exports.re_exports),
                _ => Vec::new(),
            };
            (name, items)
        })
        .collect()
}

/// Formats exported functions, sorted by name.
fn format_functions(functions: &[ExportedFunction]) -> Vec<String> {
    let mut sorted: Vec<&ExportedFunction> = functions.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted.iter().map(|f| format!("- `{}`", f.signature)).collect()
}

/// Formats exported types, sorted by name.
fn format_types(types: &[ExportedType]) -> Vec<String> {
    let mut sorted: Vec<&ExportedType> = types.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|t| match &t.definition {
            Some(def) if !def.is_empty() => format!("- `{} {{ {} }}`", t.name, def),
            _ => format!("- `{}`", t.name),
        })
        .collect()
}

/// Formats exported classes, sorted by name.
fn format_classes(classes: &[ExportedClass]) -> Vec<String> {
    let mut sorted: Vec<&ExportedClass> = classes.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|c| match &c.signature {
            Some(sig) if !sig.is_empty() => format!("- `{sig}`"),
            _ => format!("- `{}`", c.name),
        })
        .collect()
}

/// Formats exported enums, sorted by name.
fn format_enums(enums: &[ExportedEnum]) -> Vec<String> {
    let mut sorted: Vec<&ExportedEnum> = enums.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|e| match &e.variants {
            Some(variants) if !variants.is_empty() => {
                format!("- `{}: {}`", e.name, variants.join(" | "))
            }
            _ => format!("- `{}`", e.name),
        })
        .collect()
}

/// Formats exported variables, sorted by name.
fn format_variables(variables: &[ExportedVariable]) -> Vec<String> {
    let mut sorted: Vec<&ExportedVariable> = variables.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|v| match &v.var_type {
            Some(vt) if !vt.is_empty() => format!("- `{}: {}`", v.name, vt),
            _ => format!("- `{}`", v.name),
        })
        .collect()
}

/// Formats re-exported symbols, sorted by name.
fn format_re_exports(re_exports: &[ReExport]) -> Vec<String> {
    let mut sorted: Vec<&ReExport> = re_exports.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|r| format!("- `{}` (from `{}`)", r.name, r.source))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_analyzer::TypeKind;

    fn empty_exports() -> Exports {
        Exports::default()
    }

    #[test]
    fn test_empty_exports_returns_none() {
        let result = format_exports(&empty_exports());
        assert_eq!(result, "None");
    }

    #[test]
    fn test_single_function_flat_list() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "greet".to_string(),
            signature: "greet(name: string): string".to_string(),
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `greet(name: string): string`");
    }

    #[test]
    fn test_multiple_functions_flat_list_sorted() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "zebra".to_string(),
            signature: "zebra(): void".to_string(),
            description: None,
        });
        exports.functions.push(ExportedFunction {
            name: "alpha".to_string(),
            signature: "alpha(): void".to_string(),
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `alpha(): void`\n- `zebra(): void`");
    }

    #[test]
    fn test_multiple_categories_use_subsections() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "doWork".to_string(),
            signature: "doWork(): void".to_string(),
            description: None,
        });
        exports.types.push(ExportedType {
            name: "Config".to_string(),
            kind: TypeKind::Interface,
            definition: Some("timeout: number".to_string()),
            description: None,
        });
        let result = format_exports(&exports);
        let expected = "\
### Functions

- `doWork(): void`

### Types

- `Config { timeout: number }`";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_type_without_definition() {
        let mut exports = empty_exports();
        exports.types.push(ExportedType {
            name: "Opaque".to_string(),
            kind: TypeKind::Type,
            definition: None,
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `Opaque`");
    }

    #[test]
    fn test_type_with_empty_definition() {
        let mut exports = empty_exports();
        exports.types.push(ExportedType {
            name: "Empty".to_string(),
            kind: TypeKind::Struct,
            definition: Some(String::new()),
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `Empty`");
    }

    #[test]
    fn test_class_with_signature() {
        let mut exports = empty_exports();
        exports.classes.push(ExportedClass {
            name: "UserService".to_string(),
            signature: Some("class UserService extends BaseService".to_string()),
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `class UserService extends BaseService`");
    }

    #[test]
    fn test_class_without_signature() {
        let mut exports = empty_exports();
        exports.classes.push(ExportedClass {
            name: "SimpleClass".to_string(),
            signature: None,
            description: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `SimpleClass`");
    }

    #[test]
    fn test_enum_with_variants() {
        let mut exports = empty_exports();
        exports.enums.push(ExportedEnum {
            name: "Status".to_string(),
            variants: Some(vec![
                "Active".to_string(),
                "Inactive".to_string(),
                "Pending".to_string(),
            ]),
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `Status: Active | Inactive | Pending`");
    }

    #[test]
    fn test_enum_without_variants() {
        let mut exports = empty_exports();
        exports.enums.push(ExportedEnum {
            name: "Color".to_string(),
            variants: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `Color`");
    }

    #[test]
    fn test_variable_with_type() {
        let mut exports = empty_exports();
        exports.variables.push(ExportedVariable {
            name: "MAX_RETRIES".to_string(),
            var_type: Some("number".to_string()),
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `MAX_RETRIES: number`");
    }

    #[test]
    fn test_variable_without_type() {
        let mut exports = empty_exports();
        exports.variables.push(ExportedVariable {
            name: "DEFAULT_NAME".to_string(),
            var_type: None,
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `DEFAULT_NAME`");
    }

    #[test]
    fn test_re_export() {
        let mut exports = empty_exports();
        exports.re_exports.push(ReExport {
            name: "helper".to_string(),
            source: "./utils".to_string(),
        });
        let result = format_exports(&exports);
        assert_eq!(result, "- `helper` (from `./utils`)");
    }

    #[test]
    fn test_determinism_multiple_runs() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "beta".to_string(),
            signature: "beta(): void".to_string(),
            description: None,
        });
        exports.functions.push(ExportedFunction {
            name: "alpha".to_string(),
            signature: "alpha(): void".to_string(),
            description: None,
        });
        exports.types.push(ExportedType {
            name: "Zeta".to_string(),
            kind: TypeKind::Interface,
            definition: Some("x: number".to_string()),
            description: None,
        });
        exports.types.push(ExportedType {
            name: "Alpha".to_string(),
            kind: TypeKind::Type,
            definition: None,
            description: None,
        });

        let run1 = format_exports(&exports);
        let run2 = format_exports(&exports);
        assert_eq!(run1, run2, "format_exports must be deterministic");
    }

    #[test]
    fn test_category_order() {
        let mut exports = empty_exports();
        // Add in reverse order to verify category ordering is fixed
        exports.re_exports.push(ReExport {
            name: "reExported".to_string(),
            source: "./mod".to_string(),
        });
        exports.variables.push(ExportedVariable {
            name: "VAR".to_string(),
            var_type: Some("string".to_string()),
        });
        exports.enums.push(ExportedEnum {
            name: "Dir".to_string(),
            variants: Some(vec!["Up".to_string(), "Down".to_string()]),
        });
        exports.classes.push(ExportedClass {
            name: "Svc".to_string(),
            signature: None,
            description: None,
        });
        exports.types.push(ExportedType {
            name: "Cfg".to_string(),
            kind: TypeKind::Interface,
            definition: None,
            description: None,
        });
        exports.functions.push(ExportedFunction {
            name: "run".to_string(),
            signature: "run(): void".to_string(),
            description: None,
        });

        let result = format_exports(&exports);
        let sections: Vec<&str> = result
            .lines()
            .filter(|l| l.starts_with("### "))
            .collect();
        assert_eq!(
            sections,
            vec![
                "### Functions",
                "### Types",
                "### Classes",
                "### Enums",
                "### Variables",
                "### Re-exports",
            ]
        );
    }

    #[test]
    fn test_empty_categories_skipped() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "run".to_string(),
            signature: "run(): void".to_string(),
            description: None,
        });
        // types, classes, enums, variables empty
        exports.re_exports.push(ReExport {
            name: "util".to_string(),
            source: "./util".to_string(),
        });

        let result = format_exports(&exports);
        assert!(result.contains("### Functions"));
        assert!(result.contains("### Re-exports"));
        assert!(!result.contains("### Types"));
        assert!(!result.contains("### Classes"));
        assert!(!result.contains("### Enums"));
        assert!(!result.contains("### Variables"));
    }

    #[test]
    fn test_all_categories_comprehensive() {
        let mut exports = empty_exports();
        exports.functions.push(ExportedFunction {
            name: "fetchData".to_string(),
            signature: "async fetchData(url: string): Promise<Response>".to_string(),
            description: Some("Fetches data from URL".to_string()),
        });
        exports.functions.push(ExportedFunction {
            name: "createUser".to_string(),
            signature: "createUser(name: string, email: string): User".to_string(),
            description: None,
        });
        exports.types.push(ExportedType {
            name: "UserConfig".to_string(),
            kind: TypeKind::Interface,
            definition: Some("name: string, email: string".to_string()),
            description: None,
        });
        exports.classes.push(ExportedClass {
            name: "AuthService".to_string(),
            signature: Some("class AuthService".to_string()),
            description: None,
        });
        exports.enums.push(ExportedEnum {
            name: "Role".to_string(),
            variants: Some(vec![
                "Admin".to_string(),
                "User".to_string(),
                "Guest".to_string(),
            ]),
        });
        exports.variables.push(ExportedVariable {
            name: "API_VERSION".to_string(),
            var_type: Some("string".to_string()),
        });
        exports.re_exports.push(ReExport {
            name: "Logger".to_string(),
            source: "./logger".to_string(),
        });

        let result = format_exports(&exports);
        let expected = "\
### Functions

- `createUser(name: string, email: string): User`
- `async fetchData(url: string): Promise<Response>`

### Types

- `UserConfig { name: string, email: string }`

### Classes

- `class AuthService`

### Enums

- `Role: Admin | User | Guest`

### Variables

- `API_VERSION: string`

### Re-exports

- `Logger` (from `./logger`)";
        assert_eq!(result, expected);
    }
}
