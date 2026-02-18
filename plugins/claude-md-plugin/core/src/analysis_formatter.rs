//! Formatter for converting `analyze-code` full AnalysisResult into compact CLAUDE.md-ready markdown.
//!
//! Converts `code_analyzer::AnalysisResult` into a compact markdown summary covering
//! Exports, Behaviors, Dependencies, Contracts, Protocol, and Analyzed Files.
//! Intended for LLM consumption — empty sections are omitted entirely.

use crate::code_analyzer::{
    AnalysisResult, Behavior, BehaviorCategory, Contract, Dependencies, FunctionContract,
    InternalDependency, Protocol, ResolutionStatus,
};
use crate::exports_formatter;

/// Formats an `AnalysisResult` into compact markdown summary.
///
/// Empty sections (no data) are omitted entirely.
/// Exports section reuses `exports_formatter::format_exports()`.
pub fn format_analysis(analysis: &AnalysisResult) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("# Analysis Summary: {}", analysis.path));

    // Exports (reuse existing formatter)
    let exports_md = exports_formatter::format_exports(&analysis.exports);
    if exports_md != "None" {
        sections.push(format!("## Exports\n\n{exports_md}"));
    }

    // Behaviors
    if let Some(s) = format_behaviors(&analysis.behaviors) {
        sections.push(s);
    }

    // Dependencies
    if let Some(s) = format_dependencies(&analysis.dependencies) {
        sections.push(s);
    }

    // Contracts
    if let Some(s) = format_contracts(&analysis.contracts) {
        sections.push(s);
    }

    // Protocol
    if let Some(s) = format_protocol(&analysis.protocol) {
        sections.push(s);
    }

    // Analyzed Files
    if !analysis.analyzed_files.is_empty() {
        sections.push(format!(
            "## Analyzed Files\n\n{}",
            analysis.analyzed_files.join(", ")
        ));
    }

    sections.join("\n\n")
}

/// Formats behaviors grouped by category (Success/Error).
fn format_behaviors(behaviors: &[Behavior]) -> Option<String> {
    if behaviors.is_empty() {
        return None;
    }

    let success: Vec<&Behavior> = behaviors
        .iter()
        .filter(|b| b.category == BehaviorCategory::Success)
        .collect();
    let error: Vec<&Behavior> = behaviors
        .iter()
        .filter(|b| b.category == BehaviorCategory::Error)
        .collect();

    let has_both = !success.is_empty() && !error.is_empty();
    let mut lines = vec!["## Behaviors".to_string(), String::new()];

    if has_both {
        // Use subsections
        lines.push("### Success".to_string());
        lines.push(String::new());
        for b in &success {
            lines.push(format!("- {} → {}", b.input, b.output));
        }
        lines.push(String::new());
        lines.push("### Error".to_string());
        lines.push(String::new());
        for b in &error {
            lines.push(format!("- {} → {}", b.input, b.output));
        }
    } else {
        // Single category — flat list (no subsection header)
        let all: Vec<&Behavior> = if !success.is_empty() {
            success
        } else {
            error
        };
        for b in &all {
            lines.push(format!("- {} → {}", b.input, b.output));
        }
    }

    Some(lines.join("\n"))
}

