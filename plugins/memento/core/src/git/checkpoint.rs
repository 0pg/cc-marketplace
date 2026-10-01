//! Bind process context to the exact index Git will commit.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{GitError, process, text, worktree_root};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexBinding {
    pub parent_head: Option<String>,
    pub staged_tree: String,
}

/// `write-tree` materializes a tree object, but does not create a commit or stage files.
/// Honor GIT_INDEX_FILE so pre-commit checks Git's temporary partial-commit index.
#[tracing::instrument(skip_all, fields(repository = %repository.display()))]
pub fn index_binding(repository: &Path) -> Result<IndexBinding, GitError> {
    let repository = worktree_root(repository)?;
    let parent_head = head(&repository)?;
    let staged_tree = tree(&repository)?;
    let after = IndexBinding {
        parent_head: head(&repository)?,
        staged_tree: tree(&repository)?,
    };
    let binding = IndexBinding {
        parent_head,
        staged_tree,
    };
    if binding != after {
        return Err(GitError::InvalidInput(
            "HEAD or commit index changed while preparing the checkpoint; retry".into(),
        ));
    }
    Ok(binding)
}

/// A missing reflog entry is a capture gap, not evidence for an inferred parent.
pub fn preceding_head(repository: &Path) -> Result<Option<String>, GitError> {
    let repository = worktree_root(repository)?;
    let reflog = text(process::git(
        &repository,
        &["reflog", "show", "--format=%H", "-n", "2", "HEAD"],
    )?)?;
    let mut observed = reflog.lines();
    let current = head(&repository)?;
    // A disabled or stale reflog is not an observation of this commit's prior HEAD.
    if observed.next() != current.as_deref() {
        return Ok(None);
    }
    Ok(observed.next().map(str::to_owned))
}

fn head(repository: &Path) -> Result<Option<String>, GitError> {
    process::git_optional(repository, &["rev-parse", "--verify", "--quiet", "HEAD"])?
        .map(text)
        .transpose()
        .map(|value| value.map(|value| value.trim().to_owned()))
}

fn tree(repository: &Path) -> Result<String, GitError> {
    Ok(text(process::git_index(repository, &["write-tree"])?)?
        .trim()
        .to_owned())
}
