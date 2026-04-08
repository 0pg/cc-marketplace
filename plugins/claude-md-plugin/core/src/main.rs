use clap::{Parser, Subcommand};
use std::path::PathBuf;

use claude_md_core::{
    TreeParser, BoundaryResolver, SchemaValidator,
    ClaudeMdParser, ConventionValidator, CodeAnalyzer,
};
use claude_md_core::tree_parser;
use claude_md_core::code_analyzer;
use claude_md_core::dependency_resolver::DependencyResolver;
use claude_md_core::claude_md_scanner::ClaudeMdScanner;
use claude_md_core::compile_target_resolver::CompileTargetResolver;
use claude_md_core::exports_formatter;
use claude_md_core::analysis_formatter;
use claude_md_core::spec_diff::SpecDiffer;
use claude_md_core::node_history::NodeHistoryDiffer;

#[derive(Parser)]
#[command(name = "claude-md-core")]
#[command(about = "Core engine for claude-md-plugin")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse directory tree and identify where CLAUDE.md is needed
    ParseTree {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Resolve boundary for a specific directory
    ResolveBoundary {
        /// Directory path to analyze
        #[arg(short, long)]
        path: PathBuf,

        /// CLAUDE.md content file to validate references (optional)
        #[arg(short, long)]
        claude_md: Option<PathBuf>,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate CLAUDE.md schema
    ValidateSchema {
        /// CLAUDE.md file to validate
        #[arg(short, long)]
        file: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Strict mode: validate DEVELOPERS.md existence (INV-3) and schema
        #[arg(long, default_value_t = false)]
        strict: bool,

        /// Directory path to analyze for conditional section evaluation.
        /// When provided, source files are scanned to determine which
        /// conditional sections (Async Contract, Concurrency Model, Protocol)
        /// are required.
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Minimum completeness score (0-100). Validation fails if the
        /// CLAUDE.md completeness score is below this threshold.
        /// Default: 0 (backward compatible, no minimum enforced).
        #[arg(long, default_value_t = 0)]
        min_completeness: u32,
    },

    /// Parse CLAUDE.md into structured JSON spec
    ParseClaudeMd {
        /// CLAUDE.md file to parse
        #[arg(short, long)]
        file: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate convention sections in CLAUDE.md files
    ValidateConvention {
        /// Project root directory
        #[arg(short, long)]
        project_root: PathBuf,

        /// Module root directories (comma-separated). Auto-detected if omitted.
        #[arg(short, long, value_delimiter = ',')]
        module_roots: Option<Vec<PathBuf>>,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Analyze source code to extract exports, dependencies, and behaviors
    AnalyzeCode {
        /// Directory or file path to analyze
        #[arg(short, long)]
        path: PathBuf,

        /// Optional file filter (comma-separated filenames)
        #[arg(short, long, value_delimiter = ',')]
        files: Option<Vec<String>>,

        /// Tree-parse result JSON file for dependency resolution.
        /// When provided, internal deps are resolved to CLAUDE.md paths.
        #[arg(short, long)]
        tree_result: Option<PathBuf>,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Scan existing CLAUDE.md files and build lightweight index
    ScanClaudeMd {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Determine which CLAUDE.md files need recompilation (incremental diff)
    DiffCompileTargets {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Format analyze-code exports into deterministic CLAUDE.md Exports markdown
    FormatExports {
        /// analyze-code output JSON file
        #[arg(short, long)]
        input: PathBuf,

        /// Output markdown file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Format full analyze-code result into compact markdown summary
    FormatAnalysis {
        /// analyze-code output JSON file
        #[arg(short, long)]
        input: PathBuf,

        /// Output markdown file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Calculate SHA-256 hash of entire CLAUDE.md file (change detection)
    ContractHash {
        /// CLAUDE.md file to hash
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Detect Requirements changes in CLAUDE.md since its last git commit,
    /// and report which source files changed since then
    DiffSpecRange {
        /// CLAUDE.md file to analyze
        #[arg(short, long)]
        file: PathBuf,

        /// Project root directory (git repo root)
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Converge CLAUDE.md/DEVELOPERS.md to current schema: rename, remove, add missing sections
    FixSchema {
        /// File to fix
        #[arg(short, long)]
        file: PathBuf,

        /// Output file path (defaults to overwriting the input file)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Document type: claude_md (default) or developers_md
        #[arg(short = 't', long = "type", default_value = "claude_md")]
        doc_type: String,

        /// Dry-run: show changes without modifying files (JSON output)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Get section-level diffs from recent N commits touching a node's CLAUDE.md/DEVELOPERS.md
    DiffNodeHistory {
        /// Node directory path (containing CLAUDE.md and/or DEVELOPERS.md)
        #[arg(short = 'p', long)]
        path: PathBuf,

        /// Project root directory (git repo root)
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Maximum number of commits to include
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Filter by commit message pattern (e.g., "^spec\\(src/auth\\):")
        #[arg(short = 'g', long)]
        grep: Option<String>,

        /// Only include commits after this commit hash (exclusive)
        #[arg(long)]
        since_commit: Option<String>,

        /// Output JSON file path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate document language consistency
    ValidateLanguage {
        /// File to validate (CLAUDE.md or DEVELOPERS.md)
        #[arg(short, long)]
        file: PathBuf,

        /// Expected language (English, Korean, Japanese, Chinese)
        #[arg(short, long)]
        expected: String,

        /// Minimum target percentage (default: 70)
        #[arg(short, long, default_value_t = 70.0)]
        threshold: f64,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::ParseTree { root, output } => {
            let parser = TreeParser::new();
            let tree_result = parser.parse(root);
            output_result(&tree_result, output.as_ref(), "parse-tree")
        }
        Commands::ResolveBoundary { path, claude_md, output } => {
            let resolver = BoundaryResolver::new();
            let boundary_result = resolver.resolve(path, claude_md.as_ref());
            output_result(&boundary_result, output.as_ref(), "resolve-boundary")
        }
        Commands::ValidateSchema { file, output, strict, dir, min_completeness } => {
            let validator = SchemaValidator::new();

            // Build validation context from directory scan if --dir is provided
            let ctx = dir.as_ref().map(|d| {
                claude_md_core::schema_validator::SchemaValidator::evaluate_conditions(d)
            });

            let mut validation_result = match (*strict, &ctx) {
                (true, Some(c)) => validator.validate_strict_with_context(file, Some(c)),
                (true, None) => validator.validate_strict(file),
                (false, Some(c)) => validator.validate_with_context(file, c),
                (false, None) => validator.validate(file),
            };

            if *strict {
                // Promote INV-3 warnings to errors
                let (inv3_warnings, remaining): (Vec<_>, Vec<_>) = validation_result.warnings
                    .into_iter()
                    .partition(|w| w.starts_with("INV-3:"));

                for warning in inv3_warnings {
                    validation_result.errors.push(claude_md_core::schema_validator::ValidationError {
                        error_type: "INV3Violation".to_string(),
                        message: warning,
                        line_number: None,
                        section: None,
                    });
                }
                validation_result.warnings = remaining;
                validation_result.valid = validation_result.errors.is_empty();
            }

            // Check minimum completeness threshold
            if *min_completeness > 0 {
                if let Some(score) = validation_result.completeness_score {
                    if score < *min_completeness {
                        validation_result.errors.push(claude_md_core::schema_validator::ValidationError {
                            error_type: "InsufficientCompleteness".to_string(),
                            message: format!(
                                "Completeness score {} is below minimum threshold {}",
                                score, min_completeness
                            ),
                            line_number: None,
                            section: None,
                        });
                        validation_result.valid = false;
                    }
                }
            }

            output_result(&validation_result, output.as_ref(), "validate-schema")
        }
        Commands::ParseClaudeMd { file, output } => {
            let parser = ClaudeMdParser::new();
            match parser.parse(file) {
                Ok(spec) => output_result(&spec, output.as_ref(), "parse-claude-md"),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::ValidateConvention { project_root, module_roots, output } => {
            let validator = ConventionValidator::new();
            let result = validator.validate(project_root, module_roots.clone());
            output_result(&result, output.as_ref(), "validate-convention")
        }
        Commands::AnalyzeCode { path, files, tree_result, output } => {
            let analyzer = CodeAnalyzer::new();
            let file_refs: Option<Vec<&str>> = files.as_ref()
                .map(|f| f.iter().map(|s| s.as_str()).collect());
            match analyzer.analyze_directory(path, file_refs.as_deref()) {
                Ok(mut result) => {
                    // Resolve internal deps if tree-parse result provided
                    if let Some(tree_path) = tree_result {
                        match std::fs::read_to_string(tree_path) {
                            Ok(json) => {
                                match serde_json::from_str::<tree_parser::TreeResult>(&json) {
                                    Ok(tree) => {
                                        let resolver = DependencyResolver::new(&tree);
                                        // Derive source_dir: path relative to tree root
                                        let source_dir = path.strip_prefix(&tree.root)
                                            .unwrap_or(path);
                                        resolver.resolve(&mut result, source_dir);
                                    }
                                    Err(e) => eprintln!("Warning: failed to parse tree result: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Warning: failed to read tree result: {}", e),
                        }
                    }
                    output_result(&result, output.as_ref(), "analyze-code")
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::ScanClaudeMd { root, output } => {
            let scanner = ClaudeMdScanner::new();
            let scan_result = scanner.scan(root);
            output_result(&scan_result, output.as_ref(), "scan-claude-md")
        }
        Commands::DiffCompileTargets { root, output } => {
            let resolver = CompileTargetResolver::new();
            let result = resolver.resolve(root);
            output_result(&result, output.as_ref(), "diff-compile-targets")
        }
        Commands::DiffSpecRange { file, root, output } => {
            let differ = SpecDiffer::new(root);
            let result = differ.diff(file);
            output_result(&result, output.as_ref(), "diff-spec-range")
        }
        Commands::DiffNodeHistory { path, root, limit, grep, since_commit, output } => {
            let differ = NodeHistoryDiffer::new(root, path);
            let result = differ.diff(*limit, grep.as_deref(), since_commit.as_deref());
            output_result(&result, output.as_ref(), "diff-node-history")
        }
        Commands::FormatExports { input, output } => {
            match std::fs::read_to_string(input) {
                Ok(json) => match serde_json::from_str::<code_analyzer::AnalysisResult>(&json) {
                    Ok(analysis) => {
                        let markdown = exports_formatter::format_exports(&analysis.exports);
                        output_text(&markdown, output.as_ref(), "format-exports")
                    }
                    Err(e) => Err(format!(
                        "Failed to parse analyze-code JSON from '{}': {}",
                        input.display(), e
                    ).into()),
                },
                Err(e) => Err(format!(
                    "Failed to read input file '{}': {}",
                    input.display(), e
                ).into()),
            }
        }
        Commands::FormatAnalysis { input, output } => {
            match std::fs::read_to_string(input) {
                Ok(json) => match serde_json::from_str::<code_analyzer::AnalysisResult>(&json) {
                    Ok(analysis) => {
                        let markdown = analysis_formatter::format_analysis(&analysis);
                        output_text(&markdown, output.as_ref(), "format-analysis")
                    }
                    Err(e) => Err(format!(
                        "Failed to parse analyze-code JSON from '{}': {}",
                        input.display(), e
                    ).into()),
                },
                Err(e) => Err(format!(
                    "Failed to read input file '{}': {}",
                    input.display(), e
                ).into()),
            }
        }
        Commands::ContractHash { file } => {
            match claude_md_core::contract_hasher::contract_hash(file) {
                Ok(hash) => {
                    println!("{}", hash);
                    Ok(())
                }
                Err(e) => Err(format!(
                    "Failed to calculate contract hash for '{}': {}",
                    file.display(), e
                ).into()),
            }
        }
        Commands::FixSchema { file, output, doc_type, dry_run } => {
            match std::fs::read_to_string(&file) {
                Ok(content) => {
                    let validator = SchemaValidator::new();
                    let ctx = file.parent().map(|d| {
                        claude_md_core::schema_validator::SchemaValidator::evaluate_conditions(d)
                    });
                    let converge_result = validator.converge_schema_with_context(&content, doc_type, ctx.as_ref());

                    if converge_result.changes.is_empty() && converge_result.warnings.is_empty() {
                        if *dry_run {
                            println!("{{\"changes\":[],\"warnings\":[]}}");
                        } else {
                            println!("No changes needed.");
                        }
                        Ok(())
                    } else if *dry_run {
                        let json = serde_json::json!({
                            "changes": converge_result.changes,
                            "warnings": converge_result.warnings,
                        });
                        println!("{}", serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| json.to_string()));
                        Ok(())
                    } else {
                        let target = output.as_ref().unwrap_or(&file);
                        match std::fs::write(target, &converge_result.content) {
                            Ok(()) => {
                                for change in &converge_result.changes {
                                    println!("  {}", change);
                                }
                                for warning in &converge_result.warnings {
                                    println!("  ⚠ {}", warning);
                                }
                                println!("Applied {} change(s) to: {}", converge_result.changes.len(), target.display());
                                Ok(())
                            }
                            Err(e) => Err(format!(
                                "Failed to write fixed file to '{}': {}",
                                target.display(), e
                            ).into()),
                        }
                    }
                }
                Err(e) => Err(format!(
                    "Failed to read CLAUDE.md '{}': {}",
                    file.display(), e
                ).into()),
            }
        }
        Commands::ValidateLanguage { file, expected, threshold, output } => {
            let validator = claude_md_core::LanguageValidator::new();
            match validator.validate(file, expected, *threshold) {
                Ok(result) => output_result(&result, output.as_ref(), "validate-language"),
                Err(e) => Err(e.to_string().into()),
            }
        }
    };

    if let Err(e) = result {
        let command_name = match cli.command {
            Commands::ParseTree { .. } => "parse-tree",
            Commands::ResolveBoundary { .. } => "resolve-boundary",
            Commands::ValidateSchema { .. } => "validate-schema",

            Commands::ParseClaudeMd { .. } => "parse-claude-md",
            Commands::ValidateConvention { .. } => "validate-convention",
            Commands::AnalyzeCode { .. } => "analyze-code",

            Commands::ScanClaudeMd { .. } => "scan-claude-md",
            Commands::DiffCompileTargets { .. } => "diff-compile-targets",
            Commands::DiffSpecRange { .. } => "diff-spec-range",
            Commands::DiffNodeHistory { .. } => "diff-node-history",
            Commands::ContractHash { .. } => "contract-hash",
            Commands::FixSchema { .. } => "fix-schema",
            Commands::FormatExports { .. } => "format-exports",
            Commands::FormatAnalysis { .. } => "format-analysis",
            Commands::ValidateLanguage { .. } => "validate-language",
        };
        eprintln!("Error in '{}' command: {}", command_name, e);
        eprintln!("Hint: Use --help for usage information");
        std::process::exit(1);
    }
}

fn output_text(
    text: &str,
    output_path: Option<&PathBuf>,
    command_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_path {
        Some(path) => {
            std::fs::write(path, format!("{}\n", text))
                .map_err(|e| format!(
                    "Failed to write {} output to '{}': {} (check directory exists and permissions)",
                    command_name, path.display(), e
                ))?;
            println!("Output written to: {}", path.display());
        }
        None => {
            println!("{}", text);
        }
    }
    Ok(())
}

fn output_result<T: serde::Serialize>(
    result: &T,
    output_path: Option<&PathBuf>,
    command_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| format!("Failed to serialize {} result to JSON: {}", command_name, e))?;

    match output_path {
        Some(path) => {
            std::fs::write(path, &json)
                .map_err(|e| format!(
                    "Failed to write output to '{}': {} (check directory exists and permissions)",
                    path.display(),
                    e
                ))?;
            println!("Output written to: {}", path.display());
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}
