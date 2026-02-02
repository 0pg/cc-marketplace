//! Python code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedClass, ExportedFunction,
    LanguageAnalyzer, PartialAnalysis,
};

/// Analyzer for Python files.
#[derive(Debug)]
pub struct PythonAnalyzer {
    // Regex patterns for Python analysis
    all_re: Regex,
    def_re: Regex,
    class_re: Regex,
    import_re: Regex,
    from_import_re: Regex,
    raise_re: Regex,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {
            // __all__ = ['name1', 'name2']
            all_re: Regex::new(
                r"__all__\s*=\s*\[([^\]]+)\]"
            ).unwrap(),

            // def function_name(params):
            def_re: Regex::new(
                r"(?m)^def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^:]+))?\s*:"
            ).unwrap(),

            // class ClassName:
            class_re: Regex::new(
                r"(?m)^class\s+(\w+)(?:\([^)]*\))?\s*:"
            ).unwrap(),

            // import package
            import_re: Regex::new(
                r"(?m)^import\s+(\w+)"
            ).unwrap(),

            // from package import ...
            from_import_re: Regex::new(
                r"(?m)^from\s+([\w.]+)\s+import"
            ).unwrap(),

            // raise ExceptionName(...)
            raise_re: Regex::new(
                r"raise\s+(?:(\w+)\.)?(\w+)"
            ).unwrap(),
        }
    }

    /// Check if a function name is public (doesn't start with _)
    fn is_public(&self, name: &str) -> bool {
        !name.starts_with('_')
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Check if this is __init__.py with __all__
        let is_init = path.file_name()
            .map(|s| s.to_string_lossy() == "__init__.py")
            .unwrap_or(false);

        // Extract __all__ symbols if present
        let all_symbols: Vec<String> = if let Some(cap) = self.all_re.captures(content) {
            let symbols_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            symbols_str
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        // Extract functions
        for cap in self.def_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip private functions
            if !self.is_public(name) {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim());

            let signature = if let Some(ret) = return_type {
                format!("{}({}) -> {}", name, params, ret)
            } else {
                format!("{}({})", name, params)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract classes
        for cap in self.class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip private classes
            if !self.is_public(name) {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            analysis.classes.push(ExportedClass {
                name: name.to_string(),
                signature: Some(format!("class {}", name)),
                description: None,
            });
        }

        // Extract dependencies
        for cap in self.import_re.captures_iter(content) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if !analysis.external_deps.contains(&package.to_string()) {
                analysis.external_deps.push(package.to_string());
            }
        }

        for cap in self.from_import_re.captures_iter(content) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if package.starts_with('.') {
                if !analysis.internal_deps.contains(&package.to_string()) {
                    analysis.internal_deps.push(package.to_string());
                }
            } else {
                let pkg_name = package.split('.').next().unwrap_or(package);
                if !analysis.external_deps.contains(&pkg_name.to_string()) {
                    analysis.external_deps.push(pkg_name.to_string());
                }
            }
        }

        // Infer behaviors from exception handling
        for cap in self.raise_re.captures_iter(content) {
            let module = cap.get(1).map(|m| m.as_str());
            let exc_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let error_name = if let Some(mod_name) = module {
                format!("{}.{}", mod_name, exc_name)
            } else {
                exc_name.to_string()
            };

            let input = if exc_name.to_lowercase().contains("expired") || error_name.contains("ExpiredSignature") {
                "Expired token"
            } else if exc_name.to_lowercase().contains("invalid") || error_name.contains("InvalidToken") {
                "Invalid token"
            } else {
                continue;
            };

            // Avoid duplicates
            if !analysis.behaviors.iter().any(|b| b.input == input) {
                analysis.behaviors.push(Behavior {
                    input: input.to_string(),
                    output: error_name,
                    category: BehaviorCategory::Error,
                });
            }
        }

        Ok(analysis)
    }

    fn extensions(&self) -> &[&str] {
        &["py"]
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
