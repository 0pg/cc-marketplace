use std::{error::Error, fs, path::Path, process::Command};

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn cli(store: &Path, args: &[&str]) -> Result<(bool, Value), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_work-context"))
        .arg(args.first().ok_or("command absent")?)
        .arg("--store")
        .arg(store)
        .args(args.get(1..).ok_or("arguments absent")?)
        .output()?;
    Ok((
        output.status.success(),
        serde_json::from_slice(&output.stdout)?,
    ))
}

fn path(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.to_str().ok_or_else(|| "non-UTF8 test path".into())
}

#[test]
fn sync_preserves_partial_completeness_across_cli_processes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = dir.path().join("context.sqlite");
    let input = dir.path().join("input.jsonl");
    let query = dir.path().join("query.json");
    fs::write(
        &input,
        "{\"id\":\"D1\",\"kind\":\"decision\",\"text\":\"first decision\"}\n{\"id\":\"D2\",\"kind\":\"decision\",\"text\":\"retained decision\"}\n",
    )?;
    let import = [
        "import",
        "--project",
        "p",
        "--source",
        "export",
        "--format",
        "journal",
        "--file",
        path(&input)?,
    ];
    assert!(cli(&store, &import)?.0);
    fs::write(
        &input,
        "{\"id\":\"D1\",\"kind\":\"decision\",\"text\":\"corrected decision\"}\n",
    )?;
    let mut partial = import.to_vec();
    partial.extend(["--completeness", "partial"]);
    assert!(cli(&store, &partial)?.0);
    assert!(cli(&store, &["sync", "--project", "p", "--source", "export"])?.0);
    fs::write(
        &query,
        serde_json::to_vec(
            &json!({"operation":"read","scope":{"project_id":"p"},"target":{"kind":"record","id":"export:D2"}}),
        )?,
    )?;
    let (ok, result) = cli(&store, &["query", "--input", path(&query)?])?;
    assert!(ok);
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("no items")?;
    let record = items
        .first()
        .and_then(|item| item.pointer("/entity/data"))
        .ok_or("no retained record")?;
    assert_eq!(record.get("body"), Some(&json!("retained decision")));
    assert_eq!(record.get("availability"), Some(&json!("available")));
    Ok(())
}

#[test]
fn model_configuration_failures_are_not_no_matches() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = dir.path().join("context.sqlite");
    let query = dir.path().join("query.json");
    let invalid = dir.path().join("invalid.json");
    assert!(cli(&store, &["init", "--project", "p"])?.0);
    fs::write(
        &query,
        serde_json::to_vec(
            &json!({"operation":"search","scope":{"project_id":"p"},"query":{"mode":"semantic","text":"why did it fail"}}),
        )?,
    )?;
    let (ok, result) = cli(&store, &["query", "--input", path(&query)?])?;
    assert!(!ok);
    assert_eq!(
        result.pointer("/error/code"),
        Some(&json!("semantic_unavailable"))
    );
    for contents in [None, Some("{broken json")] {
        if let Some(contents) = contents {
            fs::write(&invalid, contents)?;
        }
        let (ok, result) = cli(
            &store,
            &[
                "query",
                "--input",
                path(&query)?,
                "--semantic-config",
                path(&invalid)?,
            ],
        )?;
        assert!(!ok);
        assert_eq!(
            result.pointer("/error/code"),
            Some(&json!("semantic_unavailable"))
        );
        assert!(result.get("status").is_none());
    }
    Ok(())
}
