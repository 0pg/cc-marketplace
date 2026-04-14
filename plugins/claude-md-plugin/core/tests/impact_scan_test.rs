use tempfile::tempdir;

#[test]
fn scans_consumers_referencing_exported_names() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("producer")).unwrap();
    std::fs::write(root.join("producer/CLAUDE.md"),
        "## Purpose\np\n## Requirements\n- REQ-1\n## Domain Context\nnone\n").unwrap();
    std::fs::write(root.join("producer/DEVELOPERS.md"),
        "## Constraints\n- CONST-1\n## Data Schemas\n```rust\npub struct Foo;\n```\n## Technical Context\nnone\n").unwrap();

    std::fs::create_dir_all(root.join("consumer_a")).unwrap();
    std::fs::write(root.join("consumer_a/CLAUDE.md"),
        "## Purpose\na\n## Requirements\n- REQ-1\n## Domain Context\nnone\n").unwrap();
    std::fs::write(root.join("consumer_a/DEVELOPERS.md"),
        "## Constraints\n- CONST-1 references Foo\n## Technical Context\nuses Foo\n").unwrap();

    let out = claude_md_core::impact_scan::scan(root, "producer").unwrap();
    assert_eq!(out, vec!["consumer_a".to_string()]);
}
