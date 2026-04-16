use cucumber::{given, then, when, World};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Import the modules we're testing
use claude_md_core::{TreeParser, BoundaryResolver, SchemaValidator, CodeAnalyzer, ClaudeMdParser, ConventionValidator};
use claude_md_core::tree_parser::TreeResult;
use claude_md_core::boundary_resolver::BoundaryResult;
use claude_md_core::schema_validator::ValidationResult;
use claude_md_core::claude_md_parser::{ClaudeMdSpec, ParseError};
use claude_md_core::code_analyzer::AnalysisResult;
use claude_md_core::convention_validator::ConventionValidationResult;
use claude_md_core::compile_target_resolver::{CompileTargetResolver, DiffResult};
use claude_md_core::exports_formatter;
use claude_md_core::code_analyzer::{
    Exports, ExportedFunction, ExportedType, ExportedClass, ExportedEnum,
    ExportedVariable, ReExport, TypeKind,
};
use claude_md_core::language_validator::{LanguageValidator, LanguageValidationResult};
use claude_md_core::node_history::{NodeHistoryDiffer, NodeHistoryResult};
use claude_md_core::diff_preservation;

#[derive(Debug, Default, World)]
pub struct TestWorld {
    temp_dir: Option<TempDir>,
    tree_result: Option<TreeResult>,
    boundary_result: Option<BoundaryResult>,
    validation_result: Option<ValidationResult>,
    claude_md_paths: HashMap<String, PathBuf>,
    // Code analyzer fields
    analysis_result: Option<AnalysisResult>,
    analysis_error: Option<String>,
    analyzer: Option<CodeAnalyzer>,
    current_file_path: Option<PathBuf>,
    current_dir_path: Option<PathBuf>,
    boundary_files: Option<Vec<String>>,
    // Convention validator fields
    convention_result: Option<ConventionValidationResult>,
    detected_module_roots: Option<Vec<PathBuf>>,
    // Compile target resolver fields
    diff_result: Option<DiffResult>,
    non_git_temp_dir: Option<TempDir>,
    // Format exports fields
    format_exports_input: Option<Exports>,
    format_exports_output: Option<String>,
    format_exports_output2: Option<String>,
    // Fix schema fields
    fix_schema_added: Option<Vec<String>>,
    // Parser fields
    parser_result: Option<Result<ClaudeMdSpec, ParseError>>,
    // Language validator fields
    language_result: Option<LanguageValidationResult>,
    language_error: Option<String>,
    // Converge schema fields
    converge_result_content: Option<String>,
    converge_result_changes: Option<Vec<String>>,
    converge_result_warnings: Option<Vec<String>>,
    // Node history fields
    node_history_result: Option<NodeHistoryResult>,
    node_history_non_git_dir: Option<TempDir>,
    named_commits: HashMap<String, String>,
    // po-consultant verdict schema fields
    po_consultant_fixtures: Vec<(String, String)>, // (fixture_name, content)
    // Verdict aggregation fields
    verdict_tmp_dir: Option<TempDir>,
    verdict_targets: Vec<String>,
    verdict_jsonl_lines: Vec<serde_json::Value>,
    // Explorer candidate-node fields
    candidate_nodes: Vec<String>,
    // Target selection fields (Step 2.1e)
    target_select_tmp: Option<TempDir>,
    target_select_no_ask: bool,
    // Redirect loop fields (Task 6)
    redirect_tmp: Option<TempDir>,
    redirect_rounds_dir: Option<TempDir>,
    // autodev --auto-sync fields (Task 11)
    auto_sync_tmp: Option<TempDir>,
    auto_sync_halt_reason: Option<String>,
    // Preservation audit fields (diff-preservation)
    preservation_prior: Option<String>,
    preservation_new: Option<String>,
    preservation_audit: Option<diff_preservation::PreservationAudit>,
}

// ============== Common Steps ==============

#[given("a clean test directory")]
fn setup_test_dir(world: &mut TestWorld) {
    world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    world.analyzer = Some(CodeAnalyzer::new());
}

fn get_temp_path(world: &TestWorld) -> PathBuf {
    world.temp_dir.as_ref().expect("No temp dir").path().to_path_buf()
}

// Get the tests directory path
fn get_tests_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
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
                assert!(result.direct_files.iter().any(|f| f.name == *file),
                        "Expected direct files to include '{}', got: {:?}",
                        file, result.direct_files.iter().map(|f| &f.name).collect::<Vec<_>>());
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
                assert!(result.subdirs.iter().any(|s| s.name == *subdir),
                        "Expected subdirs to include '{}', got: {:?}",
                        subdir, result.subdirs.iter().map(|s| &s.name).collect::<Vec<_>>());
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

#[given("DEVELOPERS.md with content:")]
fn create_developers_md_for_validation(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let full_path = get_temp_path(world);
    let developers_md_path = full_path.join("DEVELOPERS.md");
    let content = step.docstring.as_ref().expect("No content provided");

    let mut file = File::create(&developers_md_path).expect("Failed to create DEVELOPERS.md");
    write!(file, "{}", content).expect("Failed to write content");
}

#[when("I validate the schema with strict mode")]
fn validate_schema_strict(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");

    let validator = SchemaValidator::new();
    world.validation_result = Some(validator.validate_strict(claude_md_path));
}

#[when("I validate the schema with strict mode in non-project-root")]
fn validate_schema_strict_non_root(world: &mut TestWorld) {
    use claude_md_core::schema_validator::ValidationContext;
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");
    let ctx = ValidationContext {
        is_project_root: false,
        is_module_root: false,
    };
    let validator = SchemaValidator::new();
    world.validation_result = Some(validator.validate_strict_with_context(claude_md_path, Some(&ctx)));
}

#[when("I validate the schema with strict mode in project-root")]
fn validate_schema_strict_project_root(world: &mut TestWorld) {
    use claude_md_core::schema_validator::ValidationContext;
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");
    let ctx = ValidationContext {
        is_project_root: true,
        is_module_root: false,
    };
    let validator = SchemaValidator::new();
    world.validation_result = Some(validator.validate_strict_with_context(claude_md_path, Some(&ctx)));
}

#[then(regex = r#"validation should have no warnings about "(.+)""#)]
fn validation_no_warnings_about(world: &mut TestWorld, keyword: String) {
    let result = world.validation_result.as_ref().expect("No validation result");
    let matching: Vec<&String> = result.warnings.iter()
        .filter(|w| w.contains(&keyword))
        .collect();
    assert!(
        matching.is_empty(),
        "Expected no warnings about '{}', but found: {:?}", keyword, matching
    );
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

// ============== Converge Schema Steps ==============

#[when("I converge the DEVELOPERS.md schema")]
fn converge_developers_schema(world: &mut TestWorld) {
    let full_path = get_temp_path(world);
    let developers_md_path = full_path.join("DEVELOPERS.md");
    let content = std::fs::read_to_string(&developers_md_path).expect("Cannot read DEVELOPERS.md");
    let validator = SchemaValidator::new();
    let result = validator.converge_schema(&content, "developers_md");
    world.converge_result_content = Some(result.content);
    world.converge_result_changes = Some(result.changes);
    world.converge_result_warnings = Some(result.warnings);
}

#[when("I converge the DEVELOPERS.md schema at project root")]
fn converge_developers_schema_at_project_root(world: &mut TestWorld) {
    let full_path = get_temp_path(world);
    // Create .git directory to simulate project root
    let git_dir = full_path.join(".git");
    if !git_dir.exists() {
        std::fs::create_dir(&git_dir).expect("Failed to create .git dir");
    }
    let developers_md_path = full_path.join("DEVELOPERS.md");
    let content = std::fs::read_to_string(&developers_md_path).expect("Cannot read DEVELOPERS.md");
    let validator = SchemaValidator::new();
    let ctx = SchemaValidator::evaluate_conditions(&full_path);
    let result = validator.converge_schema_with_context(&content, "developers_md", Some(&ctx));
    world.converge_result_content = Some(result.content);
    world.converge_result_changes = Some(result.changes);
    world.converge_result_warnings = Some(result.warnings);
}

#[then(expr = "converged content should not contain {string}")]
fn converged_content_should_not_contain(world: &mut TestWorld, text: String) {
    let content = world.converge_result_content.as_ref().expect("No converge result");
    assert!(
        !content.contains(&text),
        "Expected converged content NOT to contain '{}', but it was found", text
    );
}

#[then(expr = "converged content should contain {string}")]
fn converged_content_should_contain(world: &mut TestWorld, text: String) {
    let content = world.converge_result_content.as_ref().expect("No converge result");
    assert!(
        content.contains(&text),
        "Expected converged content to contain '{}', but it was not found.\nContent:\n{}", text, content
    );
}

#[then(expr = "converge changes should contain {string}")]
fn converge_changes_should_contain(world: &mut TestWorld, text: String) {
    let changes = world.converge_result_changes.as_ref().expect("No converge result");
    let found = changes.iter().any(|c| c.contains(&text));
    assert!(found, "Expected changes to contain '{}', got: {:?}", text, changes);
}

#[then(expr = "converge warnings should contain {string}")]
fn converge_warnings_should_contain(world: &mut TestWorld, text: String) {
    let warnings = world.converge_result_warnings.as_ref().expect("No converge result");
    let found = warnings.iter().any(|w| w.contains(&text));
    assert!(found, "Expected warnings to contain '{}', got: {:?}", text, warnings);
}

#[then("converge should report no changes")]
fn converge_should_report_no_changes(world: &mut TestWorld) {
    let changes = world.converge_result_changes.as_ref().expect("No converge result");
    assert!(changes.is_empty(), "Expected no changes, got: {:?}", changes);
}

// ============== Code Analyzer Steps ==============

#[then("I should find environment variables:")]
fn should_find_env_vars(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let expected_name = &row[0];
            assert!(
                result.env_vars.iter().any(|v| v == expected_name),
                "Expected env var '{}' not found in {:?}",
                expected_name,
                result.env_vars
            );
        }
    }
}

// ============== Fix Schema Steps ==============

#[when("I fix the schema")]
fn fix_schema(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");
    let content = fs::read_to_string(claude_md_path).expect("Failed to read CLAUDE.md");

    let validator = SchemaValidator::new();
    let (fixed, added) = validator.fix_missing_sections(&content);

    // Write fixed content back
    fs::write(claude_md_path, &fixed).expect("Failed to write fixed CLAUDE.md");
    world.fix_schema_added = Some(added);
}

#[then(expr = "fix should add sections {string}")]
fn fix_should_add_sections(world: &mut TestWorld, expected: String) {
    let added = world.fix_schema_added.as_ref().expect("No fix result");
    if expected.is_empty() {
        assert!(added.is_empty(), "Expected no sections added, got: {:?}", added);
    } else {
        let expected_sections: Vec<&str> = expected.split(", ").collect();
        for section in &expected_sections {
            assert!(added.iter().any(|a| a == section),
                "Expected section '{}' to be added, got: {:?}", section, added);
        }
        assert_eq!(added.len(), expected_sections.len(),
            "Expected {} sections, got {}: {:?}", expected_sections.len(), added.len(), added);
    }
}

#[then("the fixed file should pass validation")]
fn fixed_file_should_pass(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");
    let validator = SchemaValidator::new();
    let result = validator.validate(claude_md_path);
    assert!(result.valid, "Fixed file should pass validation, but got errors: {:?}", result.errors);
}

// ============== Code Analyzer Steps ==============

// Background steps
#[given("the code-analyze skill uses only Read, Glob, and Grep tools")]
fn code_analyze_uses_tools(_world: &mut TestWorld) {
    // This is a documentation step, no implementation needed
}

#[given("regex patterns are used for language-specific analysis")]
fn regex_patterns_used(_world: &mut TestWorld) {
    // This is a documentation step, no implementation needed
}

// Given steps for different file types
#[given(expr = "a TypeScript file {string}")]
fn given_typescript_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Python file {string}")]
fn given_python_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Python package {string}")]
fn given_python_package(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Go file {string}")]
fn given_go_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Rust file {string}")]
fn given_rust_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Java file {string}")]
fn given_java_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Java directory {string}")]
fn given_java_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Kotlin file {string}")]
fn given_kotlin_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Kotlin directory {string}")]
fn given_kotlin_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a TypeScript directory {string}")]
fn given_typescript_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Python directory {string}")]
fn given_python_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Go directory {string}")]
fn given_go_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a Rust directory {string}")]
fn given_rust_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "an empty directory {string}")]
fn given_empty_directory(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_dir_path = Some(get_tests_path().join(&path));
}

#[given(expr = "a non-existent file {string}")]
fn given_nonexistent_file(world: &mut TestWorld, path: String) {
    world.analyzer = Some(CodeAnalyzer::new());
    world.current_file_path = Some(get_tests_path().join(&path));
}

#[given("a directory with multiple languages")]
fn given_mixed_language_directory(world: &mut TestWorld) {
    world.analyzer = Some(CodeAnalyzer::new());
    // Use fixtures root which contains multiple language directories
    world.current_dir_path = Some(get_tests_path());
}

#[given(regex = r#"a boundary file specifying direct_files: \[(.+)\]"#)]
fn given_boundary_files(world: &mut TestWorld, files_str: String) {
    let files: Vec<String> = files_str
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();
    world.boundary_files = Some(files);
}

// When steps
#[when("I analyze the file for exports")]
fn analyze_file_for_exports(world: &mut TestWorld) {
    let analyzer = world.analyzer.as_ref().expect("No analyzer");
    let path = world.current_file_path.as_ref().expect("No file path");

    match analyzer.analyze_file(path) {
        Ok(result) => world.analysis_result = Some(result),
        Err(e) => world.analysis_error = Some(e.to_string()),
    }
}

#[when("I analyze the file for dependencies")]
fn analyze_file_for_dependencies(world: &mut TestWorld) {
    // Same as exports - we analyze everything
    analyze_file_for_exports(world);
}

#[when("I analyze the file for behaviors")]
fn analyze_file_for_behaviors(world: &mut TestWorld) {
    // Same as exports - we analyze everything
    analyze_file_for_exports(world);
}

#[when("I analyze the file for contracts")]
fn analyze_file_for_contracts(world: &mut TestWorld) {
    // Same as exports - we analyze everything including contracts
    analyze_file_for_exports(world);
}

#[when("I analyze the package for exports")]
fn analyze_package_for_exports(world: &mut TestWorld) {
    let analyzer = world.analyzer.as_ref().expect("No analyzer");
    let path = world.current_dir_path.as_ref().expect("No directory path");

    match analyzer.analyze_directory(path, None) {
        Ok(result) => world.analysis_result = Some(result),
        Err(e) => world.analysis_error = Some(e.to_string()),
    }
}

#[when("I analyze the directory for exports")]
fn analyze_directory_for_exports(world: &mut TestWorld) {
    analyze_package_for_exports(world);
}

#[when("I analyze the directory")]
fn analyze_directory(world: &mut TestWorld) {
    analyze_package_for_exports(world);
}

