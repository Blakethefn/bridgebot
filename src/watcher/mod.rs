use anyhow::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::config::Config as AppConfig;
use crate::rules;

/// Events emitted by the file watcher.
#[derive(Debug, Clone)]
pub enum VaultEvent {
    /// A file in the vault was created.
    FileCreated(PathBuf),
    /// A file in the vault was modified.
    FileModified(PathBuf),
    /// A file in the vault was removed.
    FileRemoved(PathBuf),
    /// A git ref changed (branch switch, commit, etc.)
    GitRefChanged(PathBuf),
    /// Periodic tick for time-based checks.
    Tick,
}

/// Run the daemon: watch the vault and git repos, dispatch events to rules.
pub async fn run(config: AppConfig) -> Result<()> {
    let (event_tx, _) = broadcast::channel::<VaultEvent>(256);

    // Start the file watcher
    let vault_path = config.daemon.vault.clone();
    let ignore = config.daemon.ignore.clone();
    let tx_vault = event_tx.clone();
    let _vault_watcher = start_vault_watcher(vault_path, ignore, tx_vault)?;

    // Start git watcher if projects_dir is set
    let _git_watcher = if let Some(ref projects_dir) = config.daemon.projects_dir {
        let tx_git = event_tx.clone();
        Some(start_git_watcher(projects_dir.clone(), tx_git)?)
    } else {
        None
    };

    // Start periodic tick
    let tx_tick = event_tx.clone();
    let interval = config.daemon.interval;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval));
        loop {
            interval.tick().await;
            if tx_tick.send(VaultEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Event processing loop
    let mut rx = event_tx.subscribe();
    info!("bridgebot daemon running. Press Ctrl+C to stop.");

    // Run initial tick for staleness checks etc.
    rules::handle_event(&config, &VaultEvent::Tick).await;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(vault_event) => {
                        debug!("event: {vault_event:?}");
                        rules::handle_event(&config, &vault_event).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("event receiver lagged by {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("event channel closed, shutting down");
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("received Ctrl+C, shutting down");
                break;
            }
        }
    }

    Ok(())
}

fn start_vault_watcher(
    vault_path: PathBuf,
    ignore: Vec<String>,
    tx: broadcast::Sender<VaultEvent>,
) -> Result<RecommendedWatcher> {
    let (fs_tx, fs_rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(fs_tx, Config::default())?;
    watcher.watch(&vault_path, RecursiveMode::Recursive)?;

    info!("watching vault: {}", vault_path.display());

    std::thread::spawn(move || {
        for result in fs_rx {
            match result {
                Ok(event) => {
                    for path in &event.paths {
                        // Skip ignored paths
                        let should_ignore = ignore.iter().any(|pat| {
                            path.components().any(|c| {
                                c.as_os_str().to_str().map(|s| s == pat).unwrap_or(false)
                            })
                        });
                        if should_ignore {
                            continue;
                        }

                        // Only care about markdown files
                        let is_md = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e == "md")
                            .unwrap_or(false);
                        if !is_md {
                            continue;
                        }

                        let vault_event = match event.kind {
                            EventKind::Create(_) => VaultEvent::FileCreated(path.clone()),
                            EventKind::Modify(_) => VaultEvent::FileModified(path.clone()),
                            EventKind::Remove(_) => VaultEvent::FileRemoved(path.clone()),
                            _ => continue,
                        };

                        if tx.send(vault_event).is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("watcher error: {e}");
                }
            }
        }
    });

    Ok(watcher)
}

fn start_git_watcher(
    projects_dir: PathBuf,
    tx: broadcast::Sender<VaultEvent>,
) -> Result<RecommendedWatcher> {
    let (fs_tx, fs_rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(fs_tx, Config::default())?;

    // Watch .git/refs and .git/HEAD for each project
    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let git_refs = path.join(".git").join("refs");
                let git_head = path.join(".git").join("HEAD");
                if git_refs.exists() {
                    let _ = watcher.watch(&git_refs, RecursiveMode::Recursive);
                }
                if git_head.exists() {
                    let _ = watcher.watch(&git_head, RecursiveMode::NonRecursive);
                }
            }
        }
    }

    info!("watching git repos in: {}", projects_dir.display());

    std::thread::spawn(move || {
        for result in fs_rx {
            match result {
                Ok(event) => {
                    for path in event.paths {
                        if tx.send(VaultEvent::GitRefChanged(path)).is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("git watcher error: {e}");
                }
            }
        }
    });

    Ok(watcher)
}
