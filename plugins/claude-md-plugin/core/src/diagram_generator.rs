use crate::claude_md_parser::{ActorSpec, BehaviorSpec, ClaudeMdSpec, UseCaseSpec};
use crate::dependency_graph::DependencyGraphResult;

/// Generator for Mermaid diagrams from CLAUDE.md specifications
pub struct DiagramGenerator;

impl DiagramGenerator {
    /// Generate a UseCase diagram (Mermaid flowchart LR) from Behavior section.
    ///
    /// v2 (Actors + UC): Uses declared actors and use cases
    /// v1 (flat Behavior): Single "User" actor + auto-grouped behaviors
    pub fn generate_usecase(spec: &ClaudeMdSpec) -> String {
        let mut lines = vec!["flowchart LR".to_string()];

        if !spec.actors.is_empty() && !spec.use_cases.is_empty() {
            // v2 mode: structured actors and use cases
            Self::generate_usecase_v2(&spec.actors, &spec.use_cases, &mut lines);
        } else if !spec.behaviors.is_empty() {
            // v1 mode: auto-generate from flat behaviors
            Self::generate_usecase_v1(&spec.behaviors, &mut lines);
        }

        lines.join("\n")
    }

    /// Generate a State diagram (Mermaid stateDiagram-v2) from Protocol section.
    ///
    /// Returns None if no states/transitions are defined.
    pub fn generate_state(spec: &ClaudeMdSpec) -> Option<String> {
        let protocol = spec.protocol.as_ref()?;

        if protocol.states.is_empty() && protocol.transitions.is_empty() {
            return None;
        }

        let mut lines = vec!["stateDiagram-v2".to_string()];

        // Add initial state transition if we have states
        if !protocol.states.is_empty() {
            lines.push(format!("    [*] --> {}", protocol.states[0]));
        }

        // Add transitions
        for transition in &protocol.transitions {
            lines.push(format!(
                "    {} --> {} : {}",
                transition.from, transition.to, transition.trigger
            ));
        }

        Some(lines.join("\n"))
    }

    /// Generate a Component diagram (Mermaid flowchart TB) from DependencyGraphResult.
    pub fn generate_component(graph: &DependencyGraphResult) -> String {
        let mut lines = vec!["flowchart TB".to_string()];

        // Create subgraphs for each module node
        for node in &graph.nodes {
            let safe_id = Self::sanitize_id(&node.path);
            lines.push(format!("    subgraph {}[\"{}\"]", safe_id, node.path));
            lines.push("        direction LR".to_string());

            for (i, export) in node.exports.iter().enumerate() {
                lines.push(format!(
                    "        {}_e{}([\"{}\"])",
                    safe_id, i, export
                ));
            }

            lines.push("    end".to_string());
        }

        // Add edges between modules
        for edge in &graph.edges {
            let from_id = Self::sanitize_id(&edge.from);
            let to_id = Self::sanitize_id(&edge.to);
            let label = edge.imported_symbols.join(", ");
            if label.is_empty() {
                lines.push(format!("    {} --> {}", from_id, to_id));
            } else {
                lines.push(format!("    {} -->|{}| {}", from_id, label, to_id));
            }
        }

        lines.join("\n")
    }

    // --- Private helpers ---

    fn generate_usecase_v2(actors: &[ActorSpec], use_cases: &[UseCaseSpec], lines: &mut Vec<String>) {
        // Declare actors
        for actor in actors {
            let safe_id = Self::sanitize_id(&actor.name);
            lines.push(format!("    {}(({}))", safe_id, actor.name));
        }

        // Declare use cases
        for uc in use_cases {
            let safe_id = Self::sanitize_id(&uc.id);
            lines.push(format!("    {}[{}]", safe_id, uc.name));
        }

        // Actor → UseCase connections
        for uc in use_cases {
            if let Some(actor_name) = &uc.actor {
                let actor_id = Self::sanitize_id(actor_name);
                let uc_id = Self::sanitize_id(&uc.id);
                lines.push(format!("    {} --> {}", actor_id, uc_id));
            }
        }

        // Include relationships (dotted arrow with "include" label)
        for uc in use_cases {
            let uc_id = Self::sanitize_id(&uc.id);
            for include in &uc.includes {
                let include_id = Self::sanitize_id(include);
                lines.push(format!("    {} -.include.-> {}", uc_id, include_id));
            }
        }

        // Extend relationships (dotted arrow with "extend" label)
        for uc in use_cases {
            let uc_id = Self::sanitize_id(&uc.id);
            for extend in &uc.extends {
                let extend_id = Self::sanitize_id(extend);
                lines.push(format!("    {} -.extend.-> {}", uc_id, extend_id));
            }
        }
    }

    fn generate_usecase_v1(behaviors: &[BehaviorSpec], lines: &mut Vec<String>) {
        // Single "User" actor
        lines.push("    User((User))".to_string());

        // Group behaviors into a single use case per unique output category
        for (i, behavior) in behaviors.iter().enumerate() {
            let uc_id = format!("UC{}", i + 1);
            let label = format!("{} → {}", behavior.input, behavior.output);
            lines.push(format!("    {}[{}]", uc_id, label));
            lines.push(format!("    User --> {}", uc_id));
        }
    }

