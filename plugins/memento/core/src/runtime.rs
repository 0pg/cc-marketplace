use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    git, ingest,
    model::*,
    security::hash,
    store::{Error, Receipt, Result, Store},
};
use chrono::Utc;

fn git_error(error: git::GitError) -> Error {
    Error::Invalid(error.to_string())
}

pub async fn register_git(store: &mut Store, project: &str, repository: &Path) -> Result<()> {
    if store.latest().await?.iter().any(|entity| matches!(entity, Entity::Source(s) if s.project_id == project && s.id == "git" && !s.authorized)) {
        return Err(Error::Invalid("Git source access was revoked; explicitly restore source access before collection".into()));
    }
    let mut src = ingest::source("git", project, SourceKind::Git);
    src.location = Some(repository.canonicalize()?.to_string_lossy().into_owned());
    src.record_kinds = vec![RecordKind::GitEvent];
    src.result_only = true;
    src.last_captured_at = Some(Utc::now().to_rfc3339());
    let status = git::hook_status(repository).map_err(git_error)?;
    src.gaps
        .push(format!("hooks: {}", serde_json::to_string(&status)?));
    src.gaps
        .push("Git records results; process context requires a journal/transcript source".into());
    store.append(Entity::Source(src)).await?;
    Ok(())
}

pub fn commit_entity(project: &str, commit: git::GitCommit, origin: bool) -> Entity {
    let observed = commit.observed_worktree.to_string_lossy().into_owned();
    Entity::Commit(Commit {
        id: format!("git:{}", commit.sha),
        project_id: project.into(),
        source_id: "git".into(),
        repository_id: project.into(),
        sha: commit.sha,
        tree: commit.tree,
        parents: commit.parents,
        paths: commit.changed_paths,
        message: commit.message,
        occurred_at: Some(commit.committed_at),
        origin_worktree: origin.then(|| observed.clone()),
        observed_worktree: Some(observed),
    })
}

pub async fn git_sync(
    store: &mut Store,
    project: &str,
    repository: &Path,
    refs: &[String],
    limit: usize,
) -> Result<serde_json::Value> {
    let report = git::reconcile(repository, refs, limit).map_err(git_error)?;
    register_git(store, project, repository).await?;
    let truncated = report.truncated;
    let resolved = report.resolved_refs;
    let receipts = store
        .append_all(
            report
                .commits
                .into_iter()
                .map(|c| commit_entity(project, c, false)),
        )
        .await?;
    Ok(
        serde_json::json!({"receipts": receipts, "truncated": truncated, "resolved_refs": resolved, "coverage": "Git result checkpoints only; no missing conversations or rewrite mapping are inferred"}),
    )
}

