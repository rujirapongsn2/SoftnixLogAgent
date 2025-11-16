use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tokio::{pin, sync::watch};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use softnix_agent::{config::AgentConfig, run_agent, ShutdownSignal};

#[derive(Parser, Debug)]
#[command(author, version, about = "Softnix Log Collector Agent", long_about = None)]
struct Cli {
    /// Path to the agent configuration file (TOML)
    #[arg(long, default_value = "configs/agent.dev.toml")]
    config: PathBuf,

    /// Validate configuration file without starting the agent
    #[arg(long)]
    check: bool,

    /// Enable debug output of normalized events
    #[arg(long)]
    debug_events: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cli = Cli::parse();
    let config = AgentConfig::load(cli.config.clone())?;
    if cli.check {
        info!("configuration is valid");
        return Ok(());
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let run_future = run_agent(config, ShutdownSignal::new(shutdown_rx), cli.debug_events);
    pin!(run_future);

    tokio::select! {
        res = &mut run_future => res,
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown signal received");
            let _ = shutdown_tx.send(true);
            run_future.await
        }
    }
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}
