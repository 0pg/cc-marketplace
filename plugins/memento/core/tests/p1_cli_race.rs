//! Real CLI/storage concurrency contracts; fixed vectors are not retrieval-quality evidence.
#![cfg(unix)]

use std::{
    error::Error,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const PROJECT: &str = "cli-revalidation-fixture";
const ORIGINAL: &str = "ORIGINAL_BODY_MUST_DISAPPEAR_AFTER_ACCESS_CHANGE";

struct Running {
    child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
    release: Option<PathBuf>,
}
impl Running {
    fn wait(&mut self, timeout: Duration) -> Result<(ExitStatus, Value), Box<dyn Error>> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                let bytes = fs::read(&self.stdout)?;
                let result = serde_json::from_slice(&bytes).map_err(|error| {
                    format!(
                        "CLI JSON: {error}; stderr: {}",
                        fs::read_to_string(&self.stderr).unwrap_or_default()
                    )
                })?;
                return Ok((status, result));
            }
            if started.elapsed() >= timeout {
                return Err("CLI exceeded bounded test deadline".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        // Release an embedding child before killing its owning CLI. The shell
        // worker also has an independent finite loop, so no child can wait forever.
        if let Some(release) = &self.release {
            let _ = fs::write(release, b"release");
        }
        for _ in 0..20 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(
    root: &Path,
    name: &str,
    command: &mut Command,
    release: Option<PathBuf>,
) -> Result<Running, Box<dyn Error>> {
    let stdout = root.join(format!("{name}.stdout"));
    let stderr = root.join(format!("{name}.stderr"));
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&stdout)?))
        .stderr(Stdio::from(File::create(&stderr)?))
        .spawn()?;
    Ok(Running {
        child,
        stdout,
        stderr,
        release,
    })
}
fn cli(database: &Path, operation: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_work-context"));
    command.arg(operation).arg("--store").arg(database);
    command
}
fn successful(root: &Path, name: &str, command: &mut Command) -> Result<Value, Box<dyn Error>> {
    let mut running = spawn(root, name, command, None)?;
    let (status, value) = running.wait(Duration::from_secs(10))?;
    if !status.success() {
        return Err(format!("{name} failed: {value}").into());
    }
    Ok(value)
}

fn exercise(change: &str, expected_error: &str) -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let database = root.join("context.sqlite");
    successful(
        root,
        "init",
        cli(&database, "init").args(["--project", PROJECT]),
    )?;
    let note = root.join("note.json");
    fs::write(
        &note,
        serde_json::to_vec(&json!({"id":"evidence","kind":"tool_result","body":ORIGINAL}))?,
    )?;
    successful(
        root,
        "note",
        cli(&database, "note")
            .args(["--project", PROJECT, "--input"])
            .arg(&note),
    )?;

    let ready = root.join("worker-ready");
    let release = root.join("worker-release");
    let done = root.join("worker-done");
    let worker = root.join("worker.sh");
    fs::write(
        &worker,
        r#"#!/bin/sh
set -eu
trap ': > "$3"' EXIT
cat >/dev/null
: > "$1"
attempt=0
while [ ! -e "$2" ]; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 500 ]; then
        exit 75
    fi
    sleep 0.01
done
printf '%s\n' '{"protocol":1,"model_id":"mechanical-cli-contract","model_revision":"fixed-1","query":[1.0,0.0],"documents":[[1.0,0.0]],"metrics":{}}'
"#,
    )?;
    let config = root.join("semantic.json");
    fs::write(
        &config,
        serde_json::to_vec(
            &json!({"command":["/bin/sh",worker,ready,release,done],"model_id":"mechanical-cli-contract","model_revision":"fixed-1","timeout_ms":8000,"min_score":0.5}),
        )?,
    )?;
    let input = root.join("query.json");
    fs::write(
        &input,
        serde_json::to_vec(
            &json!({"operation":"search","scope":{"project_id":PROJECT},"query":{"text":"why did it fail","mode":"semantic"}}),
        )?,
    )?;
    let mut query = spawn(
        root,
        "query",
        cli(&database, "query")
            .arg("--input")
            .arg(&input)
            .arg("--semantic-config")
            .arg(&config),
        Some(release.clone()),
    )?;
    let started = Instant::now();
    while !ready.exists() {
        if let Some(status) = query.child.try_wait()? {
            return Err(format!(
                "query exited before the provider barrier: {status}; {}",
                fs::read_to_string(&query.stdout)?
            )
            .into());
        }
        if started.elapsed() > Duration::from_secs(5) {
            return Err("embedding provider did not reach ready barrier".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    // This is a separate CLI process/SQLite connection while the original query
    // is blocked inside its local provider, after reading the old body.
    if change == "revoke" {
        successful(
            root,
            "mutation",
            cli(&database, "source-access").args([
                "--project",
                PROJECT,
                "--source",
                "journal",
                "--allow",
                "false",
            ]),
        )?;
    } else {
        successful(
            root,
            "mutation",
            cli(&database, "delete-record").args(["--project", PROJECT, "--id", "evidence"]),
        )?;
    }
    fs::write(&release, b"release")?;
    let (status, response) = query.wait(Duration::from_secs(10))?;
    assert!(
        !status.success(),
        "query must reject stale model input: {response}"
    );
    assert_eq!(
        response.pointer("/error/code").and_then(Value::as_str),
        Some(expected_error)
    );
    assert!(!serde_json::to_string(&response)?.contains(ORIGINAL));
    assert!(response.get("items").is_none());
    assert!(
        done.exists(),
        "embedding worker must have exited before response"
    );
    Ok(())
}

#[test]
fn concurrent_source_revocation_is_rechecked_after_the_cli_embedding_process() -> TestResult {
    exercise("revoke", "invalid_scope")
}

#[test]
fn concurrent_record_deletion_is_rechecked_after_the_cli_embedding_process() -> TestResult {
    exercise("delete", "stale_cursor")
}