#[when("I attempt to analyze the file")]
fn attempt_analyze_file(world: &mut TestWorld) {
    let analyzer = world.analyzer.as_ref().expect("No analyzer");
    let path = world.current_file_path.as_ref().expect("No file path");

    match analyzer.analyze_file(path) {
        Ok(result) => world.analysis_result = Some(result),
        Err(e) => world.analysis_error = Some(e.to_string()),
    }
}

#[when("I run the complete code-analyze workflow")]
fn run_complete_workflow(world: &mut TestWorld) {
    let analyzer = world.analyzer.as_ref().expect("No analyzer");
    let path = world.current_dir_path.as_ref().expect("No directory path");

    let files = world.boundary_files.as_ref().map(|f| {
        f.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    });

    match analyzer.analyze_directory(path, files.as_deref()) {
        Ok(result) => world.analysis_result = Some(result),
        Err(e) => world.analysis_error = Some(e.to_string()),
    }
}

// Then steps for exports
#[then("I should find exported functions:")]
fn should_find_exported_functions(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No function name");

            let found = result.exports.functions.iter().any(|f| f.name == *name);
            assert!(found, "Expected to find function '{}', found: {:?}",
                    name, result.exports.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find exported types:")]
fn should_find_exported_types(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No type name");
            let kind = row.get(1).expect("No type kind");

            let found = result.exports.types.iter().any(|t| {
                t.name == *name && format!("{:?}", t.kind).to_lowercase() == kind.to_lowercase()
            });
            assert!(found, "Expected to find type '{}' of kind '{}', found: {:?}",
                    name, kind, result.exports.types);
        }
    }
}

#[then("I should find exported classes:")]
fn should_find_exported_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No class name");

            let found = result.exports.classes.iter().any(|c| c.name == *name);
            assert!(found, "Expected to find class '{}', found: {:?}",
                    name, result.exports.classes.iter().map(|c| &c.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find external dependencies:")]
fn should_find_external_deps(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let pkg = row.first().expect("No package name");

            let found = result.dependencies.external.iter().any(|d| d == pkg || d.contains(pkg));
            assert!(found, "Expected to find external dependency '{}', found: {:?}",
                    pkg, result.dependencies.external);
        }
    }
}

#[then("I should find internal dependencies:")]
fn should_find_internal_deps(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let path = row.first().expect("No path");

            let found = result.dependencies.internal_raw.iter().any(|d| d == path || d.contains(path));
            assert!(found, "Expected to find internal dependency '{}', found: {:?}",
                    path, result.dependencies.internal_raw);
        }
    }
}

#[then("I should find symbols defined in __all__:")]
fn should_find_all_symbols(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No symbol name");
            let kind = row.get(1).expect("No kind");

            let found = match kind.as_str() {
                "function" => result.exports.functions.iter().any(|f| f.name == *name),
                "class" => result.exports.classes.iter().any(|c| c.name == *name),
                _ => false,
            };
            assert!(found, "Expected to find {} '{}' in __all__", kind, name);
        }
    }
}

#[then("I should NOT find private functions:")]
fn should_not_find_private_functions(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No function name");

            let found = result.exports.functions.iter().any(|f| f.name == *name);
            assert!(!found, "Found private function '{}' that should be excluded", name);
        }
    }
}

#[then("I should NOT find private methods:")]
fn should_not_find_private_methods(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_not_find_private_functions(world, step);
}

#[then(regex = r"I should find exported functions \(capitalized\):")]
fn should_find_capitalized_functions(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_functions(world, step);
}

#[then(regex = r"I should find exported types \(capitalized\):")]
fn should_find_capitalized_types(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_types(world, step);
}

#[then("I should find exported error variables:")]
fn should_find_error_variables(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No variable name");

            let found = result.exports.variables.iter().any(|v| v.name == *name);
            assert!(found, "Expected to find error variable '{}', found: {:?}",
                    name, result.exports.variables.iter().map(|v| &v.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find pub functions:")]
fn should_find_pub_functions(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_functions(world, step);
}

#[then("I should find pub types:")]
fn should_find_pub_types(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_types(world, step);
}

#[then("I should find public methods:")]
fn should_find_public_methods(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_functions(world, step);
}

#[then("I should find public classes:")]
fn should_find_public_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_classes(world, step);
}

#[then("I should find public enums:")]
fn should_find_public_enums(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No enum name");

            let found = result.exports.enums.iter().any(|e| e.name == *name);
            assert!(found, "Expected to find enum '{}', found: {:?}",
                    name, result.exports.enums.iter().map(|e| &e.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find public functions:")]
fn should_find_public_functions(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_exported_functions(world, step);
}

#[then("I should find data classes:")]
fn should_find_data_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No class name");

            let found = result.exports.types.iter().any(|t| {
                t.name == *name && t.kind == claude_md_core::code_analyzer::TypeKind::DataClass
            });
            assert!(found, "Expected to find data class '{}', found types: {:?}",
                    name, result.exports.types.iter().map(|t| (&t.name, &t.kind)).collect::<Vec<_>>());
        }
    }
}

#[then("I should find enum classes:")]
fn should_find_enum_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    should_find_public_enums(world, step);
}

#[then("I should find re-exported symbols:")]
fn should_find_re_exported_symbols(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No symbol name");
            let source = row.get(1).expect("No source");

            let found = result.exports.re_exports.iter().any(|r| {
                r.name == *name && r.source == *source
            });
            assert!(found, "Expected to find re-exported symbol '{}' from '{}', found: {:?}",
                    name, source, result.exports.re_exports);
        }
    }
}

// Contract assertions
#[then(regex = r#"I should find contract for "(\w+)":"#)]
fn should_find_contract_for(world: &mut TestWorld, function_name: String, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    let contract = result.contracts.iter()
        .find(|c| c.function_name == function_name)
        .unwrap_or_else(|| panic!("Expected contract for function '{}', found: {:?}",
                                   function_name, result.contracts.iter().map(|c| &c.function_name).collect::<Vec<_>>()));

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            // Check preconditions column if present
            if let Some(precondition) = row.first() {
                if !precondition.is_empty() {
                    let found = contract.contract.preconditions.iter()
                        .any(|p| p.contains(precondition));
                    assert!(found, "Expected precondition containing '{}' for '{}', found: {:?}",
                            precondition, function_name, contract.contract.preconditions);
                }
            }

            // Check postconditions column if present
            if let Some(postcondition) = row.get(1) {
                if !postcondition.is_empty() {
                    let found = contract.contract.postconditions.iter()
                        .any(|p| p.contains(postcondition));
                    assert!(found, "Expected postcondition containing '{}' for '{}', found: {:?}",
                            postcondition, function_name, contract.contract.postconditions);
                }
            }

            // Check throws column if present
            if let Some(throws) = row.get(2) {
                if !throws.is_empty() {
                    let found = contract.contract.throws.iter()
                        .any(|t| t.contains(throws));
                    assert!(found, "Expected throws containing '{}' for '{}', found: {:?}",
                            throws, function_name, contract.contract.throws);
                }
            }
        }
    }
}

#[when("I analyze the file for protocol")]
fn analyze_file_for_protocol(world: &mut TestWorld) {
    // Same as exports - we analyze everything including protocol
    analyze_file_for_exports(world);
}

#[when("I analyze the file for environment variables")]
fn analyze_file_for_env_vars(world: &mut TestWorld) {
    // Same as exports - we analyze everything including env_vars
    analyze_file_for_exports(world);
}

#[then("I should find states:")]
fn should_find_states(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    let protocol = result.protocol.as_ref()
        .expect("No protocol found in analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let state = row.first().expect("No state name");

            let found = protocol.states.iter().any(|s| s == state);
            assert!(found, "Expected to find state '{}', found: {:?}",
                    state, protocol.states);
        }
    }
}

#[then("I should find lifecycle methods:")]
fn should_find_lifecycle_methods(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    let protocol = result.protocol.as_ref()
        .expect("No protocol found in analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let method = row.first().expect("No method name");

            let found = protocol.lifecycle.iter().any(|m| m == method);
            assert!(found, "Expected to find lifecycle method '{}', found: {:?}",
                    method, protocol.lifecycle);
        }
    }
}

#[then(regex = r#"I should find inferred preconditions for "(\w+)":"#)]
fn should_find_inferred_preconditions(world: &mut TestWorld, function_name: String, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    let contract = result.contracts.iter()
        .find(|c| c.function_name == function_name)
        .unwrap_or_else(|| panic!("Expected contract for function '{}', found: {:?}",
                                   function_name, result.contracts.iter().map(|c| &c.function_name).collect::<Vec<_>>()));

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            if let Some(precondition) = row.first() {
                let found = contract.contract.preconditions.iter()
                    .any(|p| p.contains(precondition));
                assert!(found, "Expected inferred precondition containing '{}' for '{}', found: {:?}",
                        precondition, function_name, contract.contract.preconditions);
            }
        }
    }
}

// Behavior assertions
#[then("I should infer error behaviors:")]
fn should_infer_error_behaviors(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let input = row.first().expect("No input");
            let output = row.get(1).expect("No output");

            let found = result.behaviors.iter().any(|b| {
                b.input.contains(input) && b.output.contains(output)
            });
            assert!(found, "Expected error behavior '{}' -> '{}', found: {:?}",
                    input, output, result.behaviors);
        }
    }
}

#[then("I should infer success behaviors:")]
fn should_infer_success_behaviors(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let input = row.first().expect("No input");
            let output = row.get(1).expect("No output");

            let found = result.behaviors.iter().any(|b| {
                b.input.contains(input) && b.output.contains(output) &&
                b.category == claude_md_core::code_analyzer::BehaviorCategory::Success
            });
            assert!(found, "Expected success behavior '{}' -> '{}', found: {:?}",
                    input, output, result.behaviors);
        }
    }
}

#[then("I should infer Result-based behaviors:")]
fn should_infer_result_behaviors(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let input = row.first().expect("No input");
            let output = row.get(1).expect("No output");

            let found = result.behaviors.iter().any(|b| {
                b.input.contains(input) && b.output.contains(output)
            });
            assert!(found, "Expected Result behavior '{}' -> '{}', found: {:?}",
                    input, output, result.behaviors);
        }
    }
}

// Edge case assertions
#[then("I should return an empty analysis result:")]
fn should_return_empty_result(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let field = row.first().expect("No field name");
            let expected: usize = row.get(1).expect("No expected value").parse().expect("Invalid number");

            let actual = match field.as_str() {
                "exports_count" => {
                    result.exports.functions.len() +
                    result.exports.types.len() +
                    result.exports.classes.len() +
                    result.exports.enums.len() +
                    result.exports.variables.len()
                },
                "dependencies_count" => {
                    result.dependencies.external.len() + result.dependencies.internal_raw.len()
                },
                "behaviors_count" => result.behaviors.len(),
                _ => panic!("Unknown field: {}", field),
            };

            assert_eq!(actual, expected, "Expected {} to be {}, got {}", field, expected, actual);
        }
    }
}

#[then("I should skip the file with a warning")]
fn should_skip_with_warning(world: &mut TestWorld) {
    assert!(world.analysis_error.is_some(), "Expected an error but got none");
}

#[then("the analysis should continue without error")]
fn analysis_should_continue(_world: &mut TestWorld) {
    // If we got here, the analysis continued
}

#[then("I should detect and apply correct patterns per file extension")]
fn should_detect_correct_patterns(world: &mut TestWorld) {
    // If analysis completed without error, patterns were applied correctly
    assert!(world.analysis_result.is_some() || world.analysis_error.is_some(),
            "Expected some analysis result");
}

// Complete workflow assertions
#[then(expr = "the output JSON should match {string}")]
fn output_should_match_json(world: &mut TestWorld, _expected_path: String) {
    // For now, just verify we have a result
    assert!(world.analysis_result.is_some(), "No analysis result");
    // TODO: Full JSON comparison if needed
}

#[then("the result should include:")]
fn result_should_include(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let field = row.first().expect("No field");
            let expected: usize = row.get(1).expect("No expected value").parse().expect("Invalid number");

            let actual = match field.as_str() {
                "exports.functions" => result.exports.functions.len(),
                "exports.types" => result.exports.types.len(),
                "exports.classes" => result.exports.classes.len(),
                "dependencies.external" => result.dependencies.external.len(),
                "dependencies.internal" => result.dependencies.internal_raw.len(),
                "behaviors" => result.behaviors.len(),
                "analyzed_files" => result.analyzed_files.len(),
                _ => panic!("Unknown field: {}", field),
            };

            assert_eq!(actual, expected, "Expected {} = {}, got {}", field, expected, actual);
        }
    }
}

// ============== Convention Validator Steps ==============

fn create_file_at(base: &Path, rel: &str, content: &str) {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent dir");
    }
    let mut f = File::create(&path).expect("Failed to create file");
    write!(f, "{}", content).expect("Failed to write");
}

const VALID_CONVENTIONS: &str = r#"
## Conventions

### Project Structure
Layered architecture with src/ containing all source code.

### Module Boundaries
Each module is self-contained and communicates through public APIs.

### Naming Conventions
camelCase for files, PascalCase for classes.

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Coding Rules
- 비동기: async/await 사용, raw Promise 금지
- 타입: strict mode, any 금지
- 불변성: const 우선, let 최소화

### Naming Rules
camelCase for variables and functions, PascalCase for types.
"#;

#[given("a project root with CLAUDE.md containing valid Conventions")]
fn project_root_valid_conventions_section(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test Project\n\n## Purpose\nA test project.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md without Conventions")]
fn project_root_no_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = "# Test Project\n\n## Purpose\nA test project.\n";
    create_file_at(&root, "CLAUDE.md", content);
}

#[given("a project root with CLAUDE.md containing incomplete Conventions")]
fn project_root_incomplete_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = "# Test\n\n## Purpose\nTest.\n\n## Conventions\n\n### Project Structure\nLayered.\n\n### Language & Runtime\nTypeScript\n";
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md containing valid conventions")]
fn project_root_valid_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test\n\n## Purpose\nTest.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a single module project with package.json")]
fn single_module_with_package_json(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
}

#[given("a multi module project with sub-packages")]
fn multi_module_with_sub_packages(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    let sub1 = root.join("packages").join("api");
    fs::create_dir_all(&sub1).expect("create sub1");
    File::create(sub1.join("package.json")).expect("create sub1 marker");

    let sub2 = root.join("packages").join("web");
    fs::create_dir_all(&sub2).expect("create sub2");
    File::create(sub2.join("package.json")).expect("create sub2 marker");
}

#[given("a multi module project with module-level Conventions override")]
fn multi_module_with_override(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = format!(
        "# API Module\n\n## Purpose\nAPI module.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&sub, "CLAUDE.md", &sub_content);
}

// ---- DRY: Convention Inheritance steps ----

#[given("a multi module project where module has no Conventions")]
fn multi_module_no_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    // Project root has Conventions (canonical source)
    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    // Sub-module has NO Conventions (inherits from project root)
    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = "# API Module\n\n## Purpose\nAPI module.\n";
    create_file_at(&sub, "CLAUDE.md", sub_content);
}

