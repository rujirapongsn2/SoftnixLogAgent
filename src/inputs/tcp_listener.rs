use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tracing::warn;

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct TcpListenerInput {
    bind: SocketAddr,
    source: String,
}

impl TcpListenerInput {
    pub fn new(bind: SocketAddr, source: String) -> Self {
        Self { bind, source }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        let listener = TcpListener::bind(self.bind)
            .await
            .with_context(|| format!("failed to bind tcp listener on {}", self.bind))?;
        loop {
            tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((stream, peer)) => {
                            let tx = sender.clone();
                            let source = format!("{}:{}", self.source, peer);
                            let mut child_shutdown = shutdown.clone_signal();
                            tokio::spawn(async move {
                                if let Err(err) = handle_client(stream, source, tx, &mut child_shutdown).await {
                                    warn!(error = %err, "tcp client handler exited");
                                }
                            });
                        }
                        Err(err) => {
                            warn!(error = %err, "tcp accept failed");
                        }
                    }
                }
                _ = shutdown.wait_trigger() => break,
            }
        }
        Ok(())
    }
}

async fn handle_client(
    stream: TcpStream,
    source: String,
    sender: EventSender,
    shutdown: &mut ShutdownSignal,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let read = tokio::select! {
            res = reader.read_line(&mut line) => res?,
            _ = shutdown.wait_trigger() => break,
        };
        if read == 0 {
            break;
        }
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if line.is_empty() {
            continue;
        }
        let event = LogEvent::new(&source, line);
        if sender.send(event).await.is_err() {
            break;
        }
    }
    Ok(())
}
