use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::process::{git, git_optional};
use super::{GitError, text, worktree_root};

const MARKER: &str = "# memento managed hook v1";
const HOOK_NAMES: [&str; 2] = ["post-commit", "post-rewrite"];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HookEntry {
    pub name: String,
    pub path: PathBuf,
    pub installed: bool,
    pub executable: bool,
    pub existing_hook: bool,
    pub preserved_hook: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HookStatus {
    pub repository: PathBuf,
    pub common_git_dir: PathBuf,
    pub hooks_path: PathBuf,
    pub configured_hooks_path: Option<String>,
    pub hooks: Vec<HookEntry>,
    /// Configuration proves neither invocation nor successful storage.
    pub invocation_state: String,
}

#[tracing::instrument(skip_all, fields(repository = %repository.display()))]
pub fn hook_status(repository: &Path) -> Result<HookStatus, GitError> {
    hook_status_with_checkpoint(repository, false)
}

fn hook_status_with_checkpoint(
    repository: &Path,
    include_checkpoint: bool,
) -> Result<HookStatus, GitError> {
    let repository = worktree_root(repository)?;
    let common_git_dir = common_dir(&repository)?;
    let hooks_path = PathBuf::from(
        text(git(
            &repository,
            &["rev-parse", "--path-format=absolute", "--git-path", "hooks"],
        )?)?
        .trim(),
    );
    let configured_hooks_path = git_optional(&repository, &["config", "--get", "core.hooksPath"])?
        .map(text)
        .transpose()?
        .map(|s| s.trim_end_matches('\n').to_owned());
    let tag = repository_tag(&common_git_dir)?;
    let mut hooks = Vec::new();
    for name in HOOK_NAMES.into_iter().chain(["pre-commit"]) {
        let path = hooks_path.join(name);
        if name == "pre-commit" && !include_checkpoint && !path.exists() {
            continue;
        }
        let content = read_existing(&path)?;
        let installed = content.as_ref().is_some_and(|bytes| {
            let body = String::from_utf8_lossy(bytes);
            body.contains(MARKER) && body.contains(&tag)
        });
        let preserved = hooks_path.join(format!("{name}.memento-original"));
        hooks.push(HookEntry {
            name: name.into(),
            executable: executable(&path)?,
            existing_hook: content.is_some() && !installed,
            installed,
            path,
            preserved_hook: fs::symlink_metadata(&preserved)
                .is_ok()
                .then_some(preserved),
        });
    }
    Ok(HookStatus {
        repository,
        common_git_dir,
        hooks_path,
        configured_hooks_path,
        hooks,
        invocation_state: "unknown_until_observed".into(),
    })
}

/// Opt-in installation. Existing hook programs are retained and invoked first.
/// A shared managed hook belonging to a different repository is never replaced.
#[tracing::instrument(skip_all, fields(repository = %repository.display(), project))]
pub fn install_hooks(
    repository: &Path,
    executable_path: &Path,
    store: &Path,
    project: &str,
) -> Result<HookStatus, GitError> {
    install_hooks_with_policy(repository, executable_path, store, project, None)
}

pub fn install_hooks_with_policy(
    repository: &Path,
    executable_path: &Path,
    store: &Path,
    project: &str,
    policy: Option<&Path>,
) -> Result<HookStatus, GitError> {
    install_with_policy(repository, executable_path, store, project, policy, false)
}

/// Opt-in gate: a missing or stale semantic checkpoint aborts the commit.
pub fn install_checkpoint_hooks_with_policy(
    repository: &Path,
    executable_path: &Path,
    store: &Path,
    project: &str,
    policy: Option<&Path>,
) -> Result<HookStatus, GitError> {
    install_with_policy(repository, executable_path, store, project, policy, true)
}

fn install_with_policy(
    repository: &Path,
    executable_path: &Path,
    store: &Path,
    project: &str,
    policy: Option<&Path>,
    enforce_checkpoints: bool,
) -> Result<HookStatus, GitError> {
    let policy = policy.map(Path::canonicalize).transpose()?;
    if project.is_empty() || project.contains('\0') {
        return Err(GitError::InvalidInput(
            "project must be a nonempty identifier".into(),
        ));
    }
    let executable_path = executable_path.canonicalize()?;
    if !executable(&executable_path)? {
        return Err(GitError::InvalidInput(
            "hook executable is not executable".into(),
        ));
    }
    let store = if store.is_absolute() {
        store.to_path_buf()
    } else {
        std::env::current_dir()?.join(store)
    };
    let status = hook_status_with_checkpoint(repository, enforce_checkpoints)?;
    let mut plans = Vec::new();
    for hook in &status.hooks {
        if hook.name == "pre-commit" && !enforce_checkpoints {
            continue;
        }
        let backup = status
            .hooks_path
            .join(format!("{}.memento-original", hook.name));
        let body = script(
            &status.common_git_dir,
            &executable_path,
            &store,
            project,
            &hook.name,
            &backup,
            policy.as_deref(),
        )?;
        let existing = read_existing(&hook.path)?;
        if let Some(existing) = &existing {
            if existing == body.as_bytes() && hook.executable {
                continue;
            }
            if String::from_utf8_lossy(existing).contains(MARKER) {
                return Err(GitError::HookConflict(format!(
                    "{} is already managed with different settings",
                    hook.path.display()
                )));
            }
        }
        if fs::symlink_metadata(&backup).is_ok() {
            return Err(GitError::HookConflict(format!(
                "preserved hook already exists: {}",
                backup.display()
            )));
        }
        plans.push((hook.path.clone(), backup, body, existing.is_some()));
    }
    fs::create_dir_all(&status.hooks_path)?;
    for (path, backup, body, existing) in plans {
        let temporary = write_temporary(&status.hooks_path, body.as_bytes())?;
        if existing && let Err(error) = fs::rename(&path, &backup) {
            let _cleanup_result = fs::remove_file(&temporary);
            return Err(error.into());
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            if existing {
                let _restore_result = fs::rename(&backup, &path);
            }
            let _cleanup_result = fs::remove_file(&temporary);
            return Err(error.into());
        }
    }
    hook_status(repository)
}

fn script(
    common_git_dir: &Path,
    executable_path: &Path,
    store: &Path,
    project: &str,
    hook: &str,
    backup: &Path,
    policy: Option<&Path>,
) -> Result<String, GitError> {
    let tag = repository_tag(common_git_dir)?;
    let common = quote_path(common_git_dir)?;
    let executable = quote_path(executable_path)?;
    let store = quote_path(store)?;
    let project = shell_quote(project);
    let backup = quote_path(backup)?;
    let policy = policy
        .map(|path| quote_path(path).map(|path| format!(" --policy {path}")))
        .transpose()?
        .unwrap_or_default();
    let input = if hook == "post-rewrite" {
        "context_input=$(mktemp \"${TMPDIR:-/tmp}/memento-hook.XXXXXX\") || {\n  printf '%s\\n' 'memento: cannot preserve rewrite input; capture skipped' >&2\n  if [ -x \"$context_previous\" ]; then exec \"$context_previous\" \"$@\"; fi\n  exit 0\n}\ntrap 'rm -f \"$context_input\"' EXIT HUP INT TERM\ncat > \"$context_input\" || { printf '%s\\n' 'memento: cannot copy rewrite input' >&2; exit 0; }\n"
    } else {
        "context_input=/dev/null\n"
    };
    if hook == "pre-commit" {
        return Ok(format!(
            "#!/bin/sh\n{MARKER}\n{tag}\ncontext_previous={backup}\nif [ -x \"$context_previous\" ]; then\n  \"$context_previous\" \"$@\"\n  context_previous_status=$?\n  if [ \"$context_previous_status\" -ne 0 ]; then exit \"$context_previous_status\"; fi\nfi\ncontext_common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)\nif [ \"$context_common\" != {common} ]; then exit 0; fi\ncontext_repository=$(git rev-parse --show-toplevel 2>/dev/null) || exit 1\n{executable} hook --repository \"$context_repository\" --store {store} --project {project}{policy} pre-commit > /dev/null &\ncontext_capture_pid=$!\n(sleep 5; kill -KILL \"$context_capture_pid\" 2>/dev/null) >/dev/null 2>&1 &\ncontext_watchdog_pid=$!\nwait \"$context_capture_pid\"\ncontext_capture_status=$?\nkill \"$context_watchdog_pid\" 2>/dev/null\nwait \"$context_watchdog_pid\" 2>/dev/null\nif [ \"$context_capture_status\" -ne 0 ]; then\n  printf '%s\\n' 'memento: commit blocked; prepare and resolve a checkpoint for the actual commit index, then retry (validation failed or timed out)' >&2\n  exit 1\nfi\nexit 0\n"
        ));
    }
    Ok(format!(
        "#!/bin/sh\n{MARKER}\n{tag}\ncontext_previous={backup}\n{input}context_previous_status=0\nif [ -x \"$context_previous\" ]; then\n  \"$context_previous\" \"$@\" < \"$context_input\"\n  context_previous_status=$?\nfi\ncontext_common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)\nif [ \"$context_common\" != {common} ]; then exit \"$context_previous_status\"; fi\ncontext_repository=$(git rev-parse --show-toplevel 2>/dev/null) || exit \"$context_previous_status\"\n{executable} hook --repository \"$context_repository\" --store {store} --project {project}{policy} {hook} \"$@\" < \"$context_input\" > /dev/null &\ncontext_capture_pid=$!\n(sleep 5; kill -KILL \"$context_capture_pid\" 2>/dev/null) >/dev/null 2>&1 &\ncontext_watchdog_pid=$!\nwait \"$context_capture_pid\"\ncontext_capture_status=$?\nkill \"$context_watchdog_pid\" 2>/dev/null\nwait \"$context_watchdog_pid\" 2>/dev/null\nif [ \"$context_capture_status\" -ne 0 ]; then\n  printf '%s\\n' 'memento: local context capture failed or timed out; Git result is unchanged' >&2\nfi\nexit \"$context_previous_status\"\n"
    ))
}

fn common_dir(repository: &Path) -> Result<PathBuf, GitError> {
    let value = text(git(
        repository,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?)?;
    Ok(PathBuf::from(value.trim()).canonicalize()?)
}

fn repository_tag(common: &Path) -> Result<String, GitError> {
    let path = common.to_str().ok_or(GitError::NonUtf8)?;
    Ok(format!(
        "# memento repository {:x}",
        Sha256::digest(path.as_bytes())
    ))
}

fn read_existing(path: &Path) -> Result<Option<Vec<u8>>, GitError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= 1024 * 1024 => {
            Ok(Some(fs::read(path)?))
        }
        Ok(_) => Err(GitError::HookConflict(format!(
            "hook is not a bounded regular file: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if fs::symlink_metadata(path).is_ok() {
                Err(GitError::HookConflict(format!(
                    "hook is a broken symlink: {}",
                    path.display()
                )))
            } else {
                Ok(None)
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn write_temporary(directory: &Path, body: &[u8]) -> Result<PathBuf, GitError> {
    for attempt in 0..8 {
        let path = directory.join(format!(
            ".memento-install-{}-{}-{attempt}",
            std::process::id(),
            chrono::Utc::now().timestamp_micros()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let result = (|| -> Result<(), GitError> {
                    file.write_all(body)?;
                    file.sync_all()?;
                    make_executable(&path)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _cleanup_result = fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(GitError::HookConflict(
        "could not allocate installation file".into(),
    ))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), GitError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), GitError> {
    Err(GitError::InvalidInput(
        "hook installation currently supports Unix Git environments".into(),
    ))
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<bool, GitError> {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() && metadata.permissions().mode() & 0o111 != 0),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(not(unix))]
fn executable(_path: &Path) -> Result<bool, GitError> {
    Ok(false)
}

fn quote_path(path: &Path) -> Result<String, GitError> {
    Ok(shell_quote(path.to_str().ok_or(GitError::NonUtf8)?))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
