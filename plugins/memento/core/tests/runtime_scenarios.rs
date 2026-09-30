use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use memento::{Store, ingest, model::*, runtime, security::RedactionPolicy};
use tempfile::TempDir;

#[path = "support/evaluation.rs"]
mod evaluation;

type TestResult = Result<(), Box<dyn Error>>;
const PROJECT: &str = "runtime-fixture";

fn git(repository: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn repository() -> Result<TempDir, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    git(
        directory.path(),
        &["init", "--quiet", "--initial-branch=main"],
    )?;
    git(directory.path(), &["config", "user.name", "Runtime Test"])?;
    git(
        directory.path(),
        &["config", "user.email", "runtime@example.invalid"],
    )?;
    git(directory.path(), &["config", "commit.gpgsign", "false"])?;
    git(
        directory.path(),
        &["config", "core.hooksPath", ".git/test-hooks"],
    )?;
    fs::write(directory.path().join("retry.rs"), "base\n")?;
    git(directory.path(), &["add", "retry.rs"])?;
    git(directory.path(), &["commit", "--quiet", "-m", "baseline"])?;
    Ok(directory)
}

async fn store(path: &Path) -> Result<Store, Box<dyn Error>> {
    let mut store = Store::open(path, RedactionPolicy::default()).await?;
    store
        .append(Entity::Source(ingest::source(
            "journal",
            PROJECT,
            SourceKind::Journal,
        )))
        .await?;
    Ok(store)
}

fn append_fixture(corpus: &mut Corpus, entity: Entity) {
    corpus.entries.push(Entry {
        sequence: corpus
            .entries
            .last()
            .map_or(1, |entry| entry.sequence.saturating_add(1)),
        captured_at: "2026-09-28T10:00:00Z".into(),
        entity,
    });
}

fn record<'a>(entities: &'a [Entity], id: &str) -> Result<&'a Record, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Record(record) if record.id == id => Some(record),
            _ => None,
        })
        .ok_or_else(|| format!("missing record {id}").into())
}

fn commit<'a>(entities: &'a [Entity], sha: &str) -> Result<&'a Commit, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::Commit(commit) if commit.sha == sha => Some(commit),
            _ => None,
        })
        .ok_or_else(|| format!("missing commit {sha}").into())
}

fn state<'a>(entities: &'a [Entity], id: &str) -> Result<&'a CodeState, Box<dyn Error>> {
    entities
        .iter()
        .find_map(|entity| match entity {
            Entity::CodeState(state) if state.id == id => Some(state),
            _ => None,
        })
        .ok_or_else(|| format!("missing state {id}").into())
}

fn file<'a>(state: &'a CodeState, path: &str) -> Result<&'a FileState, Box<dyn Error>> {
    state
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| format!("missing file {path}").into())
}

