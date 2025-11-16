use anyhow::Result;
use tokio::{
    io::{self, AsyncWriteExt, BufWriter},
    sync::mpsc,
};
use tracing::debug;

use crate::event::LogEvent;

pub struct StdoutSink;

impl Default for StdoutSink {
    fn default() -> Self {
        Self
    }
}

impl StdoutSink {
    pub async fn run(
        self,
        mut receiver: mpsc::Receiver<LogEvent>,
        debug_events: bool,
    ) -> Result<()> {
        let mut writer = BufWriter::new(io::stdout());
        while let Some(event) = receiver.recv().await {
            if debug_events {
                debug!(?event.normalized, ?event.indicators, "normalized event");
            }
            let mut line = format!(
                "{} [{}] {}",
                event
                    .ingested_at
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                event.source,
                event.line
            );
            if !event.indicators.is_empty() {
                let summary = event
                    .indicators
                    .iter()
                    .map(|ioc| format!("{}={}", ioc.kind.as_str(), ioc.value))
                    .collect::<Vec<_>>()
                    .join(",");
                line.push_str(&format!(" [ioc:{}]", summary));
            }
            line.push('\n');
            writer.write_all(line.as_bytes()).await?;
            writer.flush().await?;
        }
        writer.flush().await?;
        Ok(())
    }
}
