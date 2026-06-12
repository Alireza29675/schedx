# schedx

[![CI](https://github.com/Alireza29675/schedx/actions/workflows/ci.yml/badge.svg)](https://github.com/Alireza29675/schedx/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/schedx.svg)](https://crates.io/crates/schedx)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Scheduler CLI for shell commands, AI agent prompts, and webhooks. Local-first, built for humans and agents.

```
schedx add "<when>"  --run | --prompt | --webhook  "<what>"
```

That one line is the whole mental model. When you have more than a few jobs, there's one file: [`schedx.yaml`](#3-put-it-in-a-file--schedxyaml).

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
```

Or from source:

```bash
cargo install schedx --locked
```

## Quick Start

Five minutes, three steps: schedule a job, watch it work, then put your schedules in a file.

### 1. Schedule your first job

```bash
schedx add "every 1h" --run "echo hello"
```

That's the whole shape of schedx: a schedule plus an action. The schedule reads
like you'd say it (`every 5m`, `in 30m`, `0 9 * * 1-5`, an ISO timestamp), and
the action is one of three kinds:

```bash
# A shell command
schedx add "in 30m" --run "say 'break time'"

# A webhook
schedx add "0 9 * * 1-5" \
  --webhook https://hooks.slack.com/services/T00/B00/xxx \
  --method POST \
  --header "Content-Type: application/json" \
  --body '{"text":"Good morning"}'

# An AI agent prompt
schedx add "0 16 * * 5" \
  --prompt "Summarize this week's open pull requests"
```

### 2. Watch it work

```bash
schedx list             # every job, its status, and when it runs next
schedx logs <job-id>    # the captured output of each run
schedx history          # what ran, when, and how it went
```

Pause, resume, skip, or remove any job the same way: `schedx pause <job-id>`,
`schedx resume <job-id>`, `schedx skip <job-id>`, `schedx rm <job-id>`.

### 3. Put it in a file — `schedx.yaml`

This is where schedx gets good. The moment you have more than a couple of
jobs, stop adding them one by one — declare them all in a `schedx.yaml` and
let schedx make reality match the file:

```yaml
name: my-jobs
jobs:
  backup:
    schedule: "every 6h"
    run: "restic backup ~/"
  morning-brief:
    schedule: "0 9 * * 1-5"
    prompt: "Summarize my unread PRs"
  slack-ping:
    schedule: "every 15m"
    webhook: "https://hooks.slack.com/services/T/B/X"
    headers:
      Authorization: "Bearer ${SLACK_TOKEN}"
```

```bash
schedx up --dry-run   # preview what would change
schedx up             # make the job store match the file
schedx down           # remove everything the file created
```

Edit the file, run `up` again — changed jobs update in place (history kept),
deleted ones are pruned, drift gets corrected. Running `up` twice changes
nothing the second time. Jobs you added with `schedx add` are **never
touched**. Secrets stay out of the committed file with `${VAR}` expansion.

The payoff: your schedules become a file you can read, version, and review.
Put `schedx.yaml` in a repo and a new machine is one `schedx up` away from
your whole setup. Full format and reconcile semantics in the
[reference](docs/REFERENCE.md#declarative-manifests-schedxyaml), real recipes
in [docs/EXAMPLES.md](docs/EXAMPLES.md#the-schedxyaml-way).

## What It Does

- **Three actions** — shell commands (`--run`), AI agent prompts (`--prompt`), HTTP webhooks (`--webhook`)
- **Any schedule** — cron (`0 9 * * 1-5`), intervals (`every 6h`), one-shot (`in 30m`), ISO-8601 timestamps
- **Declarative** — `schedx.yaml` reconciled with `up`/`down`; docker-compose for schedules
- **Full lifecycle** — pause, resume, skip, remove; per-job run history and captured logs
- **`--json` on every command** — structured output for scripts, CI, and agents
- **Local-first** — plain files under `~/.schedx/`; no cloud, no daemon, no database; atomic writes
- **System-native tick** — systemd (Linux) and launchd (macOS) provide the heartbeat; scheduling decisions stay in schedx

## Use schedx from your AI agent

schedx ships an agent skill — a `SKILL.md` that teaches Claude Code, Codex,
Gemini CLI, Cursor, and opencode how to drive schedx. Install it once and your
agent can schedule, list, and manage jobs on your behalf.

```bash
npx skills add Alireza29675/schedx
```

That installs the skill into every agent it finds. Target one with `-a`
(`claude-code`, `codex`, `cursor`, `gemini-cli`, `opencode`):

```bash
npx skills add Alireza29675/schedx -a claude-code
```

No Node? `schedx setup` installs the same skill straight from the binary — no
network, and version-matched to your installed schedx so it never drifts:

```bash
schedx setup           # detect your agents and install for each
schedx setup --list    # show what's installed and its version
```

Both paths write the same canonical skill (`skills/schedx/`). See the
[skill reference](skills/schedx/reference.md) for everything the agent learns.

## Why I Built This

AI agents are great at doing things right now. But most real work happens over
time — a security audit that runs every night, a news digest compiled every
morning, a deploy pipeline that checks back in 30 minutes. There was no clean
way to give agents the time dimension. cron wasn't built for this. launchd
wasn't built for this. So I built schedx.

I've been using it daily for months — my agent schedules its own security
reviews, compiles my morning news briefing, and checks back on deploys. The
idea underneath is simple: if your agent can run a command, it can schedule
one.

## Examples

The pattern that sold me on it — two agents audit nightly, a third judges, and
I only hear about it when something's wrong:

```bash
schedx add "0 2 * * *" \
  --run "claude -p 'Run a full security scan of this machine: open ports, failed logins, unusual processes' > /tmp/audit-claude.md && \
         codex -p 'Review network connections and firewall rules on this host' > /tmp/audit-codex.md && \
         claude -p 'You are a security judge. Review these two audit reports and only alert me if something needs attention: $(cat /tmp/audit-claude.md /tmp/audit-codex.md)'"
```

[docs/EXAMPLES.md](docs/EXAMPLES.md) has the full recipe collection — DevOps,
webhooks, agent workflows, one-shot tasks, monitoring, and manifests.

## How It Compares

| | cron | launchd | at | schedx |
|---|---|---|---|---|
| **Schedules** | Cron expressions | Calendar intervals (XML plists) | One time, once | Cron, `every 6h`, `in 30m`, ISO-8601 |
| **Agent prompts** | — | — | — | `--prompt`, first-class |
| **History & logs** | — | Last exit status | — | Per-job run history + captured logs |
| **Structured output** | — | — | — | `--json` on every command |

## Security

- Owner-only permissions on all data files and directories
- HTTPS required for webhooks by default (`http://` must be explicitly allowed)
- Secrets and sensitive headers are redacted in output
- No shell interpretation unless you pass `--shell` explicitly

Full policy in [SECURITY.md](SECURITY.md).

## Update

```bash
# Cargo installs
cargo install schedx --locked --force

# Direct installs
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
```

No silent background updates. Upgrades are always explicit.

## Status

Pre-1.0. Semantic versioning applies, but breaking changes can still happen in minor releases while the CLI and persistence model settle. Linux and macOS only.

## Contributing

- [Contributor guide](docs/ONBOARDING.md)
- [Release process](docs/RELEASING.md)

## License

MIT
