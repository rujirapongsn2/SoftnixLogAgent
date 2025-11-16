use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tracing::warn;

use super::EventSender;
use crate::{event::LogEvent, shutdown::ShutdownSignal};

pub struct UdpListenerInput {
    bind: SocketAddr,
    source: String,
}

impl UdpListenerInput {
    pub fn new(bind: SocketAddr, source: String) -> Self {
        Self { bind, source }
    }

    pub async fn run(self, sender: EventSender, mut shutdown: ShutdownSignal) -> Result<()> {
        let socket = UdpSocket::bind(self.bind)
            .await
            .with_context(|| format!("failed to bind udp listener on {}", self.bind))?;
        let mut buf = vec![0u8; 8192];
        loop {
            let len;
            let peer;
            tokio::select! {
                res = socket.recv_from(&mut buf) => {
                    match res {
                        Ok((read, addr)) => {
                            len = read;
                            peer = addr;
                        }
                        Err(err) => {
                            warn!(error = %err, "udp receive failed");
                            continue;
                        }
                    }
                }
                _ = shutdown.wait_trigger() => break,
            }

            if len == 0 {
                continue;
            }

            let payload = String::from_utf8_lossy(&buf[..len]).trim().to_string();
            if payload.is_empty() {
                continue;
            }
            let source = format!("{}:{}", self.source, peer);
            let event = LogEvent::new(source, payload);
            if sender.send(event).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        net::UdpSocket,
        sync::{mpsc, watch},
        time::{timeout, Duration},
    };

    #[tokio::test]
    async fn receives_datagram() {
        let temp = UdpSocket::bind("127.0.0.1:0").await.expect("bind temp");
        let addr = temp.local_addr().unwrap();
        drop(temp);
        let input = UdpListenerInput::new(addr, "udp-test".into());
        let (tx, mut rx) = mpsc::channel(8);
        let (_stop_tx, stop_rx) = watch::channel(false);
        let handle = tokio::spawn(async move {
            input
                .run(tx, ShutdownSignal::new(stop_rx))
                .await
                .expect("udp run");
        });

        let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind sender");
        let payload = b"hello";
        socket.send_to(payload, addr).await.expect("send packet");

        let event = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert!(event.line.contains("hello"));
        handle.abort();
    }
}
