use claude_md_core::diff_preservation;

#[test]
fn empty_drift_when_all_declared_sections_verbatim() {
    let prior = "## Technical Context\nUses X\n## Decision Log\nD-1: keep X\n";
    let new_  = "## Technical Context\nUses X\n## Decision Log\nD-1: keep X\n";
    let result = diff_preservation::audit(prior, new_, &["Technical Context", "Decision Log"]);
    assert!(result.drifted.is_empty());
    assert_eq!(result.preserved, vec!["Technical Context".to_string(), "Decision Log".to_string()]);
}

#[test]
fn body_changed_detected() {
    let prior = "## Technical Context\nUses X library\n";
    let new_  = "## Technical Context\nUses library X\n";
    let result = diff_preservation::audit(prior, new_, &["Technical Context"]);
    assert_eq!(result.drifted.len(), 1);
    assert_eq!(result.drifted[0].section, "Technical Context");
    assert_eq!(result.drifted[0].reason, "body_changed");
    assert!(result.preserved.is_empty());
}

#[test]
fn removed_section_detected() {
    let prior = "## Roadmap\n- item A\n## Technical Context\nX\n";
    let new_  = "## Technical Context\nX\n";
    let result = diff_preservation::audit(prior, new_, &["Roadmap"]);
    assert_eq!(result.drifted.len(), 1);
    assert_eq!(result.drifted[0].section, "Roadmap");
    assert_eq!(result.drifted[0].reason, "removed");
}

#[test]
fn undeclared_sections_are_ignored() {
    let prior = "## Technical Context\nX\n## Constraints\nCONST-1: foo\n";
    let new_  = "## Technical Context\nX\n## Constraints\nCONST-1: bar\n";
    let result = diff_preservation::audit(prior, new_, &["Technical Context"]);
    assert!(result.drifted.is_empty());
}

#[test]
fn section_absent_in_prior_but_declared_is_noop() {
    // A section the caller didn't have in prior cannot drift; it's simply "nothing to preserve".
    let prior = "## Technical Context\nX\n";
    let new_  = "## Technical Context\nX\n## Roadmap\n- new item\n";
    let result = diff_preservation::audit(prior, new_, &["Roadmap"]);
    // Not in prior → caller could not have "preserved" it. We report as drifted with reason "absent_in_prior"
    // to let the reviewer decide, rather than silently passing.
    assert_eq!(result.drifted.len(), 1);
    assert_eq!(result.drifted[0].reason, "absent_in_prior");
}
