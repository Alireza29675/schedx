# schedx Reference

Complete reference for schedx — a local-first scheduler CLI for recurring jobs, AI agent prompts, and webhooks.

## Overview

schedx schedules shell commands (`--run`), AI agent prompts (`--prompt`), and HTTP webhooks (`--webhook`) using cron expressions, human intervals, one-shot timers, or ISO-8601 timestamps. All state lives as plain files under `~/.schedx/`. Every command supports `--json` for structured output.

## Installation

### From crates.io

```bash
cargo install schedx --locked
```

### From GitHub Releases

```bash
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
```

Environment variables for the installer:
- `SCHEDX_VERSION` — install a specific version (default: latest)
- `SCHEDX_INSTALL_DIR` — installation directory (default: `$HOME/.local/bin`)
- `SCHEDX_REPO` — GitHub repository (default: `Alireza29675/schedx`)

### Building from source

```bash
git clone https://github.com/Alireza29675/schedx.git
cd schedx
cargo build --release
# Binary at target/release/schedx
```

Requires Rust 1.85.0+ (MSRV).

### Verifying installation

```bash
schedx --version
```

## Quick Start

```bash
# Run something every hour
schedx add "every 1h" --run "echo hello"

# One-shot reminder in 30 minutes
schedx add "in 30m" --run "say 'break time'"

# AI prompt on weekday mornings
schedx add "0 9 * * 1-5" --prompt "Summarize today's open PRs"

# Check your jobs
schedx list

# View logs and history
schedx logs <job-id>
schedx history

# Manage jobs
schedx pause <job-id>
schedx resume <job-id>
schedx rm <job-id>
```

## Concepts

### Jobs

A job is a scheduled task with an action, a schedule, and a status. Jobs are identified by a unique ID (auto-generated nanoid) and an optional human-readable name.

### Actions

Every job performs one of three actions:

| Action | Flag | Description |
|--------|------|-------------|
| **Run** | `--run` | Execute a shell command |
| **Prompt** | `--prompt` | Send a prompt to a registered AI agent |
| **Webhook** | `--webhook` | Make an HTTP request |

### Job Lifecycle

Jobs move through these statuses:

```
active → paused → active (via resume)
active → completed (one-shot jobs after execution)
completed → archived (auto-archive after configurable hours)
archived → completed (via unarchive)
```

- **active** — scheduled and running normally
- **paused** — schedule suspended, can be resumed
- **completed** — one-shot job that has executed
- **archived** — auto-archived after completion (configurable via `archive_after_hours`)

### Agents

Agent profiles define how to invoke AI coding agents for `--prompt` jobs. Each profile specifies a binary, arguments, and whether to pass the prompt via stdin or as the last argument.

## Schedule Formats

### Cron expressions

Standard 5-field cron format: `minute hour day-of-month month day-of-week`

```bash
schedx add "0 9 * * 1-5" --run "..."     # 9am weekdays
schedx add "*/15 * * * *" --run "..."     # Every 15 minutes
schedx add "0 0 1 * *" --run "..."        # First of every month at midnight
schedx add "30 14 * * 0" --run "..."      # 2:30pm every Sunday
```

### Human intervals (`every`)

Recurring schedules using natural syntax:

```bash
schedx add "every 10s" --run "..."        # Every 10 seconds
schedx add "every 5m" --run "..."         # Every 5 minutes
schedx add "every 2h" --run "..."         # Every 2 hours
schedx add "every 1d" --run "..."         # Every day
```

**Limits:** minutes 1-59, hours 1-23, days 1-30. Seconds use an interval-based scheduler (sub-minute precision); minutes and above convert to cron expressions.

### One-shot timers (`in`)

Run once after a delay:

```bash
schedx add "in 30s" --run "..."           # 30 seconds from now
schedx add "in 5m" --run "..."            # 5 minutes from now
schedx add "in 2h" --run "..."            # 2 hours from now
schedx add "in 1d" --run "..."            # 1 day from now
```

Bare durations also work as one-shot: `schedx add "30m" --run "..."` is equivalent to `schedx add "in 30m" --run "..."`.

### Absolute timestamps (ISO-8601)

Run once at an exact time:

