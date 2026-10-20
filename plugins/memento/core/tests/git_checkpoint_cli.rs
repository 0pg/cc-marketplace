//! Exercise checkpoints at real Git boundaries, including Git's temporary index.
#![cfg(unix)]

use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use memento::{
    Store,
    capture::{Decision, RecordRef, Request, Resolution, Scope},
    git, ingest,
    model::*,
    security::RedactionPolicy,
};
use serde_json::json;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;
const PROJECT: &str = "upload-app";

struct Fixture {
    _directory: TempDir,
    repository: PathBuf,
    database: PathBuf,
    scope: Scope,
}

fn git_command(repository: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repository)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command
}

fn success(output: Output) -> Result<String, Box<dyn Error>> {
    if !output.status.success() {
        return Err(format!(
            "subprocess failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn git_run(repository: &Path, arguments: &[&str]) -> Result<String, Box<dyn Error>> {
    success(git_command(repository).args(arguments).output()?)
}

impl Fixture {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let repository = directory.path().join("repository");
        fs::create_dir(&repository)?;
        git_run(&repository, &["init", "--quiet", "--initial-branch=main"])?;
        git_run(&repository, &["config", "user.name", "Memento Test"])?;
        git_run(
            &repository,
            &["config", "user.email", "memento@example.invalid"],
        )?;
        git_run(&repository, &["config", "commit.gpgsign", "false"])?;
        git_run(
            &repository,
            &["config", "core.hooksPath", ".git/test-hooks"],
        )?;
        let repository = repository.canonicalize()?;
        let database = directory.path().join("context.sqlite");
        let mut store = Store::open(&database, RedactionPolicy::default()).await?;
        store
            .append(Entity::Source(ingest::source(
                "journal",
                PROJECT,
                SourceKind::Journal,
            )))
            .await?;
        let scope = Scope {
            project_id: PROJECT.into(),
            repository: repository.to_string_lossy().into_owned(),
            work_id: "upload-429".into(),
            session_id: "codex-session".into(),
            turn_id: "turn-1".into(),
        };
        Ok(Self {
            _directory: directory,
            repository,
            database,
            scope,
        })
    }

    fn install(&self) -> TestResult {
        let status = git::install_checkpoint_hooks_with_policy(
            &self.repository,
            Path::new(env!("CARGO_BIN_EXE_memento")),
            &self.database,
            PROJECT,
            None,
        )?;
        assert_eq!(status.hooks.len(), 3);
        assert!(status.hooks.iter().all(|hook| hook.installed));
        Ok(())
    }

    async fn prepare(&self, id: &str, index: Option<&Path>) -> TestResult {
        // A child environment is safe in Rust 2024 and does not affect parallel tests.
        let mut command = Command::new(env!("CARGO_BIN_EXE_memento"));
        command
            .args(["checkpoint", "--store"])
            .arg(&self.database)
            .args(["--input", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", index);
        }
        let mut child = command.spawn()?;
        child
            .stdin
            .take()
            .ok_or("missing stdin")?
            .write_all(&serde_json::to_vec(&json!({
                "operation": "prepare_commit", "scope": self.scope,
                "event_id": id,
                "detail": "작업자 수만 제한하고 기존 재시도와 토큰 갱신 동작을 유지한다."
            }))?)?;
        success(child.wait_with_output()?)?;

        let mut store = Store::open(&self.database, RedactionPolicy::default()).await?;
        let record_id = format!("context:{id}");
        let mut record = Record::new(
            &record_id,
            PROJECT,
            "journal",
            RecordKind::Decision,
            "업로드 작업자 수를 4개로 제한. 로컬 40개 성공, 운영 피크와 처리량은 미검증.",
        );
        record.work_ids = vec![self.scope.work_id.clone()];
        record.session_id = Some(self.scope.session_id.clone());
        record.association = Association::Explicit;
        let status = store
            .checkpoint(Request::Status {
                scope: self.scope.clone(),
            })
            .await?;
        let event = status
            .events
            .iter()
            .find(|event| event.event_id == id)
            .ok_or("missing commit event")?;
        let origin = event.origin.as_ref().ok_or("missing commit observation")?;
        record.representation = Representation::Claim;
        record.context_id = event.context_id.clone();
        record.derived = true;
        record.fidelity = Fidelity::SummaryOnly;
        record.evidence.push(Evidence {
            source_id: origin.source_id.clone(),
            record_id: Some(origin.record_id.clone()),
            revision: origin.revision.clone(),
            locator: "commit observation".into(),
            availability: Availability::Available,
            range: Some(TextRange {
                start_line: 1,
                end_line: 1,
            }),
            purpose: EvidencePurpose::Origin,
            span: None,
        });
        let revision = record.revision.clone();
        let receipt = store.append(Entity::Record(record)).await?;
        assert!(receipt.durable);
        let reply = store
            .checkpoint(Request::Resolve {
                scope: self.scope.clone(),
                event_id: id.into(),
                resolution: Resolution::Records {
                    records: vec![RecordRef {
                        source_id: "journal".into(),
                        record_id,
                        revision,
                        sequence: receipt.sequence,
                    }],
                },
            })
            .await?;
        assert_eq!(reply.decision, Decision::Allow);
        Ok(())
    }

    async fn assert_link(&self, id: &str, sha: &str) -> TestResult {
        let mut store = Store::open(&self.database, RedactionPolicy::default()).await?;
        let reply = store
            .checkpoint(Request::Status {
                scope: self.scope.clone(),
            })
            .await?;
        let event = reply
            .events
            .iter()
            .find(|event| event.event_id == id)
            .ok_or("checkpoint missing")?;
        assert_eq!(event.commit_shas, [sha], "commit association for {id}");
        assert!(
            store
                .latest()
                .await?
                .iter()
                .any(|entity| matches!(entity, Entity::Commit(commit) if commit.sha == sha))
        );
        Ok(())
    }
}

#[tokio::test]
async fn missing_and_changed_index_block_commit_then_fresh_context_links_initial_and_amend()
-> TestResult {
    let fixture = Fixture::new().await?;
    let repo = &fixture.repository;
    fs::write(repo.join("retry.rs"), "workers=8\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    fixture.install()?;
    let missing = git_command(repo)
        .args(["commit", "-m", "initial"])
        .output()?;
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("commit blocked"));
    assert!(
        !git_command(repo)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()?
            .status
            .success()
    );

    fixture.prepare("initial-old", None).await?;
    fs::write(repo.join("retry.rs"), "workers=4\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    assert!(
        !git_command(repo)
            .args(["commit", "-m", "stale"])
            .output()?
            .status
            .success()
    );
    fixture.prepare("initial-current", None).await?;
    git_run(repo, &["commit", "--quiet", "-m", "limit workers"])?;
    let initial = git_run(repo, &["rev-parse", "HEAD"])?;
    fixture.assert_link("initial-current", &initial).await?;

    fixture.prepare("amend", None).await?;
    git_run(
        repo,
        &["commit", "--quiet", "--amend", "-m", "limit upload workers"],
    )?;
    let amended = git_run(repo, &["rev-parse", "HEAD"])?;
    assert_ne!(initial, amended);
    fixture.assert_link("amend", &amended).await?;
    Ok(())
}

#[tokio::test]
async fn partial_commit_checks_temporary_index_and_leaves_other_staged_change() -> TestResult {
    let fixture = Fixture::new().await?;
    let repo = &fixture.repository;
    for path in ["retry.rs", "logging.rs"] {
        fs::write(repo.join(path), "baseline\n")?;
    }
    git_run(repo, &["add", "retry.rs", "logging.rs"])?;
    git_run(repo, &["commit", "--quiet", "-m", "baseline"])?;
    for path in ["retry.rs", "logging.rs"] {
        fs::write(repo.join(path), "change\n")?;
    }
    git_run(repo, &["add", "retry.rs", "logging.rs"])?;
    fixture.install()?;
    fixture.prepare("both-staged", None).await?;
    let partial = git_command(repo)
        .args(["commit", "--only", "retry.rs", "-m", "retry only"])
        .output()?;
    assert!(!partial.status.success());
    assert_eq!(git_run(repo, &["show", "HEAD:retry.rs"])?, "baseline");

    let index = fixture.database.with_file_name("partial-index");
    success(
        git_command(repo)
            .env("GIT_INDEX_FILE", &index)
            .args(["read-tree", "HEAD"])
            .output()?,
    )?;
    success(
        git_command(repo)
            .env("GIT_INDEX_FILE", &index)
            .args(["add", "retry.rs"])
            .output()?,
    )?;
    fixture.prepare("actual-partial", Some(&index)).await?;
    git_run(
        repo,
        &[
            "commit",
            "--quiet",
            "--only",
            "retry.rs",
            "-m",
            "retry only",
        ],
    )?;
    let sha = git_run(repo, &["rev-parse", "HEAD"])?;
    fixture.assert_link("actual-partial", &sha).await?;
    assert_eq!(git_run(repo, &["show", "HEAD:retry.rs"])?, "change");
    assert_eq!(git_run(repo, &["show", "HEAD:logging.rs"])?, "baseline");
    assert_eq!(
        git_run(repo, &["diff", "--cached", "--name-only"])?,
        "logging.rs"
    );
    Ok(())
}

#[tokio::test]
async fn shared_worktree_hook_rejects_receipt_from_other_worktree() -> TestResult {
    let fixture = Fixture::new().await?;
    let repo = &fixture.repository;
    fs::write(repo.join("retry.rs"), "baseline\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    git_run(repo, &["commit", "--quiet", "-m", "baseline"])?;
    let other = fixture.database.with_file_name("other-worktree");
    success(
        git_command(repo)
            .args(["worktree", "add", "--quiet", "--detach"])
            .arg(&other)
            .output()?,
    )?;
    // Relative .git/hooksPath does not exist in a linked worktree, so select one shared absolute path.
    let hooks = repo.join(".git/shared-hooks");
    success(
        git_command(repo)
            .args(["config", "core.hooksPath"])
            .arg(&hooks)
            .output()?,
    )?;
    fixture.install()?;
    fs::write(repo.join("retry.rs"), "same change\n")?;
    fs::write(other.join("retry.rs"), "same change\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    git_run(&other, &["add", "retry.rs"])?;
    fixture.prepare("main-worktree", None).await?;
    assert_eq!(git::index_binding(repo)?, git::index_binding(&other)?);
    assert!(
        !git_command(&other)
            .args(["commit", "-m", "other worktree"])
            .output()?
            .status
            .success()
    );
    git_run(repo, &["commit", "--quiet", "-m", "authorized worktree"])?;
    Ok(())
}

#[tokio::test]
async fn missing_reflog_keeps_git_result_without_inventing_checkpoint_association() -> TestResult {
    let fixture = Fixture::new().await?;
    let repo = &fixture.repository;
    fs::write(repo.join("retry.rs"), "baseline\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    git_run(repo, &["commit", "--quiet", "-m", "baseline"])?;
    git_run(repo, &["config", "core.logAllRefUpdates", "false"])?;
    fs::remove_dir_all(repo.join(".git/logs"))?;
    fs::write(repo.join("retry.rs"), "change\n")?;
    git_run(repo, &["add", "retry.rs"])?;
    fixture.install()?;
    fixture.prepare("no-reflog", None).await?;
    git_run(repo, &["commit", "--quiet", "-m", "change"])?;
    let sha = git_run(repo, &["rev-parse", "HEAD"])?;
    let mut store = Store::open(&fixture.database, RedactionPolicy::default()).await?;
    let reply = store
        .checkpoint(Request::Status {
            scope: fixture.scope,
        })
        .await?;
    assert!(
        reply
            .events
            .first()
            .ok_or("missing event")?
            .commit_shas
            .is_empty()
    );
    assert!(
        store
            .latest()
            .await?
            .iter()
            .any(|entity| matches!(entity, Entity::Commit(commit) if commit.sha == sha))
    );
    Ok(())
}

#[test]
fn native_gate_preserves_legacy_rejection_and_fails_closed_on_timeout() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir()?;
    let repo = directory.path();
    git_run(repo, &["init", "--quiet"])?;
    let hooks = repo.join(".git/hooks");
    let previous = hooks.join("pre-commit");
    fs::write(
        &previous,
        "#!/bin/sh\nprintf '%s\\n' legacy-check >&2\nexit 7\n",
    )?;
    fs::set_permissions(&previous, fs::Permissions::from_mode(0o755))?;
    let binary = repo.join("slow-capture");
    fs::write(&binary, "#!/bin/sh\nexec sleep 30\n")?;
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))?;
    git::install_checkpoint_hooks_with_policy(
        repo,
        &binary,
        &repo.join("context.sqlite"),
        PROJECT,
        None,
    )?;
    let legacy = Command::new(&previous).current_dir(repo).output()?;
    assert_eq!(legacy.status.code(), Some(7));
    assert_eq!(String::from_utf8(legacy.stderr)?, "legacy-check\n");
    fs::write(
        hooks.join("pre-commit.memento-original"),
        "#!/bin/sh\nexit 0\n",
    )?;
    let start = Instant::now();
    let timeout = Command::new(&previous).current_dir(repo).output()?;
    assert!(!timeout.status.success());
    assert!(start.elapsed() < Duration::from_secs(12));
    assert!(String::from_utf8_lossy(&timeout.stderr).contains("commit blocked"));
    Ok(())
}