pub async fn hook(
    store: &mut Store,
    project: &str,
    repository: &Path,
    kind: &str,
    rewrite_command: Option<&str>,
    input: &str,
) -> Result<Vec<Receipt>> {
    register_git(store, project, repository).await?;
    match kind {
        "pre-commit" => {
            let repository = git::worktree_root(repository).map_err(git_error)?;
            let binding = git::index_binding(&repository).map_err(git_error)?;
            store.check_commit(project, &repository, &binding).await?;
            Ok(Vec::new())
        }
        "post-commit" => {
            let commit = git::read_commit(repository, "HEAD").map_err(git_error)?;
            let parent_head = git::preceding_head(repository).map_err(git_error);
            let binding = match parent_head {
                Ok(Some(parent_head)) => Some(git::IndexBinding {
                    parent_head: Some(parent_head),
                    staged_tree: commit.tree.clone(),
                }),
                Ok(None) if commit.parents.is_empty() => Some(git::IndexBinding {
                    parent_head: None,
                    staged_tree: commit.tree.clone(),
                }),
                Ok(None) => None,
                Err(error) => {
                    tracing::warn!(%error, "commit checkpoint parent was not observed");
                    None
                }
            };
            let sha = commit.sha.clone();
            let receipts = store
                .append_all([commit_entity(project, commit, true)])
                .await?;
            // Git success and checkpoint association are separate outcomes.
            if let Some(binding) = binding {
                match store.link_commit(project, repository, &binding, &sha).await {
                    Ok(()) | Err(Error::Capture(crate::capture::Error::CommitNotReady)) => {}
                    Err(error) => {
                        tracing::warn!(%error, commit_sha = %sha, "commit captured; checkpoint link failed");
                    }
                }
            }
            Ok(receipts)
        }
        "post-rewrite" => {
            let mappings = git::parse_rewrites(
                rewrite_command
                    .ok_or_else(|| Error::Invalid("rewrite command is required".into()))?,
                input,
            )
            .map_err(git_error)?;
            let mut receipts = Vec::new();
            for mapping in mappings {
                for sha in [&mapping.old_sha, &mapping.new_sha] {
                    let commit = git::read_commit(repository, sha).map_err(git_error)?;
                    receipts.push(store.append(commit_entity(project, commit, false)).await?);
                }
                let mut record = Record::new(
                    &format!("rewrite:{}:{}", mapping.old_sha, mapping.new_sha),
                    project,
                    "git",
                    RecordKind::GitEvent,
                    &format!(
                        "{} maps {} to {}",
                        mapping.command, mapping.old_sha, mapping.new_sha
                    ),
                );
                record.commit_shas = vec![mapping.old_sha.clone(), mapping.new_sha.clone()];
                let evidence = Evidence {
                    source_id: "git".into(),
                    record_id: Some(record.id.clone()),
                    revision: record.revision.clone(),
                    locator: format!("git-hook:{}", mapping.command),
                    availability: Availability::Available,
                    range: None,
                };
                let link = Relation {
                    id: format!("derived:{}:{}", mapping.new_sha, mapping.old_sha),
                    project_id: project.into(),
                    source_id: "git".into(),
                    from: Target::Commit {
                        repository_id: project.into(),
                        commit_sha: mapping.new_sha,
                    },
                    to: Target::Commit {
                        repository_id: project.into(),
                        commit_sha: mapping.old_sha,
                    },
                    kind: RelationKind::DerivedFrom,
                    nature: Nature::Observed,
                    evidence: vec![evidence],
                    applies_to: Vec::new(),
                };
                receipts.push(store.append(Entity::Record(record)).await?);
                receipts.push(store.append(Entity::Relation(link)).await?);
            }
            Ok(receipts)
        }
        _ => Err(Error::Invalid("unsupported hook".into())),
    }
}

pub async fn observe(
    store: &mut Store,
    project: &str,
    repository: &Path,
    paths: &[PathBuf],
) -> Result<CodeState> {
    let snapshot = git::snapshot(repository, paths).map_err(git_error)?;
    register_git(store, project, repository).await?;
    let id = format!("state:{}", hash(&serde_json::to_vec(&snapshot)?));
    let mut changed_during_observation = snapshot.changed_during_observation;
    let files = snapshot
        .paths
        .into_iter()
        .map(|p| {
            let status = snapshot
                .status
                .iter()
                .find(|entry| entry.path == p.path)
                .map(|entry| FileStatus {
                    index: file_change(&entry.index_status),
                    working: file_change(&entry.working_status),
                    original_path: entry.original_path.clone(),
                });
            let working_kind = match p.working.kind {
                git::WorkingKind::File => WorkingFileKind::File,
                git::WorkingKind::Symlink => WorkingFileKind::Symlink,
                git::WorkingKind::Directory => WorkingFileKind::Directory,
                git::WorkingKind::Missing => WorkingFileKind::Missing,
                git::WorkingKind::Ignored => WorkingFileKind::Ignored,
            };
            // Only the selected regular file is captured, and only if it still
            // matches the observed content. Symlinks/ignored files are metadata.
            let working_content = if working_kind == WorkingFileKind::File {
                match std::fs::read(snapshot.worktree.join(&p.path)) {
                    Ok(bytes) if p.working.sha256.as_deref() == Some(hash(&bytes).as_str()) => {
                        String::from_utf8(bytes).ok()
                    }
                    _ => {
                        changed_during_observation = true;
                        None
                    }
                }
            } else {
                None
            };
            FileState {
                path: p.path,
                head_hash: p.head.map(|x| x.object_id),
                index_hash: p
                    .index
                    .iter()
                    .find(|x| x.stage == 0)
                    .map(|x| x.object_id.clone()),
                working_hash: p.working.sha256,
                working_content,
                working_kind,
                status,
                index_entries: p
                    .index
                    .into_iter()
                    .map(|entry| IndexEntry {
                        mode: entry.mode,
                        object_id: entry.object_id,
                        stage: entry.stage,
                    })
                    .collect(),
            }
        })
        .collect();
    let state = CodeState {
        id,
        project_id: project.into(),
        source_id: "git".into(),
        repository_id: project.into(),
        worktree_id: Some(snapshot.worktree_git_dir.to_string_lossy().into_owned()),
        commit_sha: snapshot.head,
        observed_at: snapshot.observed_at,
        changed_during_observation,
        files,
    };
    let Entity::CodeState(state) = store.sanitize(&Entity::CodeState(state))? else {
        return Err(Error::Invalid("invalid sanitized code state".into()));
    };
    store.append(Entity::CodeState(state.clone())).await?;
    Ok(state)
}

