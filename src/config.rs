use std::{net::SocketAddr, path::PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub inputs: Vec<InputConfig>,
    #[serde(default)]
    pub output: OutputConfig,
}

impl AgentConfig {
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(&path)?;
        let config: AgentConfig = toml::from_str(&raw)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.inputs.is_empty() {
            anyhow::bail!("at least one input must be configured");
        }
        for input in &self.inputs {
            match input {
                InputConfig::Stdin { .. } => {}
                InputConfig::FileTail { path, .. } => {
                    if !path.exists() {
                        anyhow::bail!("file_tail path {} does not exist", path.display());
                    }
                }
                InputConfig::TcpListener { bind, .. } | InputConfig::UdpListener { bind, .. } => {
                    bind.parse::<SocketAddr>()
                        .with_context(|| format!("invalid bind address {bind}"))?;
                }
                InputConfig::Process { program, .. } => {
                    if program.is_empty() {
                        anyhow::bail!("process program must not be empty");
                    }
                }
                InputConfig::Journald { .. } => {
                    #[cfg(not(target_os = "linux"))]
                    anyhow::bail!("journald input is only supported on Linux builds");
                }
                InputConfig::WindowsEventLog { .. } => {
                    #[cfg(not(target_os = "windows"))]
                    anyhow::bail!("windows_event_log input is only supported on Windows builds");
                }
            }
        }

        match &self.output {
            OutputConfig::Stdout {} => {}
            OutputConfig::Syslog(cfg) => cfg.validate()?,
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub channel_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self { channel_size: 1024 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputConfig {
    Stdin {
        name: Option<String>,
    },
    FileTail {
        path: PathBuf,
        name: Option<String>,
        #[serde(default)]
        read_from_beginning: bool,
        #[serde(default = "default_poll_ms")]
        poll_interval_ms: u64,
    },
    TcpListener {
        bind: String,
        name: Option<String>,
    },
    UdpListener {
        bind: String,
        name: Option<String>,
    },
    Process {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        name: Option<String>,
    },
    Journald {
        units: Option<Vec<String>>,
        name: Option<String>,
    },
    WindowsEventLog {
        log: String,
        name: Option<String>,
    },
}

impl Default for InputConfig {
    fn default() -> Self {
        InputConfig::Stdin { name: None }
    }
}

fn default_poll_ms() -> u64 {
    500
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputConfig {
    Stdout {},
    Syslog(SyslogOutputConfig),
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig::Stdout {}
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyslogOutputConfig {
    #[serde(default = "default_syslog_protocol")]
    pub protocol: SyslogProtocol,
    pub address: String,
    #[serde(default = "default_syslog_format")]
    pub format: SyslogFormat,
    pub hostname: Option<String>,
    pub app_name: Option<String>,
    #[serde(default = "default_syslog_facility")]
    pub facility: u8,
}

impl SyslogOutputConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.facility > 23 {
            anyhow::bail!("syslog facility must be between 0 and 23");
        }
        self.address
            .parse::<SocketAddr>()
            .context("invalid syslog address")?;
        Ok(())
    }
}

fn default_syslog_protocol() -> SyslogProtocol {
    SyslogProtocol::Udp
}

fn default_syslog_format() -> SyslogFormat {
    SyslogFormat::Rfc3164
}

fn default_syslog_facility() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyslogProtocol {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyslogFormat {
    Rfc3164,
    Rfc5424,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_facility() {
        let cfg = SyslogOutputConfig {
            protocol: SyslogProtocol::Udp,
            address: "127.0.0.1:514".into(),
            format: SyslogFormat::Rfc3164,
            hostname: None,
            app_name: None,
            facility: 99,
        };
        assert!(cfg.validate().is_err());
    }
}