```bash
schedx add "2026-04-01T03:00:00Z" --run "..."
schedx add "2026-03-30T15:30:00-07:00" --run "..."
```

Must be in the future. Uses RFC-3339 format.

## Commands Reference

### Global flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (supported on all commands that produce output) |
| `--version` | Print version |
| `--help` | Print help |

---

### `schedx add`

Add a new scheduled job. Exactly one action (`--run`, `--prompt`, or `--webhook`) is required.

```
schedx add <schedule> [flags]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<schedule>` | Schedule expression (cron, `in Xm/h/d`, `every Xm/h/d`, bare duration, or ISO-8601) |

**Action flags (exactly one required):**

| Flag | Description |
|------|-------------|
| `--run <command>` | Shell command to execute |
| `--prompt <text>` | Prompt text for an AI agent |
| `--webhook <url>` | URL to call |

**Common flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--name <name>` | — | Human-readable job name |
| `--tag <tag>` | — | Tag for filtering (repeatable) |
| `--timeout <secs>` | 300 | Kill job after N seconds |
| `--stdin` | false | Read command/prompt text from stdin |
| `--json` | false | Output job details as JSON |

**Run-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--shell` | false | Execute via `/bin/sh -lc` instead of argv splitting |
| `--workdir <dir>` | — | Working directory for execution |
| `--on-failure <cmd>` | — | Command to run if this job fails |
| `--on-failure-shell` | false | Use shell execution for the failure command |

**Prompt-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--agent <name>` | config default | Agent profile to use |

**Webhook-specific flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--method <method>` | POST | HTTP method (GET, POST, PUT, PATCH, DELETE) |
| `--header <header>` | — | HTTP header in `Key: Value` format (repeatable) |
| `--body <body>` | — | Request body |

**Examples:**

```bash
# Shell command every hour
schedx add "every 1h" --run "echo hello"

# Shell command with shell interpretation
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
  --on-failure "curl -X POST https://alerts.example.com/backup-failed" \
  --name "nightly-backup"
```

---

### `schedx up`

