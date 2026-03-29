use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Parsed YAML frontmatter from a markdown note.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Frontmatter {
    #[serde(rename = "type")]
    pub note_type: Option<String>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub owner: Option<String>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    /// Catch-all for extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Extract and parse YAML frontmatter from a markdown file's content.
pub fn parse_frontmatter(content: &str) -> Result<Option<Frontmatter>> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Ok(None);
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .or_else(|| after_first.find("\r\n---"));

    let yaml_block = match end {
        Some(pos) => &after_first[..pos],
        None => return Ok(None),
    };

    let fm: Frontmatter =
        serde_yaml::from_str(yaml_block).context("parsing frontmatter YAML")?;
    Ok(Some(fm))
}

/// Parse frontmatter from a file on disk.
pub fn parse_frontmatter_from_file(path: &Path) -> Result<Option<Frontmatter>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    parse_frontmatter(&content)
}

/// Update a single frontmatter field in a markdown file.
/// Rewrites the file in place.
pub fn update_frontmatter_field(path: &Path, field: &str, value: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;

    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        anyhow::bail!("file has no frontmatter: {}", path.display());
    }

    let after_first = &trimmed[3..];
    let end_pos = after_first
        .find("\n---")
        .or_else(|| after_first.find("\r\n---"))
        .ok_or_else(|| anyhow::anyhow!("unterminated frontmatter in {}", path.display()))?;

    let yaml_block = &after_first[..end_pos];
    let rest = &after_first[end_pos..];

    // Parse as a map, update the field, and re-serialize
    let mut map: serde_yaml::Mapping = serde_yaml::from_str(yaml_block)
        .context("parsing frontmatter for update")?;

    map.insert(
        serde_yaml::Value::String(field.to_string()),
        serde_yaml::Value::String(value.to_string()),
    );

    let new_yaml = serde_yaml::to_string(&map).context("serializing updated frontmatter")?;
    // serde_yaml adds a trailing newline; trim it for clean formatting
    let new_yaml = new_yaml.trim_end();

    let new_content = format!("---\n{new_yaml}\n{rest}");

    std::fs::write(path, new_content)
        .with_context(|| format!("writing updated frontmatter to {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\ntype: task\nstatus: active\ncreated: 2026-03-29\n---\n\n# My Task\n";
        let fm = parse_frontmatter(content).unwrap().unwrap();
        assert_eq!(fm.note_type.as_deref(), Some("task"));
        assert_eq!(fm.status.as_deref(), Some("active"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just a heading\n\nSome content.";
        let fm = parse_frontmatter(content).unwrap();
        assert!(fm.is_none());
    }
}
