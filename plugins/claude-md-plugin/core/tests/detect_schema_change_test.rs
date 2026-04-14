use claude_md_core::detect_schema_change;

#[test]
fn detects_data_schemas_section_change() {
    let before = "## Constraints\nCONST-1\n## Data Schemas\ntype Foo = {a: int}\n";
    let after  = "## Constraints\nCONST-1\n## Data Schemas\ntype Foo = {a: int, b: str}\n";
    assert!(detect_schema_change::changed(before, after));
}

#[test]
fn ignores_constraints_only_changes() {
    let before = "## Constraints\nCONST-1\n## Data Schemas\ntype Foo = {a: int}\n";
    let after  = "## Constraints\nCONST-1\nCONST-2\n## Data Schemas\ntype Foo = {a: int}\n";
    assert!(!detect_schema_change::changed(before, after));
}

#[test]
fn missing_section_in_before_counts_as_change_when_after_has_content() {
    let before = "## Constraints\nCONST-1\n";
    let after  = "## Constraints\nCONST-1\n## Data Schemas\ntype Foo = {}\n";
    assert!(detect_schema_change::changed(before, after));
}