fn file_change(status: &str) -> FileChange {
    match status {
        " " => FileChange::Unmodified,
        "M" => FileChange::Modified,
        "T" => FileChange::TypeChanged,
        "A" => FileChange::Added,
        "D" => FileChange::Deleted,
        "R" => FileChange::Renamed,
        "C" => FileChange::Copied,
        "U" => FileChange::Unmerged,
        "?" => FileChange::Untracked,
        "!" => FileChange::Ignored,
        _ => FileChange::Unknown,
    }
}

pub struct RunRequest<'a> {
    pub project: &'a str,
    pub source: &'a str,
    pub work: &'a str,
    pub session: &'a str,
    pub execution: &'a str,
    pub repository: &'a Path,
    pub paths: &'a [PathBuf],
    pub command: &'a [String],
}

/// Capture a requested command, not a command retrieved from historical records.
pub async fn capture_run(store: &mut Store, request: RunRequest<'_>) -> Result<Vec<Receipt>> {
    let executable = request
        .command
        .first()
        .ok_or_else(|| Error::Invalid("run requires an executable after --".into()))?;
    if store.latest().await?.iter().any(|entity| matches!(entity, Entity::Record(r) if r.project_id == request.project && r.execution.as_ref().is_some_and(|e| e.id == request.execution))) {
        return Err(Error::Invalid("execution ID already exists; retries require a new ID".into()));
    }
    let before = observe(store, request.project, request.repository, request.paths).await?;
    let started = Utc::now().to_rfc3339();
    let mut attempt = Record::new(
        &format!("{}:attempt", request.execution),
        request.project,
        request.source,
        RecordKind::Attempt,
        "Command invocation started; completion has not been observed",
    );
    attempt.work_ids = vec![request.work.into()];
    attempt.association = Association::Explicit;
    attempt.session_id = Some(request.session.into());
    attempt.worktree_id.clone_from(&before.worktree_id);
    attempt.code_refs = before
        .files
        .iter()
        .map(|file| CodeRef {
            state_id: before.id.clone(),
            path: file.path.clone(),
            range: None,
        })
        .collect();
    attempt.attempt_id = Some(attempt.id.clone());
    attempt.attempt_outcome = Some(AttemptOutcome::Running);
    attempt.occurred_at = Some(started.clone());
    attempt.paths = request
        .paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    attempt.partial = true;
    attempt.execution = Some(Execution {
        id: request.execution.into(),
        command: serde_json::to_string(request.command)?,
        tool_name: Some("process".into()),
        tool_input: Some(serde_json::to_string(request.command)?),
        cwd: Some(request.repository.to_string_lossy().into_owned()),
        started_at: Some(started.clone()),
        ended_at: None,
        exit_code: None,
        last_observed_state: "running".into(),
        observed_at: Some(started),
        liveness: Liveness::Unknown,
        before_state: Some(before.id.clone()),
        after_state: None,
        scope: Vec::new(),
        environment: Environment {
            os: Some(std::env::consts::OS.into()),
            ..Environment::default()
        },
    });
    let mut receipts = vec![store.append(Entity::Record(attempt.clone())).await?];
    receipts.extend(link_observation(store, &attempt, &before, "before").await?);
    let output = Command::new(executable)
        .args(request.command.iter().skip(1))
        .current_dir(request.repository)
        .output();
    let after = observe(store, request.project, request.repository, request.paths).await?;
    let ended = Utc::now().to_rfc3339();
    let (body, exit_code, passed) = match output {
        Ok(output) => (
            format!(
                "stdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            output.status.code(),
            output.status.success(),
        ),
        Err(error) => (format!("failed to start command: {error}"), None, false),
    };
    let mut result = Record::new(
        &format!("{}:result", request.execution),
        request.project,
        request.source,
        RecordKind::ToolResult,
        &body,
    );
    result.work_ids.clone_from(&attempt.work_ids);
    result.association = Association::Explicit;
    result.session_id.clone_from(&attempt.session_id);
    result.worktree_id.clone_from(&attempt.worktree_id);
    result.code_refs = after
        .files
        .iter()
        .map(|file| CodeRef {
            state_id: after.id.clone(),
            path: file.path.clone(),
            range: None,
        })
        .collect();
    result.paths.clone_from(&attempt.paths);
    result.attempt_id.clone_from(&attempt.attempt_id);
    result.occurred_at = Some(ended.clone());
    result.execution.clone_from(&attempt.execution);
    if let Some(exec) = &mut result.execution {
        exec.ended_at = Some(ended.clone());
        exec.exit_code = exit_code;
        exec.observed_at = Some(ended);
        exec.last_observed_state = if passed { "succeeded" } else { "failed" }.into();
        exec.liveness = Liveness::Stopped;
        exec.after_state = Some(after.id.clone());
    }
    receipts.push(store.append(Entity::Record(result.clone())).await?);
    receipts.extend(link_observation(store, &result, &after, "after").await?);
    attempt.partial = false;
    attempt.attempt_outcome = Some(if passed {
        AttemptOutcome::Succeeded
    } else {
        AttemptOutcome::Failed
    });
    attempt.execution.clone_from(&result.execution);
    attempt.revision = hash(&serde_json::to_vec(&result)?);
    attempt.body = "Command completed; see the linked output and code observations. A successful command is not automatically a test or an approved decision.".into();
    receipts.push(store.append(Entity::Record(attempt.clone())).await?);
    receipts.push(
        store
            .append(Entity::Relation(Relation {
                id: format!("{}:response", request.execution),
                project_id: request.project.into(),
                source_id: request.source.into(),
                from: Target::Record {
                    id: result.id.clone(),
                },
                to: Target::Record { id: attempt.id },
                kind: RelationKind::RespondsTo,
                nature: Nature::Observed,
                evidence: vec![Evidence {
                    source_id: request.source.into(),
                    record_id: Some(result.id),
                    revision: result.revision,
                    locator: format!("execution:{}", request.execution),
                    availability: Availability::Available,
                    range: None,
                }],
                applies_to: Vec::new(),
            }))
            .await?,
    );
    Ok(receipts)
}

async fn link_observation(
    store: &mut Store,
    record: &Record,
    state: &CodeState,
    phase: &str,
) -> Result<Vec<Receipt>> {
    let relations = state.files.iter().map(|file| {
        Entity::Relation(Relation {
            id: format!("{}:{phase}:{}", record.id, file.path),
            project_id: record.project_id.clone(),
            source_id: record.source_id.clone(),
            from: Target::Record {
                id: record.id.clone(),
            },
            to: Target::Code {
                state_id: state.id.clone(),
                path: file.path.clone(),
                range: None,
            },
            kind: RelationKind::RelatedTo,
            nature: Nature::Observed,
            evidence: vec![Evidence {
                source_id: record.source_id.clone(),
                record_id: Some(record.id.clone()),
                revision: record.revision.clone(),
                locator: format!("observation:{phase}:{}", state.id),
                availability: Availability::Available,
                range: None,
            }],
            applies_to: vec![file.path.clone()],
        })
    });
    // Temporal observation only: concurrent edits are not attributed to this command.
    store.append_all(relations).await
}
