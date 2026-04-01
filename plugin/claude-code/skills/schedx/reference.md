# schedx Complete Reference

This is the detailed reference for schedx. For a quick overview, see [SKILL.md](SKILL.md).

## Installation

```bash
# From crates.io
cargo install schedx --locked

# From GitHub Releases
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh

# Installer env vars:
# SCHEDX_VERSION — specific version (default: latest)
# SCHEDX_INSTALL_DIR — directory (default: $HOME/.local/bin)
```

## Concepts

### Actions

| Action | Flag | Description |
|--------|------|-------------|
| **Run** | `--run` | Execute a shell command |
| **Prompt** | `--prompt` | Send a prompt to a registered AI agent |
| **Webhook** | `--webhook` | Make an HTTP request |

### Job Lifecycle

```
active → paused → active (via resume)
active → completed (one-shot jobs after execution)
completed → archived (auto-archive after configurable hours)
archived → completed (via unarchive)
```

### Agents

Agent profiles define how to invoke AI coding agents for `--prompt` jobs. Each profile specifies a binary, arguments, and whether to pass the prompt via stdin or as the last argument.

## Schedule Formats

### Cron expressions

Standard 5-field: `minute hour day-of-month month day-of-week`

```bash
schedx add "0 9 * * 1-5" --run "..."     # 9am weekdays
schedx add "*/15 * * * *" --run "..."     # Every 15 minutes
schedx add "0 0 1 * *" --run "..."        # First of every month
schedx add "30 14 * * 0" --run "..."      # 2:30pm every Sunday
```

### Human intervals (`every`)

```bash
schedx add "every 10s" --run "..."        # Every 10 seconds
schedx add "every 5m" --run "..."         # Every 5 minutes
schedx add "every 2h" --run "..."         # Every 2 hours
schedx add "every 1d" --run "..."         # Every day
```

Limits: minutes 1-59, hours 1-23, days 1-30. Seconds use interval-based scheduler (sub-minute precision); minutes+ convert to cron.

### One-shot timers (`in`)

```bash
schedx add "in 30s" --run "..."           # 30 seconds from now
schedx add "in 5m" --run "..."            # 5 minutes from now
schedx add "in 2h" --run "..."            # 2 hours from now
```

Bare durations work too: `schedx add "30m" --run "..."` = `schedx add "in 30m" --run "..."`.

### Absolute timestamps (ISO-8601)

```bash
schedx add "2026-04-01T03:00:00Z" --run "..."
schedx add "2026-03-30T15:30:00-07:00" --run "..."
```

Must be in the future. RFC-3339 format.

## Commands Reference

### Global flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (all commands that produce output) |
| `--version` | Print version |
| `--help` | Print help |

---

### `schedx add`

Add a new scheduled job. Exactly one action required.

```
schedx add <schedule> [flags]
```

**Action flags (exactly one required):**

| Flag | Description |
|------|-------------|
| `--run <command>` | Shell command to execute |
| `--prompt <text>` | Prompt text for an AI agent |
| `--webhook <url>` | URL to call |

**Common flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--name <name>` | -- | Human-readable job name |
| `--tag <tag>` | -- | Tag for filtering (repeatable) |
| `--timeout <secs>` | 300 | Kill job after N seconds |
| `--stdin` | false | Read command/prompt text from stdin |
| `--json` | false | Output job details as JSON |

**Run-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--shell` | false | Execute via `/bin/sh -lc` instead of argv splitting |
| `--workdir <dir>` | -- | Working directory for execution |
| `--on-failure <cmd>` | -- | Command to run if this job fails |
| `--on-failure-shell` | false | Use shell for the failure command |

**Prompt-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--agent <name>` | config default | Agent profile to use |

**Webhook-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--method <method>` | POST | HTTP method (GET, POST, PUT, PATCH, DELETE) |
| `--header <header>` | -- | HTTP header `Key: Value` (repeatable) |
| `--body <body>` | -- | Request body |

**Examples:**

```bash
# Shell command every hour
schedx add "every 1h" --run "echo hello"

# Shell with shell interpretation
schedx add "0 2 * * *" --run "pg_dump mydb > /backups/$(date +%F).sql" --shell

# AI prompt with specific agent
schedx add "0 9 * * 1-5" --prompt "Summarize today's PRs" --agent claude

# Webhook with headers and body
schedx add "every 5m" \
  --webhook https://hooks.slack.com/services/T/B/X \
  --method POST \
  --header "Content-Type: application/json" \
  --body '{"text":"ping"}'

# One-shot with timeout and name
schedx add "in 2h" --run "./deploy.sh staging" \
  --name "staging-deploy" --tag deploy --timeout 600

# Job with failure handler
schedx add "0 2 * * *" --run "./backup.sh" \
  --on-failure "curl -X POST https://alerts.example.com/failed" \
  --name "nightly-backup"
```

---

### `schedx list`

List scheduled jobs. Shows active jobs by default.

| Flag | Default | Description |
|------|---------|-------------|
| `--status <status>` | active | Filter: `active`, `paused`, `completed`, `archived` |
| `--tag <tag>` | -- | Filter by tag |
| `--all` | false | Show all including archived |
| `--json` | false | Output as JSON |