    fn sanitize_id(name: &str) -> String {
        name.replace('/', "_")
            .replace('-', "_")
            .replace(' ', "_")
            .replace('.', "_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_md_parser::*;

    #[test]
    fn test_generate_usecase_v2() {
        let spec = ClaudeMdSpec {
            actors: vec![
                ActorSpec { name: "User".to_string(), description: "End user".to_string() },
                ActorSpec { name: "System".to_string(), description: "Internal system".to_string() },
            ],
            use_cases: vec![
                UseCaseSpec {
                    id: "UC-1".to_string(),
                    name: "Token Validation".to_string(),
                    actor: Some("User".to_string()),
                    behaviors: vec![],
                    includes: vec!["UC-3".to_string()],
                    extends: vec![],
                },
                UseCaseSpec {
                    id: "UC-2".to_string(),
                    name: "Token Issuance".to_string(),
                    actor: Some("System".to_string()),
                    behaviors: vec![],
                    includes: vec![],
                    extends: vec!["UC-1".to_string()],
                },
                UseCaseSpec {
                    id: "UC-3".to_string(),
                    name: "Token Parsing".to_string(),
                    actor: Some("System".to_string()),
                    behaviors: vec![],
                    includes: vec![],
                    extends: vec![],
                },
            ],
            ..Default::default()
        };

        let diagram = DiagramGenerator::generate_usecase(&spec);
        assert!(diagram.contains("flowchart LR"));
        assert!(diagram.contains("User((User))"));
        assert!(diagram.contains("System((System))"));
        assert!(diagram.contains("UC_1[Token Validation]"));
        assert!(diagram.contains("User --> UC_1"));
        assert!(diagram.contains("UC_1 -.include.-> UC_3"));
        assert!(diagram.contains("UC_2 -.extend.-> UC_1"));
    }

    #[test]
    fn test_generate_usecase_v1_fallback() {
        let spec = ClaudeMdSpec {
            behaviors: vec![
                BehaviorSpec {
                    input: "valid token".to_string(),
                    output: "Claims".to_string(),
                    category: BehaviorCategory::Success,
                },
                BehaviorSpec {
                    input: "expired token".to_string(),
                    output: "TokenExpiredError".to_string(),
                    category: BehaviorCategory::Error,
                },
            ],
            ..Default::default()
        };

        let diagram = DiagramGenerator::generate_usecase(&spec);
        assert!(diagram.contains("flowchart LR"));
        assert!(diagram.contains("User((User))"));
        assert!(diagram.contains("UC1[valid token → Claims]"));
        assert!(diagram.contains("UC2[expired token → TokenExpiredError]"));
    }

    #[test]
    fn test_generate_state() {
        let spec = ClaudeMdSpec {
            protocol: Some(ProtocolSpec {
                states: vec![
                    "Idle".to_string(),
                    "Loading".to_string(),
                    "Loaded".to_string(),
                    "Error".to_string(),
                ],
                transitions: vec![
                    TransitionSpec { from: "Idle".to_string(), trigger: "load()".to_string(), to: "Loading".to_string() },
                    TransitionSpec { from: "Loading".to_string(), trigger: "success".to_string(), to: "Loaded".to_string() },
                    TransitionSpec { from: "Loading".to_string(), trigger: "failure".to_string(), to: "Error".to_string() },
                ],
                lifecycle: vec![],
            }),
            ..Default::default()
        };

        let diagram = DiagramGenerator::generate_state(&spec).unwrap();
        assert!(diagram.contains("stateDiagram-v2"));
        assert!(diagram.contains("[*] --> Idle"));
        assert!(diagram.contains("Idle --> Loading : load()"));
        assert!(diagram.contains("Loading --> Loaded : success"));
        assert!(diagram.contains("Loading --> Error : failure"));
    }

    #[test]
    fn test_generate_state_none_when_empty() {
        let spec = ClaudeMdSpec::default();
        assert!(DiagramGenerator::generate_state(&spec).is_none());
    }

    #[test]
    fn test_generate_component() {
        use crate::dependency_graph::*;

        let graph = DependencyGraphResult {
            root: ".".to_string(),
            analyzed_at: "2024-01-01".to_string(),
            nodes: vec![
                ModuleNode {
                    path: "auth".to_string(),
                    has_claude_md: true,
                    summary: None,
                    exports: vec!["validateToken".to_string(), "Claims".to_string()],
                    symbol_entries: vec![],
                },
                ModuleNode {
                    path: "config".to_string(),
                    has_claude_md: true,
                    summary: None,
                    exports: vec!["JWT_SECRET".to_string()],
                    symbol_entries: vec![],
                },
            ],
            edges: vec![
                DependencyEdge {
                    from: "auth".to_string(),
                    to: "config".to_string(),
                    edge_type: "internal".to_string(),
                    imported_symbols: vec!["JWT_SECRET".to_string()],
                    valid: true,
                },
            ],
            violations: vec![],
            summary: Summary {
                total_nodes: 2,
                total_edges: 1,
                valid_edges: 1,
                violations_count: 0,
            },
        };

        let diagram = DiagramGenerator::generate_component(&graph);
        assert!(diagram.contains("flowchart TB"));
        assert!(diagram.contains("subgraph auth[\"auth\"]"));
        assert!(diagram.contains("auth_e0([\"validateToken\"])"));
        assert!(diagram.contains("auth -->|JWT_SECRET| config"));
    }
}
