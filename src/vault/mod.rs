mod frontmatter;
mod links;
mod notes;

pub use frontmatter::{parse_frontmatter, update_frontmatter_field};
pub use links::find_broken_links;
pub use notes::{find_notes, find_notes_by_type, NoteInfo};

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Represents the vault root and provides methods to query it.
#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
    pub ignore: Vec<String>,
}

impl Vault {
    pub fn new(root: PathBuf, ignore: Vec<String>) -> Self {
        Self { root, ignore }
    }

    /// Find all markdown files in the vault, respecting ignore patterns.
    pub fn all_notes(&self) -> Result<Vec<PathBuf>> {
        find_notes(&self.root, &self.ignore)
    }

    /// Find notes with a specific frontmatter type (e.g. "task", "output", "project").
    pub fn notes_by_type(&self, note_type: &str) -> Result<Vec<NoteInfo>> {
        find_notes_by_type(&self.root, &self.ignore, note_type)
    }

    /// Check a single file for broken wikilinks.
    pub fn check_links_in_file(&self, file: &Path) -> Result<Vec<String>> {
        find_broken_links(&self.root, file)
    }

    /// Check all vault files for broken wikilinks.
    pub fn check_all_links(&self) -> Result<Vec<(PathBuf, Vec<String>)>> {
        let notes = self.all_notes()?;
        let mut results = Vec::new();
        for note in notes {
            let broken = find_broken_links(&self.root, &note)?;
            if !broken.is_empty() {
                results.push((note, broken));
            }
        }
        Ok(results)
    }
}
