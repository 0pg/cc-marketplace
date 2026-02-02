use cucumber::{given, then, when, World};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// Import the modules we're testing
use claude_md_core::{TreeParser, BoundaryResolver, SchemaValidator};
use claude_md_core::tree_parser::TreeResult;
use claude_md_core::boundary_resolver::BoundaryResult;
use claude_md_core::schema_validator::ValidationResult;

#[derive(Debug, Default, World)]
pub struct TestWorld {
    temp_dir: Option<TempDir>,
    tree_result: Option<TreeResult>,
    boundary_result: Option<BoundaryResult>,
    validation_result: Option<ValidationResult>,
    claude_md_paths: HashMap<String, PathBuf>,
}

// ============== Common Steps ==============

#[given("a clean test directory")]
fn setup_test_dir(world: &mut TestWorld) {
    world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
}

fn get_temp_path(world: &TestWorld) -> PathBuf {
    world.temp_dir.as_ref().expect("No temp dir").path().to_path_buf()
}

// ============== Tree Parser Steps ==============

#[given(expr = "directory {string} contains source files:")]
fn create_dir_with_source_files(world: &mut TestWorld, path: String, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(file) = row.first() {
                File::create(full_path.join(file)).expect("Failed to create file");
            }
        }
    }
}

#[given(expr = "directory {string} has subdirectories:")]
fn create_subdirectories(world: &mut TestWorld, path: String, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(subdir) = row.first() {
                fs::create_dir_all(full_path.join(subdir)).expect("Failed to create subdir");
            }
        }
    }
}

#[given(expr = "directory {string} exists")]
fn create_empty_dir(world: &mut TestWorld, path: String) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");
}

#[given(expr = "directory {string} contains files:")]
fn create_dir_with_files(world: &mut TestWorld, path: String, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(file) = row.first() {
                File::create(full_path.join(file)).expect("Failed to create file");
            }
        }
    }
}

#[when("I parse the tree")]
fn parse_tree(world: &mut TestWorld) {
    let parser = TreeParser::new();
    world.tree_result = Some(parser.parse(&get_temp_path(world)));
}

#[then(expr = "{string} should need CLAUDE.md")]
fn should_need_claude_md(world: &mut TestWorld, path: String) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let found = result.needs_claude_md.iter().any(|d| {
        d.path.to_string_lossy().contains(&path) || d.path.ends_with(&path)
    });
    assert!(found, "Expected {} to need CLAUDE.md, but it doesn't. Found: {:?}",
            path, result.needs_claude_md.iter().map(|d| &d.path).collect::<Vec<_>>());
}

#[then(expr = "{string} should not need CLAUDE.md")]
fn should_not_need_claude_md(world: &mut TestWorld, path: String) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let found = result.needs_claude_md.iter().any(|d| {
        d.path.to_string_lossy().contains(&path) || d.path.ends_with(&path)
    });
    assert!(!found, "Expected {} to NOT need CLAUDE.md, but it does", path);
}

#[then(expr = "{string} should be excluded")]
fn should_be_excluded(world: &mut TestWorld, path: String) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let found = result.excluded.iter().any(|p| {
        p.to_string_lossy().contains(&path) || p.ends_with(&path)
    });
    assert!(found, "Expected {} to be excluded, but it isn't. Excluded: {:?}",
            path, result.excluded);
}

#[then(expr = "the reason should mention {string}")]
fn reason_should_mention(world: &mut TestWorld, text: String) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let has_reason = result.needs_claude_md.iter().any(|d| d.reason.contains(&text));
    assert!(has_reason, "Expected reason to mention '{}', reasons: {:?}",
            text, result.needs_claude_md.iter().map(|d| &d.reason).collect::<Vec<_>>());
}

#[then(expr = "the source file count should be {int}")]
fn source_file_count_should_be(world: &mut TestWorld, count: usize) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let dir = result.needs_claude_md.last().expect("No directory found");
    assert_eq!(dir.source_file_count, count,
               "Expected {} source files, got {}", count, dir.source_file_count);
}

