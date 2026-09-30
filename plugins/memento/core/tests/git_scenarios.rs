use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use work_context::git::{self, WorkingKind};

type TestResult = Result<(), Box<dyn Error>>;

fn command(repository: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn repository() -> Result<TempDir, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    command(
        directory.path(),
        &["init", "--quiet", "--initial-branch=main"],
    )?;
    command(directory.path(), &["config", "user.name", "Context Test"])?;
    command(
        directory.path(),
        &["config", "user.email", "context@example.invalid"],
    )?;
    command(directory.path(), &["config", "commit.gpgsign", "false"])?;
    command(
        directory.path(),
        &["config", "core.hooksPath", ".git/test-hooks"],
    )?;
    Ok(directory)
}

fn commit(repository: &Path, message: &str) -> Result<String, Box<dyn Error>> {
    command(repository, &["commit", "--quiet", "-m", message])?;
    command(repository, &["rev-parse", "HEAD"])
}

#[test]
fn partial_commit_keeps_head_index_and_working_distinct() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    fs::write(repo.join("retry.rs"), "baseline\n")?;
    command(repo, &["add", "retry.rs"])?;
    let first = commit(repo, "baseline\n\nOriginal conditions")?;
    fs::write(repo.join("retry.rs"), "retry change\n")?;
    command(repo, &["add", "retry.rs"])?;
    fs::write(repo.join("retry.rs"), "retry change\nlog change\n")?;
    fs::write(repo.join("new.txt"), "untracked relevant output")?;
    let before = git::snapshot(repo, &["retry.rs".into(), "new.txt".into()])?;
    assert_eq!(before.head.as_deref(), Some(first.as_str()));
    assert!(!before.changed_during_observation);
    let retry = before
        .paths
        .iter()
        .find(|path| path.path == "retry.rs")
        .ok_or("missing path")?;
    let head = retry.head.as_ref().ok_or("missing HEAD file")?;
    let index = retry.index.first().ok_or("missing index file")?;
    assert_ne!(head.object_id, index.object_id);
    assert!(before.status.iter().any(|entry| entry.path == "retry.rs"
        && entry.index_status == "M"
        && entry.working_status == "M"));
    assert!(
        before
            .status
            .iter()
            .any(|entry| entry.path == "new.txt" && entry.index_status == "?")
    );
    let second = commit(repo, "retry only")?;
    let result = git::read_commit(repo, &second)?;
    assert_eq!(result.parents, vec![first.clone()]);
    assert_eq!(result.changed_paths, vec!["retry.rs"]);
    assert!(result.origin_worktree.is_none());
    assert_eq!(command(repo, &["show", "HEAD:retry.rs"])?, "retry change");
    let after = git::snapshot(repo, &["retry.rs".into()])?;
    assert!(
        after
            .status
            .iter()
            .any(|entry| entry.index_status == " " && entry.working_status == "M")
    );
    assert_eq!(
        fs::read_to_string(repo.join("retry.rs"))?,
        "retry change\nlog change\n"
    );
    let initial = git::read_commit(repo, &first)?;
    assert!(initial.parents.is_empty());
    assert_eq!(initial.diff_parent, None);
    assert_eq!(initial.message, "baseline\n\nOriginal conditions");
    assert!(!initial.committed_at.is_empty());
    Ok(())
}

#[test]
fn initial_empty_and_bounded_reconcile_work_without_history_inference() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let unborn = git::snapshot(repo, &[])?;
    assert!(unborn.head.is_none());
    command(repo, &["commit", "--quiet", "--allow-empty", "-m", "empty"])?;
    let first = git::read_commit(repo, "HEAD")?;
    assert!(first.parents.is_empty());
    assert!(first.changed_paths.is_empty());
    command(
        repo,
        &["commit", "--quiet", "--allow-empty", "-m", "another"],
    )?;
    let result = git::reconcile(repo, &["HEAD".into()], 1)?;
    assert!(result.truncated);
    assert_eq!(result.commits.len(), 1);
    assert!(
        result
            .commits
            .iter()
            .all(|entry| entry.origin_worktree.is_none())
    );
    assert!(git::reconcile(repo, &[], 10).is_err());
    assert!(git::reconcile(repo, &["HEAD".into()], 101).is_err());
    assert!(git::read_commit(repo, "--all").is_err());
    Ok(())
}

