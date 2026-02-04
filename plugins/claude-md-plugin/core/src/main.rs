use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod tree_parser;
mod boundary_resolver;
mod schema_validator;
mod bracket_utils;
mod claude_md_parser;
mod code_analyzer;
mod dependency_graph;

use tree_parser::TreeParser;
use boundary_resolver::BoundaryResolver;
use schema_validator::SchemaValidator;
use claude_md_parser::ClaudeMdParser;
use dependency_graph::DependencyGraphBuilder;

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
        Commands::ValidateSchema { file, output } => {
            let validator = SchemaValidator::new();
            let validation_result = validator.validate(file);
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
    };

    if let Err(e) = result {
        let command_name = match cli.command {
            Commands::ParseTree { .. } => "parse-tree",
            Commands::ResolveBoundary { .. } => "resolve-boundary",
            Commands::ValidateSchema { .. } => "validate-schema",
            Commands::ParseClaudeMd { .. } => "parse-claude-md",
            Commands::DependencyGraph { .. } => "dependency-graph",
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