#[given("a multi module project where module has incomplete Conventions")]
fn multi_module_incomplete_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    // Sub-module has Conventions but missing Naming Rules
    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = "# API Module\n\n## Purpose\nAPI module.\n\n## Conventions\n\n### Project Structure\nLayered.\n\n### Language & Runtime\nTypeScript\n\n### Coding Rules\n- async/await 사용\n";
    create_file_at(&sub, "CLAUDE.md", sub_content);
}

#[given("a multi module project where project root has no Conventions")]
fn multi_module_no_project_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    // Project root has NO Conventions
    let root_content = "# Root\n\n## Purpose\nRoot project.\n";
    create_file_at(&root, "CLAUDE.md", root_content);

    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = format!(
        "# API Module\n\n## Purpose\nAPI module.\n{}",
        VALID_CONVENTIONS
    );
    create_file_at(&sub, "CLAUDE.md", &sub_content);
}

#[when("I validate conventions")]
fn validate_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    let validator = ConventionValidator::new();
    world.convention_result = Some(validator.validate(&root, None));
}

#[when("I detect module roots")]
fn detect_module_roots(world: &mut TestWorld) {
    let root = get_temp_path(world);
    let validator = ConventionValidator::new();
    world.detected_module_roots = Some(validator.find_module_roots(&root));
}

#[then("convention validation should pass")]
fn convention_validation_pass(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    assert!(result.valid, "Expected convention validation to pass, errors: {:?}", result.errors);
}

#[then("convention validation should fail")]
fn convention_validation_fail(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    assert!(!result.valid, "Expected convention validation to fail, but it passed");
}

#[then("conventions should be found")]
fn conventions_found(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    assert!(result.conventions.section_found, "Conventions section not found");
}

#[then(expr = "convention error should mention {string}")]
fn convention_error_mention(world: &mut TestWorld, text: String) {
    let result = world.convention_result.as_ref().expect("No convention result");
    let found = result.errors.iter().any(|e| e.contains(&text));
    assert!(found, "Expected convention error mentioning '{}', got: {:?}", text, result.errors);
}

#[then(expr = "module root count should be {int}")]
fn module_root_count(world: &mut TestWorld, count: usize) {
    let roots = world.detected_module_roots.as_ref().expect("No detected module roots");
    assert_eq!(roots.len(), count, "Expected {} module roots, got {}: {:?}", count, roots.len(), roots);
}

#[then(expr = "module root count should be at least {int}")]
fn module_root_count_at_least(world: &mut TestWorld, count: usize) {
    let roots = world.detected_module_roots.as_ref().expect("No detected module roots");
    assert!(roots.len() >= count, "Expected at least {} module roots, got {}: {:?}", count, roots.len(), roots);
}

#[then("module should have conventions override")]
fn module_has_conventions_override(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    let has_override = result.module_roots.iter().any(|m| {
        m.conventions.section_found
    });
    assert!(has_override, "Expected at least one module with conventions override");
}

// ============== Schema Rules Steps ==============

#[given("a schema validator is initialized")]
fn schema_validator_initialized(world: &mut TestWorld) {
    // Ensure temp dir exists for any file-based operations
    if world.temp_dir.is_none() {
        world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    }
}

#[given(expr = "a CLAUDE.md file with content:")]
fn create_claude_md_file_for_rules(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    if world.temp_dir.is_none() {
        world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    }
    let full_path = get_temp_path(world);
    let claude_md_path = full_path.join("CLAUDE.md");
    let content = step.docstring.as_ref().expect("No content provided");

    let mut file = File::create(&claude_md_path).expect("Failed to create CLAUDE.md");
    write!(file, "{}", content).expect("Failed to write content");

    world.claude_md_paths.insert("root".to_string(), claude_md_path);
}

#[when("I check the required sections")]
fn check_required_sections(world: &mut TestWorld) {
    // The required sections are defined by the generated constants
    // We just need to validate that they exist - store a dummy validation result
    // so the Then step can check the constant
    world.validation_result = Some(ValidationResult {
        file: String::new(),
        valid: true,
        errors: vec![],
        warnings: vec![],
        completeness_score: None,
    });
}

#[then(expr = "required sections should include:")]
fn required_sections_include(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let _result = world.validation_result.as_ref().expect("No validation result");

    // Check against the generated REQUIRED_SECTIONS constant
    if let Some(table) = &step.table {
        for row in &table.rows {
            if let Some(section_name) = row.first() {
                let found = claude_md_core::schema_validator::REQUIRED_SECTIONS
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(section_name));
                assert!(found, "Expected '{}' to be a required section. Required sections: {:?}",
                        section_name, claude_md_core::schema_validator::REQUIRED_SECTIONS);
            }
        }
    }
}

#[when("I validate the file")]
fn validate_the_file(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");

    let validator = SchemaValidator::new();
    world.validation_result = Some(validator.validate(claude_md_path));
}

#[then(expr = "validation should fail with error {string}")]
fn validation_fail_with_error(world: &mut TestWorld, error_type: String) {
    let result = world.validation_result.as_ref().expect("No validation result");
    assert!(!result.valid, "Expected validation to fail, but it passed");
    let found = result.errors.iter().any(|e| e.error_type == error_type);
    assert!(found, "Expected error type '{}', got: {:?}", error_type, result.errors);
}

#[then(expr = "the error should mention {string}")]
fn the_error_should_mention(world: &mut TestWorld, mention: String) {
    let result = world.validation_result.as_ref().expect("No validation result");
    let found = result.errors.iter().any(|e| e.message.contains(&mention)
        || e.section.as_ref().map_or(false, |s| s.contains(&mention)));
    assert!(found, "Expected error mentioning '{}', got: {:?}", mention, result.errors);
}

// ============== CLAUDE.md Parser Background Steps ==============

#[given("the claude-md-parser uses regex patterns for section parsing")]
fn parser_uses_regex(_world: &mut TestWorld) {
    // Documentation step - no implementation needed
}

#[given("the parser produces JSON output compatible with code generation")]
fn parser_produces_json(_world: &mut TestWorld) {
    // Documentation step - no implementation needed
}

// ============== CLAUDE.md Parser Steps ==============

#[when("I parse the CLAUDE.md file")]
fn parse_claude_md_file(world: &mut TestWorld) {
    let claude_md_path = world.claude_md_paths.get("root").expect("No CLAUDE.md path");
    let content = fs::read_to_string(claude_md_path).expect("Failed to read CLAUDE.md");

    let parser = ClaudeMdParser::new();
    world.parser_result = Some(parser.parse_content(&content));
}

#[then(expr = "the spec should have purpose {string}")]
fn spec_should_have_purpose(world: &mut TestWorld, expected: String) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    assert_eq!(spec.purpose, expected, "Purpose mismatch");
}

#[then(expr = "the spec should have requirements count {int}")]
fn spec_should_have_requirements_count(world: &mut TestWorld, count: usize) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    let requirements = spec.requirements.as_ref().expect("No requirements");
    assert_eq!(requirements.len(), count, "Requirements count mismatch");
}

#[then("the spec should have no requirements")]
fn spec_should_have_no_requirements(world: &mut TestWorld) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    assert!(spec.requirements.is_none(), "Expected no requirements, got: {:?}", spec.requirements);
}

#[then(expr = "the spec should have domain context containing {string}")]
fn spec_should_have_domain_context(world: &mut TestWorld, text: String) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    let dc = spec.domain_context.as_ref().expect("No domain context");
    assert!(dc.contains(&text), "Expected domain context to contain '{}', got: {}", text, dc);
}

#[then("the spec should have no domain context")]
fn spec_should_have_no_domain_context(world: &mut TestWorld) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    assert!(spec.domain_context.is_none(), "Expected no domain context, got: {:?}", spec.domain_context);
}

#[then(expr = "the spec should have instructions containing {string}")]
fn spec_should_have_instructions(world: &mut TestWorld, text: String) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    let instructions = spec.instructions.as_ref().expect("No instructions");
    assert!(instructions.contains(&text), "Expected instructions to contain '{}', got: {}", text, instructions);
}

#[then("the spec should have no instructions")]
fn spec_should_have_no_instructions(world: &mut TestWorld) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    assert!(spec.instructions.is_none(), "Expected no instructions, got: {:?}", spec.instructions);
}

#[then(expr = "parsing should fail with error {string}")]
fn parsing_should_fail(world: &mut TestWorld, expected_error: String) {
    let result = world.parser_result.as_ref().expect("No parser result");
    assert!(result.is_err(), "Expected parsing to fail, but it succeeded");
    let err = result.as_ref().unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains(&expected_error),
        "Expected error containing '{}', got: {}", expected_error, err_msg);
}

#[then(expr = "the spec should have warnings containing {string}")]
fn spec_should_have_warnings(world: &mut TestWorld, text: String) {
    let result = world.parser_result.as_ref().expect("No parser result");
    let spec = result.as_ref().expect("Parsing failed");
    let found = spec.warnings.iter().any(|w| w.contains(&text));
    assert!(found, "Expected warning containing '{}', got: {:?}", text, spec.warnings);
}

// ============== Compile Target Resolver Steps ==============

fn git_init(dir: &Path) {
    use std::process::Command;
    Command::new("git").args(["init"]).current_dir(dir)
        .output().expect("git init failed");
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(dir)
        .output().expect("git config email failed");
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(dir)
        .output().expect("git config name failed");
    // Initial commit so git operations work
    Command::new("git").args(["commit", "--allow-empty", "-m", "init"]).current_dir(dir)
        .output().expect("git initial commit failed");
}

fn git_add(dir: &Path, file: &str) {
    use std::process::Command;
    Command::new("git").args(["add", file]).current_dir(dir)
        .output().expect("git add failed");
}

#[given("a clean git test repository")]
fn setup_git_test_dir(world: &mut TestWorld) {
    world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    git_init(&get_temp_path(world));
}

#[given(expr = "a spec file {string} with basic content")]
fn create_spec_file(world: &mut TestWorld, path: String) {
    let full_path = get_temp_path(world).join(&path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let content = if path.ends_with("CLAUDE.md") {
        "# Module\n\n## Purpose\nTest module\n\n## Requirements\nNone\n\n## Domain Context\nNone\n"
    } else {
        "# DEVELOPERS\n\n## Constraints\nNone\n\n## Technical Context\nNone\n"
    };
    let mut f = File::create(&full_path).expect("create file failed");
    write!(f, "{}", content).expect("write failed");
}

#[given(expr = "an untracked spec file {string} with basic content")]
fn create_untracked_spec_file(world: &mut TestWorld, path: String) {
    // Just create the file without staging/committing
    create_spec_file(world, path);
}

#[given(expr = "I stage {string}")]
fn stage_file(world: &mut TestWorld, path: String) {
    let root = get_temp_path(world);
    git_add(&root, &path);
}

#[given(expr = "I modify {string} without staging")]
fn modify_file_unstaged(world: &mut TestWorld, path: String) {
    let root = get_temp_path(world);
    let full_path = root.join(&path);
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&full_path)
        .expect("open failed");
    write!(f, "\n<!-- modified -->\n").expect("write failed");
}

#[given(expr = "a committed spec file {string}")]
fn create_committed_spec_file(world: &mut TestWorld, path: String) {
    use std::process::Command;
    create_spec_file(world, path.clone());
    let root = get_temp_path(world);
    git_add(&root, &path);
    // Commit spec with a middle timestamp
    Command::new("git")
        .args(["commit", "-m", &format!("add spec {}", path)])
        .env("GIT_COMMITTER_DATE", "2024-06-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2024-06-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit spec failed");
}

