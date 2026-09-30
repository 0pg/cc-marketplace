//! Optional export of synthetic inputs and actual responses for blind answer review.
use memento::{
    model::{Corpus, Query},
    query,
};
use std::{error::Error, path::PathBuf};

pub fn capture(
    scenario: &str,
    variant: &str,
    question: &str,
    corpus: &Corpus,
    queries: &[Query],
) -> Result<(), Box<dyn Error>> {
    let Some(directory) = std::env::var_os("MEMENTO_EVALUATION_DIR") else {
        return Ok(());
    };
    if !scenario
        .chars()
        .chain(variant.chars())
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("invalid evaluation case name".into());
    }
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory)?;
    let results = queries
        .iter()
        .map(|q| match query::execute(corpus, q) {
            Ok(result) => serde_json::to_value(result),
            Err(error) => Ok(serde_json::json!({"error":error})),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let value = serde_json::json!({"scenario":scenario,"variant":variant,"question":question,"corpus":corpus,"queries":queries,"results":results});
    std::fs::write(
        directory.join(format!("{scenario}-{variant}.json")),
        serde_json::to_vec(&value)?,
    )?;
    Ok(())
}
