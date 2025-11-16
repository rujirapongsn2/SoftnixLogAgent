use anyhow::Result;
use tokio::sync::mpsc;

use crate::{config::OutputConfig, event::LogEvent};

mod stdout;
mod syslog;

pub type EventReceiver = mpsc::Receiver<LogEvent>;

pub async fn run_output(config: OutputConfig, receiver: EventReceiver) -> Result<()> {
    match config {
        OutputConfig::Stdout {} => stdout::StdoutSink::default().run(receiver).await,
        OutputConfig::Syslog(cfg) => syslog::SyslogSink::new(cfg).run(receiver).await,
    }
}