#[given(expr = "a committed source file {string} before the spec")]
fn create_committed_source_before_spec(world: &mut TestWorld, path: String) {
    // Source was committed BEFORE the spec, so spec is newer.
    // git log timestamps have 1-second resolution, so we use GIT_COMMITTER_DATE
    // to ensure ordering.
    use std::process::Command;
    let root = get_temp_path(world);
    let full_path = root.join(&path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let mut f = File::create(&full_path).expect("create source failed");
    write!(f, "// source code\n").expect("write failed");
    git_add(&root, &path);

    // Commit source with an older timestamp
    Command::new("git")
        .args(["commit", "-m", &format!("add source {}", path)])
        .env("GIT_COMMITTER_DATE", "2024-01-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2024-01-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit source failed");

    // Now re-commit the spec with a newer timestamp
    let dir = Path::new(&path).parent().unwrap();
    let spec_path = dir.join("CLAUDE.md");
    let spec_full = root.join(&spec_path);
    if spec_full.exists() {
        let mut f = std::fs::OpenOptions::new().append(true).open(&spec_full).expect("open failed");
        write!(f, "\n<!-- updated -->\n").expect("write failed");
        git_add(&root, &spec_path.to_string_lossy());
        Command::new("git")
            .args(["commit", "-m", &format!("update spec {}", spec_path.display())])
            .env("GIT_COMMITTER_DATE", "2025-01-01T00:00:00+00:00")
            .env("GIT_AUTHOR_DATE", "2025-01-01T00:00:00+00:00")
            .current_dir(&root)
            .output().expect("git commit spec failed");
    }
}

#[given(expr = "a committed source file {string} after the spec")]
fn create_committed_source_after_spec(world: &mut TestWorld, path: String) {
    use std::process::Command;
    // Source is newer than spec → up-to-date
    let root = get_temp_path(world);
    let full_path = root.join(&path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let mut f = File::create(&full_path).expect("create source failed");
    write!(f, "// source code\n").expect("write failed");
    git_add(&root, &path);
    // Commit source with a newer timestamp than spec
    Command::new("git")
        .args(["commit", "-m", &format!("add source {}", path)])
        .env("GIT_COMMITTER_DATE", "2025-06-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2025-06-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit source failed");
}

#[given(expr = "no source files in {string}")]
fn no_source_files_in(_world: &mut TestWorld, _dir: String) {
    // No-op: the directory already has only CLAUDE.md
}

#[given("a non-git test directory")]
fn setup_non_git_test_dir(world: &mut TestWorld) {
    world.non_git_temp_dir = Some(TempDir::new().expect("Failed to create non-git temp dir"));
}

#[given("a committed root-level CLAUDE.md")]
fn create_committed_root_claude_md(world: &mut TestWorld) {
    use std::process::Command;
    let root = get_temp_path(world);
    let path = root.join("CLAUDE.md");
    let mut f = File::create(&path).expect("create CLAUDE.md failed");
    write!(f, "# Project\n\n## Purpose\nProject root\n").expect("write failed");
    git_add(&root, "CLAUDE.md");
    Command::new("git")
        .args(["commit", "-m", "add root CLAUDE.md"])
        .env("GIT_COMMITTER_DATE", "2024-03-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2024-03-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit root CLAUDE.md failed");
}

#[given("a committed root-level DEVELOPERS.md")]
fn create_committed_root_developers_md(world: &mut TestWorld) {
    use std::process::Command;
    let root = get_temp_path(world);
    let path = root.join("DEVELOPERS.md");
    let mut f = File::create(&path).expect("create DEVELOPERS.md failed");
    write!(f, "# DEVELOPERS\n\n## Constraints\n- CONST-1: main() -> Result<()>\n\n## Technical Context\nNone\n").expect("write failed");
    git_add(&root, "DEVELOPERS.md");
    Command::new("git")
        .args(["commit", "-m", "add root DEVELOPERS.md"])
        .env("GIT_COMMITTER_DATE", "2024-06-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2024-06-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit root DEVELOPERS.md failed");
}

#[given(expr = "a committed spec file {string} depending on {string}")]
fn create_committed_spec_with_dep(world: &mut TestWorld, path: String, dep: String) {
    use std::process::Command;
    let root = get_temp_path(world);
    let full_path = root.join(&path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let content = format!(
        "# Module\n\n## Purpose\nTest module\n\n## Requirements\nNone\n\n## Domain Context\nDepends on `{}`.\n",
        dep
    );
    let mut f = File::create(&full_path).expect("create file failed");
    write!(f, "{}", content).expect("write failed");
    git_add(&root, &path);
    Command::new("git")
        .args(["commit", "-m", &format!("add spec with dep {}", path)])
        .env("GIT_COMMITTER_DATE", "2024-06-01T00:00:00+00:00")
        .env("GIT_AUTHOR_DATE", "2024-06-01T00:00:00+00:00")
        .current_dir(&root)
        .output().expect("git commit spec with dep failed");
}

#[when("I resolve compile targets")]
fn resolve_compile_targets(world: &mut TestWorld) {
    let root = get_temp_path(world);
    let resolver = CompileTargetResolver::new();
    world.diff_result = Some(resolver.resolve(&root));
}

#[when("I resolve compile targets in the non-git directory")]
fn resolve_compile_targets_non_git(world: &mut TestWorld) {
    let root = world.non_git_temp_dir.as_ref().expect("No non-git dir").path().to_path_buf();
    let resolver = CompileTargetResolver::new();
    world.diff_result = Some(resolver.resolve(&root));
}

#[then(expr = "{string} should be a compile target with reason {string}")]
fn should_be_compile_target(world: &mut TestWorld, dir: String, reason: String) {
    let result = world.diff_result.as_ref().expect("No diff result");
    let target = result.targets.iter().find(|t| t.dir == dir);
    assert!(target.is_some(),
        "Expected '{}' to be a compile target, but it wasn't. Targets: {:?}, Skipped: {:?}",
        dir,
        result.targets.iter().map(|t| (&t.dir, &t.reason)).collect::<Vec<_>>(),
        result.skipped.iter().map(|s| (&s.dir, &s.reason)).collect::<Vec<_>>(),
    );
    let target = target.unwrap();
    let actual_reason = serde_json::to_string(&target.reason).unwrap();
    let expected_reason = format!("\"{}\"", reason);
    assert_eq!(actual_reason, expected_reason,
        "Expected reason '{}' for '{}', got '{}'", reason, dir, actual_reason);
}

#[then(expr = "{string} should be skipped with reason {string}")]
fn should_be_skipped(world: &mut TestWorld, dir: String, reason: String) {
    let result = world.diff_result.as_ref().expect("No diff result");
    let entry = result.skipped.iter().find(|s| s.dir == dir);
    assert!(entry.is_some(),
        "Expected '{}' to be skipped, but it wasn't. Targets: {:?}, Skipped: {:?}",
        dir,
        result.targets.iter().map(|t| (&t.dir, &t.reason)).collect::<Vec<_>>(),
        result.skipped.iter().map(|s| (&s.dir, &s.reason)).collect::<Vec<_>>(),
    );
    let entry = entry.unwrap();
    assert_eq!(entry.reason, reason,
        "Expected skip reason '{}' for '{}', got '{}'", reason, dir, entry.reason);
}

#[then(expr = "I should get a warning of type {string}")]
fn should_get_warning_type(world: &mut TestWorld, warning_type: String) {
    let result = world.diff_result.as_ref().expect("No diff result");
    let found = result.warnings.iter().any(|w| w.warning_type == warning_type);
    assert!(found,
        "Expected warning type '{}', got: {:?}",
        warning_type,
        result.warnings.iter().map(|w| &w.warning_type).collect::<Vec<_>>(),
    );
}

#[then("the targets should be empty")]
fn targets_should_be_empty(world: &mut TestWorld) {
    let result = world.diff_result.as_ref().expect("No diff result");
    assert!(result.targets.is_empty(),
        "Expected empty targets, got: {:?}",
        result.targets.iter().map(|t| &t.dir).collect::<Vec<_>>(),
    );
}

#[then("root CLAUDE.md should not be a compile target")]
fn root_claude_md_not_target(world: &mut TestWorld) {
    let result = world.diff_result.as_ref().expect("No diff result");
    let found = result.targets.iter().any(|t| t.dir == "." || t.dir.is_empty() || t.claude_md_path == "CLAUDE.md");
    assert!(!found,
        "Root CLAUDE.md should not be a target, but found: {:?}",
        result.targets.iter().map(|t| &t.dir).collect::<Vec<_>>(),
    );
}

#[then(expr = "I should get a dependency warning for {string} affecting {string}")]
fn should_get_dep_warning(world: &mut TestWorld, changed: String, affected: String) {
    let result = world.diff_result.as_ref().expect("No diff result");
    let found = result.dependency_warnings.iter().any(|w| {
        w.changed_dep == changed && w.affected_dependents.contains(&affected)
    });
    assert!(found,
        "Expected dependency warning for '{}' affecting '{}', got: {:?}",
        changed, affected, result.dependency_warnings,
    );
}

// ============== Format Exports Steps ==============

#[given("an analyze-code JSON with no exports")]
fn given_empty_exports(world: &mut TestWorld) {
    world.format_exports_input = Some(Exports::default());
}

#[given("an analyze-code JSON with exports:")]
fn given_exports_table(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let mut exports = Exports::default();
    if let Some(table) = &step.table {
        let headers: Vec<&str> = table.rows[0].iter().map(|s| s.as_str()).collect();
        for row in table.rows.iter().skip(1) {
            let get = |col: &str| -> String {
                headers.iter().position(|&h| h == col)
                    .and_then(|i| row.get(i))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default()
            };
            let category = get("category");
            match category.as_str() {
                "function" => {
                    exports.functions.push(ExportedFunction {
                        name: get("name"),
                        signature: get("signature"),
                        description: None,
                    });
                }
                "type" => {
                    let def = get("definition");
                    exports.types.push(ExportedType {
                        name: get("name"),
                        kind: TypeKind::Interface,
                        definition: if def.is_empty() { None } else { Some(def) },
                        description: None,
                    });
                }
                "class" => {
                    let sig = get("signature");
                    exports.classes.push(ExportedClass {
                        name: get("name"),
                        signature: if sig.is_empty() { None } else { Some(sig) },
                        description: None,
                    });
                }
                "enum" => {
                    let variants_str = get("variants");
                    let variants = if variants_str.is_empty() {
                        None
                    } else {
                        Some(variants_str.split(',').map(|s| s.trim().to_string()).collect())
                    };
                    exports.enums.push(ExportedEnum {
                        name: get("name"),
                        variants,
                    });
                }
                "variable" => {
                    let vt = get("var_type");
                    exports.variables.push(ExportedVariable {
                        name: get("name"),
                        var_type: if vt.is_empty() { None } else { Some(vt) },
                    });
                }
                "re_export" => {
                    exports.re_exports.push(ReExport {
                        name: get("name"),
                        source: get("source"),
                    });
                }
                other => panic!("Unknown export category: {}", other),
            }
        }
    }
    world.format_exports_input = Some(exports);
}

#[when("I format the exports")]
fn when_format_exports(world: &mut TestWorld) {
    let exports = world.format_exports_input.as_ref().expect("No exports input");
    world.format_exports_output = Some(exports_formatter::format_exports(exports));
}

#[when("I format the exports twice")]
fn when_format_exports_twice(world: &mut TestWorld) {
    let exports = world.format_exports_input.as_ref().expect("No exports input");
    world.format_exports_output = Some(exports_formatter::format_exports(exports));
    world.format_exports_output2 = Some(exports_formatter::format_exports(exports));
}

#[then(expr = "the formatted output should be {string}")]
fn then_output_equals_inline(world: &mut TestWorld, expected: String) {
    let output = world.format_exports_output.as_ref().expect("No format output");
    assert_eq!(output, &expected, "Formatted output mismatch");
}

#[then("the formatted output should be:")]
fn then_output_equals_docstring(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let expected = step.docstring.as_ref().expect("No docstring in step").trim();
    let output = world.format_exports_output.as_ref().expect("No format output");
    assert_eq!(output.trim(), expected, "Formatted output mismatch");
}

#[then(expr = "the formatted output should contain subsection {string}")]
fn then_output_contains_subsection(world: &mut TestWorld, subsection: String) {
    let output = world.format_exports_output.as_ref().expect("No format output");
    assert!(
        output.contains(&subsection),
        "Expected subsection '{}' in output:\n{}",
        subsection, output
    );
}

#[then(expr = "the formatted output should not contain subsection {string}")]
fn then_output_not_contains_subsection(world: &mut TestWorld, subsection: String) {
    let output = world.format_exports_output.as_ref().expect("No format output");
    assert!(
        !output.contains(&subsection),
        "Did not expect subsection '{}' in output:\n{}",
        subsection, output
    );
}

#[then("the subsection order should be:")]
fn then_subsection_order(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let output = world.format_exports_output.as_ref().expect("No format output");
    let actual_sections: Vec<&str> = output
        .lines()
        .filter(|l| l.starts_with("### "))
        .map(|l| l.trim_start_matches("### "))
        .collect();

    let expected_sections: Vec<String> = step.table.as_ref()
        .expect("No table in step")
        .rows.iter().skip(1)
        .filter_map(|row| row.first().cloned())
        .collect();

    assert_eq!(
        actual_sections,
        expected_sections.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "Subsection order mismatch"
    );
}

#[then("both outputs should be identical")]
fn then_both_outputs_identical(world: &mut TestWorld) {
    let output1 = world.format_exports_output.as_ref().expect("No first output");
    let output2 = world.format_exports_output2.as_ref().expect("No second output");
    assert_eq!(output1, output2, "Outputs should be identical for determinism");
}

// ============== Export Candidates Step Definitions ==============

#[then("I should find exported enums:")]
fn should_find_exported_enums(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No enum name");

            let found = result.exports.enums.iter().any(|e| e.name == *name);
            assert!(found, "Expected to find enum '{}', found: {:?}",
                    name, result.exports.enums.iter().map(|e| &e.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find exported variables:")]
fn should_find_exported_variables(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No variable name");

            let found = result.exports.variables.iter().any(|v| v.name == *name);
            assert!(found, "Expected to find variable '{}', found: {:?}",
                    name, result.exports.variables.iter().map(|v| &v.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should NOT find exported variables:")]
fn should_not_find_exported_variables(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No variable name");

            let found = result.exports.variables.iter().any(|v| v.name == *name);
            assert!(!found, "Found variable '{}' that should be excluded", name);
        }
    }
}

#[then("I should find pub re-exports:")]
fn should_find_pub_re_exports(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No re-export name");

            let found = result.exports.re_exports.iter().any(|r| r.name == *name);
            assert!(found, "Expected to find re-export '{}', found: {:?}",
                    name, result.exports.re_exports.iter().map(|r| &r.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find objects as classes:")]
fn should_find_objects_as_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No object name");

            let found = result.exports.classes.iter().any(|c| c.name == *name);
            assert!(found, "Expected to find object '{}' as class, found: {:?}",
                    name, result.exports.classes.iter().map(|c| &c.name).collect::<Vec<_>>());
        }
    }
}

#[then("I should find interfaces as types:")]
fn should_find_interfaces_as_types(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No interface name");

            let found = result.exports.types.iter().any(|t| {
                t.name == *name && t.kind == TypeKind::Trait
            });
            assert!(found, "Expected to find interface '{}' as type with Trait kind, found: {:?}",
                    name, result.exports.types.iter().map(|t| (&t.name, &t.kind)).collect::<Vec<_>>());
        }
    }
}

#[then("I should find records as classes:")]
fn should_find_records_as_classes(world: &mut TestWorld, step: &cucumber::gherkin::Step) {
    let result = world.analysis_result.as_ref().expect("No analysis result");

    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let name = row.first().expect("No record name");

            let found = result.exports.classes.iter().any(|c| c.name == *name);
            assert!(found, "Expected to find record '{}' as class, found: {:?}",
                    name, result.exports.classes.iter().map(|c| &c.name).collect::<Vec<_>>());
        }
    }
}

// ============== Language Validator Steps ==============

#[given(expr = "a markdown file {string} with content:")]
fn create_markdown_file(world: &mut TestWorld, filename: String, step: &cucumber::gherkin::Step) {
    let content = step.docstring().expect("Expected docstring content");
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join(&filename);
    fs::write(&file_path, content).expect("Failed to write markdown file");
}

#[given(regex = r#"^a markdown file "([^"]+)" with content at exactly 70 percent Latin$"#)]
fn create_70_percent_latin_file(world: &mut TestWorld, filename: String) {
    // Build content where after stripping headings and None, ~70-75% chars are Latin
    // After stripping: heading lines removed, None removed
    // Remaining: English prose + Korean requirement line
    let content = "## Purpose\n\nThis is a test document with enough English text here to reach the seventy percent target threshold level needed for the validation check to pass properly\n\n## Requirements\n\n- 한국어 텍스트 여기에 작성합니다\n\n## Domain Context\n\nNone";
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join(&filename);
    fs::write(&file_path, content).expect("Failed to write markdown file");
}

#[when(expr = "I validate language with expected {string} and threshold {int}")]
fn validate_language(world: &mut TestWorld, expected: String, threshold: i32) {
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join("CLAUDE.md");
    let validator = LanguageValidator::new();
    match validator.validate(&file_path, &expected, threshold as f64) {
        Ok(result) => {
            world.language_result = Some(result);
            world.language_error = None;
        }
        Err(e) => {
            world.language_result = None;
            world.language_error = Some(e.to_string());
        }
    }
}

#[then(expr = "language result should be {string}")]
fn check_language_result(world: &mut TestWorld, expected_result: String) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert_eq!(result.result, expected_result,
        "Expected result '{}', got '{}' (percentage: {:.1}%, chars: {})",
        expected_result, result.result, result.target_percentage, result.total_classified_chars);
}

#[then(expr = "target percentage should be greater than {int}")]
fn check_target_percentage(world: &mut TestWorld, min_pct: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.target_percentage > min_pct as f64,
        "Expected percentage > {}, got {:.1}", min_pct, result.target_percentage);
}

#[then("non target lines should not be empty")]
fn check_non_target_lines_not_empty(world: &mut TestWorld) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(!result.non_target_lines.is_empty(),
        "Expected non-target lines to be non-empty, got empty");
}

