use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom},
    time::sleep,
};

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct FileTailInput {
    path: PathBuf,
    source: String,
    read_from_beginning: bool,
    poll_interval: Duration,
}

impl FileTailInput {
    pub fn new(
        path: PathBuf,
        source: String,
        read_from_beginning: bool,
        poll_interval_ms: u64,
    ) -> Self {
        Self {
            path,
            source,
            read_from_beginning,
            poll_interval: Duration::from_millis(poll_interval_ms.max(100)),
        }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        loop {
            if shutdown.is_triggered() {
                break;
            }
            let file = tokio::select! {
                result = File::open(&self.path) => result,
                _ = shutdown.wait_trigger() => break,
            }
            .with_context(|| format!("failed to open {}", self.path.display()))?;

            let mut reader = BufReader::new(file);
            if !self.read_from_beginning {
                reader
                    .seek(SeekFrom::End(0))
                    .await
                    .with_context(|| format!("failed to seek {}", self.path.display()))?;
            }

            let should_exit = self
                .read_loop(&mut reader, &sender, shutdown.clone_signal())
                .await?;
            if should_exit || sender.is_closed() {
                break;
            }
            tokio::select! {
                _ = sleep(self.poll_interval) => {},
                _ = shutdown.wait_trigger() => break,
            }
        }
        Ok(())
    }

    async fn read_loop(
        &self,
        reader: &mut BufReader<File>,
        sender: &EventSender,
        mut shutdown: ShutdownSignal,
    ) -> Result<bool> {
        loop {
            let mut line = String::new();
            let read = tokio::select! {
                res = reader.read_line(&mut line) => res
                    .with_context(|| format!("failed reading {}", self.path.display()))?,
                _ = shutdown.wait_trigger() => return Ok(true),
            };

            if read == 0 {
                tokio::select! {
                    _ = sleep(self.poll_interval) => continue,
                    _ = shutdown.wait_trigger() => return Ok(true),
                }
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
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::{
        fs::OpenOptions,
        io::AsyncWriteExt,
        sync::{mpsc, watch},
        time::{timeout, Duration},
    };

    #[tokio::test]
    async fn emits_appended_lines() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();

        let input = FileTailInput::new(path.clone(), "test".into(), true, 50);
        let (tx, mut rx) = mpsc::channel(8);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let handle = tokio::spawn(async move {
            input
                .run(tx, ShutdownSignal::new(shutdown_rx))
                .await
                .expect("file tail run");
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .expect("open temp file");
        file.write_all(b"hello\n").await.expect("write line");
        file.flush().await.expect("flush");

        let event = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("receive timed out")
            .expect("event");
        assert_eq!(event.line, "hello");
        handle.abort();
    }
}
