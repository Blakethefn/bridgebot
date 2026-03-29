mod triggers;

use crate::actions;
use crate::config::Config;
use crate::vault::Vault;
use crate::watcher::VaultEvent;
use tracing::{debug, error, info};

/// Handle a single event by evaluating all rules against it.
pub async fn handle_event(config: &Config, event: &VaultEvent) {
    let vault = Vault::new(
        config.daemon.vault.clone(),
        config.daemon.ignore.clone(),
    );

    for rule in &config.rules {
        if rule.enabled == Some(false) {
            continue;
        }

        if triggers::matches(rule, event, &vault, config) {
            info!("rule '{}' triggered", rule.name);
            if let Err(e) = actions::execute(rule, event, &vault, config).await {
                error!("rule '{}' action failed: {e}", rule.name);
            }
        }
    }
}

/// Run all periodic/staleness rules once (for `bridgebot run` one-shot mode).
pub async fn run_once(config: &Config) -> anyhow::Result<()> {
    handle_event(config, &VaultEvent::Tick).await;

    // Also run vault-wide link checks if any rule wants them
    let vault = Vault::new(
        config.daemon.vault.clone(),
        config.daemon.ignore.clone(),
    );

    for rule in &config.rules {
        if rule.enabled == Some(false) {
            continue;
        }
        if rule.action == "check-links" && rule.scope.as_deref() != Some("changed-file") {
            info!("running vault-wide link check for rule '{}'", rule.name);
            match vault.check_all_links() {
                Ok(results) => {
                    for (file, broken) in &results {
                        for link in broken {
                            info!(
                                "broken link in {}: [[{}]]",
                                file.display(),
                                link
                            );
                        }
                    }
                    if results.is_empty() {
                        debug!("no broken links found");
                    }
                }
                Err(e) => error!("link check failed: {e}"),
            }
        }
    }

    Ok(())
}