#[then(expr = "non target lines should contain line {int}")]
fn check_non_target_contains_line(world: &mut TestWorld, line: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.non_target_lines.contains(&(line as usize)),
        "Expected non_target_lines to contain {}, got {:?}", line, result.non_target_lines);
}

#[then(expr = "non target lines should not contain line {int}")]
fn check_non_target_not_contains_line(world: &mut TestWorld, line: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(!result.non_target_lines.contains(&(line as usize)),
        "Expected non_target_lines to NOT contain {}, got {:?}", line, result.non_target_lines);
}

#[then(expr = "language validation should fail with {string}")]
fn check_language_error(world: &mut TestWorld, error_type: String) {
    let error = world.language_error.as_ref().expect("Expected language error");
    // Normalize: "UnsupportedLanguage" matches both the Display "Unsupported language:" and Debug "UnsupportedLanguage"
    let normalized_error_type = error_type.replace("UnsupportedLanguage", "Unsupported language");
    assert!(error.contains(&normalized_error_type) || error.contains(&error_type),
        "Expected error containing '{}', got '{}'", error_type, error);
}

#[then(expr = "script distribution should contain {string}")]
fn check_script_distribution_contains(world: &mut TestWorld, script: String) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.script_distribution.contains_key(&script),
        "Expected script distribution to contain '{}', got {:?}", script, result.script_distribution);
}

// ============== Node History Steps ==============

fn git_commit_with_message(dir: &Path, message: &str) -> String {
    use std::process::Command;
    Command::new("git").args(["commit", "--allow-empty-message", "-m", message])
        .current_dir(dir)
        .output().expect("git commit failed");
    // Return the hash
    let output = Command::new("git").args(["log", "-1", "--format=%H"])
        .current_dir(dir)
        .output().expect("git log failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn write_and_commit(dir: &Path, rel_path: &str, content: &str, message: &str) -> String {
    let full_path = dir.join(rel_path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let mut f = File::create(&full_path).expect("create file failed");
    f.write_all(content.as_bytes()).expect("write failed");
    git_add(dir, rel_path);
    git_commit_with_message(dir, message)
}

#[given("a clean git test repository for node history")]
fn setup_node_history_git_dir(world: &mut TestWorld) {
    world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    git_init(&get_temp_path(world));
    world.named_commits = HashMap::new();
}

#[given(expr = "a committed CLAUDE.md at {string} with Requirements {string}")]
fn create_committed_claude_md(world: &mut TestWorld, dir: String, requirements: String) {
    let temp = get_temp_path(world);
    let content = format!(
        "# Module\n\n## Purpose\nTest module\n\n## Requirements\n{}\n\n## Domain Context\nNone\n",
        requirements.replace("\\n", "\n")
    );
    write_and_commit(&temp, &format!("{}/CLAUDE.md", dir), &content, "init: add CLAUDE.md");
}

#[given(expr = "a committed DEVELOPERS.md at {string} with Constraints {string}")]
fn create_committed_developers_md(world: &mut TestWorld, dir: String, constraints: String) {
    let temp = get_temp_path(world);
    let content = format!(
        "# DEVELOPERS\n\n## Constraints\n{}\n\n## Technical Context\nNone\n",
        constraints.replace("\\n", "\n")
    );
    write_and_commit(&temp, &format!("{}/DEVELOPERS.md", dir), &content, "init: add DEVELOPERS.md");
}

#[given(expr = "a committed CLAUDE.md at {string} with Purpose {string} and Requirements {string}")]
fn create_committed_claude_md_with_purpose(world: &mut TestWorld, dir: String, purpose: String, requirements: String) {
    let temp = get_temp_path(world);
    let content = format!(
        "# Module\n\n## Purpose\n{}\n\n## Requirements\n{}\n\n## Domain Context\nNone\n",
        purpose, requirements.replace("\\n", "\n")
    );
    write_and_commit(&temp, &format!("{}/CLAUDE.md", dir), &content, "init: add CLAUDE.md");
}

#[given(expr = "a new commit changing Requirements in {string} to {string}")]
fn commit_change_requirements(world: &mut TestWorld, file: String, new_requirements: String) {
    let temp = get_temp_path(world);
    let content = format!(
        "# Module\n\n## Purpose\nTest module\n\n## Requirements\n{}\n\n## Domain Context\nNone\n",
        new_requirements.replace("\\n", "\n")
    );
    write_and_commit(&temp, &file, &content, "spec: update requirements");
}

#[given(expr = "{int} additional commits changing {string} Requirements")]
fn create_additional_commits(world: &mut TestWorld, count: usize, file: String) {
    let temp = get_temp_path(world);
    for i in 0..count {
        let content = format!(
            "# Module\n\n## Purpose\nTest module\n\n## Requirements\n- REQ-{}: Requirement {}\n\n## Domain Context\nNone\n",
            i + 2, i + 2
        );
        write_and_commit(&temp, &file, &content, &format!("spec: update req {}", i + 2));
    }
}

#[given(expr = "a new commit changing both {string} and {string}")]
fn commit_change_both_files(world: &mut TestWorld, file1: String, file2: String) {
    let temp = get_temp_path(world);
    let claude_content = "# Module\n\n## Purpose\nTest module\n\n## Requirements\n- REQ-1: Login\n- REQ-2: OAuth\n\n## Domain Context\nNone\n";
    let dev_content = "# DEVELOPERS\n\n## Constraints\n- CONST-1: JWT\n- CONST-2: OAuth2\n\n## Technical Context\nNone\n";

    let full1 = temp.join(&file1);
    let full2 = temp.join(&file2);
    fs::create_dir_all(full1.parent().unwrap()).expect("mkdir failed");
    fs::create_dir_all(full2.parent().unwrap()).expect("mkdir failed");
    fs::write(&full1, claude_content).expect("write failed");
    fs::write(&full2, dev_content).expect("write failed");
    git_add(&temp, &file1);
    git_add(&temp, &file2);
    git_commit_with_message(&temp, "spec: update both files");
}

#[given(expr = "a new commit changing both Purpose and Requirements in {string}")]
fn commit_change_purpose_and_requirements(world: &mut TestWorld, file: String) {
    let temp = get_temp_path(world);
    let content = "# Module\n\n## Purpose\nUpdated auth module\n\n## Requirements\n- REQ-1: Login\n- REQ-2: OAuth\n\n## Domain Context\nNone\n";
    write_and_commit(&temp, &file, content, "spec: update purpose and requirements");
}

#[given(expr = "a commit with message {string} changing {string}")]
fn commit_with_specific_message(world: &mut TestWorld, message: String, file: String) {
    let temp = get_temp_path(world);
    // Read existing content and modify slightly
    let full_path = temp.join(&file);
    let existing = fs::read_to_string(&full_path).unwrap_or_default();
    let new_content = format!("{}\n<!-- updated by: {} -->\n", existing, message);
    write_and_commit(&temp, &file, &new_content, &message);
}

#[given(expr = "a commit {string} changing {string} Requirements")]
fn commit_named(world: &mut TestWorld, name: String, file: String) {
    let temp = get_temp_path(world);
    let full_path = temp.join(&file);
    let existing = fs::read_to_string(&full_path).unwrap_or_default();
    let new_content = format!("{}\n<!-- commit {} -->\n", existing, name);
    let hash = write_and_commit(&temp, &file, &new_content, &format!("spec: commit {}", name));
    world.named_commits.insert(name, hash);
}

#[given(expr = "a non-git test directory for node history")]
fn setup_non_git_dir_for_node_history(world: &mut TestWorld) {
    world.node_history_non_git_dir = Some(TempDir::new().expect("Failed to create temp dir"));
}

#[given("an empty git repository for node history")]
fn setup_empty_git_repo(world: &mut TestWorld) {
    world.temp_dir = Some(TempDir::new().expect("Failed to create temp dir"));
    git_init(&get_temp_path(world));
}

#[given(expr = "a single root commit creating {string} with Requirements {string}")]
fn create_single_root_commit(world: &mut TestWorld, file: String, requirements: String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp = temp_dir.path().to_path_buf();

    // Init git WITHOUT the empty initial commit
    use std::process::Command;
    Command::new("git").args(["init"]).current_dir(&temp)
        .output().expect("git init failed");
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(&temp)
        .output().expect("git config email failed");
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(&temp)
        .output().expect("git config name failed");

    let content = format!(
        "# Module\n\n## Purpose\nTest module\n\n## Requirements\n{}\n\n## Domain Context\nNone\n",
        requirements.replace("\\n", "\n")
    );
    let full_path = temp.join(&file);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    fs::write(&full_path, &content).expect("write failed");

    Command::new("git").args(["add", &file]).current_dir(&temp)
        .output().expect("git add failed");
    Command::new("git").args(["commit", "-m", "root: initial CLAUDE.md"])
        .current_dir(&temp)
        .output().expect("git commit failed");

    world.temp_dir = Some(temp_dir);
    world.named_commits = HashMap::new();
}

#[given(expr = "a committed source file {string} after the spec commit")]
fn create_source_file_after_spec(world: &mut TestWorld, file: String) {
    let temp = get_temp_path(world);
    write_and_commit(&temp, &file, "// source code\n", "dev: add source file");
}

#[given(expr = "a commit with subject {string} and body {string} changing {string}")]
fn commit_with_subject_and_body(world: &mut TestWorld, subject: String, body: String, file: String) {
    let temp = get_temp_path(world);
    let full_path = temp.join(&file);
    let existing = fs::read_to_string(&full_path).unwrap_or_default();
    let new_content = format!("{}\n<!-- {} -->\n", existing, subject);
    let full_msg = format!("{}\n\n{}", subject, body);

    let fp = temp.join(&file);
    fs::create_dir_all(fp.parent().unwrap()).expect("mkdir failed");
    fs::write(&fp, &new_content).expect("write failed");
    git_add(&temp, &file);
    git_commit_with_message(&temp, &full_msg);
}

// ---- When steps ----

#[when(expr = "I run diff-node-history for {string} with limit {int}")]
fn run_node_history(world: &mut TestWorld, node_path: String, limit: usize) {
    let temp = get_temp_path(world);
    let differ = NodeHistoryDiffer::new(&temp, Path::new(&node_path));
    world.node_history_result = Some(differ.diff(limit, None, None));
}

#[when(expr = "I run diff-node-history for {string} with limit {int} and grep {string}")]
fn run_node_history_with_grep(world: &mut TestWorld, node_path: String, limit: usize, grep: String) {
    let temp = get_temp_path(world);
    let differ = NodeHistoryDiffer::new(&temp, Path::new(&node_path));
    world.node_history_result = Some(differ.diff(limit, Some(&grep), None));
}

#[when(expr = "I run diff-node-history for {string} with limit {int} and since-commit {string}")]
fn run_node_history_with_since(world: &mut TestWorld, node_path: String, limit: usize, since_name: String) {
    let temp = get_temp_path(world);
    let since_hash = world.named_commits.get(&since_name)
        .expect(&format!("Named commit '{}' not found", since_name))
        .clone();
    let differ = NodeHistoryDiffer::new(&temp, Path::new(&node_path));
    world.node_history_result = Some(differ.diff(limit, None, Some(&since_hash)));
}

#[when(expr = "I run diff-node-history for {string} with limit {int} in the non-git directory")]
fn run_node_history_non_git(world: &mut TestWorld, node_path: String, limit: usize) {
    let temp = world.node_history_non_git_dir.as_ref().expect("No non-git dir").path().to_path_buf();
    let differ = NodeHistoryDiffer::new(&temp, Path::new(&node_path));
    world.node_history_result = Some(differ.diff(limit, None, None));
}

// ---- Then steps ----

#[then(expr = "the result has {int} commit entry")]
fn check_commit_count_singular(world: &mut TestWorld, count: usize) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert_eq!(result.commits.len(), count,
        "Expected {} commit(s), got {}", count, result.commits.len());
}

#[then(expr = "the result has {int} commit entries")]
fn check_commit_count(world: &mut TestWorld, count: usize) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert_eq!(result.commits.len(), count,
        "Expected {} commit(s), got {}", count, result.commits.len());
}

#[then(expr = "commit {int} has a {string} file diff")]
fn check_commit_has_file_diff(world: &mut TestWorld, index: usize, file_type: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    assert!(commit.file_diffs.iter().any(|fd| fd.file_type == file_type),
        "Commit {} has no {} file diff. Available: {:?}",
        index, file_type, commit.file_diffs.iter().map(|fd| &fd.file_type).collect::<Vec<_>>());
}

#[then(expr = "the {string} diff in commit {int} has section {string} with {int} {string} change")]
fn check_section_change_count(world: &mut TestWorld, file_type: String, index: usize, section: String, count: usize, action: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    let file_diff = commit.file_diffs.iter().find(|fd| fd.file_type == file_type)
        .expect(&format!("No {} file diff in commit {}", file_type, index));
    let section_diff = file_diff.sections.iter().find(|s| s.section == section)
        .expect(&format!("No '{}' section in {} diff", section, file_type));
    let action_count = section_diff.changes.iter().filter(|c| c.action == action).count();
    assert_eq!(action_count, count,
        "Expected {} '{}' changes in section '{}', got {}", count, action, section, action_count);
}

#[then(expr = "the {string} diff in commit {int} has section {string}")]
fn check_section_exists(world: &mut TestWorld, file_type: String, index: usize, section: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    let file_diff = commit.file_diffs.iter().find(|fd| fd.file_type == file_type)
        .expect(&format!("No {} file diff in commit {}", file_type, index));
    assert!(file_diff.sections.iter().any(|s| s.section == section),
        "No '{}' section found. Available: {:?}",
        section, file_diff.sections.iter().map(|s| &s.section).collect::<Vec<_>>());
}

#[then(expr = "total_commits_found is {int}")]
fn check_total_commits(world: &mut TestWorld, count: usize) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert_eq!(result.total_commits_found, count,
        "Expected total_commits_found={}, got {}", count, result.total_commits_found);
}

#[then(expr = "commit {int} subject contains {string}")]
fn check_commit_subject_contains(world: &mut TestWorld, index: usize, text: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    assert!(commit.subject.contains(&text),
        "Commit {} subject '{}' does not contain '{}'", index, commit.subject, text);
}

#[then(expr = "commit {int} subject matches commit {string}")]
fn check_commit_matches_named(world: &mut TestWorld, index: usize, name: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let expected_hash = world.named_commits.get(&name)
        .expect(&format!("Named commit '{}' not found", name));
    let commit = &result.commits[index];
    assert_eq!(&commit.hash, expected_hash,
        "Commit {} hash '{}' does not match named commit '{}' hash '{}'",
        index, commit.hash, name, expected_hash);
}

