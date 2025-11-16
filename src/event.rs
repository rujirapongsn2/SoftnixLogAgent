use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct LogEvent {
    pub source: String,
    pub line: String,
    pub ingested_at: DateTime<Utc>,
    pub indicators: Vec<IocMatch>,
    pub metadata: ParsedMetadata,
}

impl LogEvent {
    pub fn new<S: Into<String>, L: Into<String>>(source: S, line: L) -> Self {
        Self {
            source: source.into(),
            line: line.into(),
            ingested_at: Utc::now(),
            indicators: Vec::new(),
            metadata: ParsedMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IocMatch {
    pub kind: IocKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IocKind {
    Ip,
}

impl IocKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            IocKind::Ip => "ip",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParsedMetadata {
    pub level: Option<String>,
    pub app_name: Option<String>,
    pub observed_ts: Option<DateTime<Utc>>,
}
