use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod tree_parser;
mod boundary_resolver;
mod schema_validator;
mod claude_md_parser;
mod code_analyzer;
mod dependency_graph;
mod auditor;
mod symbol_index;
mod diagram_generator;
mod migrator;

use tree_parser::TreeParser;
use boundary_resolver::BoundaryResolver;
use schema_validator::SchemaValidator;
use claude_md_parser::ClaudeMdParser;
use dependency_graph::DependencyGraphBuilder;
use auditor::Auditor;
use symbol_index::SymbolIndexBuilder;
use diagram_generator::DiagramGenerator;
use migrator::Migrator;

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

        /// Root directory for symbol index (enables cross-reference validation)
        #[arg(long)]
        with_index: Option<PathBuf>,
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

    /// Build dependency graph for the project
    DependencyGraph {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Audit directory tree for CLAUDE.md completeness
    Audit {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Only show directories where status is 'missing' or 'unexpected'
        #[arg(long, default_value = "false")]
        only_issues: bool,
    },

    /// Build symbol index for cross-reference resolution
    SymbolIndex {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Find a symbol by name (go-to-definition)
        #[arg(long)]
        find: Option<String>,

        /// Find references to a symbol anchor
        #[arg(long)]
        references: Option<String>,

        /// Skip cache and force full rebuild
        #[arg(long, default_value = "false")]
        no_cache: bool,
    },

    /// Generate UseCase diagram (Mermaid) from CLAUDE.md Behavior section
    GenerateUsecase {
        /// CLAUDE.md file to parse
        #[arg(short, long)]
        file: PathBuf,

        /// Output file path for Mermaid diagram
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate State diagram (Mermaid) from CLAUDE.md Protocol section
    GenerateState {
        /// CLAUDE.md file to parse
        #[arg(short, long)]
        file: PathBuf,

        /// Output file path for Mermaid diagram
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate Component diagram (Mermaid) from dependency graph
    GenerateComponent {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output file path for Mermaid diagram
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Migrate CLAUDE.md files from v1 to v2 format
    Migrate {
        /// Root directory to scan
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Preview changes without writing
        #[arg(long, default_value = "false")]
        dry_run: bool,
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
        Commands::ValidateSchema { file, output, with_index } => {
            let validator = SchemaValidator::new();
            let validation_result = if let Some(root) = with_index {
                let index_builder = SymbolIndexBuilder::new();
                match index_builder.build_with_cache(root, false) {
                    Ok(index) => validator.validate_with_index(file, &index),
                    Err(_) => validator.validate(file),
                }
            } else {
                validator.validate(file)
            };
            output_result(&validation_result, output.as_ref(), "validate-schema")
        }
        Commands::ParseClaudeMd { file, output } => {
            let parser = ClaudeMdParser::new();
            match parser.parse(file) {
                Ok(spec) => output_result(&spec, output.as_ref(), "parse-claude-md"),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::DependencyGraph { root, output } => {
            let builder = DependencyGraphBuilder::new();
            match builder.build(root) {
                Ok(graph) => output_result(&graph, output.as_ref(), "dependency-graph"),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::Audit { root, output, only_issues } => {
            let auditor = Auditor::new();
            let result = auditor.audit(root, *only_issues);
            output_result(&result, output.as_ref(), "audit")
        }
        Commands::SymbolIndex { root, output, find, references, no_cache } => {
            let builder = SymbolIndexBuilder::new();
            match builder.build_with_cache(root, *no_cache) {
                Ok(index) => {
                    // If --find is specified, filter to matching symbols
                    if let Some(name) = find {
                        let found = SymbolIndexBuilder::find_symbol(&index, name);
                        output_result(&found, output.as_ref(), "symbol-index")
                    }
                    // If --references is specified, find references to anchor
                    else if let Some(anchor) = references {
                        let refs = SymbolIndexBuilder::find_references(&index, anchor);
                        output_result(&refs, output.as_ref(), "symbol-index")
                    }
                    // Otherwise, output the full index
                    else {
                        output_result(&index, output.as_ref(), "symbol-index")
                    }
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::GenerateUsecase { file, output } => {
            let parser = ClaudeMdParser::new();
            match parser.parse(file) {
                Ok(spec) => {
                    let diagram = DiagramGenerator::generate_usecase(&spec);
                    output_text(&diagram, output.as_ref(), "generate-usecase")
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::GenerateState { file, output } => {
            let parser = ClaudeMdParser::new();
            match parser.parse(file) {
                Ok(spec) => {
                    match DiagramGenerator::generate_state(&spec) {
                        Some(diagram) => output_text(&diagram, output.as_ref(), "generate-state"),
                        None => {
                            eprintln!("No Protocol section with states/transitions found");
                            Ok(())
                        }
                    }
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::GenerateComponent { root, output } => {
            let builder = DependencyGraphBuilder::new();
            match builder.build(root) {
                Ok(graph) => {
                    let diagram = DiagramGenerator::generate_component(&graph);
                    output_text(&diagram, output.as_ref(), "generate-component")
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Commands::Migrate { root, output, dry_run } => {
            let migrator = Migrator::new();
            let results = migrator.migrate_all(root, *dry_run);
            output_result(&results, output.as_ref(), "migrate")
        }
    };

    if let Err(e) = result {
        let command_name = match cli.command {
            Commands::ParseTree { .. } => "parse-tree",
            Commands::ResolveBoundary { .. } => "resolve-boundary",
            Commands::ValidateSchema { .. } => "validate-schema",
            Commands::ParseClaudeMd { .. } => "parse-claude-md",
            Commands::DependencyGraph { .. } => "dependency-graph",
            Commands::Audit { .. } => "audit",
            Commands::SymbolIndex { .. } => "symbol-index",
            Commands::GenerateUsecase { .. } => "generate-usecase",
            Commands::GenerateState { .. } => "generate-state",
            Commands::GenerateComponent { .. } => "generate-component",
            Commands::Migrate { .. } => "migrate",
        };
        eprintln!("Error in '{}' command: {}", command_name, e);
        eprintln!("Hint: Use --help for usage information");
        std::process::exit(1);
    }
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

/// Output plain text (for Mermaid diagrams)
fn output_text(
    text: &str,
    output_path: Option<&PathBuf>,
    command_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_path {
        Some(path) => {
            std::fs::write(path, text)
                .map_err(|e| format!(
                    "Failed to write {} output to '{}': {} (check directory exists and permissions)",
                    command_name,
                    path.display(),
                    e
                ))?;
            println!("Output written to: {}", path.display());
        }
        None => {
            println!("{}", text);
        }
    }

    Ok(())
}
