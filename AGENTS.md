# Repository Guidelines

## Project Structure & Module Organization
Keep `Cargo.toml`, `rust-toolchain.toml`, and workspace docs (such as `PRD.md` and this guide) in the repository root. Runtime code belongs in `src/`, whose subfolders mirror the PRD pipeline (inputs, pipeline, threat_intel, outputs). Configuration samples live under `configs/` (for example `configs/agent.dev.toml`) and should reference artifacts from `assets/ti/` where the offline SQLite Threat Intel database is stored. Put integration tests in `tests/` and their fixtures under `tests/data/`, keeping sample logs and TI snapshots there for repeatable runs.

## Build, Test, and Development Commands
- `cargo fmt` — apply the enforced Rust formatting profile before committing.
- `cargo clippy --all-targets -- -D warnings` — static analysis; treat warnings as build failures.
- `cargo test --all-features` — run unit + integration coverage, including enrichment paths.
- `cargo run --bin softnix_agent -- --config configs/agent.dev.toml` — run the agent locally with the referenced TOML config.

## Coding Style & Naming Conventions
Follow the default `rustfmt` width and 4-space indentation. Modules and files remain `snake_case`, structs/enums are `CamelCase`, and config keys should mirror the TOML paths (e.g., `threat_intel.enable_offline`). Use lowercase underscore-delimited log tags and run `cargo fmt` + `cargo clippy` before opening a PR.

## Testing Guidelines
Unit tests sit next to implementation files, while scenario tests go under `tests/` with filenames ending in `_spec.rs`. Prefer descriptive names such as `threat_intel_blocks_malicious_ip`. Keep fixtures small by trimming sample logs to 10–20 lines and using the `tests/data/ti_small.db` snapshot. Execute `cargo test threat_intel` before touching enrichment code and add regression tests whenever a parser or output format changes. Track coverage with `cargo tarpaulin --ignore-tests` when possible.

## Commit & Pull Request Guidelines
The repository history is still bootstrapping, so follow the Conventional Commits shape: `type(scope): concise summary` (e.g., `feat(pipeline): add journald tailer`). Reference the PRD section that motivated the change when helpful, and keep commits focused. Pull requests should describe the behavior change, list validation commands, link tracking tickets, and attach logs or screenshots when touching deployment artifacts. Request reviews from both a pipeline maintainer and a threat-intel maintainer whenever a change spans those boundaries.

## Security & Configuration Tips
Treat `configs/` files with secrets as templates only; keep real credentials in environment overrides or OS-specific secret stores. Run the agent under a dedicated user whose service unit drops capabilities. Before publishing binaries, verify `ti.db` does not contain customer data and ensure TLS is enabled in `outputs.syslog` when shipping to external collectors.