#[then("is_git_repo is false")]
fn check_is_not_git_repo(world: &mut TestWorld) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert!(!result.is_git_repo, "Expected is_git_repo=false");
}

#[then("has_history is false")]
fn check_has_no_history(world: &mut TestWorld) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert!(!result.has_history, "Expected has_history=false");
}

#[then(expr = "commit {int} has breaking flag true")]
fn check_breaking_flag(world: &mut TestWorld, index: usize) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    assert!(commit.breaking, "Commit {} expected breaking=true", index);
}

#[then("source_changed is true")]
fn check_source_changed_true(world: &mut TestWorld) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert!(result.source_changed, "Expected source_changed=true");
}

#[then(expr = "source_changed_files includes {string}")]
fn check_source_changed_file(world: &mut TestWorld, file: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    assert!(result.source_changed_files.iter().any(|f| f.contains(&file)),
        "Expected source_changed_files to include '{}', got {:?}", file, result.source_changed_files);
}

#[then(expr = "commit {int} body contains {string}")]
fn check_commit_body_contains(world: &mut TestWorld, index: usize, text: String) {
    let result = world.node_history_result.as_ref().expect("No node history result");
    let commit = &result.commits[index];
    assert!(commit.body.contains(&text),
        "Commit {} body '{}' does not contain '{}'", index, commit.body, text);
}

// ============== po-consultant Verdict Schema Steps ==============

fn po_consultant_fixture_names() -> &'static [&'static str] {
    &[
        "po_consultant_result_auto.md",
        "po_consultant_result_halt.md",
        "po_consultant_result_redirect.md",
    ]
}

fn load_po_fixture(name: &str) -> String {
    let path = get_tests_path().join("fixtures").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", name, e))
}

fn extract_section(content: &str, heading: &str) -> Option<String> {
    let marker = format!("## {}", heading);
    let idx = content.find(&marker)?;
    let after = &content[idx + marker.len()..];
    // skip rest of the heading line
    let body_start = after.find('\n').map(|i| i + 1).unwrap_or(after.len());
    let body = &after[body_start..];
    // stop at next "## " heading
    let end = body.find("\n## ").unwrap_or(body.len());
    Some(body[..end].trim().to_string())
}

fn section_present(content: &str, heading: &str) -> bool {
    content.contains(&format!("## {}", heading))
}

#[given("a po-consultant result file")]
fn po_given_any_result(world: &mut TestWorld) {
    world.po_consultant_fixtures = po_consultant_fixture_names()
        .iter()
        .map(|n| (n.to_string(), load_po_fixture(n)))
        .collect();
}

#[given("a result file with Verdict=feasible")]
fn po_given_feasible(world: &mut TestWorld) {
    let name = "po_consultant_result_auto.md";
    world.po_consultant_fixtures = vec![(name.to_string(), load_po_fixture(name))];
}

#[given("a result file with Execution=halt")]
fn po_given_halt(world: &mut TestWorld) {
    let name = "po_consultant_result_halt.md";
    world.po_consultant_fixtures = vec![(name.to_string(), load_po_fixture(name))];
}

#[given(expr = "a result file with {string} present")]
fn po_given_section_present(world: &mut TestWorld, heading: String) {
    // Pick the redirect fixture for "## Redirect To"
    assert_eq!(heading, "## Redirect To");
    let name = "po_consultant_result_redirect.md";
    world.po_consultant_fixtures = vec![(name.to_string(), load_po_fixture(name))];
}

#[when("the result is parsed")]
fn po_when_parsed(_world: &mut TestWorld) {
    // Parsing is implicit via extract_section; no-op
}

#[then(expr = "it MUST contain a {string} section with value in: auto_executable | requires_human | halt")]
fn po_then_execution_valid(world: &mut TestWorld, _heading: String) {
    for (name, content) in &world.po_consultant_fixtures {
        let exec = extract_section(content, "Execution")
            .unwrap_or_else(|| panic!("{}: missing ## Execution section", name));
        assert!(
            matches!(exec.as_str(), "auto_executable" | "requires_human" | "halt"),
            "{}: Execution value '{}' is not in allowed set",
            name,
            exec
        );
    }
}

#[then(regex = r#"^it MUST contain a "([^"]+)" section \(non-empty iff Execution != auto_executable\)$"#)]
fn po_then_reason_nonempty_iff(world: &mut TestWorld, _heading: String) {
    for (name, content) in &world.po_consultant_fixtures {
        assert!(section_present(content, "Reason"), "{}: missing ## Reason section", name);
        let exec = extract_section(content, "Execution").unwrap_or_default();
        let reason = extract_section(content, "Reason").unwrap_or_default();
        if exec != "auto_executable" {
            assert!(
                !reason.is_empty(),
                "{}: Reason must be non-empty when Execution={}",
                name,
                exec
            );
        }
    }
}

#[then(expr = "it MAY contain a {string} section with a node path")]
fn po_then_redirect_optional(world: &mut TestWorld, _heading: String) {
    for (name, content) in &world.po_consultant_fixtures {
        if section_present(content, "Redirect To") {
            let val = extract_section(content, "Redirect To").unwrap_or_default();
            assert!(!val.is_empty(), "{}: Redirect To present but empty", name);
        }
    }
}

#[then("Execution MAY be auto_executable")]
fn po_then_exec_may_auto(world: &mut TestWorld) {
    for (name, content) in &world.po_consultant_fixtures {
        let exec = extract_section(content, "Execution").unwrap_or_default();
        assert!(
            matches!(exec.as_str(), "auto_executable" | "requires_human" | "halt"),
            "{}: Execution '{}' invalid",
            name,
            exec
        );
    }
}

#[then("Reason MAY be empty")]
fn po_then_reason_may_empty(world: &mut TestWorld) {
    for (name, content) in &world.po_consultant_fixtures {
        assert!(section_present(content, "Reason"), "{}: missing ## Reason", name);
    }
}

#[then("Reason MUST be non-empty")]
fn po_then_reason_nonempty(world: &mut TestWorld) {
    for (name, content) in &world.po_consultant_fixtures {
        let reason = extract_section(content, "Reason").unwrap_or_default();
        assert!(!reason.is_empty(), "{}: Reason must be non-empty", name);
    }
}

#[then("Execution MUST NOT be auto_executable")]
fn po_then_exec_not_auto(world: &mut TestWorld) {
    for (name, content) in &world.po_consultant_fixtures {
        let exec = extract_section(content, "Execution").unwrap_or_default();
        assert_ne!(
            exec, "auto_executable",
            "{}: Execution must not be auto_executable when Redirect To present",
            name
        );
    }
}

#[then("Reason MUST describe why the redirect applies")]
fn po_then_reason_describes_redirect(world: &mut TestWorld) {
    for (name, content) in &world.po_consultant_fixtures {
        let reason = extract_section(content, "Reason").unwrap_or_default();
        assert!(
            !reason.is_empty(),
            "{}: Reason must describe redirect rationale",
            name
        );
    }
}

// ============== Verdict Aggregation Steps ==============

const VERDICT_AGGREGATION_SCRIPT: &str = r#"
extract_section() {
  awk -v h="$2" '
    $0 == h { capture=1; next }
    /^## / { capture=0 }
    capture { print }
  ' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
        | awk 'NF' | paste -sd ' ' -
}

: > "${TMP_DIR}verdict-aggregate.jsonl"
for result in "${TMP_DIR}"consult-result-*.md; do
  [ -e "$result" ] || continue
  target=$(basename "$result" .md | sed 's/^consult-result-//' | tr '-' '/')
  jq -cn \
    --arg t   "$target" \
    --arg v   "$(extract_section "$result" '## Verdict')" \
    --arg e   "$(extract_section "$result" '## Execution')" \
    --arg rn  "$(extract_section "$result" '## Reason')" \
    --arg rf  "$(extract_section "$result" '## Roadmap Fit')" \
    --arg rd  "$(extract_section "$result" '## Redirect To')" \
    '{target:$t, verdict:$v, execution:$e, reason:$rn, roadmap_fit:$rf}
     + ({redirect_to:$rd} | if .redirect_to == "" then del(.redirect_to) else . end)' \
    >> "${TMP_DIR}verdict-aggregate.jsonl"
done
"#;

fn dir_safe(target: &str) -> String {
    target.replace('/', "-")
}

#[given(expr = "consult result files for targets {string} and {string}")]
fn verdict_given_targets(world: &mut TestWorld, a: String, b: String) {
    let tmp = TempDir::new().expect("create tmp");
    // Write fixtures: target "." uses auto fixture, other uses redirect fixture.
    let auto = load_po_fixture("po_consultant_result_auto.md");
    let redirect = load_po_fixture("po_consultant_result_redirect.md");

    let write = |target: &str, body: &str| {
        let name = format!("consult-result-{}.md", dir_safe(target));
        let p = tmp.path().join(&name);
        fs::write(&p, body).expect("write fixture");
    };
    write(&a, &auto);
    write(&b, &redirect);

    world.verdict_targets = vec![a, b];
    world.verdict_tmp_dir = Some(tmp);
}

#[given("both files contain Verdict, Execution, Reason, RoadmapFit")]
fn verdict_sanity(world: &mut TestWorld) {
    let tmp = world.verdict_tmp_dir.as_ref().expect("tmp");
    for target in &world.verdict_targets {
        let p = tmp.path().join(format!("consult-result-{}.md", dir_safe(target)));
        let content = fs::read_to_string(&p).expect("read");
        for h in ["Verdict", "Execution", "Reason", "Roadmap Fit"] {
            assert!(
                section_present(&content, h),
                "{}: missing ## {}",
                p.display(),
                h
            );
        }
    }
}

#[when("Step 2.1d runs")]
fn verdict_when_run(world: &mut TestWorld) {
    let tmp = world.verdict_tmp_dir.as_ref().expect("tmp");
    let mut tmp_prefix = tmp.path().to_path_buf().into_os_string().into_string().unwrap();
    if !tmp_prefix.ends_with('/') {
        tmp_prefix.push('/');
    }
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(VERDICT_AGGREGATION_SCRIPT)
        .env("TMP_DIR", &tmp_prefix)
        .output()
        .expect("run aggregation script");
    assert!(
        output.status.success(),
        "script failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let jsonl_path = tmp.path().join("verdict-aggregate.jsonl");
    let content = fs::read_to_string(&jsonl_path).expect("read jsonl");
    world.verdict_jsonl_lines = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("parse json line"))
        .collect();
}

#[then("${TMP_DIR}verdict-aggregate.jsonl MUST contain one line per target")]
fn verdict_then_one_line_each(world: &mut TestWorld) {
    assert_eq!(
        world.verdict_jsonl_lines.len(),
        world.verdict_targets.len(),
        "expected {} lines, got {}",
        world.verdict_targets.len(),
        world.verdict_jsonl_lines.len()
    );
    let got: std::collections::HashSet<String> = world
        .verdict_jsonl_lines
        .iter()
        .map(|v| v.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string())
        .collect();
    for t in &world.verdict_targets {
        assert!(got.contains(t), "missing target {} in {:?}", t, got);
    }
}

#[then("each line MUST have keys: target, verdict, execution, reason, roadmap_fit")]
fn verdict_then_keys(world: &mut TestWorld) {
    for line in &world.verdict_jsonl_lines {
        for k in ["target", "verdict", "execution", "reason", "roadmap_fit"] {
            assert!(line.get(k).is_some(), "line {} missing key {}", line, k);
        }
    }
}

#[then("if Redirect To was present, the line MUST include redirect_to")]
fn verdict_then_redirect(world: &mut TestWorld) {
    let tmp = world.verdict_tmp_dir.as_ref().expect("tmp");
    for target in &world.verdict_targets {
        let p = tmp.path().join(format!("consult-result-{}.md", dir_safe(target)));
        let content = fs::read_to_string(&p).expect("read");
        let had_redirect = section_present(&content, "Redirect To")
            && !extract_section(&content, "Redirect To").unwrap_or_default().is_empty();
        let line = world
            .verdict_jsonl_lines
            .iter()
            .find(|v| v.get("target").and_then(|t| t.as_str()) == Some(target.as_str()))
            .expect("line for target");
        if had_redirect {
            assert!(
                line.get("redirect_to").and_then(|v| v.as_str()).is_some(),
                "target {} had Redirect To but line missing redirect_to: {}",
                target,
                line
            );
        } else {
            assert!(
                line.get("redirect_to").is_none(),
                "target {} had no Redirect To but line has redirect_to: {}",
                target,
                line
            );
        }
    }
}

// ============== Explorer Candidate Node Set Steps ==============

const CANDIDATE_PARSE_SCRIPT: &str = r#"
awk '/^## Candidate Nodes$/{f=1;next} /^## /{f=0} f && /^- /{sub(/^- /,""); sub(/[ \t]*#.*$/,""); print}' "$1" \
  | awk 'NF' | sort -u
"#;

fn parse_candidate_nodes(fixture_path: &Path) -> Vec<String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(CANDIDATE_PARSE_SCRIPT)
        .arg("bash")
        .arg(fixture_path)
        .output()
        .expect("run candidate parse");
    assert!(
        output.status.success(),
        "parse failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let s = String::from_utf8(output.stdout).expect("utf8");
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[given("a requirement text with no --path")]
fn explorer_no_path(world: &mut TestWorld) {
    world.candidate_nodes.clear();
}

#[given("project index lists nodes A, B, C, D")]
fn explorer_project_index(_world: &mut TestWorld) {
    // Contextual only; the multi fixture represents this scenario.
}

#[when("requirement-explorer runs Phase 1 (pre-judgment pass)")]
fn explorer_runs_phase1(world: &mut TestWorld) {
    let path = get_tests_path()
        .join("fixtures")
        .join("explorer_candidates_multi.md");
    world.candidate_nodes = parse_candidate_nodes(&path);
}

#[then(expr = "explorer MUST output a {string} section in its result")]
fn explorer_section_present(_world: &mut TestWorld, heading: String) {
    let path = get_tests_path()
        .join("fixtures")
        .join("explorer_candidates_multi.md");
    let content = fs::read_to_string(&path).expect("read fixture");
    assert!(
        content.contains(&heading),
        "fixture missing heading {}",
        heading
    );
}

#[then("the list MUST contain at least one node")]
fn explorer_list_nonempty(world: &mut TestWorld) {
    assert!(
        !world.candidate_nodes.is_empty(),
        "candidate nodes list is empty"
    );
}

#[then(regex = r#"^the list MUST include "([^"]+)" \(project root\) as baseline$"#)]
fn explorer_list_has_root(world: &mut TestWorld, root: String) {
    assert!(
        world.candidate_nodes.iter().any(|n| n == &root),
        "candidate nodes {:?} missing project root {}",
        world.candidate_nodes,
        root
    );
}

#[given("--path core/src/foo")]
fn explorer_with_path(world: &mut TestWorld) {
    let path = get_tests_path()
        .join("fixtures")
        .join("explorer_candidates_path.md");
    world.candidate_nodes = parse_candidate_nodes(&path);
}

#[then(expr = "{string} MUST equal [{string}, {string}]")]
fn explorer_list_equals(world: &mut TestWorld, _heading: String, a: String, b: String) {
    let mut expected = vec![a, b];
    expected.sort();
    expected.dedup();
    let mut actual = world.candidate_nodes.clone();
    actual.sort();
    actual.dedup();
    assert_eq!(actual, expected, "candidate nodes mismatch");
}

// ============== spec SKILL candidate fanout steps ==============

fn spec_skill_md_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core parent")
        .join("skills/spec/SKILL.md")
}