---

### `schedx get`

```
schedx get <id|name> [--json]
```

---

### `schedx run`

Manually trigger a job immediately.

```
schedx run <id|name>
```

---

### `schedx edit`

Modify an existing job. Only specified fields change.

| Flag | Description |
|------|-------------|
| `--name <name>` | New job name |
| `--schedule <expr>` | New schedule expression |
| `--prompt <text>` | New prompt text (prompt jobs) |
| `--run <command>` | New command (run jobs) |
| `--agent <name>` | New agent (prompt jobs) |
| `--timeout <secs>` | New timeout |
| `--json` | Output updated job as JSON |

---

### `schedx rm`

```
schedx rm <id|name> [--force]
```

---

### `schedx pause` / `schedx resume`

```
schedx pause <id|name>
schedx resume <id|name>
```

---

### `schedx unarchive`

Restore an archived job back to completed.

```
schedx unarchive <id|name>
```

---

### `schedx skip`

Skip the next N scheduled runs.

```
schedx skip <id|name> [--times N]
```

Default: 1 run.

---

### `schedx logs`

```
schedx logs <id|name> [--run <run-id>] [--lines <N>]
```

---

### `schedx history`

```
schedx history [id|name] [--limit N] [--json]
```

Default limit: 20. Without job ID, shows all jobs.

---

### `schedx agent`

```bash
schedx agent add <name> --bin <path> [--arg <arg>]... [--prompt-stdin]
schedx agent rm <name>
schedx agent list [--json]
schedx agent default <name>
```

**Examples:**

```bash
schedx agent add claude --bin claude \
  --arg "-p" --arg "--permission-mode" --arg "auto"

schedx agent add codex --bin codex \
  --arg "exec" --arg "--full-auto"

schedx agent add gemini --bin gemini \
  --arg "-p" --arg "--yolo"

schedx agent default claude
```

---

### `schedx config`

```
schedx config                     # Show all
schedx config <key>               # Get value
schedx config <key> <value>       # Set value
```

**Configuration keys:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_agent` | string | -- | Default agent for `--prompt` jobs |
| `backup_count` | integer | 10 | State backups to keep |
| `log_retention_days` | integer | 30 | Days to keep logs |
| `default_timeout_seconds` | integer | 300 | Default job timeout |
| `allow_insecure_http` | boolean | false | Allow `http://` webhooks |
| `backend` | string | auto | Backend: `auto`, `systemd`, `launchd`, `none` |
| `on_failure` | string | -- | Global failure command |
| `on_failure_shell` | boolean | false | Use shell for global failure command |
| `max_concurrent_fallbacks` | integer | 10 | Max concurrent fallback processes |
| `archive_after_hours` | integer | 48 | Hours before auto-archive (0 = disabled) |

---

### `schedx upgrade`

```
schedx upgrade [--force]
```

---

### `schedx repair`

Fix backend/state inconsistencies.

```
schedx repair [--json]
```

---

### `schedx daemon`

Run dispatcher in foreground.

```
schedx daemon [--interval <secs>]
```

Default interval: 10 seconds.

## JSON Output for Scripting

```bash
JOB_ID=$(schedx add "every 1h" --run "echo hi" --json | jq -r '.id')
schedx list --json | jq '.[].name'
schedx history "$JOB_ID" --json --limit 1 | jq '.[0].status'
```

## Data Directory

```
~/.schedx/
├── jobs.json              # All job definitions
├── config.json            # Config + agent profiles
├── run-history.jsonl      # Append-only run records
├── locks/                 # File locks
├── logs/                  # Per-run output logs
└── backups/               # State snapshots
```

Override with `SCHEDX_HOME` env var. Permissions: dirs `0700`, files `0600`.

## Security

- **No shell by default.** `shell-words` argv parsing. Shell needs explicit `--shell`.
- **HTTPS required for webhooks.** HTTP blocked unless `allow_insecure_http = true`.
- **Secrets redacted.** Headers matching authorization/token/api-key/cookie are masked.
- **Owner-only permissions.** `0700` dirs, `0600` files.
- **No background network.** schedx never phones home.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Internal error |
| 2 | Usage error |
| 3 | Job not found |
| 4 | Schedule parse error |
| 5 | Security violation |
| 6 | Backend unavailable |

## Run Statuses

| Status | Meaning |
|--------|---------|
| `success` | Exit code 0 |
| `failed` | Non-zero exit code |
| `timeout` | Exceeded timeout |
| `skipped_overlap` | Previous run still in progress |
| `internal_error` | schedx internal failure |

## Failure Handling

```bash
# Per-job failure handler
schedx add "0 2 * * *" --run "./backup.sh" \
  --on-failure "curl -X POST https://alerts.example.com/failed"

# Global fallback
schedx config on_failure "notify-send 'Job failed'"
```

Per-job `--on-failure` takes precedence over global.

## Troubleshooting

1. Check backend: `schedx repair`
2. Verify job is active: `schedx get <id>`
3. Check logs: `schedx logs <id>`
4. Run manually: `schedx run <id>`
5. Daemon mode (no systemd/launchd): `schedx daemon --interval 10`
