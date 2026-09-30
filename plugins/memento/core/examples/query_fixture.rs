//! Evaluate a query against an exported synthetic corpus without collecting user history.
use memento::{
    model::{Corpus, Query},
    query,
};
use serde::Deserialize;
use std::{error::Error, io::Read, process::ExitCode};

#[derive(Deserialize)]
struct Fixture {
    corpus: Corpus,
}
fn run() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: query_fixture exported-case.json < query.json")?;
    let fixture: Fixture = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let request: Query = serde_json::from_str(&input)?;
    let response = query::execute(&fixture.corpus, &request)?;
    serde_json::to_writer(std::io::stdout().lock(), &response)?;
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
