//! Bounded, read-only Git observations and opt-in local hook integration.

mod hooks;
mod process;

pub use hooks::{HookEntry, HookStatus, hook_status, install_hooks, install_hooks_with_policy};

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use process::{git, git_optional};

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Git I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git command failed: {0}")]
    Command(String),
    #[error("Git command exceeded its five-second limit")]
    Timeout,
    #[error("Git output exceeded the supported size")]
    OutputLimit,
    #[error("Git returned unsupported non-UTF-8 text")]
    NonUtf8,
    #[error("invalid Git input: {0}")]
    InvalidInput(String),
    #[error("unsupported Git output: {0}")]
    InvalidOutput(String),
    #[error("hook installation conflict: {0}")]
    HookConflict(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitEntry {
    pub mode: String,
    pub object_id: String,
    pub stage: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingKind {
    File,
    Symlink,
    Directory,
    Missing,
    Ignored,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkingFile {
    pub kind: WorkingKind,
    /// Content equality aid only; a digest is not a recoverable snapshot.
    pub sha256: Option<String>,
    pub byte_length: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitPathState {
    pub path: String,
    pub head: Option<GitEntry>,
    /// All stages are preserved for an unresolved merge.
    pub index: Vec<GitEntry>,
    pub working: WorkingFile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub original_path: Option<String>,
    pub index_status: String,
    pub working_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GitSnapshot {
    pub repository: PathBuf,
    pub worktree: PathBuf,
    pub worktree_git_dir: PathBuf,
    pub observed_at: String,
    pub head: Option<String>,
    pub paths: Vec<GitPathState>,
    pub status: Vec<GitStatusEntry>,
    pub changed_during_observation: bool,
    /// A second observation is returned when it differs, without claiming atomicity.
    pub after: Option<GitObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitObservation {
    pub head: Option<String>,
    pub paths: Vec<GitPathState>,
    pub status: Vec<GitStatusEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GitCommit {
    pub sha: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub subject: String,
    pub message: String,
    pub committed_at: String,
    pub changed_paths: Vec<String>,
    /// Merge changes are relative to this parent; None means an initial commit.
    pub diff_parent: Option<String>,
    pub observed_worktree: PathBuf,
    /// Reading a commit cannot recover its original creation worktree.
    pub origin_worktree: Option<PathBuf>,
    pub observed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GitReconcile {
    pub requested_refs: Vec<String>,
    pub resolved_refs: Vec<String>,
    pub commits: Vec<GitCommit>,
    pub truncated: bool,
    pub limit: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitRewrite {
    pub command: String,
    pub old_sha: String,
    pub new_sha: String,
}

/// Observe only explicitly requested file contents. Empty paths return metadata only.
#[tracing::instrument(skip_all, fields(repository = %repository.display(), path_count = paths.len()))]
pub fn snapshot(repository: &Path, paths: &[PathBuf]) -> Result<GitSnapshot, GitError> {
    snapshot_between(repository, paths, || Ok(()))
}

fn snapshot_between(
    repository: &Path,
    paths: &[PathBuf],
    between: impl FnOnce() -> Result<(), GitError>,
) -> Result<GitSnapshot, GitError> {
    let worktree = worktree_root(repository)?;
    let paths = validate_paths(paths)?;
    let worktree_git_dir =
        PathBuf::from(text(git(&worktree, &["rev-parse", "--absolute-git-dir"])?)?.trim());
    let observed_at = Utc::now().to_rfc3339();
    let before = observe(&worktree, &paths)?;
    between()?;
    let after = observe(&worktree, &paths)?;
    let changed = before != after;
    Ok(GitSnapshot {
        repository: worktree.clone(),
        worktree,
        worktree_git_dir,
        observed_at,
        head: before.head,
        paths: before.paths,
        status: before.status,
        changed_during_observation: changed,
        after: changed.then_some(after),
    })
}

#[tracing::instrument(skip_all, fields(repository = %repository.display(), revision))]
pub fn read_commit(repository: &Path, revision: &str) -> Result<GitCommit, GitError> {
    let worktree = worktree_root(repository)?;
    let sha = resolve(&worktree, revision)?;
    let data = text(git(
        &worktree,
        &[
            "show",
            "--no-patch",
            "--format=%H%x00%T%x00%P%x00%ct%x00%B",
            &sha,
            "--",
        ],
    )?)?;
    let mut fields = data.trim_end_matches('\n').splitn(5, '\0');
    let recorded_sha = required(&mut fields, "commit SHA")?.to_owned();
    let tree = required(&mut fields, "tree SHA")?.to_owned();
    let parents: Vec<String> = required(&mut fields, "parents")?
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let timestamp = required(&mut fields, "commit timestamp")?
        .parse::<i64>()
        .map_err(|_| GitError::InvalidOutput("invalid commit timestamp".into()))?;
    let committed_at = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| GitError::InvalidOutput("commit timestamp is out of range".into()))?
        .to_rfc3339();
    let message = required(&mut fields, "message")?.to_owned();
    let subject = match message.lines().next() {
        Some(line) => line.to_owned(),
        None => String::new(),
    };
    let diff_parent = parents.first().cloned();
    let mut args = vec![
        "diff-tree",
        "--no-commit-id",
        "--name-only",
        "--no-renames",
        "-r",
        "-z",
    ];
    if let Some(parent) = &diff_parent {
        args.push(parent);
    } else {
        args.push("--root");
    }
    args.extend([sha.as_str(), "--"]);
    let changed_paths = nul_text(git(&worktree, &args)?)?;
    Ok(GitCommit {
        sha: recorded_sha,
        tree,
        parents,
        subject,
        message,
        committed_at,
        changed_paths,
        diff_parent,
        observed_worktree: worktree,
        origin_worktree: None,
        observed_at: Utc::now().to_rfc3339(),
    })
}

/// Read a bounded slice of commits reachable from explicit refs, without fetching.
#[tracing::instrument(skip_all, fields(repository = %repository.display(), ref_count = refs.len(), limit))]
pub fn reconcile(
    repository: &Path,
    refs: &[String],
    limit: usize,
) -> Result<GitReconcile, GitError> {
    if refs.is_empty() || refs.len() > 32 || !(1..=100).contains(&limit) {
        return Err(GitError::InvalidInput(
            "provide 1–32 refs and a limit from 1 to 100".into(),
        ));
    }
    let resolved_refs: Vec<String> = refs
        .iter()
        .map(|r| resolve(repository, r))
        .collect::<Result<_, _>>()?;
    let count = format!("--max-count={}", limit.saturating_add(1));
    let mut args = vec!["rev-list", "--topo-order", count.as_str()];
    args.extend(resolved_refs.iter().map(String::as_str));
    args.push("--");
    let output = text(git(repository, &args)?)?;
    let shas: Vec<&str> = output.lines().filter(|s| !s.is_empty()).collect();
    let commits = shas
        .iter()
        .take(limit)
        .map(|sha| read_commit(repository, sha))
        .collect::<Result<_, _>>()?;
    Ok(GitReconcile {
        requested_refs: refs.to_vec(),
        resolved_refs,
        commits,
        truncated: shas.len() > limit,
        limit,
    })
}

pub fn parse_rewrites(command: &str, input: &str) -> Result<Vec<GitRewrite>, GitError> {
    if !matches!(command, "amend" | "rebase") || input.len() > 4 * 1024 * 1024 {
        return Err(GitError::InvalidInput(
            "unsupported rewrite command or oversized input".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for line in input.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let old_sha = fields
            .next()
            .ok_or_else(|| GitError::InvalidInput("missing rewrite old SHA".into()))?;
        let new_sha = fields
            .next()
            .ok_or_else(|| GitError::InvalidInput("missing rewrite new SHA".into()))?;
        if !object_id(old_sha) || !object_id(new_sha) || old_sha.len() != new_sha.len() {
            return Err(GitError::InvalidInput("invalid rewrite object IDs".into()));
        }
        if seen.insert((old_sha.to_owned(), new_sha.to_owned())) {
            result.push(GitRewrite {
                command: command.into(),
                old_sha: old_sha.into(),
                new_sha: new_sha.into(),
            });
        }
    }
    Ok(result)
}

fn observe(repository: &Path, paths: &[String]) -> Result<GitObservation, GitError> {
    let head = git_optional(
        repository,
        &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"],
    )?
    .map(text)
    .transpose()?
    .map(|s| s.trim().to_owned());
    let mut args = vec![
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=normal",
        "--",
    ];
    args.extend(paths.iter().map(String::as_str));
    let status = parse_status(git(repository, &args)?)?;
    let states = paths
        .iter()
        .map(|path| observe_path(repository, head.as_deref(), path))
        .collect::<Result<_, _>>()?;
    Ok(GitObservation {
        head,
        paths: states,
        status,
    })
}

fn observe_path(
    repository: &Path,
    head: Option<&str>,
    path: &str,
) -> Result<GitPathState, GitError> {
    let head = match head {
        Some(sha) => {
            let entries = nul_text(git(repository, &["ls-tree", "-z", sha, "--", path])?)?;
            entries
                .first()
                .map(|entry| parse_entry(entry, false))
                .transpose()?
        }
        None => None,
    };
    let index = nul_text(git(repository, &["ls-files", "--stage", "-z", "--", path])?)?
        .iter()
        .map(|entry| parse_entry(entry, true))
        .collect::<Result<Vec<_>, _>>()?;
    let ignored = git_optional(repository, &["check-ignore", "-q", "--", path])?.is_some();
    let working = if ignored && head.is_none() && index.is_empty() {
        WorkingFile {
            kind: WorkingKind::Ignored,
            sha256: None,
            byte_length: None,
        }
    } else {
        working_file(repository, path)?
    };
    Ok(GitPathState {
        path: path.into(),
        head,
        index,
        working,
    })
}

fn working_file(repository: &Path, path: &str) -> Result<WorkingFile, GitError> {
    let file = repository.join(path);
    // Parent symlinks can escape the authorized worktree; the leaf symlink is hashed as a link.
    if let Some(parent) = file.parent() {
        match parent.canonicalize() {
            Ok(canonical) if !canonical.starts_with(repository) => {
                return Err(GitError::InvalidInput(
                    "requested path escapes the worktree".into(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(WorkingFile {
                    kind: WorkingKind::Missing,
                    sha256: None,
                    byte_length: None,
                });
            }
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    let metadata = match fs::symlink_metadata(&file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WorkingFile {
                kind: WorkingKind::Missing,
                sha256: None,
                byte_length: None,
            });
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.is_dir() {
        return Ok(WorkingFile {
            kind: WorkingKind::Directory,
            sha256: None,
            byte_length: None,
        });
    }
    let (kind, bytes) = if metadata.file_type().is_symlink() {
        let target = fs::read_link(&file)?;
        let value = target.to_str().ok_or(GitError::NonUtf8)?;
        (WorkingKind::Symlink, value.as_bytes().to_vec())
    } else if metadata.is_file() {
        if metadata.len() > 16 * 1024 * 1024 {
            return Err(GitError::InvalidInput(
                "requested file exceeds 16 MiB content observation limit".into(),
            ));
        }
        let mut bytes = Vec::new();
        fs::File::open(&file)?
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(GitError::InvalidInput(
                "requested file grew beyond the content observation limit".into(),
            ));
        }
        (WorkingKind::File, bytes)
    } else {
        return Err(GitError::InvalidInput(
            "requested path is not a regular file or symlink".into(),
        ));
    };
    let length = u64::try_from(bytes.len()).map_err(|_| GitError::OutputLimit)?;
    Ok(WorkingFile {
        kind,
        sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
        byte_length: Some(length),
    })
}

fn parse_entry(entry: &str, staged: bool) -> Result<GitEntry, GitError> {
    let (metadata, _) = entry
        .split_once('\t')
        .ok_or_else(|| GitError::InvalidOutput("entry missing path separator".into()))?;
    let mut fields = metadata.split_whitespace();
    let mode = required(&mut fields, "entry mode")?.to_owned();
    let (object_id, stage) = if staged {
        let object_id = required(&mut fields, "entry object")?.to_owned();
        let stage = required(&mut fields, "entry stage")?
            .parse::<u8>()
            .map_err(|_| GitError::InvalidOutput("invalid index stage".into()))?;
        (object_id, stage)
    } else {
        let _kind = required(&mut fields, "entry kind")?;
        (required(&mut fields, "entry object")?.to_owned(), 0)
    };
    Ok(GitEntry {
        mode,
        object_id,
        stage,
    })
}

fn parse_status(bytes: Vec<u8>) -> Result<Vec<GitStatusEntry>, GitError> {
    let value = text(bytes)?;
    let mut records = value.split('\0').filter(|s| !s.is_empty());
    let mut result = Vec::new();
    while let Some(record) = records.next() {
        let index = record
            .get(..1)
            .ok_or_else(|| GitError::InvalidOutput("status index field".into()))?;
        let working = record
            .get(1..2)
            .ok_or_else(|| GitError::InvalidOutput("status working field".into()))?;
        let path = record
            .get(3..)
            .ok_or_else(|| GitError::InvalidOutput("status path".into()))?;
        let original_path = if matches!(index, "R" | "C") || matches!(working, "R" | "C") {
            Some(
                records
                    .next()
                    .ok_or_else(|| GitError::InvalidOutput("rename source path".into()))?
                    .into(),
            )
        } else {
            None
        };
        result.push(GitStatusEntry {
            path: path.into(),
            original_path,
            index_status: index.into(),
            working_status: working.into(),
        });
    }
    Ok(result)
}

fn validate_paths(paths: &[PathBuf]) -> Result<Vec<String>, GitError> {
    if paths.len() > 100 {
        return Err(GitError::InvalidInput(
            "at most 100 paths may be observed".into(),
        ));
    }
    let mut result = BTreeSet::new();
    for path in paths {
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(GitError::InvalidInput(
                "paths must be relative files without parent traversal".into(),
            ));
        }
        let value = path.to_str().ok_or(GitError::NonUtf8)?;
        if value.contains('\0') {
            return Err(GitError::InvalidInput("NUL in path".into()));
        }
        result.insert(value.into());
    }
    Ok(result.into_iter().collect())
}

fn resolve(repository: &Path, revision: &str) -> Result<String, GitError> {
    if revision.is_empty() || revision.starts_with('-') || revision.contains('\0') {
        return Err(GitError::InvalidInput("invalid revision".into()));
    }
    let target = format!("{revision}^{{commit}}");
    Ok(text(git(
        repository,
        &["rev-parse", "--verify", "--end-of-options", &target],
    )?)?
    .trim()
    .into())
}

pub(super) fn worktree_root(repository: &Path) -> Result<PathBuf, GitError> {
    let value = text(git(repository, &["rev-parse", "--show-toplevel"])?)?;
    Ok(PathBuf::from(value.trim()).canonicalize()?)
}

pub(super) fn text(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes).map_err(|_| GitError::NonUtf8)
}

fn nul_text(bytes: Vec<u8>) -> Result<Vec<String>, GitError> {
    Ok(text(bytes)?
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

fn required<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<&'a str, GitError> {
    fields
        .next()
        .ok_or_else(|| GitError::InvalidOutput(format!("missing {name}")))
}

fn object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_change_between_observations() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        git(directory.path(), &["init", "--quiet"])?;
        fs::write(directory.path().join("file.txt"), "before")?;
        let observed = snapshot_between(directory.path(), &[PathBuf::from("file.txt")], || {
            fs::write(directory.path().join("file.txt"), "after")?;
            Ok(())
        })?;
        assert!(observed.changed_during_observation);
        assert!(observed.after.is_some());
        Ok(())
    }
}
