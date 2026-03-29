use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::frontmatter::{parse_frontmatter_from_file, Frontmatter};

/// Basic info about a vault note.
#[derive(Debug, Clone)]
pub struct NoteInfo {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub modified: std::time::SystemTime,
}

/// Find all markdown files in a directory, respecting ignore patterns.
pub fn find_notes(root: &Path, ignore: &[String]) -> Result<Vec<PathBuf>> {
    let mut notes = Vec::new();
    walk_dir(root, root, ignore, &mut notes)?;
    Ok(notes)
}

/// Find all markdown files with a specific frontmatter type.
pub fn find_notes_by_type(root: &Path, ignore: &[String], note_type: &str) -> Result<Vec<NoteInfo>> {
    let all = find_notes(root, ignore)?;
    let mut matched = Vec::new();

    for path in all {
        if let Ok(Some(fm)) = parse_frontmatter_from_file(&path) {
            if fm.note_type.as_deref() == Some(note_type) {
                let modified = path
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                matched.push(NoteInfo {
                    path,
                    frontmatter: fm,
                    modified,
                });
            }
        }
    }

    Ok(matched)
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    ignore: &[String],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Skip ignored directories/files
        if ignore.iter().any(|pat| name == pat.as_str()) {
            continue;
        }

        if path.is_dir() {
            walk_dir(root, &path, ignore, out)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "md" {
                    out.push(path);
                }
            }
        }
    }

    Ok(())
}
