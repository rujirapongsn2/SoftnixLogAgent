# Test Cases & Scenarios

> 📝 Remember: every completed feature must add or update a relevant test case (unit, integration, or manual scenario). Update this document alongside code.

## 1. Configuration Validation & `--check`
- **Scenario:** Run configuration validation without starting inputs.
- **Steps:** `cargo run -- --config configs/agent.syslog.toml --check`.
- **Expected:** Process exits with `INFO configuration is valid`. If config is broken (e.g., bad facility, missing file), command fails with descriptive error.

## 2. STDIN Input Flow
- **Scenario:** Verify stdin ingestion and IOC tagging.
- **Steps:**
  1. Ensure config contains:
     ```toml
     [[inputs]]
     type = "stdin"
     name = "stdin"
     ```
  2. Run `cargo run -- --config configs/agent.dev.toml`.
  3. Type `WARN routerd[42]: 2024-02-01T01:02:03Z drop src=10.0.0.1 dst=6.6.6.6` and press Enter.
- **Expected:** STDOUT line resembling `... [stdin] WARN ... [ioc:ip=10.0.0.1,ip=6.6.6.6]`.

## 3. File Tail Input
- **Scenario:** Tail a live file.
- **Steps:**
  1. Configure:
     ```toml
     [[inputs]]
     type = "file_tail"
     path = "./tmp.log"
     read_from_beginning = true
     poll_interval_ms = 250
     ```
  2. `touch tmp.log` and run `cargo run -- --config configs/agent.dev.toml`.
  3. In another shell, `echo "test line" >> tmp.log`.
- **Expected:** Agent prints `test line` sourced from `tmp.log` within poll interval.

## 4. TCP Listener Input
- **Scenario:** Ingest logs over TCP.
- **Steps:**
  1. Configure:
     ```toml
     [[inputs]]
     type = "tcp_listener"
     bind = "127.0.0.1:9000"
     name = "tcp"
     ```
  2. Run agent, then `nc 127.0.0.1 9000` and send `hello over tcp`.
- **Expected:** STDOUT shows `hello over tcp` with `[tcp:peer]` source.

## 5. UDP Listener Input
- **Scenario:** Ingest datagrams.
- **Steps:**
  1. Configure:
     ```toml
     [[inputs]]
     type = "udp_listener"
     bind = "0.0.0.0:9001"
     name = "udp-ingest"
     ```
  2. Start agent; run `echo "udp event" | nc -u 127.0.0.1 9001`.
- **Expected:** Event printed immediately with `[udp-ingest:127.0.0.1:port] udp event`.

## 6. Process Runner Input
- **Scenario:** Capture stdout/stderr from a command.
- **Steps:**
  1. Configure:
     ```toml
     [[inputs]]
     type = "process"
     program = "bash"
     args = ["-c", "for i in 1 2; do echo process $i; sleep 1; done"]
     name = "proc"
     ```
  2. Run agent.
- **Expected:** Events `process 1`, `process 2` tagged with `proc:stdout`; process exits cleanly.

## 6.1 Process Runner Health Check (Linux)
- **Scenario:** Ensure simple Linux process (e.g., `ls`) can be invoked and monitored.
- **Steps:**
  1. Configure:
     ```toml
     [[inputs]]
     type = "process"
     program = "/bin/bash"
     args = ["-c", "ls /tmp && echo done"]
     name = "proc-check"
     ```
  2. Run agent once; process finishes immediately.
- **Expected:** Agent emits one or more lines from `ls /tmp`, followed by `done`; source tag `proc-check:stdout`; no lingering child processes.

## 7. Journald Input (Linux)
- **Scenario:** Follow systemd logs.
- **Steps:**
  1. On Linux, configure:
     ```toml
     [[inputs]]
     type = "journald"
     units = ["ssh.service"]
     name = "journald"
     ```
  2. Trigger a journal entry (e.g., SSH login).
- **Expected:** Corresponding journal message appears with source `journald` or `ssh.service`.

## 8. Windows Event Log Input (Windows)
- **Scenario:** Stream Windows event logs via PowerShell.
- **Steps:**
  1. On Windows, configure:
     ```toml
     [[inputs]]
     type = "windows_event_log"
     log = "System"
     name = "win-events"
     ```
  2. Run the agent and trigger an event (e.g., restart a service or check Windows event viewer).
- **Expected:** JSON payloads from `Get-WinEvent` appear with source `win-events`; child PowerShell process stops on Ctrl+C.

## 9. Syslog Output (UDP/TCP)
- **Scenario:** Validate syslog sink.
- **Steps:**
  1. Start listener: `nc -ul 5514` (UDP) or `nc -l 5514` (TCP).
  2. Run agent with `configs/agent.syslog.toml` and send STDIN events.
- **Expected:** `nc` displays RFC3164-formatted lines; agent terminal stays quiet except for logs.

## 10. Pipeline Normalization & Metadata
- **Scenario:** Ensure level/app/timestamp parsing and IoC extraction.
- **Steps:** run `cargo test pipeline::tests`.
- **Expected:** All tests pass; metadata fields populated as asserted.

## 10.1 Normalized Record Inspection
- **Scenario:** Verify structured fields (hostname/app/pid/key-values) produced by the parser.
- **Steps:**
  1. Run the targeted unit test `cargo test pipeline::tests::normalizes_syslog_structure`.
  2. Optional manual check:
     - Run `cargo run -- --config configs/agent.dev.toml --debug-events`.
     - Type `Oct 12 10:00:00 host01 nginx[123]: GET /foo status=200 latency=10ms`.
- **Expected:** Unit test passes; debug output shows `hostname=host01`, `app_name=nginx`, `pid=123`, and key-values `status=200`, `latency=10ms`.

## 11. UDP Listener Unit Test
- **Scenario:** Regression for UDP module.
- **Steps:** run `cargo test inputs::udp_listener::tests::receives_datagram`.
- **Expected:** Test passes, confirming datagrams are processed with shutdown-safe behavior.

## 12. Config Validation Unit Tests
- **Scenario:** Syslog facility guard.
- **Steps:** run `cargo test config::tests::rejects_invalid_facility`.
- **Expected:** Test passes and ensures invalid facilities are rejected.

---
- Keep expanding this file alongside code. When adding new outputs, pipeline stages, or security controls, specify how to verify them (automated test + manual scenario if needed).
