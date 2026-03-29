use crate::config::{Config, RuleConfig};
use crate::vault::Vault;
use crate::watcher::VaultEvent;

/// Check if a rule's trigger matches the current event.
pub fn matches(rule: &RuleConfig, event: &VaultEvent, vault: &Vault, config: &Config) -> bool {
    let trigger = rule.trigger.as_str();

    match event {
        VaultEvent::Tick => matches_tick_trigger(trigger, vault),
        VaultEvent::FileCreated(path) => {
            matches_file_created_trigger(trigger, path, vault)
        }
        VaultEvent::FileModified(path) => {
            matches_file_modified_trigger(trigger, path, vault)
        }
        VaultEvent::FileRemoved(_) => false,
        VaultEvent::GitRefChanged(path) => {
            matches_git_trigger(trigger, path, config)
        }
    }
}

fn matches_tick_trigger(trigger: &str, _vault: &Vault) -> bool {
    // Staleness triggers fire on Tick events
    // e.g. "task.active AND task.no_update_days > 7"
    if trigger.contains("task.active") && trigger.contains("no_update_days") {
        return true; // The action handler will do the actual filtering
    }

    // vault.broken_link without scope = periodic full check
    if trigger == "vault.broken_link" {
        return true;
    }

    false
}

fn matches_file_created_trigger(trigger: &str, path: &std::path::Path, _vault: &Vault) -> bool {
    // "output.created" — fires when a new output note appears
    if trigger == "output.created" {
        if let Ok(Some(fm)) = crate::vault::parse_frontmatter(
            &std::fs::read_to_string(path).unwrap_or_default(),
        ) {
            return fm.note_type.as_deref() == Some("output");
        }
    }

    // "vault.file_saved" fires on create too
    if trigger == "vault.file_saved" {
        return true;
    }

    false
}

fn matches_file_modified_trigger(trigger: &str, _path: &std::path::Path, _vault: &Vault) -> bool {
    if trigger == "vault.file_saved" {
        return true;
    }

    false
}

fn matches_git_trigger(trigger: &str, _path: &std::path::Path, _config: &Config) -> bool {
    // "git.commit AND task.matches_branch"
    if trigger.contains("git.commit") {
        return true; // Action handler will check branch matching
    }

    false
}
