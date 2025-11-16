use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
};
use tracing::warn;

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct ProcessInput {
    program: String,
    args: Vec<String>,
    source: String,
}

impl ProcessInput {
    pub fn new(program: String, args: Vec<String>, source: String) -> Self {
        Self {
            program,
            args,
            source,
        }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn process {}", self.program))?;

        let mut tasks = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            tasks.push(spawn_reader(
                stdout,
                sender.clone(),
                format!("{}:stdout", self.source),
                shutdown.clone_signal(),
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            tasks.push(spawn_reader(
                stderr,
                sender.clone(),
                format!("{}:stderr", self.source),
                shutdown.clone_signal(),
            ));
        }

        tokio::select! {
            status = child.wait() => {
                if let Err(err) = status {
                    warn!(error = %err, "process wait failed");
                }
            }
            _ = shutdown.wait_trigger() => {
                if let Err(err) = child.kill().await {
                    warn!(error = %err, "failed to kill child process");
                }
            }
        }

        for task in tasks {
            task.abort();
        }
        Ok(())
    }
}

fn spawn_reader(
    stream: impl AsyncRead + Unpin + Send + 'static,
    sender: EventSender,
    source: String,
    mut shutdown: ShutdownSignal,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stream);
        loop {
            let mut line = String::new();
            let read = tokio::select! {
                res = reader.read_line(&mut line) => match res {
                    Ok(len) => len,
                    Err(err) => {
                        warn!(error = %err, source = %source, "process stream read failed");
                        break;
                    }
                },
                _ = shutdown.wait_trigger() => break,
            };
            if read == 0 {
                break;
            }
            let payload = line.trim_end_matches(['\n', '\r']).to_string();
            if payload.is_empty() {
                continue;
            }
            let event = LogEvent::new(&source, payload);
            if sender.send(event).await.is_err() {
                break;
            }
        }
    })
}