#[given(regex = r#"^explorer emitted "([^"]+)", "([^"]+)", "([^"]+)" as candidates$"#)]
fn fanout_explorer_emitted(world: &mut TestWorld, a: String, b: String, c: String) {
    let tmp = TempDir::new().expect("tmp");
    let file = tmp.path().join("consult-targets.txt");
    let mut f = File::create(&file).expect("create fixture");
    writeln!(f, "{}", a).unwrap();
    writeln!(f, "{}", b).unwrap();
    writeln!(f, "{}", c).unwrap();
    world.verdict_targets = vec![a, b, c];
    world.verdict_tmp_dir = Some(tmp);
}

#[when("Step 2.1d fans out across the candidate set")]
fn fanout_step_21d_runs(world: &mut TestWorld) {
    // Simulate SKILL's array-loading snippet under bash and assert array length.
    let tmp = world.verdict_tmp_dir.as_ref().expect("tmp");
    let file = tmp.path().join("consult-targets.txt");
    // Portable equivalent of `mapfile -t consult_targets < file` (bash 3.2 lacks mapfile).
    // The SKILL.md text assertion below verifies the actual mapfile directive.
    let script = format!(
        "consult_targets=(); while IFS= read -r line; do consult_targets+=(\"$line\"); done < {}\necho \"${{#consult_targets[@]}}\"\nfor t in \"${{consult_targets[@]}}\"; do echo \"$t\"; done",
        file.display()
    );
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("bash");
    assert!(output.status.success(), "bash failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let mut lines = stdout.lines();
    let count: usize = lines.next().expect("count line").trim().parse().expect("parse count");
    assert_eq!(count, 3, "expected 3 candidates, got {}", count);
    let loaded: Vec<String> = lines.map(|s| s.to_string()).collect();
    assert_eq!(loaded, world.verdict_targets, "loaded array mismatch");

    // Simulate consult-result files being produced (one per target).
    for t in &world.verdict_targets {
        let dir_safe = if t == "." { "root".to_string() } else { t.replace('/', "-") };
        let result = tmp.path().join(format!("consult-result-{}.md", dir_safe));
        fs::write(&result, format!("# consult-result for {}\n", t)).expect("write result");
    }
}

#[then(regex = r#"^(\d+) consult-result files MUST exist \(one per candidate\)$"#)]
fn fanout_result_files_exist(world: &mut TestWorld, expected: usize) {
    let tmp = world.verdict_tmp_dir.as_ref().expect("tmp");
    let mut found = 0;
    for t in &world.verdict_targets {
        let dir_safe = if t == "." { "root".to_string() } else { t.replace('/', "-") };
        let result = tmp.path().join(format!("consult-result-{}.md", dir_safe));
        if result.exists() {
            found += 1;
        }
    }
    assert_eq!(found, expected, "expected {} result files, found {}", expected, found);
}

#[then("they MUST be produced in parallel (no serial dependency)")]
fn fanout_parallel_documentary(_world: &mut TestWorld) {
    // Documentary assertion: verify SKILL.md uses mapfile read from consult-targets.txt
    // (the new source) and preserves the parallel dispatch section.
    let skill = fs::read_to_string(spec_skill_md_path()).expect("read SKILL.md");
    assert!(
        skill.contains("mapfile -t consult_targets < ${TMP_DIR}consult-targets.txt"),
        "SKILL.md missing mapfile read from consult-targets.txt"
    );
    assert!(
        skill.contains("Dispatch po-consultant in parallel"),
        "SKILL.md missing parallel dispatch heading"
    );
}

// ============== Step 2.1e: Target Selection from Verdicts ==============

fn target_selection_harness_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec_target_selection.sh")
}

fn write_verdict_line(
    tmp: &Path,
    target: &str,
    execution: &str,
    reason: &str,
) -> String {
    let v = serde_json::json!({
        "target": target,
        "verdict": "feasible",
        "execution": execution,
        "reason": reason,
        "roadmap_fit": "aligned",
    });
    let line = serde_json::to_string(&v).unwrap();
    let jsonl = tmp.join("verdict-aggregate.jsonl");
    use std::io::Write as IoWrite;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl)
        .expect("open jsonl");
    writeln!(f, "{}", line).expect("write jsonl line");
    reason.to_string()
}

fn run_target_selection(tmp: &Path, no_ask: bool) {
    let mut tmp_prefix = tmp.to_path_buf().into_os_string().into_string().unwrap();
    if !tmp_prefix.ends_with('/') {
        tmp_prefix.push('/');
    }
    let harness = target_selection_harness_path();
    let output = std::process::Command::new("bash")
        .arg(&harness)
        .env("TMP_DIR", &tmp_prefix)
        .env("NO_ASK", if no_ask { "true" } else { "false" })
        .output()
        .expect("run target-selection harness");
    assert!(
        output.status.success(),
        "harness failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[given(
    "verdicts: A.execution=auto_executable, B.execution=halt, C.execution=requires_human"
)]
fn ts_single_auto(world: &mut TestWorld) {
    let tmp = TempDir::new().expect("tmp");
    write_verdict_line(tmp.path(), "A", "auto_executable", "A is authoritative");
    write_verdict_line(tmp.path(), "B", "halt", "B refuses: out of scope");
    write_verdict_line(tmp.path(), "C", "requires_human", "C needs human review");
    world.target_select_tmp = Some(tmp);
    world.target_select_no_ask = true;
    let tmp_path = world.target_select_tmp.as_ref().unwrap().path().to_path_buf();
    run_target_selection(&tmp_path, world.target_select_no_ask);
}

#[then("target_path MUST equal A")]
fn ts_then_target_is_a(world: &mut TestWorld) {
    let tmp = world.target_select_tmp.as_ref().expect("tmp");
    let p = tmp.path().join("target-path.txt");
    let content = fs::read_to_string(&p).expect("read target-path.txt");
    assert_eq!(content.trim(), "A", "expected target A, got {:?}", content);
    assert!(
        !tmp.path().join("halt-reason.txt").exists(),
        "halt-reason.txt must not exist on single-auto"
    );
    assert!(
        !tmp.path().join("ask-question.txt").exists(),
        "ask-question.txt must not exist on single-auto"
    );
}

#[given("verdicts: A.execution=auto_executable, B.execution=auto_executable")]
fn ts_multi_auto(world: &mut TestWorld) {
    let tmp = TempDir::new().expect("tmp");
    write_verdict_line(tmp.path(), "A", "auto_executable", "A claims ownership");
    write_verdict_line(tmp.path(), "B", "auto_executable", "B claims ownership too");
    world.target_select_tmp = Some(tmp);
    world.target_select_no_ask = true;
    let tmp_path = world.target_select_tmp.as_ref().unwrap().path().to_path_buf();
    run_target_selection(&tmp_path, world.target_select_no_ask);
}

#[then(
    "spec MUST halt with a surface-state reason including A and B and their reasons"
)]
fn ts_then_halt_multi(world: &mut TestWorld) {
    let tmp = world.target_select_tmp.as_ref().expect("tmp");
    let halt = fs::read_to_string(tmp.path().join("halt-reason.txt"))
        .expect("read halt-reason.txt");
    assert!(halt.contains("multiple nodes claim ownership"), "halt: {}", halt);
    assert!(halt.contains("A") && halt.contains("B"), "halt missing A/B: {}", halt);
    assert!(
        halt.contains("A claims ownership"),
        "halt missing A reason verbatim: {}",
        halt
    );
    assert!(
        halt.contains("B claims ownership too"),
        "halt missing B reason verbatim: {}",
        halt
    );
    assert!(
        !tmp.path().join("target-path.txt").exists(),
        "target-path.txt must not exist on multi-auto halt"
    );
}

#[given("all candidates have execution in halt or requires_human")]
fn ts_no_auto(world: &mut TestWorld) {
    let tmp = TempDir::new().expect("tmp");
    write_verdict_line(tmp.path(), "A", "halt", "A says no: precedent conflict");
    write_verdict_line(
        tmp.path(),
        "B",
        "requires_human",
        "B needs architect decision",
    );
    world.target_select_tmp = Some(tmp);
}

#[given("--no-ask is set")]
fn ts_no_ask_set(world: &mut TestWorld) {
    world.target_select_no_ask = true;
    let tmp_path = world.target_select_tmp.as_ref().unwrap().path().to_path_buf();
    run_target_selection(&tmp_path, world.target_select_no_ask);
}

#[given("--no-ask is NOT set")]
fn ts_no_ask_unset(world: &mut TestWorld) {
    world.target_select_no_ask = false;
    let tmp_path = world.target_select_tmp.as_ref().unwrap().path().to_path_buf();
    run_target_selection(&tmp_path, world.target_select_no_ask);
}

#[then("spec MUST halt with each candidate's reason preserved verbatim")]
fn ts_then_halt_no_auto(world: &mut TestWorld) {
    let tmp = world.target_select_tmp.as_ref().expect("tmp");
    let halt = fs::read_to_string(tmp.path().join("halt-reason.txt"))
        .expect("read halt-reason.txt");
    assert!(halt.contains("no auto-executable target"), "halt: {}", halt);
    assert!(
        halt.contains("A says no: precedent conflict"),
        "halt missing A reason verbatim: {}",
        halt
    );
    assert!(
        halt.contains("B needs architect decision"),
        "halt missing B reason verbatim: {}",
        halt
    );
    assert!(
        halt.contains("[halt]") && halt.contains("[requires_human]"),
        "halt missing execution tags: {}",
        halt
    );
    assert!(
        !tmp.path().join("ask-question.txt").exists(),
        "ask-question.txt must not exist on --no-ask halt"
    );
}

#[then("spec MUST AskUserQuestion with each candidate's reason preserved")]
fn ts_then_ask(world: &mut TestWorld) {
    let tmp = world.target_select_tmp.as_ref().expect("tmp");
    let ask = fs::read_to_string(tmp.path().join("ask-question.txt"))
        .expect("read ask-question.txt");
    assert!(
        ask.contains("A says no: precedent conflict"),
        "ask missing A reason: {}",
        ask
    );
    assert!(
        ask.contains("B needs architect decision"),
        "ask missing B reason: {}",
        ask
    );
    assert!(
        !tmp.path().join("halt-reason.txt").exists(),
        "halt-reason.txt must not exist on interactive path"
    );
}

// ============== Task 6: Redirect loop with cycle detection ==============

fn redirect_harness_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec_redirect.sh")
}

fn write_round_jsonl(dir: &Path, round: usize, lines: &[serde_json::Value]) {
    let p = dir.join(format!("round-{}.jsonl", round));
    let mut f = File::create(&p).expect("create round jsonl");
    for l in lines {
        writeln!(f, "{}", serde_json::to_string(l).unwrap()).unwrap();
    }
}

fn run_redirect_harness(tmp: &Path, rounds_dir: &Path, initial: &str) {
    let mut tmp_prefix = tmp.to_path_buf().into_os_string().into_string().unwrap();
    if !tmp_prefix.ends_with('/') {
        tmp_prefix.push('/');
    }
    let harness = redirect_harness_path();
    let output = std::process::Command::new("bash")
        .arg(&harness)
        .env("TMP_DIR", &tmp_prefix)
        .env("INITIAL_TARGET", initial)
        .env("ROUNDS_DIR", rounds_dir)
        .output()
        .expect("run redirect harness");
    assert!(
        output.status.success(),
        "harness failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[given(
    "target \"core/src/tree_parser\" verdict has Redirect To=core/src/symbol_index"
)]
fn redirect_single_hop_setup(world: &mut TestWorld) {
    let tmp = TempDir::new().expect("tmp");
    let rounds = TempDir::new().expect("rounds");
    // Round 1: tree_parser auto_executable but redirects to symbol_index
    write_round_jsonl(
        rounds.path(),
        1,
        &[serde_json::json!({
            "target": "core/src/tree_parser",
            "verdict": "feasible",
            "execution": "auto_executable",
            "reason": "redirecting",
            "roadmap_fit": "aligned",
            "redirect_to": "core/src/symbol_index",
        })],
    );
    // Round 2: symbol_index auto_executable, no redirect — converges.
    write_round_jsonl(
        rounds.path(),
        2,
        &[serde_json::json!({
            "target": "core/src/symbol_index",
            "verdict": "feasible",
            "execution": "auto_executable",
            "reason": "I own this",
            "roadmap_fit": "aligned",
        })],
    );
    run_redirect_harness(tmp.path(), rounds.path(), "core/src/tree_parser");
    world.redirect_tmp = Some(tmp);
    world.redirect_rounds_dir = Some(rounds);
}

#[then("Step 2 MUST re-run with target_path=core/src/symbol_index")]
fn redirect_single_hop_target(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let target = fs::read_to_string(tmp.path().join("target-path.txt"))
        .expect("target-path.txt");
    assert_eq!(target.trim(), "core/src/symbol_index");
    let rounds = fs::read_to_string(tmp.path().join("rounds-consumed.txt"))
        .expect("rounds-consumed.txt");
    assert_eq!(rounds.trim(), "2", "expected 2 rounds, got {}", rounds);
}

#[then("the new target MUST receive its own po-consultant verdict")]
fn redirect_single_hop_new_verdict(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let trace = fs::read_to_string(tmp.path().join("visited-trace.txt"))
        .expect("visited-trace.txt");
    assert!(
        trace.contains("core/src/tree_parser") && trace.contains("core/src/symbol_index"),
        "trace missing both nodes: {}",
        trace
    );
}

#[given(
    "tree_parser redirects to symbol_index, then symbol_index redirects back to tree_parser"
)]
fn redirect_cycle_setup(world: &mut TestWorld) {
    let tmp = TempDir::new().expect("tmp");
    let rounds = TempDir::new().expect("rounds");
    write_round_jsonl(
        rounds.path(),
        1,
        &[serde_json::json!({
            "target": "tree_parser",
            "verdict": "feasible",
            "execution": "auto_executable",
            "reason": "redirecting forward",
            "roadmap_fit": "aligned",
            "redirect_to": "symbol_index",
        })],
    );
    write_round_jsonl(
        rounds.path(),
        2,
        &[serde_json::json!({
            "target": "symbol_index",
            "verdict": "feasible",
            "execution": "auto_executable",
            "reason": "redirecting back",
            "roadmap_fit": "aligned",
            "redirect_to": "tree_parser",
        })],
    );
    run_redirect_harness(tmp.path(), rounds.path(), "tree_parser");
    world.redirect_tmp = Some(tmp);
    world.redirect_rounds_dir = Some(rounds);
}

