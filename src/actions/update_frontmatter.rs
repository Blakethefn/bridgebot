use anyhow::Result;
use chrono::Utc;
use tracing::{debug, info};

use crate::config::{Config, RuleConfig};
use crate::vault::{self, Vault};
use crate::watcher::VaultEvent;

/// Update a frontmatter field on a note when triggered.
/// Primarily used for auto-updating the "updated" date when git activity is detected.
pub fn run(rule: &RuleConfig, event: &VaultEvent, vault: &Vault, config: &Config) -> Result<()> {
    let field = rule
        .field
        .as_deref()
        .unwrap_or("updated");

    match event {
        VaultEvent::GitRefChanged(ref_path) => {
            handle_git_change(rule, ref_path, vault, config, field)
        }
        VaultEvent::FileCreated(path) | VaultEvent::FileModified(path) => {
            // Direct frontmatter update on vault files
            let value = Utc::now().format("%Y-%m-%d").to_string();
            vault::update_frontmatter_field(path, field, &value)?;
            info!("updated {field} on {}", path.display());
            Ok(())
        }
        _ => Ok(()),
    }
}

fn handle_git_change(
    _rule: &RuleConfig,
    ref_path: &std::path::Path,
    vault: &Vault,
    _config: &Config,
    field: &str,
) -> Result<()> {
    // Extract project name from the git ref path
    // e.g. /path/to/projects/myproject/.git/refs/heads/main -> "myproject"
    let project_name = ref_path
        .ancestors()
        .find(|p| p.join(".git").exists() || p.file_name().map(|f| f == ".git").unwrap_or(false))
        .and_then(|p| {
            if p.file_name().map(|f| f == ".git").unwrap_or(false) {
                p.parent()
            } else {
                Some(p)
            }
        })
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    let project_name = match project_name {
        Some(name) => name,
        None => {
            debug!("could not extract project name from {}", ref_path.display());
            return Ok(());
        }
    };

    // Find the current branch name
    let repo_path = ref_path
        .ancestors()
        .find(|p| p.join(".git").exists())
        .or_else(|| {
            ref_path.ancestors().find(|p| {
                p.file_name().map(|f| f == ".git").unwrap_or(false)
            }).and_then(|p| p.parent())
        });

    let _branch = repo_path
        .and_then(|p| crate::git::current_branch(p).ok().flatten());

    // Find matching task notes for this project/branch
    let tasks = vault.notes_by_type("task")?;
    let value = Utc::now().format("%Y-%m-%d").to_string();

    for task in &tasks {
        let matches_project = task
            .frontmatter
            .project
            .as_deref()
            .map(|p| p == project_name)
            .unwrap_or(false);

        if !matches_project {
            continue;
        }

        // If we have a branch name, optionally check if the task note name contains it
        // This is a loose heuristic — can be tightened later
        if task.frontmatter.status.as_deref() == Some("active") {
            vault::update_frontmatter_field(&task.path, field, &value)?;
            info!(
                "updated {field} on task '{}' (git activity in {project_name})",
                task.path.display()
            );
        }
    }

    Ok(())
}