#[test]
fn rewrite_parser_preserves_many_to_one_and_deduplicates_delivery() -> TestResult {
    let first = "1".repeat(40);
    let second = "2".repeat(40);
    let combined = "3".repeat(40);
    let input = format!("{first} {combined}\n{second} {combined}\n{first} {combined}\n");
    let rewrites = git::parse_rewrites("rebase", &input)?;
    assert_eq!(rewrites.len(), 2);
    assert_eq!(rewrites.first().ok_or("missing rewrite")?.old_sha, first);
    assert!(rewrites.iter().all(|rewrite| rewrite.new_sha == combined));
    assert!(git::parse_rewrites("cherry-pick", &input).is_err());
    assert!(git::parse_rewrites("amend", "short invalid").is_err());
    Ok(())
}

#[test]
fn ignored_content_is_not_hashed_and_path_escape_is_rejected() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    fs::write(repo.join(".gitignore"), ".env\n")?;
    fs::write(repo.join(".env"), "SECRET=not-for-capture")?;
    let snapshot = git::snapshot(repo, &[".env".into()])?;
    let path = snapshot.paths.first().ok_or("missing ignored state")?;
    assert_eq!(path.working.kind, WorkingKind::Ignored);
    assert!(path.working.sha256.is_none());
    assert!(git::snapshot(repo, &[PathBuf::from("../outside")]).is_err());
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir()?;
        fs::write(outside.path().join("private"), "outside")?;
        std::os::unix::fs::symlink(outside.path(), repo.join("escape"))?;
        assert!(git::snapshot(repo, &[PathBuf::from("escape/private")]).is_err());
        std::os::unix::fs::symlink(outside.path().join("private"), repo.join("link"))?;
        let linked = git::snapshot(repo, &[PathBuf::from("link")])?;
        assert_eq!(
            linked.paths.first().ok_or("missing link")?.working.kind,
            WorkingKind::Symlink
        );
    }
    Ok(())
}

#[cfg(unix)]
fn executable_file(path: &Path, content: &str) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn hook_install_preserves_custom_path_legacy_output_input_and_exit() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let custom = repo.join("custom-hooks");
    fs::create_dir(&custom)?;
    command(repo, &["config", "core.hooksPath", "custom-hooks"])?;
    executable_file(
        &custom.join("post-rewrite"),
        "#!/bin/sh\ncat > legacy-input\nprintf '%s\\n' \"legacy:$1\"\nprintf '%s\\n' legacy-error >&2\nexit 7\n",
    )?;
    executable_file(
        &custom.join("post-commit"),
        "#!/bin/sh\nprintf '%s\\n' legacy-commit\nexit 0\n",
    )?;
    let stub = repo.join("context stub's executable");
    executable_file(
        &stub,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > capture-args\ncat > capture-input\nexit 0\n",
    )?;
    let status = git::install_hooks(repo, &stub, &repo.join("store's data"), "test-project")?;
    assert_eq!(
        status.configured_hooks_path.as_deref(),
        Some("custom-hooks")
    );
    assert!(
        status
            .hooks
            .iter()
            .all(|hook| hook.installed && hook.executable)
    );
    assert!(custom.join("post-rewrite.work-context-original").exists());
    git::install_hooks(repo, &stub, &repo.join("store's data"), "test-project")?;
    let successful = Command::new(custom.join("post-commit"))
        .current_dir(repo)
        .output()?;
    assert!(successful.status.success());
    assert_eq!(String::from_utf8(successful.stdout)?, "legacy-commit\n");
    assert!(successful.stderr.is_empty());
    let input = format!("{} {}\n", "1".repeat(40), "2".repeat(40));
    let mut child = Command::new(custom.join("post-rewrite"))
        .arg("amend")
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing stdin")?
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(String::from_utf8(output.stdout)?, "legacy:amend\n");
    assert_eq!(String::from_utf8(output.stderr)?, "legacy-error\n");
    assert_eq!(fs::read_to_string(repo.join("legacy-input"))?, input);
    assert_eq!(fs::read_to_string(repo.join("capture-input"))?, input);
    let args = fs::read_to_string(repo.join("capture-args"))?;
    assert!(args.contains("post-rewrite\namend\n"));
    assert!(args.contains("test-project\n"));
    assert_eq!(
        command(repo, &["config", "--get", "core.hooksPath"])?,
        "custom-hooks"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn actual_amend_and_squash_deliver_original_sha_mappings() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let stub = repo.join("capture");
    executable_file(
        &stub,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > capture-args\ncat > capture-input\nexit 0\n",
    )?;
    git::install_hooks(repo, &stub, &repo.join("store"), "test-project")?;
    fs::write(repo.join("retry.rs"), "first")?;
    command(repo, &["add", "retry.rs"])?;
    let original = commit(repo, "base")?;
    fs::write(repo.join("retry.rs"), "amended")?;
    command(repo, &["add", "retry.rs"])?;
    command(repo, &["commit", "--quiet", "--amend", "--no-edit"])?;
    let amended = command(repo, &["rev-parse", "HEAD"])?;
    let mappings = git::parse_rewrites("amend", &fs::read_to_string(repo.join("capture-input"))?)?;
    assert_eq!(mappings.len(), 1);
    let mapping = mappings.first().ok_or("missing amend mapping")?;
    assert_eq!(mapping.old_sha, original);
    assert_eq!(mapping.new_sha, amended);
    fs::write(repo.join("retry.rs"), "fixed up")?;
    command(repo, &["add", "retry.rs"])?;
    let fixup = commit(repo, "fixup! base")?;
    command(
        repo,
        &[
            "-c",
            "sequence.editor=true",
            "rebase",
            "--interactive",
            "--root",
            "--autosquash",
        ],
    )?;
    let squashed = command(repo, &["rev-parse", "HEAD"])?;
    let mappings = git::parse_rewrites("rebase", &fs::read_to_string(repo.join("capture-input"))?)?;
    assert!(
        mappings
            .iter()
            .any(|mapping| mapping.old_sha == amended && mapping.new_sha == squashed)
    );
    assert!(
        mappings
            .iter()
            .any(|mapping| mapping.old_sha == fixup && mapping.new_sha == squashed)
    );
    assert_eq!(git::read_commit(repo, &original)?.subject, "base");
    assert_eq!(command(repo, &["show", "HEAD:retry.rs"])?, "fixed up");
    Ok(())
}

