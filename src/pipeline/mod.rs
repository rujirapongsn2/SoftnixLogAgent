use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::mpsc;

use crate::event::{IocKind, IocMatch, LogEvent, ParsedMetadata};

static IPV4_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?P<ip>(?:\d{1,3}\.){3}\d{1,3})").expect("valid IPv4 regex"));
static LEVEL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:level=)?(?P<level>INFO|WARN|ERROR|DEBUG|TRACE)\b")
        .expect("valid level regex")
});
static APP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?P<app>[A-Za-z0-9_./-]+)(?:\[\d+\])?:").expect("valid app regex"));
static RFC3339_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2}))")
        .expect("valid rfc3339 regex")
});

pub async fn run_pipeline(
    mut receiver: mpsc::Receiver<LogEvent>,
    sender: mpsc::Sender<LogEvent>,
    stats: PipelineStats,
) -> Result<()> {
    while let Some(mut event) = receiver.recv().await {
        normalize(&mut event);
        if sender.send(event).await.is_err() {
            break;
        }
        stats.inc();
    }
    Ok(())
}

pub fn normalize(event: &mut LogEvent) {
    event.indicators = extract_indicators(&event.line);
    event.metadata = parse_metadata(&event.line);
}

fn extract_indicators(line: &str) -> Vec<IocMatch> {
    IPV4_REGEX
        .captures_iter(line)
        .filter_map(|caps| caps.name("ip").map(|m| m.as_str().to_string()))
        .filter(|ip| is_valid_ipv4(ip))
        .map(|ip| IocMatch {
            kind: IocKind::Ip,
            value: ip,
        })
        .collect()
}

fn parse_metadata(line: &str) -> ParsedMetadata {
    let mut meta = ParsedMetadata::default();

    if let Some(level_caps) = LEVEL_REGEX.captures(line) {
        if let Some(level) = level_caps.name("level") {
            meta.level = Some(level.as_str().to_uppercase());
        }
    }

    if let Some(app_caps) = APP_REGEX.captures(line) {
        if let Some(app) = app_caps.name("app") {
            meta.app_name = Some(app.as_str().trim().to_string());
        }
    }

    if let Some(ts_caps) = RFC3339_REGEX.captures(line) {
        if let Some(ts) = ts_caps.name("ts") {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(ts.as_str()) {
                meta.observed_ts = Some(parsed.with_timezone(&Utc));
            }
        }
    }

    meta
}

fn is_valid_ipv4(ip: &str) -> bool {
    ip.split('.').all(|oct| matches!(oct.parse::<u8>(), Ok(_)))
}

#[derive(Clone, Default)]
pub struct PipelineStats {
    processed: Arc<AtomicU64>,
}

impl PipelineStats {
    pub fn inc(&self) {
        self.processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn processed(&self) -> u64 {
        self.processed.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn extracts_valid_ipv4s() {
        let mut event = LogEvent::new("test", "deny 10.1.1.1 connecting to 10.2.2.2");
        normalize(&mut event);
        assert_eq!(event.indicators.len(), 2);
        assert_eq!(event.indicators[0].kind, IocKind::Ip);
        assert_eq!(event.indicators[0].value, "10.1.1.1");
        assert_eq!(event.indicators[1].value, "10.2.2.2");
    }

    #[test]
    fn ignores_invalid_octets() {
        let mut event = LogEvent::new("test", "blocked 999.1.1.1");
        normalize(&mut event);
        assert!(event.indicators.is_empty());
    }

    #[test]
    fn parses_level_and_app_name() {
        let mut event = LogEvent::new("test", "WARN routerd[1234]: drop src=1.1.1.1");
        normalize(&mut event);
        assert_eq!(event.metadata.level.as_deref(), Some("WARN"));
        assert_eq!(event.metadata.app_name.as_deref(), Some("routerd"));
    }

    #[test]
    fn parses_rfc3339_timestamp() {
        let mut event = LogEvent::new("test", "2024-02-01T01:02:03Z firewall allow src=10.0.0.1");
        normalize(&mut event);
        let ts = event.metadata.observed_ts.expect("timestamp");
        assert_eq!(ts, Utc.with_ymd_and_hms(2024, 2, 1, 1, 2, 3).unwrap());
    }
}
