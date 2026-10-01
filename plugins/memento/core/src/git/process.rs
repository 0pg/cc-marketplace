use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::GitError;

const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;

pub(super) fn git(repository: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let output = run(repository, args, false)?;
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(output.stdout)
}

pub(super) fn git_optional(repository: &Path, args: &[&str]) -> Result<Option<Vec<u8>>, GitError> {
    let output = run(repository, args, false)?;
    if output.status.success() {
        Ok(Some(output.stdout))
    } else if output.status.code() == Some(1) {
        Ok(None)
    } else {
        Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ))
    }
}

/// Commit checkpoints must inspect Git's temporary index for partial commits.
pub(super) fn git_index(repository: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let output = run(repository, args, true)?;
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(output.stdout)
}

fn run(repository: &Path, args: &[&str], preserve_index: bool) -> Result<Output, GitError> {
    let mut command = Command::new("git");
    // Hook callers inherit Git-local variables; -C alone does not override them.
    for (name, _) in std::env::vars_os() {
        if name.to_str().is_some_and(|value| {
            value.starts_with("GIT_") && !(preserve_index && value == "GIT_INDEX_FILE")
        }) {
            command.env_remove(name);
        }
    }
    if args.first() != Some(&"check-ignore") {
        command.env("GIT_LITERAL_PATHSPECS", "1");
    }
    command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .args([
            "--no-pager",
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-C",
        ])
        .arg(repository)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitError::Command("missing stdout pipe".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GitError::Command("missing stderr pipe".into()))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(5) {
            timed_out = true;
            let _kill_result = child.kill();
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| GitError::Command("stdout reader failed".into()))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| GitError::Command("stderr reader failed".into()))??;
    if timed_out {
        return Err(GitError::Timeout);
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_bounded(reader: impl Read) -> Result<Vec<u8>, GitError> {
    let mut bytes = Vec::new();
    reader.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).map_err(|_| GitError::OutputLimit)? > OUTPUT_LIMIT {
        return Err(GitError::OutputLimit);
    }
    Ok(bytes)
}
