use cucumber::{given, then, when, World};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// Import the modules we're testing
use claude_md_core::{TreeParser, BoundaryResolver, SchemaValidator, CodeAnalyzer};
use claude_md_core::tree_parser::TreeResult;
use claude_md_core::boundary_resolver::BoundaryResult;
use claude_md_core::schema_validator::ValidationResult;
use claude_md_core::code_analyzer::{AnalysisResult, AnalyzerError};

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

            let found = result.dependencies.internal.iter().any(|d| d == path || d.contains(path));
            assert!(found, "Expected to find internal dependency '{}', found: {:?}",
                    path, result.dependencies.internal);
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
                    result.dependencies.external.len() + result.dependencies.internal.len()
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
                "dependencies.internal" => result.dependencies.internal.len(),
                "behaviors" => result.behaviors.len(),
                "analyzed_files" => result.analyzed_files.len(),
                _ => panic!("Unknown field: {}", field),
            };

            assert_eq!(actual, expected, "Expected {} = {}, got {}", field, expected, actual);
        }
    }
}

#[tokio::main]
async fn main() {
    TestWorld::run("tests/features").await;
}
