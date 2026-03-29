use anyhow::{Context, Result};
use git2::Repository;
use std::path::{Path, PathBuf};

/// Info about a git repository discovered in the projects directory.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub name: String,
    pub branch: Option<String>,
    pub has_uncommitted: bool,
    pub last_commit_time: Option<i64>,
}

/// Scan a directory for git repositories (non-recursive, one level deep).
pub fn scan_repos(projects_dir: &Path) -> Result<Vec<RepoInfo>> {
    let mut repos = Vec::new();

    let entries = std::fs::read_dir(projects_dir)
        .with_context(|| format!("reading projects dir: {}", projects_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let git_dir = path.join(".git");
        if !git_dir.exists() {
            continue;
        }

        match read_repo_info(&path) {
            Ok(info) => repos.push(info),
            Err(e) => {
                tracing::warn!("skipping repo {}: {e}", path.display());
            }
        }
    }

    Ok(repos)
}

/// Read info about a single git repository.
pub fn read_repo_info(path: &Path) -> Result<RepoInfo> {
    let repo = Repository::open(path)
        .with_context(|| format!("opening repo: {}", path.display()))?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from));

    let has_uncommitted = has_changes(&repo);

    let last_commit_time = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| commit.time().seconds());

    Ok(RepoInfo {
        path: path.to_path_buf(),
        name,
        branch,
        has_uncommitted,
        last_commit_time,
    })
}

fn has_changes(repo: &Repository) -> bool {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false);

    match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => !statuses.is_empty(),
        Err(_) => false,
    }
}

/// Get the current branch name for a repo at the given path.
pub fn current_branch(path: &Path) -> Result<Option<String>> {
    let repo = Repository::open(path)?;
    Ok(repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from)))
}

/// Get the timestamp of the last commit on HEAD.
pub fn last_commit_timestamp(path: &Path) -> Result<Option<i64>> {
    let repo = Repository::open(path)?;
    Ok(repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| commit.time().seconds()))
}
