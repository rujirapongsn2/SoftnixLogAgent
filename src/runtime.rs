use anyhow::{Context, Result};
use tokio::{
    sync::mpsc,
    time::{interval, Duration},
};
use tracing::{error, info};

use crate::{
    config::AgentConfig,
    inputs, outputs,
    pipeline::{self, PipelineStats},
    shutdown::ShutdownSignal,
};

pub async fn run_agent(config: AgentConfig, shutdown: ShutdownSignal) -> Result<()> {
    let AgentConfig {
        runtime,
        inputs: input_configs,
        output,
    } = config;

    if input_configs.is_empty() {
        anyhow::bail!("no inputs configured");
    }

    let buffer = runtime.channel_size.max(1);
    let (ingest_sender, ingest_receiver) = mpsc::channel(buffer);
    let (processed_sender, processed_receiver) = mpsc::channel(buffer);
    let mut join_handles = Vec::new();

    for input in input_configs {
        let tx = ingest_sender.clone();
        let input_shutdown = shutdown.clone_signal();
        join_handles.push(tokio::spawn(async move {
            if let Err(err) = inputs::run_input(input, tx, input_shutdown).await {
                error!(error = %err, "input task exited with error");
            }
        }));
    }
    drop(ingest_sender);

    let stats = PipelineStats::default();
    let pipeline_stats = stats.clone();
    let pipeline_handle = tokio::spawn(async move {
        if let Err(err) =
            pipeline::run_pipeline(ingest_receiver, processed_sender, pipeline_stats).await
        {
            error!(error = %err, "pipeline task exited with error");
        }
    });

    let input_count = join_handles.len();
    info!(inputs = input_count, "agent started");

    let output_task =
        tokio::spawn(async move { outputs::run_output(output, processed_receiver).await });

    let stats_shutdown = shutdown.clone_signal();
    let stats_handle = tokio::spawn(async move { log_health(stats, stats_shutdown).await });

    for handle in join_handles {
        handle.await.context("input task panicked")?;
    }

    pipeline_handle.await.context("pipeline task panicked")?;
    output_task.await.context("output task panicked")??;

    if !stats_handle.is_finished() {
        stats_handle.abort();
    }
    let _ = stats_handle.await;

    info!("agent stopped");
    Ok(())
}

async fn log_health(stats: PipelineStats, mut shutdown: ShutdownSignal) {
    let mut ticker = interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                info!(processed = stats.processed(), "agent health");
            }
            _ = shutdown.wait_trigger() => {
                info!("health logger stopping");
                break;
            }
        }
    }
    info!(processed = stats.processed(), "final health snapshot");
}