#[cfg(unix)]
#[test]
fn hook_capture_failure_does_not_fail_commit_and_timeout_is_bounded() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let stub = repo.join("capture");
    executable_file(&stub, "#!/bin/sh\nexit 9\n")?;
    let status = git::install_hooks(repo, &stub, &repo.join("store"), "test-project")?;
    command(
        repo,
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "commit survives capture failure",
        ],
    )?;
    assert_eq!(
        git::read_commit(repo, "HEAD")?.subject,
        "commit survives capture failure"
    );
    executable_file(&stub, "#!/bin/sh\nexec sleep 30\n")?;
    let started = Instant::now();
    let result = Command::new(status.hooks_path.join("post-commit"))
        .current_dir(repo)
        .output()?;
    assert!(result.status.success());
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(String::from_utf8_lossy(&result.stderr).contains("failed or timed out"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_worktree_and_shared_hooks_keep_origin_scope() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    command(repo, &["commit", "--quiet", "--allow-empty", "-m", "start"])?;
    // Absolute configured hooks path is shared by both worktrees.
    let hooks = repo.join("hooks");
    let hooks_text = hooks.to_str().ok_or("non UTF-8 fixture")?;
    command(repo, &["config", "core.hooksPath", hooks_text])?;
    let stub = repo.join("capture");
    executable_file(
        &stub,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > capture-args\nexit 0\n",
    )?;
    git::install_hooks(repo, &stub, &repo.join("store"), "test-project")?;
    let linked = directory.path().join("linked");
    let linked_text = linked.to_str().ok_or("non UTF-8 fixture")?;
    command(
        repo,
        &["worktree", "add", "--quiet", "-b", "feature", linked_text],
    )?;
    assert!(linked.join(".git").is_file());
    fs::write(linked.join("retry.rs"), "linked change")?;
    command(&linked, &["add", "retry.rs"])?;
    commit(&linked, "linked result")?;
    let observed = git::snapshot(&linked, &["retry.rs".into()])?;
    assert_eq!(observed.worktree, linked.canonicalize()?);
    assert_ne!(observed.worktree_git_dir, repo.join(".git"));
    let args = fs::read_to_string(linked.join("capture-args"))?;
    assert!(args.contains(linked.canonicalize()?.to_str().ok_or("non UTF-8 fixture")?));
    assert!(
        git::hook_status(&linked)?
            .hooks
            .iter()
            .all(|hook| hook.installed)
    );
    // Another repository sharing this hooks path keeps its old hook behavior only.
    let other = repository()?;
    command(other.path(), &["config", "core.hooksPath", hooks_text])?;
    command(
        other.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "other"],
    )?;
    assert!(!other.path().join("capture-args").exists());
    assert!(
        git::install_hooks(other.path(), &stub, &repo.join("store"), "another-project").is_err()
    );
    Ok(())
}
