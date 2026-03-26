# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and the project follows Semantic Versioning.

## [Unreleased]

## [0.2.0] - 2026-03-26

### Added
- `schedx edit` command to modify job properties after creation (prompt, command, name, agent, timeout, schedule).
- Before/after diff output for edits in both human-readable and JSON formats.
- Full action details in `schedx get` output: prompt text, command, agent, webhook URL, method, and headers.
- Input validation (length limits, type constraints) and secret redaction in edit diffs.
- Schedule change resets scheduling anchor to prevent catchup bursts.
- In-flight run warning when changing schedule mid-execution.
- TTY hint for `--prompt-stdin` interactive usage.
- Name uniqueness check on rename.

## [0.1.0] - 2026-03-10

- Initial public release candidate.
- Scheduler CLI for commands, prompts, and webhooks.
- JSON output, logs, history, backups, and system backend repair commands.
