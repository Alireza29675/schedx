# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

schedx is a local-first scheduler CLI written in Rust that orchestrates commands, AI agent prompts, and webhooks. All state lives as plain files under `~/.schedx/` (or `SCHEDX_HOME`). The scheduler logic is backend-agnostic — systemd/launchd provide the heartbeat tick, but all scheduling decisions happen in Rust.

## Build & Development Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build (LTO, stripped)
cargo test --locked                  # Run all tests
cargo test --locked -- <test_name>   # Run a single test
cargo fmt --check                    # Check formatting
cargo fmt                            # Auto-format
cargo clippy --all-targets -- -D warnings  # Lint (CI-strict)
```

Tests require `SCHEDX_BACKEND=none` (set automatically by the test harness). The test harness creates a temp `SCHEDX_HOME` per test for isolation.

## Architecture

The codebase is split by responsibility:

- **`src/cli.rs`** — Clap command definitions
- **`src/commands/`** — User-facing command handlers (add, list, run, pause, etc.) plus hidden `_dispatch` and `_exec`
- **`src/engine/`** — Runtime: dispatcher (due-job detection), executor (run/prompt/webhook execution), file locking, log rotation
- **`src/model/`** — Serializable domain types: Job, Action (Run/Prompt/Webhook), JobSchedule, RunRecord, Config
- **`src/schedule/`** — Schedule parsing: cron, `in Xm/h/d`, `every Xm/h/d`, ISO-8601
- **`src/store/`** — Filesystem persistence: atomic writes (temp+rename+fsync), state, history (append-only JSONL), backups, paths
- **`src/backend/`** — System scheduler integration (systemd, launchd, none)
- **`src/output/`** — Human-readable tables and JSON formatting
- **`src/util/`** — nanoid generation, secret redaction

### Core Execution Flow

1. **`_dispatch`** (triggered by systemd timer/launchd agent/daemon loop) takes a dispatch lock, finds due jobs, writes `in_flight` claims, spawns `_exec` subprocesses
2. **`_exec`** verifies the claim, takes a per-job lock (overlap prevention), executes the action, captures output to log files, updates `last_run`/`run_count`, appends to `run-history.jsonl`

### Three Locking Scopes

- **State lock** (`locks/state.lock`) — guards `jobs.json` mutations
- **Dispatch lock** (`locks/dispatch.lock`) — one dispatcher tick at a time
- **Job lock** (`locks/job-{id}.lock`) — prevents concurrent runs of same job

### Key Design Decisions

- `jobs.json` is source of truth, not systemd/launchd units
- All writes are atomic (temp file → fsync → rename → parent fsync)
- History is append-only (`run-history.jsonl`); `last_run` in jobs.json is just a summary
- `--json` output is a stable public interface used by AI agents
- Default concurrency policy: `forbid` (overlap recorded as `skipped_overlap`)
- One-shot jobs transition to `completed` after execution
- Catch-up policy: `latest` (run once for most recent due time, not per missed tick)

### Exit Codes

0=success, 1=internal error, 2=usage error, 3=job not found, 4=schedule parse error, 5=security violation, 6=backend unavailable

## Testing

Tests are CLI-first integration tests using `assert_cmd` + `predicates`. The harness (`tests/helpers/mod.rs`) creates isolated temp directories with `SCHEDX_BACKEND=none`.

Test files are task-oriented: `cli_add.rs`, `cli_list.rs`, `cli_run.rs`, `cli_dispatch.rs`, `cli_lifecycle.rs`, `cli_history.rs`, `cli_agent.rs`, `schedule_parse.rs`.

## Security Invariants

- Default command execution uses `shell-words` argv splitting (no shell). Shell mode requires explicit `--shell` flag.
- Webhooks require `https://` by default; `http://` needs `config allow_insecure_http true`
- Sensitive headers (authorization, token, api-key, cookie) are redacted in output
- Data directory: `0700`, data files: `0600`

## Commit & PR Conventions

Commit messages use **conventional commits** with a `type(scope)` prefix: `fix(ci):`, `ci(release):`, `docs:`, `refactor:`, `feat:`, etc. Scope is optional but preferred when the change targets a specific area (e.g., `scheduler`, `ci`, `actions`). The subject is lowercase and imperative.

PRs target the `main` branch. Releases are triggered automatically when the version in `Cargo.toml` changes on `main`.

## Rust Toolchain

- MSRV: 1.85.0 (pinned in `rust-toolchain.toml`)
- Edition: 2024
- Clippy: pedantic warnings enabled (with specific allows for `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`)
- Format: `max_width = 100`, `use_field_init_shorthand = true`