/// Formats dependencies (external + internal).
fn format_dependencies(deps: &Dependencies) -> Option<String> {
    let has_external = !deps.external.is_empty();
    let has_internal = !deps.internal.is_empty();

    if !has_external && !has_internal {
        return None;
    }

    let mut lines = vec!["## Dependencies".to_string(), String::new()];

    if has_external {
        lines.push("### External".to_string());
        lines.push(String::new());
        let mut sorted = deps.external.clone();
        sorted.sort();
        for dep in &sorted {
            lines.push(format!("- {dep}"));
        }
    }

    if has_internal {
        if has_external {
            lines.push(String::new());
        }
        lines.push("### Internal".to_string());
        lines.push(String::new());

        // Group by resolution status
        let mut exact: Vec<&InternalDependency> = Vec::new();
        let mut ancestor: Vec<&InternalDependency> = Vec::new();
        let mut unresolved: Vec<&InternalDependency> = Vec::new();

        for dep in &deps.internal {
            match &dep.resolution {
                ResolutionStatus::Exact => exact.push(dep),
                ResolutionStatus::Ancestor { .. } => ancestor.push(dep),
                ResolutionStatus::Unresolved => unresolved.push(dep),
            }
        }

        // Sort each group by claude_md_path/raw_import
        exact.sort_by(|a, b| a.claude_md_path.cmp(&b.claude_md_path));
        ancestor.sort_by(|a, b| a.claude_md_path.cmp(&b.claude_md_path));
        unresolved.sort_by(|a, b| a.raw_import.cmp(&b.raw_import));

        for dep in &exact {
            lines.push(format!("- `{}` (Exact)", dep.claude_md_path));
        }
        for dep in &ancestor {
            if let ResolutionStatus::Ancestor { distance } = &dep.resolution {
                lines.push(format!(
                    "- `{}` (Ancestor, distance={})",
                    dep.claude_md_path, distance
                ));
            }
        }
        for dep in &unresolved {
            lines.push(format!("- `{}` (Unresolved)", dep.raw_import));
        }
    }

    Some(lines.join("\n"))
}

/// Formats function contracts.
fn format_contracts(contracts: &[FunctionContract]) -> Option<String> {
    if contracts.is_empty() {
        return None;
    }

    let mut lines = vec!["## Contracts".to_string(), String::new()];

    let mut sorted: Vec<&FunctionContract> = contracts.iter().collect();
    sorted.sort_by(|a, b| a.function_name.cmp(&b.function_name));

    for fc in &sorted {
        lines.push(format!("### {}", fc.function_name));
        lines.push(String::new());
        append_contract_fields(&mut lines, &fc.contract);
    }

    Some(lines.join("\n"))
}

fn append_contract_fields(lines: &mut Vec<String>, contract: &Contract) {
    if !contract.preconditions.is_empty() {
        for pre in &contract.preconditions {
            lines.push(format!("- Pre: {pre}"));
        }
    }
    if !contract.postconditions.is_empty() {
        for post in &contract.postconditions {
            lines.push(format!("- Post: {post}"));
        }
    }
    if !contract.invariants.is_empty() {
        for inv in &contract.invariants {
            lines.push(format!("- Invariant: {inv}"));
        }
    }
    if !contract.throws.is_empty() {
        for thr in &contract.throws {
            lines.push(format!("- Throws: {thr}"));
        }
    }
}