Apply a declarative manifest (`schedx.yaml`), reconciling the job store to match it. See [Declarative Manifests](#declarative-manifests-schedxyaml) for the file format and semantics.

```
schedx up [flags]
```

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <path>` | `schedx.yaml` | Path to the manifest file |
| `--dry-run` | false | Show what would change without applying anything |
| `--force` | false | Accept a moved manifest file (updates the recorded path) |
| `--json` | false | Output the reconcile report as JSON |

`up` is idempotent: running it twice in a row changes nothing the second time. Jobs added with `schedx add` are never touched.

**Examples:**

```bash
# Preview the changes
schedx up --dry-run

# Apply
schedx up

# Apply a manifest from another directory
schedx up -f ~/projects/backups/schedx.yaml
```

---

### `schedx down`

Remove the jobs a declarative manifest owns. Jobs added with `schedx add` are never touched.

```
schedx down [flags]
```

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <path>` | `schedx.yaml` | Path to the manifest file |
| `--manifest <name>` | — | Target a manifest by name instead of by file (works after the yaml is gone) |
| `--dry-run` | false | Show what would be removed without removing anything |
| `--force` | false | Skip the recorded-path check (after moving the manifest file) |
| `--json` | false | Output the removal report as JSON |

`down` works even after the manifest file is deleted: run it from the manifest's directory, or pass `--manifest <name>`.

**Examples:**

```bash
# Preview
schedx down --dry-run

# Remove everything this manifest created
schedx down

# The yaml is gone but the jobs remain
schedx down --manifest backups
```

---

### `schedx list`

List scheduled jobs. Shows active jobs by default.

```
schedx list [flags]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--status <status>` | active | Filter by status: `active`, `paused`, `completed`, `archived` |
| `--tag <tag>` | — | Filter by tag |
| `--all` | false | Show all jobs including archived (conflicts with `--status`) |
| `--json` | false | Output as JSON |

**Examples:**

```bash
schedx list                    # Active jobs
schedx list --all              # Everything
schedx list --status paused    # Only paused jobs
schedx list --tag security     # Filter by tag
schedx list --json             # JSON output
```

---

### `schedx get`

Show details for a specific job.

```
schedx get <id|name> [--json]
```

---

### `schedx run`

Manually trigger a job immediately, regardless of its schedule.

```
schedx run <id|name>
```

The job executes in the foreground and outputs its result.

---

### `schedx edit`

Modify an existing job's properties. Only specified fields are changed.

```
schedx edit <id|name> [flags]
```

| Flag | Description |
|------|-------------|
| `--name <name>` | New job name |
| `--schedule <expr>` | New schedule expression |
| `--prompt <text>` | New prompt text (prompt jobs only) |
| `--prompt-stdin` | Read new prompt from stdin (prompt jobs only) |
| `--run <command>` | New command (run jobs only) |
| `--agent <name>` | New agent (prompt jobs only) |
| `--timeout <secs>` | New timeout |
| `--json` | Output updated job as JSON |

**Examples:**

```bash
schedx edit my-job --name "new-name"
schedx edit my-job --schedule "every 2h"
schedx edit my-job --timeout 600
```

---

### `schedx rm`

Remove a job.

```
schedx rm <id|name> [--force]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | false | Skip confirmation prompt |

---

### `schedx pause`

Pause a job's schedule. The job remains in state but won't be dispatched.

```
schedx pause <id|name>
```

---

### `schedx resume`

Resume a paused job.

```
schedx resume <id|name>
```

---

### `schedx unarchive`

Restore an archived job back to completed status.

```
schedx unarchive <id|name>
```

---

### `schedx skip`

Skip the next N scheduled runs of a recurring job.

```
schedx skip <id|name> [--times N]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--times <N>` | 1 | Number of runs to skip |
| `--json` | false | Output as JSON |

---

### `schedx logs`

View output logs for a job's runs.

```
schedx logs <id|name> [flags]
```

| Flag | Description |
|------|-------------|
| `--run <run-id>` | Show logs for a specific run (default: latest) |
| `--lines <N>` | Show last N lines |

---

### `schedx history`

View run history.

```
schedx history [id|name] [flags]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit <N>` | 20 | Maximum number of records |
| `--json` | false | Output as JSON |

When called without a job ID, shows history for all jobs.

---

### `schedx agent`

Manage agent profiles for `--prompt` jobs.

#### `schedx agent add`

```
schedx agent add <name> --bin <path> [flags]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--bin <path>` | (required) | Path to agent binary |
| `--arg <arg>` | — | Additional argument (repeatable) |
| `--prompt-stdin` | false | Pass prompt via stdin instead of as last argument |

**Examples:**

```bash
schedx agent add claude --bin claude \
  --arg "-p" --arg "--permission-mode" --arg "auto"

schedx agent add codex --bin codex \
  --arg "exec" --arg "--full-auto"
```

#### `schedx agent rm`

```
schedx agent rm <name>
```

#### `schedx agent list`

```
schedx agent list [--json]
```

#### `schedx agent default`

Set the default agent used when `--agent` is not specified on `schedx add --prompt`.

```
schedx agent default <name>
```

---

### `schedx config`

Read or set configuration values.

```
schedx config                     # Show all config
schedx config <key>               # Get a value
schedx config <key> <value>       # Set a value
```

**Configuration keys:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_agent` | string | — | Default agent for `--prompt` jobs |
| `backup_count` | integer | 10 | Number of state backups to keep |
| `log_retention_days` | integer | 30 | Days to keep log files |
| `default_timeout_seconds` | integer | 300 | Default job timeout |
| `allow_insecure_http` | boolean | false | Allow `http://` webhook URLs |
| `backend` | string | auto | Scheduling backend: `auto`, `systemd`, `launchd`, `none` |
| `on_failure` | string | — | Global failure command (applies to jobs without their own) |
| `on_failure_shell` | boolean | false | Use shell for global failure command |
| `max_concurrent_fallbacks` | integer | 10 | Maximum concurrent fallback processes |
| `archive_after_hours` | integer | 48 | Hours after completion before auto-archive (0 = disabled) |

---

### `schedx upgrade`

Upgrade schedx to the latest version.

```
schedx upgrade [--force]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | false | Force upgrade even if already on latest |

---

### `schedx repair`

Repair backend state and fix inconsistencies between `jobs.json` and the system scheduler (systemd/launchd).

```
schedx repair [--json]
```

---

### `schedx daemon`

Run the scheduler in the foreground. Useful for debugging or environments without systemd/launchd.

```
schedx daemon [--interval <secs>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--interval <secs>` | 10 | Dispatch tick interval in seconds |

---

### `schedx setup`

Install the schedx agent skill into your AI coding agents. The skill is embedded
in the binary and stamped with the current version, so it never drifts from the
schedx you have installed.

```
schedx setup [--agent <name>] [--all] [--force] [--dry-run] [--list]
```

| Flag | Description |
|------|-------------|
| `--agent <name>` | Install for one agent: `claude`, `codex`, `cursor`, `gemini`, `opencode` |
| `--all` | Install for every supported agent, not just detected ones |
| `--force` | Overwrite an existing skill even if it is already current |
| `--dry-run` | Show what would be installed without writing files |
| `--list` | Show installed skills and their versions |

With no flags it detects the agents on your machine and installs for each.
Install paths:

| Agent | Path |
|-------|------|
| Claude Code | `~/.claude/skills/schedx/` |
| Codex, Gemini CLI, opencode | `~/.agents/skills/schedx/` (the shared skills path they all read) |
| Cursor | `.cursor/skills/schedx/` in the current project (Cursor reads skills per project) |

For a one-line cross-agent install without schedx itself, see
[Agent Integration](#agent-integration).

## Declarative Manifests (schedx.yaml)

Declare all your schedules in one file, then `schedx up` to make the job store match it and `schedx down` to remove everything it created — think docker-compose for schedules.

### File format

```yaml
name: home-server          # optional — defaults to the directory name
defaults:                  # optional — fill fields a job omits
  timeout: 300
  tags: [declared]

jobs:                      # map keyed by job name
  backup-home:
    schedule: "every 6h"   # same syntax as `schedx add`
    run: "restic backup ~/"
    shell: true            # optional (run only)
    workdir: "~/"          # optional (run only)
    timeout: 600           # optional seconds
    tags: [backup]
    paused: false          # optional — see "Pause" below
    on_failure: "notify-send 'backup failed'"
    on_failure_shell: true

  daily-report:
    schedule: "0 9 * * 1-5"
    prompt: "Summarize yesterday's git activity"
    agent: claude          # optional (prompt only)

  ping:
    schedule: "every 15m"
    webhook: "https://example.com/hook"
    method: POST           # optional, default POST
    headers:
      Authorization: "Bearer ${HOOK_TOKEN}"
    body: '{"ping":true}'
```

Each job takes exactly one of `run`, `prompt`, or `webhook`, with the same validation and security rules as `schedx add` (HTTPS-required webhooks, no shell unless explicit, same length limits). Unknown keys are rejected — a typo is an error, never silently ignored.

### Environment variables

`${VAR}` in any string value expands from the environment at `up` time, so secrets stay out of the committed file:

```yaml
headers:
  Authorization: "Bearer ${HOOK_TOKEN}"
```

- A missing variable is a hard error (named, with the job context) — nothing is applied.
- `$${` escapes a literal `${`; a lone `$` is literal.
- Changing a variable's value counts as a change: the next `up` updates the job.

### How `up` reconciles

`up` diffs the manifest against what it last applied and against the live store, then applies the whole result in one atomic write:

| Situation | Action |
|-----------|--------|
| Job in yaml, not in store | created |
| Job unchanged | untouched (`up` twice = "No changes") |
| Yaml changed | updated **in place** — same id, run history preserved |
| Job edited imperatively (e.g. `schedx edit`) | drift corrected back to the yaml |
| Job removed imperatively (`schedx rm`) | recreated |
| Job removed from the yaml | removed from the store |
| Name owned by a job you added with `schedx add` | hard error — nothing applied, never adopted silently |

Jobs created by `up` carry a `managed_by` marker; **jobs added imperatively with `schedx add` are never touched by `up` or `down`.**

### Pause

`paused:` is three-state: `true` keeps the job paused, `false` keeps it active, and *omitting it* leaves runtime pause alone — `schedx pause`/`resume` keep working without `up` fighting you.

### One-shots

A completed one-shot with an unchanged spec stays completed (`up` never re-fires it). To re-fire one, change its spec — the job is recreated fresh.

### State and safety

- Each manifest records what it applied in `~/.schedx/manifests/<name>.json`. Two manifests resolving to the same name from different directories are refused (this is what prevents one project's `up` from pruning another's jobs); `--force` accepts a deliberate move.
- If the state file is lost (crash, deleted directory), ownership recovers from the markers on the jobs themselves: an unchanged manifest re-records its state automatically, but **modifying or removing a job whose ownership can't be confirmed requires `--force`** — a same-named manifest from another directory can never silently destroy jobs it didn't create.
- Validation reports **all** problems at once, and any error means **nothing** is applied.
- The whole reconcile lands in one atomic `jobs.json` write — there is no partially-applied state to recover from.

## Agent Integration

### Teaching your agent to use schedx (the skill)

schedx ships a `SKILL.md` that teaches AI coding agents how to drive it. One
canonical skill (`skills/schedx/`) is read natively by Claude Code, Codex,
Gemini CLI, Cursor, and opencode.

```bash
# Cross-agent, no schedx required for the install step:
npx skills add Alireza29675/schedx              # all detected agents
npx skills add Alireza29675/schedx -a codex     # one agent

# From the binary (no Node, version-matched, offline):
schedx setup            # detect and install
schedx setup --list     # what's installed
```

Both write the same skill. See [`schedx setup`](#schedx-setup) for the per-agent
install paths. (Gemini CLI support is verified against Google's published skills
docs, which route the skill via the shared `~/.agents/skills/` path.)

This is separate from registering an agent for `--prompt` jobs, below — the
skill teaches an agent to *call* schedx; a profile lets schedx *call* an agent.

### Registering agents

Before using `--prompt`, register at least one agent profile:

```bash
# Claude Code (Anthropic)
schedx agent add claude --bin claude \
  --arg "-p" --arg "--permission-mode" --arg "auto"

# Codex (OpenAI)
schedx agent add codex --bin codex \
  --arg "exec" --arg "--full-auto"

# Gemini CLI (Google)
schedx agent add gemini --bin gemini \
  --arg "-p" --arg "--yolo"

# OpenCode
schedx agent add opencode --bin opencode --arg "-p"

# Set a default
schedx agent default claude
```

### The `--prompt` action

When a prompt job fires, schedx:
1. Looks up the agent profile (from `--agent` flag or `default_agent` config)
2. Constructs the command: `<bin> <args...> "<prompt text>"`
3. Executes it as a subprocess with the configured timeout
4. Captures stdout/stderr to the run log file

### JSON output for scripting

Every command supports `--json`. This makes schedx scriptable by AI agents and CI pipelines:

```bash
# Get job ID from add
JOB_ID=$(schedx add "every 1h" --run "echo hi" --json | jq -r '.id')

# List jobs as JSON
schedx list --json | jq '.[].name'

# Check history
schedx history "$JOB_ID" --json --limit 1 | jq '.[0].status'
```

## Data & Storage

### Directory layout

All data lives under `~/.schedx/` (override with `SCHEDX_HOME` environment variable):

```
~/.schedx/
├── jobs.json              # Source of truth — all job definitions
├── config.json            # User configuration and agent profiles
├── manifests/             # Declarative manifest state (one file per `schedx up` manifest)
│   └── {name}.json
├── run-history.jsonl      # Append-only run records (one JSON object per line)
├── locks/                 # File locks
│   ├── state.lock         # Guards jobs.json mutations
│   ├── dispatch.lock      # One dispatcher at a time
│   └── job-{id}.lock      # Per-job overlap prevention
├── logs/                  # Per-run output logs
│   └── {run-id}.log
└── backups/               # State snapshots
    └── jobs-{timestamp}.json
```

### File permissions

- Data directory: `0700` (owner only)
- Data files: `0600` (owner only)

### Environment variables

| Variable | Description |
|----------|-------------|
| `SCHEDX_HOME` | Data directory (default: `~/.schedx/`) |
| `SCHEDX_BACKEND` | Override backend: `systemd`, `launchd`, `none` |

## Security

- **No shell by default.** Commands are split using `shell-words` (argv parsing). Shell interpretation requires the explicit `--shell` flag.
- **HTTPS required for webhooks.** `http://` URLs are blocked unless `allow_insecure_http` is set to `true`.
- **Secrets redacted.** Headers matching `authorization`, `token`, `api-key`, `cookie` (case-insensitive) are masked in output and logs.
- **Owner-only permissions.** Data directory is `0700`, files are `0600`.
- **No background network.** schedx never phones home. Upgrade checks read a cached local file.

Full security policy: [SECURITY.md](../SECURITY.md)

## Architecture

### Execution model

1. **Heartbeat tick**: systemd timer (Linux) or launchd agent (macOS) fires on a regular cadence
2. **Dispatch**: `schedx _dispatch` takes a dispatch lock, scans `jobs.json` for due jobs
3. **Claim**: For each due job, writes an `in_flight` claim to `jobs.json`
4. **Execute**: Spawns `schedx _exec <job-id>` subprocess per job
5. **Record**: `_exec` captures output, updates `last_run`, appends to `run-history.jsonl`

### Locking

| Lock | File | Purpose |
|------|------|---------|
| State lock | `locks/state.lock` | Guards `jobs.json` mutations |
| Dispatch lock | `locks/dispatch.lock` | One dispatcher tick at a time |
| Job lock | `locks/job-{id}.lock` | Prevents concurrent runs of same job |

### Atomic writes

All file writes use: temp file → fsync → atomic rename → parent directory fsync. This prevents corruption on crash.

### Backend integration

schedx owns all scheduling logic. The system backend (systemd/launchd) only provides a heartbeat tick — it fires `schedx _dispatch` on a regular cadence. If neither is available, `schedx daemon` runs the dispatch loop in the foreground.

### Catch-up policy

When the system was asleep or the dispatcher missed ticks, schedx runs the job once for the **latest** due time (not once per missed interval).

### Overlap policy

Default: `forbid`. If a job is still running when its next scheduled time arrives, the run is recorded as `skipped_overlap`.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Internal error |
| 2 | Usage error (bad flags, invalid arguments) |
| 3 | Job not found |
| 4 | Schedule parse error |
| 5 | Security violation (e.g., insecure HTTP blocked) |
| 6 | Backend unavailable (systemd/launchd not working) |

## Run Statuses

| Status | Meaning |
|--------|---------|
| `success` | Job completed with exit code 0 |
| `failed` | Job completed with non-zero exit code |
| `timeout` | Job killed after exceeding timeout |
| `skipped_overlap` | Skipped because previous run still in progress |
| `internal_error` | schedx internal failure |

## Failure Handling

Jobs can have an `--on-failure` command that runs when the job fails, times out, or hits an internal error:

```bash
schedx add "0 2 * * *" --run "./backup.sh" \
  --on-failure "curl -X POST https://alerts.example.com/failed" \
  --name "nightly-backup"
```

A global fallback can be set via `schedx config on_failure <command>`. Per-job `--on-failure` takes precedence over the global setting.

The failure command receives context via environment variables and its own log capture.

## Real-World Examples

See [EXAMPLES.md](EXAMPLES.md) for real-world recipes covering:

- DevOps & infrastructure (backups, health checks, SSL monitoring)
- Webhooks & notifications (Slack, CI triggers, heartbeat monitors)
- AI agent workflows (PR summaries, security audits, morning briefings)
- One-shot & timed tasks (reminders, migrations, deploys)
- Monitoring & data pipelines (disk alerts, exchange rates, log rotation)
- Job lifecycle management (create, pause, skip, history, remove)

## Troubleshooting

### Jobs not running

1. Check the backend is installed: `schedx repair`
2. Verify the job is active: `schedx get <id>`
3. Check logs: `schedx logs <id>`
4. Try running manually: `schedx run <id>`

### Daemon mode

If systemd/launchd isn't available, run the dispatcher manually:

```bash
schedx daemon --interval 10
```

### Repair

Fix inconsistencies between schedx state and the system scheduler:

```bash
schedx repair
```

### Checking version

```bash
schedx --version
schedx upgrade        # Upgrade to latest
```
