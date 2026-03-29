use anyhow::Result;
use tracing::{error, info};

use crate::config::{Config, RuleConfig};
use crate::vault::Vault;
use crate::watcher::VaultEvent;

/// Execute a shell command defined in the rule config.
pub async fn run(rule: &RuleConfig, event: &VaultEvent, vault: &Vault, _config: &Config) -> Result<()> {
    let command = rule
        .command
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("rule '{}' has action 'shell' but no 'command' field", rule.name))?;

    // Template substitution
    let command = substitute(command, event, vault);

    info!("executing shell command: {command}");

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            "shell command failed (exit {}): {stderr}",
            output.status.code().unwrap_or(-1)
        );
        anyhow::bail!("shell command failed: {command}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        info!("shell output: {stdout}");
    }

    Ok(())
}

fn substitute(command: &str, event: &VaultEvent, vault: &Vault) -> String {
    let mut result = command.to_string();

    result = result.replace("{vault}", &vault.root.display().to_string());

    match event {
        VaultEvent::FileCreated(path)
        | VaultEvent::FileModified(path)
        | VaultEvent::FileRemoved(path)
        | VaultEvent::GitRefChanged(path) => {
            result = result.replace("{file}", &path.display().to_string());
            result = result.replace(
                "{file.name}",
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            );
        }
        VaultEvent::Tick => {}
    }

    result
}
