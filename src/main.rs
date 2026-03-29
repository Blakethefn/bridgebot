mod actions;
mod config;
mod git;
mod rules;
mod vault;
mod watcher;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "bridgebot", version, about = "Obsidian vault + git automation daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon, watching vault and git repos for changes
    Start {
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Run all rules once and exit (useful for cron or testing)
    Run {
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Validate config file and print parsed result
    Check {
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Generate a default config file
    Init {
        /// Output path (defaults to ./bridgebot.toml)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn resolve_config_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }

    // Check current directory
    let local = PathBuf::from("bridgebot.toml");
    if local.exists() {
        return local;
    }

    // Check XDG config
    if let Some(config_dir) = dirs::config_dir() {
        let xdg = config_dir.join("bridgebot").join("bridgebot.toml");
        if xdg.exists() {
            return xdg;
        }
    }

    // Check home directory
    if let Some(home) = dirs::home_dir() {
        let home_config = home.join(".bridgebot.toml");
        if home_config.exists() {
            return home_config;
        }
    }

    local
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config } => {
            let path = resolve_config_path(config);
            let cfg = config::load(&path)?;
            info!("starting bridgebot daemon with {} rule(s)", cfg.rules.len());
            watcher::run(cfg).await?;
        }
        Commands::Run { config } => {
            let path = resolve_config_path(config);
            let cfg = config::load(&path)?;
            info!("running all rules once");
            rules::run_once(&cfg).await?;
        }
        Commands::Check { config } => {
            let path = resolve_config_path(config);
            match config::load(&path) {
                Ok(cfg) => {
                    println!("Config OK: {} rule(s) loaded", cfg.rules.len());
                    println!("Vault: {}", cfg.daemon.vault.display());
                    println!(
                        "Projects: {}",
                        cfg.daemon
                            .projects_dir
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(not set)".into())
                    );
                    for rule in &cfg.rules {
                        println!("  rule: {} [{}] -> {}", rule.name, rule.trigger, rule.action);
                    }
                }
                Err(e) => {
                    eprintln!("Config error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Init { output } => {
            let path = output.unwrap_or_else(|| PathBuf::from("bridgebot.toml"));
            if path.exists() {
                warn!("file already exists: {}", path.display());
                eprintln!("Refusing to overwrite existing config. Remove it first or use a different path.");
                std::process::exit(1);
            }
            let default_config = config::default_config_str();
            std::fs::write(&path, default_config)?;
            println!("Wrote default config to {}", path.display());
        }
    }

    Ok(())
}
