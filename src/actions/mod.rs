mod check_links;
mod link_from_hub;
mod notify_action;
mod shell;
mod update_frontmatter;

use anyhow::Result;

use crate::config::{Config, RuleConfig};
use crate::vault::Vault;
use crate::watcher::VaultEvent;

/// Execute the action defined by a rule.
pub async fn execute(
    rule: &RuleConfig,
    event: &VaultEvent,
    vault: &Vault,
    config: &Config,
) -> Result<()> {
    match rule.action.as_str() {
        "notify" => notify_action::run(rule, event, vault, config),
        "check-links" => check_links::run(rule, event, vault, config),
        "link-from-hub" => link_from_hub::run(rule, event, vault, config),
        "update-frontmatter" => update_frontmatter::run(rule, event, vault, config),
        "shell" => shell::run(rule, event, vault, config).await,
        other => {
            anyhow::bail!("unknown action: {other}");
        }
    }
}
