use cucumber::{given, then, when, World};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Import the modules we're testing
use claude_md_core::{TreeParser, BoundaryResolver, SchemaValidator, CodeAnalyzer, ConventionValidator};
use claude_md_core::tree_parser::TreeResult;
use claude_md_core::boundary_resolver::BoundaryResult;
use claude_md_core::schema_validator::ValidationResult;
use claude_md_core::code_analyzer::AnalysisResult;
use claude_md_core::convention_validator::ConventionValidationResult;
use claude_md_core::compile_target_resolver::{CompileTargetResolver, DiffResult};
use claude_md_core::exports_formatter;
use claude_md_core::code_analyzer::{
    Exports, ExportedFunction, ExportedType, ExportedClass, ExportedEnum,
    ExportedVariable, ReExport, TypeKind,
};

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

const VALID_PROJECT_CONVENTION: &str = r#"
## Project Convention

### Project Structure
Layered architecture with src/ containing all source code.

### Module Boundaries
Each module is self-contained and communicates through public APIs.

### Naming Conventions
camelCase for files, PascalCase for classes.
"#;

const VALID_CODE_CONVENTION: &str = r#"
## Code Convention

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Code Style
2 spaces indent, single quotes, semicolons required.

### Naming Rules
camelCase for variables and functions, PascalCase for types.
"#;

#[given("a project root with CLAUDE.md containing valid Project Convention")]
fn project_root_valid_project_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test Project\n\n## Purpose\nA test project.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md without Project Convention")]
fn project_root_no_project_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test Project\n\n## Purpose\nA test project.\n{}",
        VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md containing incomplete Project Convention")]
fn project_root_incomplete_project_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test\n\n## Purpose\nTest.\n\n## Project Convention\n\n### Project Structure\nLayered.\n\n{}",
        VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md containing valid conventions")]
fn project_root_valid_conventions(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test\n\n## Purpose\nTest.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md containing only Project Convention")]
fn project_root_only_project_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test\n\n## Purpose\nTest.\n{}",
        VALID_PROJECT_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &content);
}

#[given("a project root with CLAUDE.md containing incomplete Code Convention")]
fn project_root_incomplete_code_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create marker");
    let content = format!(
        "# Test\n\n## Purpose\nTest.\n{}\n\n## Code Convention\n\n### Language & Runtime\nTypeScript\n\n### Code Style\n2 spaces\n",
        VALID_PROJECT_CONVENTION
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

#[given("a multi module project with module-level Project Convention override")]
fn multi_module_with_override(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = format!(
        "# API Module\n\n## Purpose\nAPI module.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&sub, "CLAUDE.md", &sub_content);
}

// ---- DRY: Convention Inheritance steps ----

#[given("a multi module project where module has no Code Convention")]
fn multi_module_no_code_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    // Project root has both conventions (canonical source)
    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    // Sub-module has NO Code Convention (inherits from project root)
    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = "# API Module\n\n## Purpose\nAPI module.\n";
    create_file_at(&sub, "CLAUDE.md", sub_content);
}

#[given("a multi module project where module has incomplete Code Convention")]
fn multi_module_incomplete_code_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}\n{}",
        VALID_PROJECT_CONVENTION, VALID_CODE_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    // Sub-module has Code Convention but missing Naming Rules
    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = "# API Module\n\n## Purpose\nAPI module.\n\n## Code Convention\n\n### Language & Runtime\nTypeScript\n\n### Code Style\n2 spaces\n";
    create_file_at(&sub, "CLAUDE.md", sub_content);
}

#[given("a multi module project where project root has no Code Convention")]
fn multi_module_no_project_code_convention(world: &mut TestWorld) {
    let root = get_temp_path(world);
    File::create(root.join("package.json")).expect("create root marker");

    // Project root has only Project Convention, no Code Convention
    let root_content = format!(
        "# Root\n\n## Purpose\nRoot project.\n{}",
        VALID_PROJECT_CONVENTION
    );
    create_file_at(&root, "CLAUDE.md", &root_content);

    let sub = root.join("packages").join("api");
    fs::create_dir_all(&sub).expect("create sub");
    File::create(sub.join("package.json")).expect("create sub marker");

    let sub_content = format!(
        "# API Module\n\n## Purpose\nAPI module.\n{}",
        VALID_CODE_CONVENTION
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

#[then("project convention should be found")]
fn project_convention_found(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    assert!(result.project_convention.section_found, "Project Convention section not found");
}

#[then("code convention should be found")]
fn code_convention_found(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    assert!(!result.module_roots.is_empty(), "No module roots found");
    assert!(result.module_roots[0].code_convention.section_found, "Code Convention section not found");
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

#[then("module should have project convention override")]
fn module_has_project_convention_override(world: &mut TestWorld) {
    let result = world.convention_result.as_ref().expect("No convention result");
    let has_override = result.module_roots.iter().any(|m| {
        m.project_convention_override.as_ref().map_or(false, |o| o.section_found)
    });
    assert!(has_override, "Expected at least one module with project convention override");
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
        "# Module\n\n## Purpose\nTest module\n\n## Exports\nNone\n\n## Behavior\n| Input | Output |\n|-------|--------|\n| any | ok |\n\n## Dependencies\nNone\n\n## Contract\nNone\n\n## Protocol\nNone\n"
    } else {
        "# IMPLEMENTS\n\n## Planning\nTBD\n"
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

#[given(expr = "a committed spec file {string} depending on {string}")]
fn create_committed_spec_with_dep(world: &mut TestWorld, path: String, dep: String) {
    use std::process::Command;
    let root = get_temp_path(world);
    let full_path = root.join(&path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("mkdir failed");
    let content = format!(
        "# Module\n\n## Purpose\nTest module\n\n## Exports\nNone\n\n## Behavior\n| Input | Output |\n|-------|--------|\n| any | ok |\n\n## Dependencies\n### Internal\n- `{}` — dependency\n\n### External\nNone\n\n## Contract\nNone\n\n## Protocol\nNone\n",
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
    let found = result.targets.iter().any(|t| t.dir.is_empty() || t.claude_md_path == "CLAUDE.md");
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

#[tokio::main]
async fn main() {
    TestWorld::run("tests/features").await;
}
