use anyhow::{self, Result};
use tokio::sync::mpsc;

use crate::{config::InputConfig, event::LogEvent, shutdown::ShutdownSignal};

mod file_tail;
mod process;
mod stdin;
mod tcp_listener;
mod udp_listener;
#[cfg(target_os = "windows")]
mod windows_eventlog;

pub type EventSender = mpsc::Sender<LogEvent>;

pub async fn run_input(
    config: InputConfig,
    sender: EventSender,
    shutdown: ShutdownSignal,
) -> Result<()> {
    match config {
        InputConfig::Stdin { name } => {
            let source = name.unwrap_or_else(|| "stdin".to_string());
            stdin::StdinInput::new(source).run(sender, shutdown).await
        }
        InputConfig::FileTail {
            path,
            name,
            read_from_beginning,
            poll_interval_ms,
        } => {
            let source = name.unwrap_or_else(|| path.to_string_lossy().to_string());
            file_tail::FileTailInput::new(path, source, read_from_beginning, poll_interval_ms)
                .run(sender, shutdown)
                .await
        }
        InputConfig::TcpListener { bind, name } => {
            let source = name.unwrap_or_else(|| "tcp".to_string());
            let addr = bind
                .parse()
                .map_err(|e: std::net::AddrParseError| anyhow::anyhow!(e))?;
            tcp_listener::TcpListenerInput::new(addr, source)
                .run(sender, shutdown)
                .await
        }
        InputConfig::UdpListener { bind, name } => {
            let source = name.unwrap_or_else(|| "udp".to_string());
            let addr = bind
                .parse()
                .map_err(|e: std::net::AddrParseError| anyhow::anyhow!(e))?;
            udp_listener::UdpListenerInput::new(addr, source)
                .run(sender, shutdown)
                .await
        }
        InputConfig::Process {
            program,
            args,
            name,
        } => {
            let source = name.unwrap_or_else(|| program.clone());
            process::ProcessInput::new(program, args, source)
                .run(sender, shutdown)
                .await
        }
        InputConfig::Journald { units, name } => {
            #[cfg(target_os = "linux")]
            {
                let mut args = vec!["-f".to_string(), "-o".to_string(), "cat".to_string()];
                if let Some(units) = units {
                    for unit in units {
                        args.push("-u".to_string());
                        args.push(unit);
                    }
                }
                let source = name.unwrap_or_else(|| "journald".to_string());
                process::ProcessInput::new("journalctl".into(), args, source)
                    .run(sender, shutdown)
                    .await
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = units;
                let _ = name;
                anyhow::bail!("journald input is only supported on Linux builds");
            }
        }
        InputConfig::WindowsEventLog { log, name } => {
            #[cfg(target_os = "windows")]
            {
                let source = name.unwrap_or_else(|| format!("windows-{}", log));
                windows_eventlog::WindowsEventLogInput::new(log, source)
                    .run(sender, shutdown)
                    .await
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = log;
                let _ = name;
                anyhow::bail!("windows_event_log input is only supported on Windows builds");
            }
        }
    }
}
