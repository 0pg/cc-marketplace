use flow_core::validate_dag_file;
use std::collections::HashSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn codes(report: &flow_core::Report) -> HashSet<String> {
    report.errors.iter().map(|e| e.code.clone()).collect()
}

#[test]
fn fixture_1_valid_control() {
    let r = validate_dag_file(&fixture("f1_valid.json")).unwrap();
    assert!(r.valid, "expected valid, got errors: {:?}", r.errors);
}

#[test]
fn fixture_2_cycle_is_rejected() {
    let r = validate_dag_file(&fixture("f2_cycle.json")).unwrap();
    assert!(!r.valid);
    let c = codes(&r);
    assert!(c.contains("CYCLE"), "missing CYCLE in {:?}", c);
    // Terminal falls out of a pure cycle for free; we accept either the combined
    // CYCLE+NO_TERMINAL or CYCLE alone. Assert only the primary invariant.
}

#[test]
fn fixture_3_typo_is_rejected() {
    let r = validate_dag_file(&fixture("f3_typo.json")).unwrap();
    assert!(!r.valid);
    assert!(codes(&r).contains("UNRESOLVED_DEP"));
}

#[test]
fn fixture_4_r5_multiparent_work_is_rejected() {
    let r = validate_dag_file(&fixture("f4_r5.json")).unwrap();
    assert!(!r.valid);
    assert!(codes(&r).contains("R5_WORK_MULTIPARENT"));
}

#[test]
fn fixture_5_r3_terminal_kind_none_is_rejected() {
    let r = validate_dag_file(&fixture("f5_r3.json")).unwrap();
    assert!(!r.valid);
    assert!(codes(&r).contains("R3_TERMINAL_KIND_NONE"));
}

#[test]
fn fixture_6_enum_violations_are_rejected() {
    let r = validate_dag_file(&fixture("f6_enum.json")).unwrap();
    assert!(!r.valid);
    let c = codes(&r);
    assert!(c.contains("ENUM_AGENT"));
    assert!(c.contains("ENUM_VALIDATOR_KIND"));
}

#[test]
fn duplicate_ids_short_circuit() {
    let dag = serde_json::json!({
        "task_id": "tdup",
        "created_at": "2026-04-17T10:00:00Z",
        "spec_ref": "spec.md",
        "nodes": [
            {"id": "a", "type": "work", "deps": [], "agent": "flow-worker", "title": "a", "spec": "...", "validator": {"kind": "command", "command": "pytest", "expected_exit": 0}, "produces": {"kind": "branch", "ref": "flow/tdup/a"}},
            {"id": "a", "type": "work", "deps": [], "agent": "flow-worker", "title": "a2", "spec": "...", "validator": {"kind": "command", "command": "pytest", "expected_exit": 0}, "produces": {"kind": "branch", "ref": "flow/tdup/a2"}}
        ]
    });
    let parsed: flow_core::DagFile = serde_json::from_value(dag).unwrap();
    let r = flow_core::validate(&parsed);
    assert!(!r.valid);
    assert!(codes(&r).contains("DUPLICATE_NODE_ID"));
}