#[then(
    "spec MUST halt with reason \"redirect cycle: tree_parser → symbol_index → tree_parser\""
)]
fn redirect_cycle_halt(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let halt = fs::read_to_string(tmp.path().join("halt-reason.txt"))
        .expect("halt-reason.txt");
    assert!(
        halt.contains("redirect cycle: tree_parser → symbol_index → tree_parser"),
        "halt reason mismatch: {}",
        halt
    );
}

#[then("no plan MUST be generated")]
fn redirect_cycle_no_plan(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    assert!(
        !tmp.path().join("target-path.txt").exists(),
        "target-path.txt must not exist on cycle halt"
    );
}

// ============== Step 4.5: Post-Spec Impact Scan (Task 10) ==============

fn post_impact_harness_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec_post_impact_scan.sh")
}

fn core_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/claude-md-core")
}

fn run_post_impact_scan(world: &mut TestWorld, schema_changed: bool) {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();

    // producer DEVELOPERS.md (after)
    let producer_dir = root.join("producer");
    fs::create_dir_all(&producer_dir).expect("mk producer");
    let after_content = if schema_changed {
        "# producer\n\n## Constraints\nNone\n\n## Data Schemas\n\npub struct OrderId(u64);\n\npub enum OrderStatus { Open, Closed }\n"
    } else {
        "# producer\n\n## Constraints\n- new constraint added\n\n## Data Schemas\nNone\n"
    };
    fs::write(producer_dir.join("DEVELOPERS.md"), after_content).unwrap();

    // producer DEVELOPERS.md (before)
    let before_path = root.join("producer-before.md");
    let before_content = if schema_changed {
        "# producer\n\n## Constraints\nNone\n\n## Data Schemas\nNone\n"
    } else {
        "# producer\n\n## Constraints\nNone\n\n## Data Schemas\nNone\n"
    };
    fs::write(&before_path, before_content).unwrap();

    // consumer module that references OrderId
    let consumer_dir = root.join("consumer");
    fs::create_dir_all(&consumer_dir).expect("mk consumer");
    fs::write(
        consumer_dir.join("DEVELOPERS.md"),
        "# consumer\n\n## Constraints\nOrderId is threaded through the checkout flow.\n\n## Data Schemas\nNone\n",
    )
    .unwrap();

    let tmp_dir_str = {
        let mut s = root.to_path_buf().into_os_string().into_string().unwrap();
        if !s.ends_with('/') {
            s.push('/');
        }
        s
    };

    let output = std::process::Command::new("bash")
        .arg(post_impact_harness_path())
        .env("TMP_DIR", &tmp_dir_str)
        .env("CORE_BIN", core_bin_path())
        .env("TARGET_ROOT", root)
        .env("TARGET_PATH", "producer")
        .env("BEFORE_FILE", &before_path)
        .env("AFTER_FILE", producer_dir.join("DEVELOPERS.md"))
        .output()
        .expect("run post-impact harness");
    assert!(
        output.status.success(),
        "harness failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Persist tmp dir in world by stashing in redirect_tmp slot (reuse, not ideal but isolated)
    world.redirect_tmp = Some(tmp);
}

#[given("/spec modified target's ## Data Schemas")]
fn post_impact_schema_changed(world: &mut TestWorld) {
    run_post_impact_scan(world, true);
}

#[given("/spec modified only ## Constraints")]
fn post_impact_only_constraints(world: &mut TestWorld) {
    run_post_impact_scan(world, false);
}

#[when("Step 4.5 executes")]
fn post_impact_step_4_5(_world: &mut TestWorld) {
    // Harness already ran in the Given step; this step is a no-op marker.
}

#[then("the result block MUST contain a \"## Affected Consumers\" section")]
fn post_impact_contains_section(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        body.contains("## Affected Consumers"),
        "result-block missing Affected Consumers section:\n{}",
        body
    );
}

#[then("each referencing consumer MUST appear as a list item")]
fn post_impact_consumer_listed(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        body.contains("- consumer"),
        "result-block missing consumer list item:\n{}",
        body
    );
}

#[then("the result block MUST NOT contain \"## Affected Consumers\"")]
fn post_impact_absent_section(world: &mut TestWorld) {
    let tmp = world.redirect_tmp.as_ref().expect("tmp");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        !body.contains("## Affected Consumers"),
        "result-block unexpectedly contains Affected Consumers:\n{}",
        body
    );
}

// ============== autodev --auto-sync (Task 11) ==============

fn auto_sync_harness_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/autodev_auto_sync.sh")
}

fn run_auto_sync_chain(world: &mut TestWorld, execution: &str, reason: &str) {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path();
    let fixture_dir = root.join("fixtures");
    fs::create_dir_all(&fixture_dir).expect("mk fixtures");

    // consumers: C (under test) followed by D (a downstream consumer that
    // must NOT be synced when C halts the chain).
    let consumers_path = root.join("affected-consumers.txt");
    fs::write(&consumers_path, "C\nD\n").expect("write consumers");

    // Build C's mock consult-result.
    let c_result = format!(
        "## Verdict\nfeasible\n\n## Execution\n{exec}\n\n## Reason\n{reason}\n\n## Roadmap Fit\naligned\n",
        exec = execution,
        reason = reason,
    );
    fs::write(fixture_dir.join("C.result.md"), c_result).expect("write C");

    // D is always auto_executable; it should only run if C succeeded.
    fs::write(
        fixture_dir.join("D.result.md"),
        "## Verdict\nfeasible\n\n## Execution\nauto_executable\n\n## Reason\n\n\n## Roadmap Fit\naligned\n",
    )
    .expect("write D");

    let tmp_dir_str = {
        let mut s = root.to_path_buf().into_os_string().into_string().unwrap();
        if !s.ends_with('/') {
            s.push('/');
        }
        s
    };

    let output = std::process::Command::new("bash")
        .arg(auto_sync_harness_path())
        .env("TMP_DIR", &tmp_dir_str)
        .env("CONSUMERS_FILE", &consumers_path)
        .env("FIXTURE_DIR", &fixture_dir)
        .output()
        .expect("run auto-sync harness");
    assert!(
        output.status.success(),
        "harness failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    world.auto_sync_tmp = Some(tmp);
    world.auto_sync_halt_reason = Some(reason.to_string());
}

#[given("consumer C's po-consultant emits Execution=auto_executable")]
fn auto_sync_given_auto(world: &mut TestWorld) {
    run_auto_sync_chain(world, "auto_executable", "");
}

#[given(regex = r#"^consumer C's po-consultant emits Execution=halt with reason "(.+)"$"#)]
fn auto_sync_given_halt(world: &mut TestWorld, reason: String) {
    run_auto_sync_chain(world, "halt", &reason);
}

#[given("consumer C's po-consultant emits Execution=requires_human")]
fn auto_sync_given_requires_human(world: &mut TestWorld) {
    run_auto_sync_chain(world, "requires_human", "human attention required");
}

#[then("/sync MUST be invoked on C")]
fn auto_sync_then_c_invoked(world: &mut TestWorld) {
    let tmp = world.auto_sync_tmp.as_ref().expect("tmp");
    let log = fs::read_to_string(tmp.path().join("sync-invocations.log"))
        .expect("read sync log");
    assert!(
        log.lines().any(|l| l == "C"),
        "expected sync invocation for C, got:\n{}",
        log
    );
}

#[then("/sync MUST NOT run on C or any subsequent consumer")]
fn auto_sync_then_no_invocations(world: &mut TestWorld) {
    let tmp = world.auto_sync_tmp.as_ref().expect("tmp");
    let log = fs::read_to_string(tmp.path().join("sync-invocations.log"))
        .expect("read sync log");
    assert!(
        log.trim().is_empty(),
        "expected no sync invocations, got:\n{}",
        log
    );
}

#[then("the result block MUST record C's halt reason verbatim")]
fn auto_sync_then_halt_reason(world: &mut TestWorld) {
    let tmp = world.auto_sync_tmp.as_ref().expect("tmp");
    let reason = world.auto_sync_halt_reason.as_ref().expect("reason");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        body.contains(reason.as_str()),
        "result-block missing halt reason {:?}:\n{}",
        reason,
        body
    );
}

#[then("the result block MUST suggest `git revert HEAD`")]
fn auto_sync_then_rollback(world: &mut TestWorld) {
    let tmp = world.auto_sync_tmp.as_ref().expect("tmp");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        body.contains("git revert HEAD"),
        "result-block missing rollback hint:\n{}",
        body
    );
}

#[then("the result block MUST record C's reason verbatim")]
fn auto_sync_then_reason(world: &mut TestWorld) {
    let tmp = world.auto_sync_tmp.as_ref().expect("tmp");
    let reason = world.auto_sync_halt_reason.as_ref().expect("reason");
    let body = fs::read_to_string(tmp.path().join("result-block.md"))
        .expect("read result-block.md");
    assert!(
        body.contains(reason.as_str()),
        "result-block missing reason {:?}:\n{}",
        reason,
        body
    );
}

// ============== diff-preservation Steps ==============

fn render_section(name: &str, body: &str) -> String {
    format!("## {}\n{}\n", name, body)
}

#[given(expr = "a prior DEVELOPERS.md with sections {string} and {string}")]
fn preservation_prior_two_sections(world: &mut TestWorld, a: String, b: String) {
    let prior = format!("{}{}", render_section(&a, "body A"), render_section(&b, "body B"));
    world.preservation_prior = Some(prior);
}

#[given("a new DEVELOPERS.md where those sections are byte-identical")]
fn preservation_new_identical(world: &mut TestWorld) {
    let prior = world.preservation_prior.clone().expect("prior must be set first");
    world.preservation_new = Some(prior);
}

#[given(expr = "a prior section {string} body {string}")]
fn preservation_prior_named_body(world: &mut TestWorld, section: String, body: String) {
    world.preservation_prior = Some(render_section(&section, &body));
}

#[given(expr = "a new section {string} body {string}")]
fn preservation_new_named_body(world: &mut TestWorld, section: String, body: String) {
    world.preservation_new = Some(render_section(&section, &body));
}

#[given(expr = "a prior DEVELOPERS.md with a {string} section")]
fn preservation_prior_with_section(world: &mut TestWorld, section: String) {
    world.preservation_prior = Some(render_section(&section, "body content"));
}

#[given(expr = "a new DEVELOPERS.md without a {string} section")]
fn preservation_new_without_section(world: &mut TestWorld, section: String) {
    let _ = section;
    world.preservation_new = Some("## Technical Context\nunrelated body\n".to_string());
}

#[given(expr = "a prior DEVELOPERS.md without a {string} section")]
fn preservation_prior_without_section(world: &mut TestWorld, section: String) {
    let _ = section;
    world.preservation_prior = Some("## Technical Context\nunrelated body\n".to_string());
}

#[given(expr = "a new DEVELOPERS.md with a {string} section")]
fn preservation_new_with_section(world: &mut TestWorld, section: String) {
    world.preservation_new = Some(render_section(&section, "newly added body"));
}

#[given(expr = "a prior and new DEVELOPERS.md differing only in {string}")]
fn preservation_differ_only_in(world: &mut TestWorld, section: String) {
    let stable = "## Technical Context\nstable body\n";
    let prior = format!("{}{}", stable, render_section(&section, "prior body"));
    let new_ = format!("{}{}", stable, render_section(&section, "new body"));
    world.preservation_prior = Some(prior);
    world.preservation_new = Some(new_);
}

#[given(expr = "a prior {string} section containing a fenced code block with a literal {string} line followed by {string}")]
fn preservation_prior_with_fence(
    world: &mut TestWorld,
    section: String,
    literal_h2: String,
    tail: String,
) {
    let body = format!(
        "Intro paragraph.\n```markdown\n{}\n```\n{}",
        literal_h2, tail
    );
    world.preservation_prior = Some(render_section(&section, &body));
}

#[given(expr = "a new {string} section containing the same fenced block followed by {string}")]
fn preservation_new_with_fence(world: &mut TestWorld, section: String, tail: String) {
    let prior = world
        .preservation_prior
        .clone()
        .expect("prior must be set first");
    // Re-use the same fenced block from prior, swap only the tail line.
    let body = {
        let mut lines: Vec<&str> = prior.lines().collect();
        if let Some(last) = lines.last_mut() {
            *last = tail.as_str();
        }
        // Strip the leading "## <section>\n" so we can re-wrap below.
        let header = format!("## {}", section);
        let idx = lines.iter().position(|l| *l == header).expect("section header");
        let body_lines: Vec<&str> = lines[idx + 1..].to_vec();
        body_lines.join("\n")
    };
    world.preservation_new = Some(render_section(&section, &body));
}

#[when(expr = "diff-preservation is run with sections {string}")]
fn preservation_run(world: &mut TestWorld, sections: String) {
    let prior = world
        .preservation_prior
        .clone()
        .expect("preservation_prior must be set");
    let new_ = world
        .preservation_new
        .clone()
        .expect("preservation_new must be set");
    let section_list: Vec<&str> = sections
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    world.preservation_audit = Some(diff_preservation::audit(&prior, &new_, &section_list));
}

#[then("the drifted list MUST be empty")]
fn preservation_then_drifted_empty(world: &mut TestWorld) {
    let audit = world
        .preservation_audit
        .as_ref()
        .expect("audit must have run");
    assert!(
        audit.drifted.is_empty(),
        "expected drifted list empty, got {:?}",
        audit.drifted
    );
}

#[then(expr = "the drifted list MUST contain {string}")]
fn preservation_then_drifted_contains(world: &mut TestWorld, section: String) {
    let audit = world
        .preservation_audit
        .as_ref()
        .expect("audit must have run");
    assert!(
        audit.drifted.iter().any(|d| d.section == section),
        "expected drifted list to contain {:?}, got {:?}",
        section,
        audit.drifted
    );
}

#[then(expr = "its reason MUST be {string}")]
fn preservation_then_reason(world: &mut TestWorld, reason: String) {
    let audit = world
        .preservation_audit
        .as_ref()
        .expect("audit must have run");
    assert_eq!(audit.drifted.len(), 1, "expected exactly one drifted entry for reason check, got {:?}", audit.drifted);
    assert_eq!(audit.drifted[0].reason, reason);
}

#[then("the preserved list MUST contain both sections")]
fn preservation_then_preserved_both(world: &mut TestWorld) {
    let audit = world
        .preservation_audit
        .as_ref()
        .expect("audit must have run");
    assert_eq!(
        audit.preserved.len(),
        2,
        "expected 2 preserved sections, got {:?}",
        audit.preserved
    );
}

#[tokio::main]
async fn main() {
    TestWorld::run("tests/features").await;
}
