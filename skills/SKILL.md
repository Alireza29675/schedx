---
name: schedx
description: >
  Schedule recurring jobs, AI agent prompts, and webhooks from the CLI using schedx.
  Use when the user wants to schedule, automate, or run something on a timer — cron jobs,
  periodic prompts, webhook calls, one-shot delayed tasks, or checking on scheduled work.
---

# schedx — Scheduler CLI

schedx is a local-first scheduler that runs shell commands, AI agent prompts, and
HTTP webhooks on a schedule. All state lives as plain files under `~/.schedx/`.
Every command supports `--json` for structured output.

## Three Action Types

**Run** — execute a shell command:
```bash
schedx add "every 1h" --run "echo hello"
schedx add "0 2 * * *" --run "pg_dump mydb > backup.sql" --shell
```

**Prompt** — send a prompt to a registered AI agent:
```bash
schedx add "0 9 * * 1-5" --prompt "Summarize today's open PRs"
```

**Webhook** — make an HTTP request:
```bash
schedx add "every 5m" --webhook https://hooks.slack.com/T/B/X \
  --method POST --header "Content-Type: application/json" \
  --body '{"text":"ping"}'
```

## Schedule Formats

| Format | Example | Meaning |
|--------|---------|---------|
| Cron | `0 9 * * 1-5` | 9am weekdays |
| Interval | `every 30m` | Every 30 minutes |
| Interval | `every 10s` | Every 10 seconds |
| One-shot | `in 30m` | 30 minutes from now |
| Bare duration | `5m` | Same as `in 5m` |
| ISO-8601 | `2026-04-01T03:00:00Z` | Exact timestamp |

Units: `s` (seconds), `m` (minutes), `h` (hours), `d` (days).

## Core Commands

```bash
schedx add "<schedule>" --run|--prompt|--webhook  # Create a job
schedx list [--all] [--status X] [--tag X] [--json] # List jobs
schedx get <id|name> [--json]                     # Job details
schedx run <id|name>                              # Trigger now
schedx edit <id|name> [--name|--schedule|...]     # Modify
schedx pause <id|name>                            # Pause
schedx resume <id|name>                           # Resume
schedx skip <id|name> [--times N]                 # Skip next N runs
schedx rm <id|name> [--force]                     # Remove
schedx logs <id|name> [--run <run-id>]            # View output
schedx history [id|name] [--limit N] [--json]     # Run history
schedx agent add <name> --bin <path> [--arg X]... # Register agent
schedx agent rm <name>                            # Remove agent
schedx agent list [--json]                        # List agents
schedx agent default <name>                       # Set default agent
schedx config [key] [value]                       # Read/set config
schedx repair [--json]                            # Fix inconsistencies
schedx upgrade [--force]                          # Update schedx
```

## Key Behaviors

- **No shell by default.** Commands are argv-split. Use `--shell` for shell interpretation.
- **HTTPS required for webhooks.** HTTP needs `schedx config allow_insecure_http true`.
- **Overlap forbidden by default.** Concurrent runs of the same job are prevented.
- **One-shot jobs auto-complete.** They transition to `completed` after execution.
- **Auto-archive.** Completed one-shot jobs are archived after 48h (configurable).
- **Secrets are redacted.** Authorization headers and tokens are masked in output.
- **Default timeout: 300s.** Override per-job with `--timeout` or globally via config.
- **Data directory:** `~/.schedx/` (override with `SCHEDX_HOME`).

## Exit Codes

0=success, 1=internal error, 2=usage error, 3=not found, 4=parse error, 5=security, 6=backend unavailable.

## Full Reference

For all flags, configuration keys, and detailed examples, check the latest reference:
https://github.com/Alireza29675/schedx/blob/main/docs/REFERENCE.md

Real-world recipes (DevOps, AI workflows, webhooks, monitoring):
https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md
