use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static WIKILINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]\|]+)(?:\|[^\]]+)?\]\]").unwrap());

/// Extract all wikilink targets from markdown content.
pub fn find_wikilinks(content: &str) -> Vec<String> {
    WIKILINK_RE
        .captures_iter(content)
        .map(|cap| cap[1].trim().to_string())
        .collect()
}

/// Given a vault root and a file, find all broken wikilinks in that file.
/// A wikilink is broken if no matching .md file exists in the vault.
pub fn find_broken_links(vault_root: &Path, file: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("reading {}", file.display()))?;

    let links = find_wikilinks(&content);
    let mut broken = Vec::new();

    for link in links {
        if !resolve_wikilink(vault_root, &link) {
            broken.push(link);
        }
    }

    Ok(broken)
}

/// Try to resolve a wikilink target to an existing file in the vault.
/// Supports both relative paths (e.g. "folder/note") and bare names (e.g. "note").
fn resolve_wikilink(vault_root: &Path, target: &str) -> bool {
    // Strip any heading anchor (e.g. "note#section")
    let target = target.split('#').next().unwrap_or(target).trim();

    if target.is_empty() {
        return true; // Self-reference
    }

    // Try direct path resolution
    let direct = vault_root.join(format!("{target}.md"));
    if direct.exists() {
        return true;
    }

    // Try as-is (might already have extension)
    let as_is = vault_root.join(target);
    if as_is.exists() {
        return true;
    }

    // Try searching all subdirectories for a matching filename
    let target_filename = format!(
        "{}.md",
        Path::new(target)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(target)
    );

    search_for_file(vault_root, &target_filename)
}

fn search_for_file(dir: &Path, filename: &str) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == filename {
                    return true;
                }
            }
        } else if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            // Skip hidden directories and common ignores
            if !dir_name.starts_with('.') && dir_name != "node_modules" {
                if search_for_file(&path, filename) {
                    return true;
                }
            }
        }
    }

    false
}

/// Resolve a wikilink to its full path if it exists.
pub fn resolve_wikilink_path(vault_root: &Path, target: &str) -> Option<PathBuf> {
    let target = target.split('#').next().unwrap_or(target).trim();

    if target.is_empty() {
        return None;
    }

    let direct = vault_root.join(format!("{target}.md"));
    if direct.exists() {
        return Some(direct);
    }

    let as_is = vault_root.join(target);
    if as_is.exists() {
        return Some(as_is);
    }

    let target_filename = format!(
        "{}.md",
        Path::new(target)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(target)
    );

    search_for_file_path(vault_root, &target_filename)
}

fn search_for_file_path(dir: &Path, filename: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == filename {
                    return Some(path);
                }
            }
        } else if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !dir_name.starts_with('.') && dir_name != "node_modules" {
                if let Some(found) = search_for_file_path(&path, filename) {
                    return Some(found);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_wikilinks() {
        let content = "See [[some-note]] and [[folder/other|display text]] for details.";
        let links = find_wikilinks(content);
        assert_eq!(links, vec!["some-note", "folder/other"]);
    }

    #[test]
    fn test_find_wikilinks_with_anchor() {
        let content = "Check [[note#heading]] for info.";
        let links = find_wikilinks(content);
        assert_eq!(links, vec!["note#heading"]);
    }
}
