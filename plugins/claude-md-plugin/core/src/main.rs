use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
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

        /// Enforce INV-3 (IMPLEMENTS.md existence) as error instead of warning
        #[arg(long, default_value_t = false)]
        strict: bool,
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

    /// Index entire project: tree-parse + code analysis for all directories
    IndexProject {
        /// Root directory to scan and analyze
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

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
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexResult {
    root: PathBuf,
    directories: Vec<DirectoryAnalysis>,
    excluded: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryAnalysis {
    path: PathBuf,
    depth: usize,
    analysis: code_analyzer::AnalysisResult,
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
        Commands::ValidateSchema { file, output, strict } => {
            let validator = SchemaValidator::new();
            let mut validation_result = validator.validate(file);

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
        Commands::IndexProject { root, output } => {
            let tree_parser = TreeParser::new();
            let tree_result = tree_parser.parse(root);
            let analyzer = CodeAnalyzer::new();
            // Borrows tree_result temporarily; copies needed data internally via clone.
            let resolver = DependencyResolver::new(&tree_result);

            let mut directories = Vec::new();
            for dir_info in &tree_result.needs_claude_md {
                let dir_path = root.join(&dir_info.path);
                match analyzer.analyze_directory(&dir_path, None) {
                    Ok(mut analysis) => {
                        resolver.resolve(&mut analysis, &dir_info.path);
                        directories.push(DirectoryAnalysis {
                            path: dir_info.path.clone(),
                            depth: dir_info.depth,
                            analysis,
                        });
                    }
                    Err(e) => eprintln!("Warning: skipping {}: {}", dir_info.path.display(), e),
                }
            }

            let index_result = IndexResult {
                root: tree_result.root,
                directories,
                excluded: tree_result.excluded,
            };
            output_result(&index_result, output.as_ref(), "index-project")
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
            Commands::IndexProject { .. } => "index-project",
            Commands::FormatExports { .. } => "format-exports",
            Commands::FormatAnalysis { .. } => "format-analysis",
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
