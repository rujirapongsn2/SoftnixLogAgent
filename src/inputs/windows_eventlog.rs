use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tracing::warn;

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct WindowsEventLogInput {
    log: String,
    source: String,
}

impl WindowsEventLogInput {
    pub fn new(log: String, source: String) -> Self {
        Self { log, source }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        let script = format!(
            "$Log = '{}'; Get-WinEvent -LogName $Log -Wait | ForEach-Object {{ $_ | ConvertTo-Json -Compress }}",
            self.log.replace('\'', "''")
        );

        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoLogo", "-NoProfile", "-Command", &script]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to launch powershell for log {}", self.log))?;

        let stdout = child
            .stdout
            .take()
            .context("missing stdout from powershell process")?;
        let mut reader = BufReader::new(stdout);

        loop {
            let mut line = String::new();
            let read = tokio::select! {
                res = reader.read_line(&mut line) => res?,
                _ = shutdown.wait_trigger() => {
                    if let Err(err) = child.kill().await {
                        warn!(error = %err, "failed to kill powershell process");
                    }
                    break;
                }
            };
            if read == 0 {
                break;
            }
            let payload = line.trim_end_matches(['\n', '\r']).to_string();
            if payload.is_empty() {
                continue;
            }
            let event = LogEvent::new(&self.source, payload);
            if sender.send(event).await.is_err() {
                break;
            }
        }

        Ok(())
    }
}
