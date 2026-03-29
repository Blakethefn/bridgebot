use anyhow::Result;
use tracing::{info, warn};

use crate::config::{Config, RuleConfig};
use crate::vault::Vault;
use crate::watcher::VaultEvent;

pub fn run(rule: &RuleConfig, event: &VaultEvent, vault: &Vault, _config: &Config) -> Result<()> {
    match rule.scope.as_deref() {
        Some("changed-file") => check_changed_file(event, vault),
        _ => check_all(vault),
    }
}

fn check_changed_file(event: &VaultEvent, vault: &Vault) -> Result<()> {
    let path = match event {
        VaultEvent::FileCreated(p) | VaultEvent::FileModified(p) => p,
        _ => return Ok(()),
    };

    let broken = vault.check_links_in_file(path)?;
    if broken.is_empty() {
        return Ok(());
    }

    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    for link in &broken {
        warn!("broken link in {file_name}: [[{link}]]");
    }

    // Also send a desktop notification
    let body = format!(
        "{} broken link(s) in {file_name}: {}",
        broken.len(),
        broken.join(", ")
    );

    let _ = notify_rust::Notification::new()
        .summary("bridgebot — broken links")
        .body(&body)
        .appname("bridgebot")
        .timeout(notify_rust::Timeout::Milliseconds(8000))
        .show();

    Ok(())
}

fn check_all(vault: &Vault) -> Result<()> {
    let results = vault.check_all_links()?;

    if results.is_empty() {
        info!("no broken links found");
        return Ok(());
    }

    let total: usize = results.iter().map(|(_, links)| links.len()).sum();
    warn!("{total} broken link(s) across {} file(s)", results.len());

    for (file, broken) in &results {
        let name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        for link in broken {
            warn!("  {name}: [[{link}]]");
        }
    }

    let _ = notify_rust::Notification::new()
        .summary("bridgebot — broken links")
        .body(&format!("{total} broken link(s) found in vault"))
        .appname("bridgebot")
        .timeout(notify_rust::Timeout::Milliseconds(8000))
        .show();

    Ok(())
}
