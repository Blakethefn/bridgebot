use anyhow::Result;
use tracing::{debug, info, warn};

use crate::config::{Config, RuleConfig};
use crate::vault::{self, Vault};
use crate::watcher::VaultEvent;

/// When a new output note is created, find its project hub and add a backlink.
pub fn run(_rule: &RuleConfig, event: &VaultEvent, vault: &Vault, _config: &Config) -> Result<()> {
    let path = match event {
        VaultEvent::FileCreated(p) => p,
        _ => return Ok(()),
    };

    // Parse the new note's frontmatter to find its project
    let content = std::fs::read_to_string(path)?;
    let fm = match vault::parse_frontmatter(&content)? {
        Some(fm) => fm,
        None => {
            debug!("no frontmatter in {}, skipping", path.display());
            return Ok(());
        }
    };

    let project = match fm.project {
        Some(ref p) => p.clone(),
        None => {
            debug!(
                "no project field in {}, skipping auto-link",
                path.display()
            );
            return Ok(());
        }
    };

    // Find the project hub
    let hub_path = find_project_hub(vault, &project)?;
    let hub_path = match hub_path {
        Some(p) => p,
        None => {
            warn!("no project hub found for '{project}', skipping auto-link");
            return Ok(());
        }
    };

    // Get the note name for the wikilink
    let note_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Check if the hub already links to this note
    let hub_content = std::fs::read_to_string(&hub_path)?;
    if hub_content.contains(&format!("[[{note_name}]]"))
        || hub_content.contains(&format!("[[{note_name}|"))
    {
        debug!("hub already links to {note_name}");
        return Ok(());
    }

    // Find the "Outputs / Handoffs" section and append the link
    let new_link = format!("- [[{note_name}]]");

    let updated = if let Some(pos) = hub_content.find("## Outputs") {
        // Find the end of the section heading line
        let after_heading = &hub_content[pos..];
        let line_end = after_heading.find('\n').unwrap_or(after_heading.len());
        let insert_pos = pos + line_end;
        format!(
            "{}\n{new_link}{}",
            &hub_content[..insert_pos],
            &hub_content[insert_pos..]
        )
    } else {
        // Append to end of file
        format!("{hub_content}\n## Outputs / Handoffs\n\n{new_link}\n")
    };

    std::fs::write(&hub_path, updated)?;
    info!(
        "linked [[{note_name}]] from project hub '{}'",
        hub_path.display()
    );

    Ok(())
}

fn find_project_hub(vault: &Vault, project: &str) -> Result<Option<std::path::PathBuf>> {
    // Look for 01-projects/<project>.md
    let direct = vault.root.join("01-projects").join(format!("{project}.md"));
    if direct.exists() {
        return Ok(Some(direct));
    }

    // Look for 01-projects/<project>/<project>.md (subfolder layout)
    let subfolder = vault
        .root
        .join("01-projects")
        .join(project)
        .join(format!("{project}.md"));
    if subfolder.exists() {
        return Ok(Some(subfolder));
    }

    // Fuzzy search through project notes
    let projects = vault.notes_by_type("project")?;
    for p in &projects {
        let name = p
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if name.contains(project) || project.contains(name) {
            return Ok(Some(p.path.clone()));
        }
    }

    Ok(None)
}