#[then(expr = "{string} should have depth {int}")]
fn should_have_depth(world: &mut TestWorld, path: String, expected_depth: usize) {
    let result = world.tree_result.as_ref().expect("No tree result");
    let dir = result.needs_claude_md.iter()
        .find(|d| d.path.to_string_lossy().contains(&path) || d.path.ends_with(&path))
        .unwrap_or_else(|| panic!("Directory '{}' not found in results. Found: {:?}",
                                   path, result.needs_claude_md.iter().map(|d| &d.path).collect::<Vec<_>>()));
    assert_eq!(dir.depth, expected_depth,
               "Expected depth {} for '{}', got {}", expected_depth, path, dir.depth);
}

#[then("the results sorted by depth descending should be:")]
fn results_sorted_by_depth(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.tree_result.as_ref().expect("No tree result");

    // Sort by depth descending (leaf-first), then by path for stable ordering
    let mut sorted_results: Vec<_> = result.needs_claude_md.iter().collect();
    sorted_results.sort_by(|a, b| {
        b.depth.cmp(&a.depth).then_with(|| a.path.cmp(&b.path))
    });

    if let Some(table) = &step.table {
        let mut expected_rows: Vec<_> = table.rows.iter().skip(1).collect();

        // Group expected results by depth and sort within groups by path
        // This matches our sorting strategy
        expected_rows.sort_by(|a, b| {
            let depth_a: usize = a.get(1).unwrap().parse().unwrap_or(0);
            let depth_b: usize = b.get(1).unwrap().parse().unwrap_or(0);
            let path_a = a.first().unwrap();
            let path_b = b.first().unwrap();
            depth_b.cmp(&depth_a).then_with(|| path_a.cmp(path_b))
        });

        assert_eq!(sorted_results.len(), expected_rows.len(),
                   "Expected {} results, got {}. Results: {:?}",
                   expected_rows.len(), sorted_results.len(),
                   sorted_results.iter().map(|d| (&d.path, d.depth)).collect::<Vec<_>>());

        for (i, (sorted_dir, expected_row)) in sorted_results.iter().zip(expected_rows.iter()).enumerate() {
            let expected_path = expected_row.first().expect("No path in row");
            let expected_depth: usize = expected_row.get(1)
                .expect("No depth in row")
                .parse()
                .expect("Invalid depth");

            assert!(sorted_dir.path.to_string_lossy().contains(expected_path) ||
                    sorted_dir.path.ends_with(expected_path),
                    "Position {}: Expected path containing '{}', got '{}'",
                    i, expected_path, sorted_dir.path.display());

            assert_eq!(sorted_dir.depth, expected_depth,
                       "Position {}: Expected depth {}, got {}",
                       i, expected_depth, sorted_dir.depth);
        }
    }
}

// ============== Boundary Resolver Steps ==============

#[given("directory structure:")]
fn create_directory_structure(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(path) = row.first() {
                let full_path = get_temp_path(world).join(path);
                fs::create_dir_all(&full_path).expect("Failed to create dir");
            }
        }
    }
}

#[given(expr = "CLAUDE.md at {string} with content:")]
fn create_claude_md(world: &mut TestWorld, path: String, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");

    let claude_md_path = full_path.join("CLAUDE.md");
    let content = step.docstring.as_ref().expect("No content provided");

    let mut file = File::create(&claude_md_path).expect("Failed to create CLAUDE.md");
    write!(file, "{}", content).expect("Failed to write content");

    world.claude_md_paths.insert(path, claude_md_path);
}

#[given(expr = "directory {string} with files:")]
fn create_dir_with_named_files(world: &mut TestWorld, path: String, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(&full_path).expect("Failed to create dir");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(file) = row.first() {
                File::create(full_path.join(file)).expect("Failed to create file");
            }
        }
    }
}

#[when(expr = "I validate references for {string}")]
fn validate_references(world: &mut TestWorld, path: String) {
    let full_path = get_temp_path(world).join(&path);
    let claude_md = world.claude_md_paths.get(&path).cloned();

    let resolver = BoundaryResolver::new();
    world.boundary_result = Some(resolver.resolve(&full_path, claude_md.as_ref()));
}

#[when(expr = "I resolve boundary for {string}")]
fn resolve_boundary(world: &mut TestWorld, path: String) {
    let full_path = get_temp_path(world).join(&path);

    let resolver = BoundaryResolver::new();
    world.boundary_result = Some(resolver.resolve(&full_path, None));
}

