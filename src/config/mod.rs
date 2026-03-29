use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DaemonConfig {
    /// Path to the Obsidian vault root
    pub vault: PathBuf,
    /// Path to the projects directory (optional, enables git watching)
    pub projects_dir: Option<PathBuf>,
    /// Polling interval for periodic checks (e.g. staleness), in seconds
    #[serde(default = "default_interval")]
    pub interval: u64,
    /// Directories within the vault to watch (defaults to all)
    #[serde(default)]
    pub watch_paths: Vec<String>,
    /// Directories to ignore
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RuleConfig {
    /// Human-readable name for this rule
    pub name: String,
    /// Trigger expression (e.g. "task.active AND task.no_update_days > 7")
    pub trigger: String,
    /// Action to perform (e.g. "notify", "link-from-hub", "check-links", "update-frontmatter", "shell")
    pub action: String,
    /// Optional message template (for notify actions)
    #[serde(default)]
    pub message: Option<String>,
    /// Optional field to update (for update-frontmatter actions)
    #[serde(default)]
    pub field: Option<String>,
    /// Optional scope limiter (e.g. "changed-file")
    #[serde(default)]
    pub scope: Option<String>,
    /// Optional shell command (for shell actions)
    #[serde(default)]
    pub command: Option<String>,
    /// Whether this rule is enabled (defaults to true)
    #[serde(default = "default_true")]
    pub enabled: Option<bool>,
}

fn default_interval() -> u64 {
    300 // 5 minutes
}

fn default_ignore() -> Vec<String> {
    vec![".obsidian".to_string(), ".git".to_string(), ".trash".to_string()]
}

fn default_true() -> Option<bool> {
    Some(true)
}

pub fn load(path: &Path) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading config: {}", path.display()))?;
    let config: Config =
        toml::from_str(&content).with_context(|| format!("parsing config: {}", path.display()))?;
    validate(&config)?;
    Ok(config)
}

fn validate(config: &Config) -> Result<()> {
    if !config.daemon.vault.exists() {
        anyhow::bail!(
            "vault path does not exist: {}",
            config.daemon.vault.display()
        );
    }
    if let Some(ref projects_dir) = config.daemon.projects_dir {
        if !projects_dir.exists() {
            anyhow::bail!(
                "projects_dir does not exist: {}",
                projects_dir.display()
            );
        }
    }
    for rule in &config.rules {
        if rule.name.is_empty() {
            anyhow::bail!("rule name cannot be empty");
        }
        if rule.trigger.is_empty() {
            anyhow::bail!("rule '{}' has empty trigger", rule.name);
        }
        if rule.action.is_empty() {
            anyhow::bail!("rule '{}' has empty action", rule.name);
        }
        let valid_actions = ["notify", "link-from-hub", "check-links", "update-frontmatter", "shell"];
        if !valid_actions.contains(&rule.action.as_str()) {
            anyhow::bail!(
                "rule '{}' has unknown action '{}'. Valid actions: {}",
                rule.name,
                rule.action,
                valid_actions.join(", ")
            );
        }
    }
    Ok(())
}

pub fn default_config_str() -> &'static str {
    r#"# bridgebot configuration
# See https://github.com/Blakethefn/bridgebot for documentation

[daemon]
# Path to your Obsidian vault root (required)
vault = "~/Documents/my-vault"

# Path to your projects directory (optional, enables git-aware rules)
# projects_dir = "~/Projects"

# How often to run periodic checks like staleness, in seconds (default: 300)
interval = 300

# Directories within the vault to watch (empty = watch all)
watch_paths = []

# Directories to ignore
ignore = [".obsidian", ".git", ".trash"]

# --- Rules ---
# Each [[rule]] defines a trigger condition and an action to take.
#
# Available triggers:
#   task.active AND task.no_update_days > N  — stale task detection
#   output.created                           — new output note appears
#   vault.file_saved                         — any vault file is saved
#   git.commit AND task.matches_branch       — commit on a task-linked branch
#   vault.broken_link                        — broken wikilink detected
#
# Available actions:
#   notify             — send a desktop notification (requires 'message')
#   link-from-hub      — auto-backlink a note from its project hub
#   check-links        — check for broken wikilinks
#   update-frontmatter — update a frontmatter field (requires 'field')
#   shell              — run a shell command (requires 'command')

[[rule]]
name = "stale-task-alert"
trigger = "task.active AND task.no_update_days > 7"
action = "notify"
message = "Task '{task.name}' has been active for {task.no_update_days} days"

[[rule]]
name = "broken-link-warning"
trigger = "vault.file_saved"
action = "check-links"
scope = "changed-file"

[[rule]]
name = "auto-backlink-output"
trigger = "output.created"
action = "link-from-hub"
"#
}
