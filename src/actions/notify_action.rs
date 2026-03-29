use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::config::{Config, RuleConfig};
use crate::vault::Vault;
use crate::watcher::VaultEvent;

pub fn run(rule: &RuleConfig, event: &VaultEvent, vault: &Vault, _config: &Config) -> Result<()> {
    match event {
        VaultEvent::Tick => handle_stale_tasks(rule, vault),
        VaultEvent::FileCreated(path) | VaultEvent::FileModified(path) => {
            let msg = rule
                .message
                .as_deref()
                .unwrap_or("bridgebot: event on {file}");
            let msg = msg.replace(
                "{file}",
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown"),
            );
            send_notification("bridgebot", &msg)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn handle_stale_tasks(rule: &RuleConfig, vault: &Vault) -> Result<()> {
    let threshold_days = parse_days_threshold(&rule.trigger).unwrap_or(7);
    let tasks = vault.notes_by_type("task")?;
    let now = Utc::now();

    for task in &tasks {
        if task.frontmatter.status.as_deref() != Some("active") {
            continue;
        }

        let days_stale = if let Some(ref updated) = task.frontmatter.updated {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(updated, "%Y-%m-%d") {
                let updated_dt = date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                (now - updated_dt).num_days()
            } else {
                continue;
            }
        } else {
            // Fall back to file modification time
            match task.modified.elapsed() {
                Ok(elapsed) => (elapsed.as_secs() / 86400) as i64,
                Err(_) => continue,
            }
        };

        if days_stale > threshold_days {
            let task_name = task
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let msg = rule
                .message
                .as_deref()
                .unwrap_or("Task '{task.name}' has been active for {task.no_update_days} days")
                .replace("{task.name}", task_name)
                .replace("{task.no_update_days}", &days_stale.to_string());

            info!("stale task: {task_name} ({days_stale} days)");
            send_notification("bridgebot — stale task", &msg)?;
        }
    }

    Ok(())
}

fn parse_days_threshold(trigger: &str) -> Option<i64> {
    // Parse "task.no_update_days > N" from trigger string
    if let Some(pos) = trigger.find("no_update_days") {
        let after = &trigger[pos..];
        // Find the number after ">"
        if let Some(gt_pos) = after.find('>') {
            let num_str = after[gt_pos + 1..].trim();
            // Take only digits
            let num: String = num_str.chars().take_while(|c| c.is_ascii_digit()).collect();
            return num.parse().ok();
        }
    }
    None
}

fn send_notification(summary: &str, body: &str) -> Result<()> {
    notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .appname("bridgebot")
        .timeout(notify_rust::Timeout::Milliseconds(10000))
        .show()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_days_threshold() {
        assert_eq!(
            parse_days_threshold("task.active AND task.no_update_days > 7"),
            Some(7)
        );
        assert_eq!(
            parse_days_threshold("task.active AND task.no_update_days > 14"),
            Some(14)
        );
        assert_eq!(parse_days_threshold("vault.file_saved"), None);
    }
}
