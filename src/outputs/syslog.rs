use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use chrono::{Local, SecondsFormat};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpStream, UdpSocket},
    sync::mpsc,
    time::sleep,
};
use tracing::warn;

use crate::{
    config::{SyslogFormat, SyslogOutputConfig, SyslogProtocol},
    event::LogEvent,
};

const SYSLOG_SEVERITY_INFO: u8 = 6;

pub struct SyslogSink {
    config: SyslogOutputConfig,
    hostname: String,
    app_name: String,
}

impl SyslogSink {
    pub fn new(config: SyslogOutputConfig) -> Self {
        let hostname = config
            .hostname
            .clone()
            .or_else(|| {
                hostname::get()
                    .ok()
                    .map(|h| h.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "softnix-agent".to_string());
        let app_name = config
            .app_name
            .clone()
            .unwrap_or_else(|| "softnix-agent".to_string());

        Self {
            config,
            hostname,
            app_name,
        }
    }

    pub async fn run(self, receiver: mpsc::Receiver<LogEvent>) -> Result<()> {
        match self.config.protocol {
            SyslogProtocol::Udp => self.run_udp(receiver).await,
            SyslogProtocol::Tcp => self.run_tcp(receiver).await,
        }
    }

    async fn run_udp(&self, mut receiver: mpsc::Receiver<LogEvent>) -> Result<()> {
        let target: SocketAddr = self
            .config
            .address
            .parse()
            .context("invalid syslog UDP address")?;
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("failed to bind UDP socket")?;

        while let Some(event) = receiver.recv().await {
            let payload = self.format_message(&event);
            if let Err(err) = socket.send_to(payload.as_bytes(), &target).await {
                warn!(target = %target, error = %err, "failed sending syslog UDP frame");
            }
        }
        Ok(())
    }

    async fn run_tcp(&self, mut receiver: mpsc::Receiver<LogEvent>) -> Result<()> {
        let target = self.config.address.clone();
        let mut writer: Option<BufWriter<TcpStream>> = None;

        while let Some(event) = receiver.recv().await {
            let payload = self.format_message(&event);
            loop {
                if writer.is_none() {
                    match TcpStream::connect(&target).await {
                        Ok(stream) => writer = Some(BufWriter::new(stream)),
                        Err(err) => {
                            warn!(target = %target, error = %err, "syslog TCP connect failed");
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }

                if let Some(w) = writer.as_mut() {
                    if let Err(err) = w.write_all(payload.as_bytes()).await {
                        warn!(target = %target, error = %err, "syslog TCP write failed");
                        writer = None;
                        continue;
                    }
                    if let Err(err) = w.write_all(b"\n").await {
                        warn!(target = %target, error = %err, "syslog TCP newline failed");
                        writer = None;
                        continue;
                    }
                    if let Err(err) = w.flush().await {
                        warn!(target = %target, error = %err, "syslog TCP flush failed");
                        writer = None;
                        continue;
                    }
                }
                break;
            }
        }

        if let Some(mut w) = writer {
            let _ = w.flush().await;
        }
        Ok(())
    }

    fn format_message(&self, event: &LogEvent) -> String {
        match self.config.format {
            SyslogFormat::Rfc3164 => self.format_rfc3164(event),
            SyslogFormat::Rfc5424 => self.format_rfc5424(event),
        }
    }

    fn pri(&self) -> u8 {
        let facility = self.config.facility.min(23);
        facility * 8 + SYSLOG_SEVERITY_INFO
    }

    fn format_rfc3164(&self, event: &LogEvent) -> String {
        let timestamp = event
            .ingested_at
            .with_timezone(&Local)
            .format("%b %e %H:%M:%S");
        format!(
            "<{}>{} {} {}: {}",
            self.pri(),
            timestamp,
            self.hostname,
            self.app_name,
            event.line
        )
    }

    fn format_rfc5424(&self, event: &LogEvent) -> String {
        let timestamp = event
            .ingested_at
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        format!(
            "<{}>1 {} {} {} - - {}",
            self.pri(),
            timestamp,
            self.hostname,
            self.app_name,
            event.line
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn test_config(format: SyslogFormat) -> SyslogOutputConfig {
        SyslogOutputConfig {
            protocol: SyslogProtocol::Udp,
            address: "127.0.0.1:5514".into(),
            format,
            hostname: Some("host".into()),
            app_name: Some("app".into()),
            facility: 1,
        }
    }

    #[test]
    fn formats_rfc5424_payload() {
        let sink = SyslogSink::new(test_config(SyslogFormat::Rfc5424));
        let mut event = LogEvent::new("src", "payload");
        event.ingested_at = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let msg = sink.format_rfc5424(&event);
        assert!(msg.contains("host app"));
        assert!(msg.contains("payload"));
    }

    #[test]
    fn formats_rfc3164_payload() {
        let sink = SyslogSink::new(test_config(SyslogFormat::Rfc3164));
        let mut event = LogEvent::new("src", "payload");
        event.ingested_at = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let msg = sink.format_rfc3164(&event);
        assert!(msg.contains("host"));
        assert!(msg.contains("payload"));
    }
}