async fn run(
    store: &mut Store,
    repo: &Path,
    execution: &str,
    command: &[String],
) -> memento::Result<Vec<memento::store::Receipt>> {
    runtime::capture_run(
        store,
        runtime::RunRequest {
            project: PROJECT,
            source: "journal",
            work: "retry-work",
            session: "session-a",
            execution,
            repository: repo,
            paths: &[PathBuf::from("retry.rs")],
            command,
        },
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn dirty_run_and_partial_commit_keep_scope_and_origin_separate() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    fs::write(repo.join("retry.rs"), "retry\n")?;
    git(repo, &["add", "retry.rs"])?;
    fs::write(repo.join("retry.rs"), "retry\nlog\n")?;
    let command = vec![
        "/bin/sh".into(),
        "-c".into(),
        "test \"$(cat retry.rs)\" = \"$(printf 'retry\\nlog')\"".into(),
    ];
    let receipts = run(&mut store, repo, "dirty-test", &command).await?;
    assert!(receipts.iter().all(|receipt| receipt.durable));
    let entities = store.latest().await?;
    let result = record(&entities, "dirty-test:result")?;
    let execution = result.execution.as_ref().ok_or("missing execution")?;
    assert_eq!(execution.exit_code, Some(0));
    assert_eq!(execution.liveness, Liveness::Stopped);
    let before = state(
        &entities,
        execution
            .before_state
            .as_deref()
            .ok_or("missing before state")?,
    )?;
    let observed = file(before, "retry.rs")?;
    assert_eq!(
        observed
            .status
            .as_ref()
            .ok_or("missing dirty status")?
            .index,
        FileChange::Modified
    );
    assert_eq!(
        observed
            .status
            .as_ref()
            .ok_or("missing dirty status")?
            .working,
        FileChange::Modified
    );
    assert_eq!(observed.working_kind, WorkingFileKind::File);
    assert_eq!(observed.working_content.as_deref(), Some("retry\nlog\n"));
    assert_eq!(observed.index_entries.len(), 1);
    assert_ne!(observed.head_hash, observed.index_hash);
    let before_commit = before.commit_sha.clone();
    let mut trace = Query::new(Operation::Trace, PROJECT);
    trace.scope.work_ids = vec!["retry-work".into()];
    trace.target = Some(Target::Code {
        state_id: before.id.clone(),
        path: "retry.rs".into(),
        range: None,
    });
    let response = memento::query::execute(&store.load().await?, &trace)?;
    assert!(
        response
            .items
            .iter()
            .any(|item| item.entity.id() == "dirty-test:attempt")
    );
    assert!(
        response
            .items
            .iter()
            .any(|item| item.entity.id() == "dirty-test:result")
    );
    assert!(
        response
            .relations
            .iter()
            .all(|relation| relation.kind != RelationKind::Verifies)
    );
    git(repo, &["commit", "--quiet", "-m", "retry hunk only"])?;
    let sha = git(repo, &["rev-parse", "HEAD"])?;
    runtime::hook(&mut store, PROJECT, repo, "post-commit", None, "").await?;
    runtime::git_sync(&mut store, PROJECT, repo, &["HEAD".into()], 10).await?;
    let after = runtime::observe(&mut store, PROJECT, repo, &["retry.rs".into()]).await?;
    let current = file(&after, "retry.rs")?;
    assert_eq!(
        current
            .status
            .as_ref()
            .ok_or("missing remaining change")?
            .index,
        FileChange::Unmodified
    );
    assert_eq!(
        current
            .status
            .as_ref()
            .ok_or("missing remaining change")?
            .working,
        FileChange::Modified
    );
    assert_eq!(current.working_hash, observed.working_hash);
    assert_ne!(before_commit.as_deref(), Some(sha.as_str()));
    assert_eq!(git(repo, &["show", "HEAD:retry.rs"])?, "retry");
    let entities = store.latest().await?;
    let committed = commit(&entities, &sha)?;
    let expected_origin = repo.canonicalize()?.to_string_lossy().into_owned();
    assert_eq!(
        committed.origin_worktree.as_deref(),
        Some(expected_origin.as_str())
    );
    assert!(entities.iter().all(|entity| !matches!(entity, Entity::Relation(relation) if relation.kind == RelationKind::Verifies)));
    assert!(
        record(&entities, "dirty-test:result")?
            .commit_shas
            .is_empty()
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn same_command_retry_is_distinct_but_reusing_execution_id_is_rejected() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    let command = vec![
        "/bin/sh".into(),
        "-c".into(),
        "test -f dependency-ready".into(),
    ];
    run(&mut store, repo, "attempt-one", &command).await?;
    assert!(
        run(&mut store, repo, "attempt-one", &command)
            .await
            .is_err()
    );
    fs::write(repo.join("dependency-ready"), "available")?;
    run(&mut store, repo, "attempt-two", &command).await?;
    let entities = store.latest().await?;
    let failed = record(&entities, "attempt-one:attempt")?;
    let passed = record(&entities, "attempt-two:attempt")?;
    assert_eq!(failed.attempt_outcome, Some(AttemptOutcome::Failed));
    assert_eq!(passed.attempt_outcome, Some(AttemptOutcome::Succeeded));
    let old_execution = failed.execution.as_ref().ok_or("missing old execution")?;
    let new_execution = passed.execution.as_ref().ok_or("missing new execution")?;
    assert_eq!(old_execution.command, new_execution.command);
    assert_ne!(old_execution.id, new_execution.id);
    assert_eq!(old_execution.exit_code, Some(1));
    assert_eq!(new_execution.exit_code, Some(0));
    let old = state(
        &entities,
        old_execution
            .before_state
            .as_deref()
            .ok_or("missing old state")?,
    )?;
    let new = state(
        &entities,
        new_execution
            .before_state
            .as_deref()
            .ok_or("missing new state")?,
    )?;
    assert_eq!(
        file(old, "retry.rs")?.working_hash,
        file(new, "retry.rs")?.working_hash
    );
    assert!(failed.verification_outcome.is_none());
    assert!(passed.verification_outcome.is_none());
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn rewrite_replay_keeps_old_commit_origin_and_one_mapping() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    let old = git(repo, &["rev-parse", "HEAD"])?;
    runtime::hook(&mut store, PROJECT, repo, "post-commit", None, "").await?;
    fs::write(repo.join("retry.rs"), "amended state\n")?;
    git(repo, &["add", "retry.rs"])?;
    git(repo, &["commit", "--quiet", "--amend", "--no-edit"])?;
    let new = git(repo, &["rev-parse", "HEAD"])?;
    let mapping = format!("{old} {new}\n");
    runtime::hook(
        &mut store,
        PROJECT,
        repo,
        "post-rewrite",
        Some("amend"),
        &mapping,
    )
    .await?;
    let replay = runtime::hook(
        &mut store,
        PROJECT,
        repo,
        "post-rewrite",
        Some("amend"),
        &mapping,
    )
    .await?;
    assert!(replay.iter().all(|receipt| receipt.duplicate));
    let entities = store.latest().await?;
    assert_eq!(entities.iter().filter(|entity| matches!(entity, Entity::Relation(relation) if relation.kind == RelationKind::DerivedFrom)).count(), 1);
    assert_eq!(commit(&entities, &old)?.sha, old);
    assert!(commit(&entities, &old)?.origin_worktree.is_some());
    assert_eq!(commit(&entities, &new)?.sha, new);
    assert!(commit(&entities, &new)?.origin_worktree.is_none());
    assert!(entities.iter().all(|entity| !matches!(entity, Entity::Record(record) if record.kind == RecordKind::Verification)));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn release_conflict_and_revert_do_not_erase_main_or_current_state() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    let release = data.path().join("release");
    git(
        repo,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "release",
            release.to_str().ok_or("non UTF-8 fixture")?,
        ],
    )?;
    // Relative hooksPath points into the per-worktree .git file; disable it with an
    // existing shared directory in this temporary fixture before making commits.
    let no_hooks = data.path().join("no-hooks");
    fs::create_dir(&no_hooks)?;
    git(
        repo,
        &[
            "config",
            "core.hooksPath",
            no_hooks.to_str().ok_or("non UTF-8 fixture")?,
        ],
    )?;
    fs::write(repo.join("retry.rs"), "new API behavior\n")?;
    git(repo, &["add", "retry.rs"])?;
    git(repo, &["commit", "--quiet", "-m", "retry improvement"])?;
    let main_sha = git(repo, &["rev-parse", "HEAD"])?;
    runtime::hook(&mut store, PROJECT, repo, "post-commit", None, "").await?;
    fs::write(release.join("retry.rs"), "legacy API behavior\n")?;
    git(&release, &["add", "retry.rs"])?;
    git(&release, &["commit", "--quiet", "-m", "release API"])?;
    let picked = Command::new("git")
        .arg("-C")
        .arg(&release)
        .args(["cherry-pick", &main_sha])
        .output()?;
    assert!(!picked.status.success());
    let conflict = runtime::observe(&mut store, PROJECT, &release, &["retry.rs".into()]).await?;
    let conflict_file = file(&conflict, "retry.rs")?;
    assert_eq!(
        conflict_file
            .index_entries
            .iter()
            .map(|entry| entry.stage)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(conflict_file.index_hash.is_none());
    assert_eq!(
        conflict_file
            .status
            .as_ref()
            .ok_or("missing conflict status")?
            .index,
        FileChange::Unmerged
    );
    assert_eq!(
        conflict_file
            .status
            .as_ref()
            .ok_or("missing conflict status")?
            .working,
        FileChange::Unmerged
    );
    fs::write(release.join("retry.rs"), "release compatible improvement\n")?;
    git(&release, &["add", "retry.rs"])?;
    git(
        &release,
        &["-c", "core.editor=true", "cherry-pick", "--continue"],
    )?;
    let picked_sha = git(&release, &["rev-parse", "HEAD"])?;
    runtime::hook(&mut store, PROJECT, &release, "post-commit", None, "").await?;
    let picked_state =
        runtime::observe(&mut store, PROJECT, &release, &["retry.rs".into()]).await?;
    git(&release, &["revert", "--no-edit", &picked_sha])?;
    runtime::hook(&mut store, PROJECT, &release, "post-commit", None, "").await?;
    let release_state =
        runtime::observe(&mut store, PROJECT, &release, &["retry.rs".into()]).await?;
    let main_state = runtime::observe(&mut store, PROJECT, repo, &["retry.rs".into()]).await?;
    git(
        &release,
        &["merge-base", "--is-ancestor", &picked_sha, "HEAD"],
    )?;
    assert_ne!(
        file(&picked_state, "retry.rs")?.working_hash,
        file(&release_state, "retry.rs")?.working_hash
    );
    assert_ne!(main_state.worktree_id, release_state.worktree_id);
    assert_eq!(main_state.commit_sha.as_deref(), Some(main_sha.as_str()));
    assert_eq!(
        fs::read_to_string(repo.join("retry.rs"))?,
        "new API behavior\n"
    );
    assert_eq!(
        fs::read_to_string(release.join("retry.rs"))?,
        "legacy API behavior\n"
    );
    let entities = store.latest().await?;
    assert_ne!(
        commit(&entities, &main_sha)?.origin_worktree,
        commit(&entities, &picked_sha)?.origin_worktree
    );
    // Git results alone supply neither the conflict-resolution rationale nor a
    // product decision to cancel the original improvement.
    assert!(entities.iter().all(
        |entity| !matches!(entity, Entity::Record(record) if record.kind == RecordKind::Decision)
    ));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn observe_retains_ignored_missing_and_untracked_distinctions() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    fs::write(repo.join(".gitignore"), ".env\n")?;
    fs::write(repo.join(".env"), "SECRET=not-collected")?;
    fs::write(repo.join("untracked"), "new")?;
    let observed = runtime::observe(
        &mut store,
        PROJECT,
        repo,
        &[".env".into(), "absent".into(), "untracked".into()],
    )
    .await?;
    assert_eq!(
        file(&observed, ".env")?.working_kind,
        WorkingFileKind::Ignored
    );
    assert!(file(&observed, ".env")?.working_hash.is_none());
    assert_eq!(
        file(&observed, "absent")?.working_kind,
        WorkingFileKind::Missing
    );
    assert_eq!(
        file(&observed, "untracked")?.working_kind,
        WorkingFileKind::File
    );
    assert_eq!(
        file(&observed, "untracked")?
            .status
            .as_ref()
            .ok_or("missing untracked status")?
            .working,
        FileChange::Untracked
    );
    assert!(file(&observed, ".env")?.working_content.is_none());
    assert!(file(&observed, "absent")?.working_content.is_none());
    assert_eq!(
        file(&observed, "untracked")?.working_content.as_deref(),
        Some("new")
    );
    let entities = store.latest().await?;
    assert_eq!(state(&entities, &observed.id)?.files, observed.files);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn observed_code_is_masked_in_returned_state_and_retained_original() -> TestResult {
    let directory = repository()?;
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    fs::write(
        directory.path().join("retry.rs"),
        "password=code-secret-example\npublic setting\n",
    )?;
    let observed =
        runtime::observe(&mut store, PROJECT, directory.path(), &["retry.rs".into()]).await?;
    assert!(!serde_json::to_string(&observed)?.contains("code-secret-example"));
    let mut query = Query::new(Operation::Read, PROJECT);
    query.target = Some(Target::Code {
        state_id: observed.id,
        path: "retry.rs".into(),
        range: None,
    });
    let response = memento::query::execute(&store.load().await?, &query)?;
    let encoded = serde_json::to_string(&response)?;
    assert!(!encoded.contains("code-secret-example"));
    assert!(encoded.contains("[REDACTED]"));
    assert_eq!(
        fs::read_to_string(directory.path().join("retry.rs"))?,
        "password=code-secret-example\npublic setting\n"
    );
    Ok(())
}

#[cfg(unix)]
struct InterruptedRun {
    process: Child,
    child_pid_file: PathBuf,
}

#[cfg(unix)]
impl Drop for InterruptedRun {
    fn drop(&mut self) {
        let _kill = self.process.kill();
        let _wait = self.process.wait();
        if let Ok(pid) = fs::read_to_string(&self.child_pid_file)
            && let Ok(pid) = pid.trim().parse::<u32>()
        {
            let _kill = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn killed_cli_retains_durable_running_attempt_and_current_dirty_file() -> TestResult {
    let directory = repository()?;
    let repo = directory.path();
    let data = tempfile::tempdir()?;
    let database = data.path().join("context.sqlite");
    let mut initial = store(&database).await?;
    let mut decision = Record::new(
        "D2",
        PROJECT,
        "journal",
        RecordKind::Decision,
        "Apply the retry patch as an uncommitted experiment, then check the command output before declaring it verified.",
    );
    decision.nature = Nature::Reported;
    decision.decision_status = Some(DecisionStatus::Accepted);
    decision.work_ids = vec!["retry-work".into()];
    decision.association = Association::Explicit;
    decision.session_id = Some("session-a".into());
    initial.append(Entity::Record(decision)).await?;
    drop(initial);
    let child = Command::new(env!("CARGO_BIN_EXE_memento"))
        .arg("run").arg("--store").arg(&database).args(["--project", PROJECT, "--work", "retry-work", "--session", "session-a", "--id", "interrupted", "--repository"])
        .arg(repo).args(["--path", "retry.rs", "--", "/bin/sh", "-c", "printf 'unfinished patch\\n' > retry.rs; printf 'patch written; command still waiting\\n' > progress-output; printf '%s' \"$$\" > running-pid; exec sleep 30"])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    let pid_file = repo.join("running-pid");
    let mut process = InterruptedRun {
        process: child,
        child_pid_file: pid_file.clone(),
    };
    let start = Instant::now();
    while !pid_file.exists() && start.elapsed() < Duration::from_secs(10) {
        if let Some(status) = process.process.try_wait()? {
            return Err(format!("capture CLI exited before command started: {status}").into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(pid_file.exists(), "command never started");
    process.process.kill()?;
    process.process.wait()?;
    drop(process);
    let mut reopened = Store::open(&database, RedactionPolicy::default()).await?;
    let mut partial_output = Record::new(
        "E7",
        PROJECT,
        "journal",
        RecordKind::ToolResult,
        &fs::read_to_string(repo.join("progress-output"))?,
    );
    partial_output.work_ids = vec!["retry-work".into()];
    partial_output.association = Association::Explicit;
    partial_output.session_id = Some("session-a".into());
    partial_output.attempt_id = Some("interrupted:attempt".into());
    partial_output.partial = true;
    let output_revision = partial_output.revision.clone();
    reopened.append(Entity::Record(partial_output)).await?;
    let entities = reopened.latest().await?;
    let attempt = record(&entities, "interrupted:attempt")?;
    assert!(attempt.partial);
    assert_eq!(attempt.attempt_outcome, Some(AttemptOutcome::Running));
    let execution = attempt
        .execution
        .as_ref()
        .ok_or("missing durable execution")?;
    assert_eq!(execution.liveness, Liveness::Unknown);
    assert!(execution.ended_at.is_none());
    assert!(execution.exit_code.is_none());
    assert!(record(&entities, "interrupted:result").is_err());
    let before = state(
        &entities,
        execution
            .before_state
            .as_deref()
            .ok_or("missing durable before state")?,
    )?;
    let current = runtime::observe(&mut reopened, PROJECT, repo, &["retry.rs".into()]).await?;
    assert_ne!(
        file(before, "retry.rs")?.working_hash,
        file(&current, "retry.rs")?.working_hash
    );
    assert_eq!(
        file(&current, "retry.rs")?
            .status
            .as_ref()
            .ok_or("missing dirty status")?
            .working,
        FileChange::Modified
    );
    let corpus = reopened.load().await?;
    let mut attempt_query = Query::new(Operation::Read, PROJECT);
    attempt_query.target = Some(Target::Record {
        id: "interrupted:attempt".into(),
    });
    let mut current_query = Query::new(Operation::Read, PROJECT);
    current_query.target = Some(Target::Code {
        state_id: current.id.clone(),
        path: "retry.rs".into(),
        range: None,
    });
    let mut resume_query = Query::new(Operation::Brief, PROJECT);
    resume_query.target = Some(Target::Work {
        id: "retry-work".into(),
    });
    resume_query.purpose = Some(BriefPurpose::Resume);
    evaluation::capture(
        "SC02",
        "A",
        "앱이 종료되기 전에 retry-work를 어디까지 했어? 남은 패치, 마지막 저장 지점과 테스트 상태를 확인하고 바로 이어서 할 수 있는지 설명해줘.",
        &corpus,
        &[
            Query::new(Operation::Sources, PROJECT),
            attempt_query,
            current_query.clone(),
            resume_query.clone(),
        ],
    )?;

    // The alternate retained dataset has no original process record or old code state.
    let mut summary_only = Corpus {
        compaction: corpus.compaction.clone(),
        entries: corpus
            .entries
            .iter()
            .filter(|entry| {
                matches!(&entry.entity, Entity::Source(_))
                    || matches!(&entry.entity, Entity::CodeState(state) if state.id == current.id)
            })
            .cloned()
            .collect(),
    };
    let mut summary = Record::new(
        "compacted-status",
        PROJECT,
        "journal",
        RecordKind::Status,
        "The previous session reportedly worked on a retry patch and had started a command before the app closed. The original process messages are no longer retained.",
    );
    summary.work_ids = vec!["retry-work".into()];
    summary.association = Association::Explicit;
    summary.nature = Nature::Reported;
    summary.fidelity = Fidelity::SummaryOnly;
    summary.partial = true;
    summary.evidence = vec![Evidence {
        source_id: "journal".into(),
        record_id: Some("E7".into()),
        revision: output_revision,
        locator: "execution:interrupted:partial-output".into(),
        availability: Availability::Missing,
        range: None,
    }];
    append_fixture(&mut summary_only, Entity::Record(summary));
    let limited = memento::query::execute(&summary_only, &resume_query)?;
    assert_eq!(limited.status, memento::query::ResponseStatus::Partial);
    assert!(limited.items.iter().all(|item| {
        !matches!(&item.entity, Entity::Record(record) if record.execution.is_some())
    }));
    evaluation::capture(
        "SC02",
        "B",
        "앱이 종료되기 전에 retry-work를 어디까지 했어? 남은 패치, 마지막 저장 지점과 테스트 상태를 확인하고 바로 이어서 할 수 있는지 설명해줘.",
        &summary_only,
        &[
            Query::new(Operation::Sources, PROJECT),
            current_query,
            resume_query,
        ],
    )?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn background_git_capture_cannot_reenable_revoked_source() -> TestResult {
    let directory = repository()?;
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    runtime::observe(&mut store, PROJECT, directory.path(), &["retry.rs".into()]).await?;
    let mut source = ingest::source("git", PROJECT, SourceKind::Git);
    source.authorized = false;
    store.append(Entity::Source(source)).await?;
    assert!(
        runtime::hook(
            &mut store,
            PROJECT,
            directory.path(),
            "post-commit",
            None,
            ""
        )
        .await
        .is_err()
    );
    assert!(
        runtime::git_sync(&mut store, PROJECT, directory.path(), &["HEAD".into()], 20)
            .await
            .is_err()
    );
    assert!(store.latest().await?.iter().any(
        |entity| matches!(entity,Entity::Source(source) if source.id=="git" && !source.authorized)
    ));
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn failed_post_commit_capture_recovers_results_and_only_retained_process() -> TestResult {
    use memento::query::{ResponseStatus, execute};
    use std::os::unix::fs::PermissionsExt;

    let directory = repository()?;
    let repo = directory.path();
    let baseline = git(repo, &["rev-parse", "HEAD"])?;
    let data = tempfile::tempdir()?;
    let mut store = store(&data.path().join("context.sqlite")).await?;
    let hooks = repo.join(".git/test-hooks");
    fs::create_dir_all(&hooks)?;
    let previous_hook = hooks.join("post-commit");
    let previous_script = "#!/bin/sh\nprintf '%s\\n' 'legacy hook invoked' >&2\nprintf invoked > .git/legacy-hook-ran\n";
    fs::write(&previous_hook, previous_script)?;
    fs::set_permissions(&previous_hook, fs::Permissions::from_mode(0o755))?;
    // A regular file as the parent directory makes the actual hook store fail.
    let blocked_parent = data.path().join("not-a-directory");
    fs::write(&blocked_parent, "fixture")?;
    let installation = memento::git::install_hooks(
        repo,
        Path::new(env!("CARGO_BIN_EXE_memento")),
        &blocked_parent.join("context.sqlite"),
        PROJECT,
    )?;
    assert!(installation.hooks.iter().all(|hook| hook.installed));
    assert_eq!(
        fs::read_to_string(hooks.join("post-commit.memento-original"))?,
        previous_script
    );
    assert_eq!(
        git(repo, &["config", "--get", "core.hooksPath"])?,
        ".git/test-hooks"
    );
    store
        .append(Entity::Record(Record::new(
            "hook-installation",
            PROJECT,
            "journal",
            RecordKind::ToolResult,
            &serde_json::to_string(&installation)?,
        )))
        .await?;
    let mut decision = Record::new(
        "C11-decision",
        PROJECT,
        "journal",
        RecordKind::Decision,
        "Use backoff after errors so retries do not repeatedly hit the same failing service at a fixed interval.",
    );
    decision.nature = Nature::Reported;
    decision.decision_status = Some(DecisionStatus::Accepted);
    decision.work_ids = vec!["retry-work".into()];
    decision.association = Association::Explicit;
    decision.session_id = Some("session-a".into());
    store.append(Entity::Record(decision.clone())).await?;
    fs::write(repo.join("retry.rs"), "retry with backoff\n")?;
    git(repo, &["add", "retry.rs"])?;
    run(
        &mut store,
        repo,
        "C11-commit",
        &[
            "git".into(),
            "commit".into(),
            "--quiet".into(),
            "-m".into(),
            "retry backoff".into(),
        ],
    )
    .await?;
    let c11 = git(repo, &["rev-parse", "HEAD"])?;
    let before_recovery = store.latest().await?;
    assert!(commit(&before_recovery, &c11).is_err());
    let output = record(&before_recovery, "C11-commit:result")?;
    assert_eq!(
        output
            .execution
            .as_ref()
            .ok_or("missing execution")?
            .exit_code,
        Some(0)
    );
    assert!(output.body.contains("legacy hook invoked"));
    assert!(
        output
            .body
            .contains("local context capture failed or timed out")
    );
    assert_eq!(
        fs::read_to_string(repo.join(".git/legacy-hook-ran"))?,
        "invoked"
    );

    for (id, duplicate) in [("recovery-first", false), ("recovery-second", true)] {
        let result = runtime::git_sync(&mut store, PROJECT, repo, &["HEAD".into()], 20).await?;
        let receipts: Vec<memento::store::Receipt> =
            serde_json::from_value(result.get("receipts").ok_or("missing receipts")?.clone())?;
        assert_eq!(receipts.len(), 2);
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.duplicate == duplicate)
        );
        store
            .append(Entity::Record(Record::new(
                id,
                PROJECT,
                "journal",
                RecordKind::ToolResult,
                &serde_json::to_string(&result)?,
            )))
            .await?;
    }
    decision.commit_shas = vec![c11.clone()];
    store.append(Entity::Record(decision.clone())).await?;
    let commit_target = Target::Commit {
        repository_id: PROJECT.into(),
        commit_sha: c11.clone(),
    };
    let link = Entity::Relation(Relation {
        id: "C11-process-link".into(),
        project_id: PROJECT.into(),
        source_id: "journal".into(),
        from: Target::Record {
            id: decision.id.clone(),
        },
        to: commit_target.clone(),
        kind: RelationKind::Supports,
        nature: Nature::Reported,
        evidence: vec![Evidence {
            source_id: "journal".into(),
            record_id: Some(decision.id.clone()),
            revision: decision.revision.clone(),
            locator: "session-a:C11-decision".into(),
            availability: Availability::Available,
            range: None,
        }],
        applies_to: Vec::new(),
    });
    assert!(!store.append(link.clone()).await?.duplicate);
    assert!(store.append(link).await?.duplicate);
    let entities = store.latest().await?;
    assert!(commit(&entities, &baseline)?.origin_worktree.is_none());
    assert!(commit(&entities, &c11)?.origin_worktree.is_none());
    let corpus = store.load().await?;
    assert_eq!(
        corpus
            .entries
            .iter()
            .filter(|entry| matches!(&entry.entity, Entity::Commit(commit) if commit.sha == c11))
            .count(),
        1
    );
    let mut baseline_query = Query::new(Operation::Read, PROJECT);
    baseline_query.target = Some(Target::Commit {
        repository_id: PROJECT.into(),
        commit_sha: baseline,
    });
    let mut commit_query = Query::new(Operation::Read, PROJECT);
    commit_query.target = Some(commit_target.clone());
    let mut brief_query = Query::new(Operation::Brief, PROJECT);
    brief_query.target = Some(commit_target);
    let mut timeline = Query::new(Operation::Timeline, PROJECT);
    timeline.scope.source_ids = vec!["journal".into()];
    let queries = vec![
        Query::new(Operation::Sources, PROJECT),
        baseline_query,
        commit_query,
        brief_query.clone(),
    ];
    let mut full_queries = queries.clone();
    full_queries.push(timeline);
    let brief = execute(&corpus, &brief_query)?;
    assert!(
        brief
            .items
            .iter()
            .any(|item| item.entity.id() == "C11-decision")
    );
    evaluation::capture(
        "SC06",
        "A",
        "최근 두 커밋의 과정이 다 기록됐어? 빠진 것만 보완하는 동기화 후 결과를 알려줘. 기존 훅과 Git 결과, 회수 가능한 과정, 재전달 중복 여부를 확인해줘.",
        &corpus,
        &full_queries,
    )?;

    let results_only = Corpus {
        compaction: corpus.compaction.clone(),
        entries: corpus
            .entries
            .iter()
            .filter(|entry| {
                matches!(&entry.entity, Entity::Source(source) if source.id == "git")
                    || matches!(&entry.entity, Entity::Commit(_))
            })
            .cloned()
            .collect(),
    };
    let missing = execute(&results_only, &brief_query)?;
    assert!(missing.items.is_empty());
    assert_eq!(missing.status, ResponseStatus::Partial);
    assert!(missing.coverage.iter().all(|coverage| coverage.result_only));
    evaluation::capture(
        "SC06",
        "B",
        "최근 두 커밋의 과정이 다 기록됐어? 빠진 것만 보완하는 동기화 후 결과를 알려줘. 기존 훅과 Git 결과, 회수 가능한 과정, 재전달 중복 여부를 확인해줘.",
        &results_only,
        &queries,
    )?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn installed_hook_applies_the_selected_custom_masking_policy() -> TestResult {
    let directory = repository()?;
    let data = tempfile::tempdir()?;
    let path = data.path().join("context.sqlite");
    let mut store = store(&path).await?;
    let policy = data.path().join("masking policy.json");
    fs::write(
        &policy,
        r#"{"literal_secrets":["CUSTOM_TEST_HOOK_SECRET"]}"#,
    )?;
    memento::git::install_hooks_with_policy(
        directory.path(),
        Path::new(env!("CARGO_BIN_EXE_memento")),
        &path,
        PROJECT,
        Some(&policy),
    )?;
    git(
        directory.path(),
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "result CUSTOM_TEST_HOOK_SECRET",
        ],
    )?;
    let corpus = store.load().await?;
    let json = serde_json::to_string(&corpus)?;
    assert!(!json.contains("CUSTOM_TEST_HOOK_SECRET"));
    assert!(json.contains("[REDACTED]"));
    assert!(
        git(
            directory.path(),
            &["show", "--no-patch", "--format=%B", "HEAD"]
        )?
        .contains("CUSTOM_TEST_HOOK_SECRET")
    );
    Ok(())
}
