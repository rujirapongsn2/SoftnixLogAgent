use anyhow::Result;
use tokio::io::{self, AsyncBufReadExt, BufReader};

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct StdinInput {
    source: String,
}

impl StdinInput {
    pub fn new(source: String) -> Self {
        Self { source }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
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
            let event = LogEvent::new(self.source.clone(), line);
            if sender.send(event).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}
