//! DAG schema types and validator for `flow`'s `dag.json`.
//!
//! Enforces:
//!   * Structural: acyclicity (R4), referential integrity (R4), ≥1 terminal (R4)
//!   * `flow` invariants: fan-in ≥ 2 ⇒ type=merge (INV-F2 / R5)
//!   * Enum correctness: `type`, `agent`, `validator.kind`
//!   * R3: `validator.kind = none` requires ≥1 successor of type ∈ {work, merge}
//!     (code-level disambiguation of the CLAUDE.md "review-type successor" prose;
//!     since no `review` node type exists in the v0.1 schema, "successor exists
//!     with type in {work, merge}" is the minimum literal reading.)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Serialize, Deserialize)]
pub struct DagFile {
    pub task_id: String,
    pub created_at: String,
    pub spec_ref: String,
    #[serde(default)]
    pub project_test_cmd: Option<String>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub title: String,
    #[serde(default)]
    pub deps: Vec<String>,
    pub agent: String,
    pub spec: String,
    pub validator: Validator,
    pub produces: Produces,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Validator {
    pub kind: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub expected_exit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Produces {
    pub kind: String,
    #[serde(rename = "ref", default)]
    pub ref_: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ValidationError {
    pub code: String,
    pub node_id: Option<String>,
    pub message: String,
}

impl ValidationError {
    fn new(code: &str, node_id: Option<&str>, message: String) -> Self {
        Self {
            code: code.to_string(),
            node_id: node_id.map(str::to_string),
            message,
        }
    }
}

const VALID_TYPES: &[&str] = &["work", "merge"];
const VALID_AGENTS: &[&str] = &["flow-worker", "flow-merger"];
const VALID_VALIDATOR_KINDS: &[&str] = &["command", "schema", "none"];

pub fn validate(dag: &DagFile) -> Report {
    let mut errors: Vec<ValidationError> = Vec::new();

    // Basic: duplicate ids would poison every later check — flag first.
    let mut id_set: HashSet<&str> = HashSet::new();
    for node in &dag.nodes {
        if !id_set.insert(node.id.as_str()) {
            errors.push(ValidationError::new(
                "DUPLICATE_NODE_ID",
                Some(&node.id),
                format!("duplicate node id: {:?}", node.id),
            ));
        }
    }
    // Stop early on duplicates: successor maps etc. become ambiguous.
    if !errors.is_empty() {
        return Report { valid: false, errors };
    }

    let id_to_node: HashMap<&str, &Node> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Enum validation
    for node in &dag.nodes {
        if !VALID_TYPES.contains(&node.node_type.as_str()) {
            errors.push(ValidationError::new(
                "ENUM_TYPE",
                Some(&node.id),
                format!(
                    "type must be in {:?}, got {:?}",
                    VALID_TYPES, node.node_type
                ),
            ));
        }
        if !VALID_AGENTS.contains(&node.agent.as_str()) {
            errors.push(ValidationError::new(
                "ENUM_AGENT",
                Some(&node.id),
                format!(
                    "agent must be in {:?}, got {:?}",
                    VALID_AGENTS, node.agent
                ),
            ));
        }
        if !VALID_VALIDATOR_KINDS.contains(&node.validator.kind.as_str()) {
            errors.push(ValidationError::new(
                "ENUM_VALIDATOR_KIND",
                Some(&node.id),
                format!(
                    "validator.kind must be in {:?}, got {:?}",
                    VALID_VALIDATOR_KINDS, node.validator.kind
                ),
            ));
        }
    }

    // Referential integrity (R4)
    for node in &dag.nodes {
        for dep in &node.deps {
            if !id_to_node.contains_key(dep.as_str()) {
                errors.push(ValidationError::new(
                    "UNRESOLVED_DEP",
                    Some(&node.id),
                    format!("deps references unknown id: {:?}", dep),
                ));
            }
        }
    }

    // INV-F2 / R5: fan-in ≥ 2 ⇒ type=merge
    for node in &dag.nodes {
        if node.deps.len() >= 2 && node.node_type == "work" {
            errors.push(ValidationError::new(
                "R5_WORK_MULTIPARENT",
                Some(&node.id),
                format!(
                    "work node has {} parents; must be type=merge (INV-F2 / R5)",
                    node.deps.len()
                ),
            ));
        }
    }

    // R4: acyclic (Kahn's algorithm) — only run on the resolvable edge subset,
    // so UNRESOLVED_DEP errors above are not double-counted as cycles.
    let indegree_result = compute_indegrees(&dag.nodes, &id_to_node);
    let cycle_nodes = detect_cycle(&dag.nodes, &id_to_node, &indegree_result);
    if !cycle_nodes.is_empty() {
        let mut sorted = cycle_nodes;
        sorted.sort();
        errors.push(ValidationError::new(
            "CYCLE",
            None,
            format!("cycle detected among nodes: {:?}", sorted),
        ));
    }

    // R4: ≥1 terminal (node that is nobody's dep).
    // Only meaningful if no cycle, but also cheap to compute either way — if
    // every node is on a cycle, the check below fires and reinforces the issue.
    let mut is_depended_on: HashSet<&str> = HashSet::new();
    for node in &dag.nodes {
        for dep in &node.deps {
            if id_to_node.contains_key(dep.as_str()) {
                is_depended_on.insert(dep.as_str());
            }
        }
    }
    let has_terminal = dag
        .nodes
        .iter()
        .any(|n| !is_depended_on.contains(n.id.as_str()));
    if !has_terminal && !dag.nodes.is_empty() {
        errors.push(ValidationError::new(
            "NO_TERMINAL",
            None,
            "DAG must contain at least one terminal node (R4)".to_string(),
        ));
    }

    // R3: validator.kind=none requires a successor of type ∈ {work, merge}.
    // Build reverse adjacency: for each node id, the set of nodes that depend on it.
    let mut successors: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in &dag.nodes {
        for dep in &node.deps {
            if id_to_node.contains_key(dep.as_str()) {
                successors.entry(dep.as_str()).or_default().push(node);
            }
        }
    }
    for node in &dag.nodes {
        if node.validator.kind == "none" {
            let has_valid_successor = successors
                .get(node.id.as_str())
                .map(|succs| {
                    succs
                        .iter()
                        .any(|s| s.node_type == "work" || s.node_type == "merge")
                })
                .unwrap_or(false);
            if !has_valid_successor {
                errors.push(ValidationError::new(
                    "R3_TERMINAL_KIND_NONE",
                    Some(&node.id),
                    "validator.kind=none requires ≥1 successor of type∈{work,merge}; terminal forbidden (R3)".to_string(),
                ));
            }
        }
    }

    Report {
        valid: errors.is_empty(),
        errors,
    }
}

struct Indegrees<'a> {
    map: HashMap<&'a str, usize>,
}

fn compute_indegrees<'a>(
    nodes: &'a [Node],
    id_to_node: &HashMap<&str, &Node>,
) -> Indegrees<'a> {
    let mut map: HashMap<&'a str, usize> = nodes.iter().map(|n| (n.id.as_str(), 0usize)).collect();
    for node in nodes {
        for dep in &node.deps {
            if id_to_node.contains_key(dep.as_str()) {
                *map.entry(node.id.as_str()).or_insert(0) += 1;
            }
        }
    }
    Indegrees { map }
}

fn detect_cycle<'a>(
    nodes: &'a [Node],
    id_to_node: &HashMap<&str, &Node>,
    indegrees: &Indegrees<'a>,
) -> Vec<String> {
    let mut indegree: HashMap<&str, usize> = indegrees.map.clone();
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter(|(_, &v)| v == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut visited = 0usize;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        // For every successor of `id`, decrement indegree.
        for node in nodes {
            if node.deps.iter().any(|d| d.as_str() == id)
                && id_to_node.contains_key(node.id.as_str())
            {
                let entry = indegree.get_mut(node.id.as_str()).unwrap();
                *entry -= 1;
                if *entry == 0 {
                    queue.push_back(node.id.as_str());
                }
            }
        }
    }
    if visited == nodes.len() {
        Vec::new()
    } else {
        // Remaining nodes with indegree > 0 are on (or fed by) a cycle.
        indegree
            .iter()
            .filter(|(_, &v)| v > 0)
            .map(|(k, _)| (*k).to_string())
            .collect()
    }
}
