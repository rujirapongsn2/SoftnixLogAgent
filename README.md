# Softnix Log Collector Agent

Rust-based agent prototype that ingests logs from multiple sources, normalizes them into `LogEvent`s, and emits them to configurable outputs. The current snapshot focuses on a core runtime, stdin/file-tail inputs, and stdout/syslog sinks to demonstrate end-to-end flow.

## Prerequisites
- Rust toolchain (1.91+). Install via `curl https://sh.rustup.rs -sSf | sh -s -- -y` if needed.
- macOS or Linux with access to target log files (for file-tail input).

## Build & Run
```bash
# format + build dependencies
cargo fmt
cargo build

# run the agent (defaults to configs/agent.dev.toml)
cargo run -- --config configs/agent.dev.toml

# validate configuration only
cargo run -- --config configs/agent.syslog.toml --check
```
The sample config enables both stdin and a file-tail input. Tail paths may need adjustment for your OS (e.g., `/var/log/system.log` on macOS or `/var/log/syslog` on Linux).

### Configuring Syslog Output
Use `configs/agent.syslog.toml` to emit RFC3164 or RFC5424 frames over UDP/TCP. Example workflow:
1. Start a local syslog receiver:
   - UDP: `nc -ul 5514`
   - TCP: `nc -l 5514`
2. Run the agent with `cargo run -- --config configs/agent.syslog.toml`.
3. Feed events through STDIN (enabled in the config) or other inputs; the agent terminal stays quiet while `nc` prints syslog-formatted records.
4. Switch `protocol`, `format`, or `address` in the config to match your collector, rerun, and validate again.

### Configuring Inputs
- **STDIN:**
  ```toml
  [[inputs]]
  type = "stdin"
  name = "stdin"
  ```
  Start the agent and type lines; each line becomes a `LogEvent`.
- **File tail:**
  ```toml
  [[inputs]]
  type = "file_tail"
  path = "./tmp.log"
  read_from_beginning = true
  poll_interval_ms = 250
  ```
  Append lines to the target file to verify events are emitted.
- **TCP listener:**
  ```toml
  [[inputs]]
  type = "tcp_listener"
  bind = "0.0.0.0:9000"
  name = "tcp-ingest"
  ```
  Use `nc localhost 9000` to stream data over TCP.
- **UDP listener:**
  ```toml
  [[inputs]]
  type = "udp_listener"
  bind = "0.0.0.0:9001"
  name = "udp-ingest"
  ```
  Send datagrams via `nc -u localhost 9001` or `logger --udp`.
- **Process runner:**
  ```toml
  [[inputs]]
  type = "process"
  program = "/usr/bin/tail"
  args = ["-F", "/var/log/nginx/error.log"]
  name = "tail-nginx"
  ```
  The agent captures stdout/stderr lines from the spawned command.
- **Journald (Linux only):**
  ```toml
  [[inputs]]
  type = "journald"
  units = ["nginx.service"]
  name = "journald"
  ```
  Requires running on Linux with `journalctl` available (internally runs `journalctl -f`).
- **Windows Event Log (Windows only):**
  ```toml
  [[inputs]]
  type = "windows_event_log"
  log = "System" # any log name accepted by Get-WinEvent
  name = "win-events"
  ```
  Runs `powershell.exe Get-WinEvent -Wait` to stream events; only available on Windows hosts.

## Running Tests
```bash
cargo fmt --check
cargo test
```
Tests currently verify the code compiles; add unit/integration tests under `tests/` as modules evolve.