/// Formats protocol (states, transitions, lifecycle).
fn format_protocol(protocol: &Option<Protocol>) -> Option<String> {
    let proto = protocol.as_ref()?;

    let has_states = !proto.states.is_empty();
    let has_transitions = !proto.transitions.is_empty();
    let has_lifecycle = !proto.lifecycle.is_empty();

    if !has_states && !has_transitions && !has_lifecycle {
        return None;
    }

    let mut lines = vec!["## Protocol".to_string(), String::new()];

    if has_states {
        lines.push("### States".to_string());
        lines.push(String::new());
        lines.push(proto.states.join(" → "));
    }

    if has_transitions {
        if has_states {
            lines.push(String::new());
        }
        lines.push("### Transitions".to_string());
        lines.push(String::new());
        for t in &proto.transitions {
            lines.push(format!("- {} → {} ({})", t.from, t.to, t.trigger));
        }
    }

    if has_lifecycle {
        if has_states || has_transitions {
            lines.push(String::new());
        }
        lines.push("### Lifecycle".to_string());
        lines.push(String::new());
        for (i, step) in proto.lifecycle.iter().enumerate() {
            lines.push(format!("{}. {step}", i + 1));
        }
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_analyzer::{
        ExportedFunction, Exports, StateTransition,
    };

    fn empty_analysis() -> AnalysisResult {
        AnalysisResult {
            path: "src/test".to_string(),
            exports: Exports::default(),
            dependencies: Dependencies::default(),
            behaviors: Vec::new(),
            contracts: Vec::new(),
            protocol: None,
            analyzed_files: Vec::new(),
        }
    }

    #[test]
    fn test_empty_analysis_minimal_output() {
        let result = format_analysis(&empty_analysis());
        assert_eq!(result, "# Analysis Summary: src/test");
    }

    #[test]
    fn test_exports_section_included() {
        let mut analysis = empty_analysis();
        analysis.exports.functions.push(ExportedFunction {
            name: "greet".to_string(),
            signature: "greet(name: string): string".to_string(),
            description: None,
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("## Exports"));
        assert!(result.contains("- `greet(name: string): string`"));
    }

    #[test]
    fn test_behaviors_both_categories() {
        let mut analysis = empty_analysis();
        analysis.behaviors.push(Behavior {
            input: "valid token".to_string(),
            output: "Claims object".to_string(),
            category: BehaviorCategory::Success,
        });
        analysis.behaviors.push(Behavior {
            input: "expired token".to_string(),
            output: "TokenExpiredError".to_string(),
            category: BehaviorCategory::Error,
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("### Success"));
        assert!(result.contains("### Error"));
        assert!(result.contains("- valid token → Claims object"));
        assert!(result.contains("- expired token → TokenExpiredError"));
    }

    #[test]
    fn test_behaviors_single_category_flat() {
        let mut analysis = empty_analysis();
        analysis.behaviors.push(Behavior {
            input: "valid input".to_string(),
            output: "result".to_string(),
            category: BehaviorCategory::Success,
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("## Behaviors"));
        assert!(result.contains("- valid input → result"));
        assert!(!result.contains("### Success"));
    }

    #[test]
    fn test_dependencies_external_only() {
        let mut analysis = empty_analysis();
        analysis.dependencies.external = vec!["jsonwebtoken".to_string(), "axios".to_string()];
        let result = format_analysis(&analysis);
        assert!(result.contains("### External"));
        // Sorted alphabetically
        let axios_pos = result.find("- axios").unwrap();
        let jwt_pos = result.find("- jsonwebtoken").unwrap();
        assert!(axios_pos < jwt_pos);
        assert!(!result.contains("### Internal"));
    }

    #[test]
    fn test_dependencies_internal_grouped_by_resolution() {
        let mut analysis = empty_analysis();
        analysis.dependencies.internal = vec![
            InternalDependency {
                raw_import: "./types".to_string(),
                resolved_dir: "src/auth/types".to_string(),
                claude_md_path: "src/auth/types/CLAUDE.md".to_string(),
                resolution: ResolutionStatus::Exact,
                is_child: true,
            },
            InternalDependency {
                raw_import: "../legacy".to_string(),
                resolved_dir: String::new(),
                claude_md_path: String::new(),
                resolution: ResolutionStatus::Unresolved,
                is_child: false,
            },
        ];
        let result = format_analysis(&analysis);
        assert!(result.contains("`src/auth/types/CLAUDE.md` (Exact)"));
        assert!(result.contains("`../legacy` (Unresolved)"));
    }

    #[test]
    fn test_contracts_with_multiple_fields() {
        let mut analysis = empty_analysis();
        analysis.contracts.push(FunctionContract {
            function_name: "validateToken".to_string(),
            contract: Contract {
                preconditions: vec!["token must be non-empty".to_string()],
                postconditions: Vec::new(),
                invariants: Vec::new(),
                throws: vec!["InvalidTokenError".to_string(), "SignatureError".to_string()],
            },
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("### validateToken"));
        assert!(result.contains("- Pre: token must be non-empty"));
        assert!(result.contains("- Throws: InvalidTokenError"));
        assert!(result.contains("- Throws: SignatureError"));
        assert!(!result.contains("- Post:"));
        assert!(!result.contains("- Invariant:"));
    }

    #[test]
    fn test_contracts_sorted_by_name() {
        let mut analysis = empty_analysis();
        analysis.contracts.push(FunctionContract {
            function_name: "zebra".to_string(),
            contract: Contract {
                preconditions: vec!["z".to_string()],
                ..Contract::default()
            },
        });
        analysis.contracts.push(FunctionContract {
            function_name: "alpha".to_string(),
            contract: Contract {
                preconditions: vec!["a".to_string()],
                ..Contract::default()
            },
        });
        let result = format_analysis(&analysis);
        let alpha_pos = result.find("### alpha").unwrap();
        let zebra_pos = result.find("### zebra").unwrap();
        assert!(alpha_pos < zebra_pos);
    }

    #[test]
    fn test_protocol_states_and_lifecycle() {
        let mut analysis = empty_analysis();
        analysis.protocol = Some(Protocol {
            states: vec![
                "Idle".to_string(),
                "Loading".to_string(),
                "Loaded".to_string(),
            ],
            transitions: Vec::new(),
            lifecycle: vec![
                "init()".to_string(),
                "start()".to_string(),
                "stop()".to_string(),
            ],
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("### States"));
        assert!(result.contains("Idle → Loading → Loaded"));
        assert!(result.contains("### Lifecycle"));
        assert!(result.contains("1. init()"));
        assert!(result.contains("2. start()"));
        assert!(result.contains("3. stop()"));
    }

    #[test]
    fn test_protocol_with_transitions() {
        let mut analysis = empty_analysis();
        analysis.protocol = Some(Protocol {
            states: vec!["A".to_string(), "B".to_string()],
            transitions: vec![StateTransition {
                from: "A".to_string(),
                to: "B".to_string(),
                trigger: "event".to_string(),
            }],
            lifecycle: Vec::new(),
        });
        let result = format_analysis(&analysis);
        assert!(result.contains("### Transitions"));
        assert!(result.contains("- A → B (event)"));
    }

    #[test]
    fn test_protocol_none_omitted() {
        let analysis = empty_analysis();
        let result = format_analysis(&analysis);
        assert!(!result.contains("## Protocol"));
    }

    #[test]
    fn test_protocol_empty_fields_omitted() {
        let mut analysis = empty_analysis();
        analysis.protocol = Some(Protocol {
            states: Vec::new(),
            transitions: Vec::new(),
            lifecycle: Vec::new(),
        });
        let result = format_analysis(&analysis);
        assert!(!result.contains("## Protocol"));
    }

    #[test]
    fn test_analyzed_files() {
        let mut analysis = empty_analysis();
        analysis.analyzed_files = vec![
            "index.ts".to_string(),
            "middleware.ts".to_string(),
            "types.ts".to_string(),
        ];
        let result = format_analysis(&analysis);
        assert!(result.contains("## Analyzed Files"));
        assert!(result.contains("index.ts, middleware.ts, types.ts"));
    }

    #[test]
    fn test_empty_sections_omitted() {
        let analysis = empty_analysis();
        let result = format_analysis(&analysis);
        assert!(!result.contains("## Exports"));
        assert!(!result.contains("## Behaviors"));
        assert!(!result.contains("## Dependencies"));
        assert!(!result.contains("## Contracts"));
        assert!(!result.contains("## Protocol"));
        assert!(!result.contains("## Analyzed Files"));
    }

    #[test]
    fn test_comprehensive_output() {
        let mut analysis = empty_analysis();
        analysis.path = "src/auth".to_string();

        analysis.exports.functions.push(ExportedFunction {
            name: "validateToken".to_string(),
            signature: "validateToken(token: string): Promise<Claims>".to_string(),
            description: None,
        });

        analysis.behaviors.push(Behavior {
            input: "valid JWT".to_string(),
            output: "Claims".to_string(),
            category: BehaviorCategory::Success,
        });
        analysis.behaviors.push(Behavior {
            input: "invalid JWT".to_string(),
            output: "InvalidTokenError".to_string(),
            category: BehaviorCategory::Error,
        });

        analysis.dependencies.external = vec!["jsonwebtoken".to_string()];

        analysis.contracts.push(FunctionContract {
            function_name: "validateToken".to_string(),
            contract: Contract {
                preconditions: vec!["token must be non-empty string".to_string()],
                postconditions: Vec::new(),
                invariants: Vec::new(),
                throws: vec!["InvalidTokenError".to_string()],
            },
        });

        analysis.analyzed_files = vec!["index.ts".to_string(), "middleware.ts".to_string()];

        let result = format_analysis(&analysis);
        assert!(result.starts_with("# Analysis Summary: src/auth"));
        assert!(result.contains("## Exports"));
        assert!(result.contains("## Behaviors"));
        assert!(result.contains("## Dependencies"));
        assert!(result.contains("## Contracts"));
        assert!(result.contains("## Analyzed Files"));
    }
}