#[then("no violation should be reported")]
fn no_violation(world: &mut TestWorld) {
    let result = world.boundary_result.as_ref().expect("No boundary result");
    if let Some(violations) = &result.violations {
        assert!(violations.is_empty(), "Expected no violations, got: {:?}", violations);
    }
}

#[then(expr = "violation {string} should be reported")]
fn violation_reported(world: &mut TestWorld, violation_type: String) {
    let result = world.boundary_result.as_ref().expect("No boundary result");
    let violations = result.violations.as_ref().expect("No violations checked");
    let found = violations.iter().any(|v| v.violation_type == violation_type);
    assert!(found, "Expected {} violation, got: {:?}", violation_type, violations);
}

#[then("multiple violations should be reported")]
fn multiple_violations(world: &mut TestWorld) {
    let result = world.boundary_result.as_ref().expect("No boundary result");
    let violations = result.violations.as_ref().expect("No violations checked");
    assert!(violations.len() > 1, "Expected multiple violations, got: {:?}", violations);
}

#[then(expr = "the violation reference should contain {string}")]
fn violation_reference_contains(world: &mut TestWorld, text: String) {
    let result = world.boundary_result.as_ref().expect("No boundary result");
    let violations = result.violations.as_ref().expect("No violations");
    let found = violations.iter().any(|v| v.reference.contains(&text));
    assert!(found, "Expected violation reference to contain '{}', got: {:?}",
            text, violations);
}

#[then("direct files should include:")]
fn direct_files_include(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.boundary_result.as_ref().expect("No boundary result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(file) = row.first() {
                assert!(result.direct_files.contains(&file.to_string()),
                        "Expected direct files to include '{}', got: {:?}",
                        file, result.direct_files);
            }
        }
    }
}

#[then("subdirs should include:")]
fn subdirs_include(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.boundary_result.as_ref().expect("No boundary result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(subdir) = row.first() {
                assert!(result.subdirs.contains(&subdir.to_string()),
                        "Expected subdirs to include '{}', got: {:?}",
                        subdir, result.subdirs);
            }
        }
    }
}

// ============== Schema Validator Steps ==============

#[given("CLAUDE.md with content:")]
fn create_claude_md_for_validation(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world);
    let claude_md_path = full_path.join("CLAUDE.md");
    let content = step.docstring.as_ref().expect("No content provided");

    let mut file = File::create(&claude_md_path).expect("Failed to create CLAUDE.md");
    write!(file, "{}", content).expect("Failed to write content");

    world.claude_md_paths.insert("root".to_string(), claude_md_path);
}

#[when("I validate the schema")]
fn validate_schema(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");

    let validator = SchemaValidator::new();
    world.validation_result = Some(validator.validate(claude_md_path));
}

#[then("validation should pass")]
fn validation_should_pass(world: &mut TestWorld) {
    let result = world.validation_result.as_ref().expect("No validation result");
    assert!(result.valid, "Expected validation to pass, but got errors: {:?}", result.errors);
}

#[then("validation should fail")]
fn validation_should_fail(world: &mut TestWorld) {
    let result = world.validation_result.as_ref().expect("No validation result");
    assert!(!result.valid, "Expected validation to fail, but it passed");
}

#[then(expr = "error should mention {string}")]
fn error_should_mention(world: &mut TestWorld, text: String) {
    let result = world.validation_result.as_ref().expect("No validation result");
    let found = result.errors.iter().any(|e| e.message.contains(&text));
    assert!(found, "Expected error mentioning '{}', got: {:?}", text, result.errors);
}

#[then("validation should have warnings")]
fn validation_should_have_warnings(world: &mut TestWorld) {
    let result = world.validation_result.as_ref().expect("No validation result");
    assert!(!result.warnings.is_empty(), "Expected warnings, got none");
}

#[then(expr = "warning should mention {string}")]
fn warning_should_mention(world: &mut TestWorld, text: String) {
    let result = world.validation_result.as_ref().expect("No validation result");
    let found = result.warnings.iter().any(|w| w.contains(&text));
    assert!(found, "Expected warning mentioning '{}', got: {:?}", text, result.warnings);
}

#[tokio::main]
async fn main() {
    TestWorld::run("tests/features").await;
}
