use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "flow-core")]
#[command(about = "Deterministic CLI for the flow plugin", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a dag.json file against the flow DAG schema and invariants.
    ValidateDag {
        /// Path to dag.json
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::ValidateDag { path } => match flow_core::validate_dag_file(&path) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                if report.valid {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "valid": false,
                    "errors": [{
                        "code": "IO_OR_PARSE_ERROR",
                        "node_id": null,
                        "message": e.to_string()
                    }]
                })).unwrap());
                ExitCode::from(2)
            }
        },
    }
}
