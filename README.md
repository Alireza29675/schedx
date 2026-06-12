# schedx

[![CI](https://github.com/Alireza29675/schedx/actions/workflows/ci.yml/badge.svg)](https://github.com/Alireza29675/schedx/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/schedx.svg)](https://crates.io/crates/schedx)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Scheduler CLI for recurring jobs, agent prompts, and webhooks.

One tool to schedule everything -- cron jobs, one-shot tasks, webhook calls, and AI agent prompts. Local-first, file-based, and designed to be used by both humans and agents.

## Why I Built This

AI agents are great at doing things right now. But most real work happens over time -- a security audit that runs every night, a news digest compiled every morning, a deploy pipeline that checks back in 30 minutes. There was no clean way to give agents the time dimension. cron wasn't built for this. launchd wasn't built for this. So I built schedx.

I've been using it daily for a few months now. My Claude Code agent schedules its own security reviews. It compiles a morning news briefing from diverse sources and emails me a fair summary. I have a job that wakes up both Codex and Claude Code to run a full network audit on my mini PC, then passes both reports to a judge agent that only pings me if something's actually wrong. It's like hiring a sysadmin who never sleeps.

The idea is simple: if your agent can run a command, it can schedule one. And once scheduling is just another CLI call with JSON output, agents can build surprisingly sophisticated workflows across time -- without any glue code.

## Install

From source:

```bash
cargo install schedx --locked
```

From GitHub Releases (macOS / Linux):

```bash
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
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

**Three action types, one interface.** Schedule shell commands (`--run`), AI agent prompts (`--prompt`), and HTTP webhooks (`--webhook`) using the same CLI.

**Declarative manifests.** Keep every schedule in a `schedx.yaml` and reconcile with `schedx up` — idempotent, drift-correcting, versionable. Think docker-compose for schedules.

**Flexible scheduling.** Cron expressions, human intervals (`every 5m`, `every 2h`), one-shot timers (`in 30m`, `in 2h`), and exact ISO-8601 timestamps.

**Full job lifecycle.** Add, pause, resume, skip, and remove jobs. View run history, logs, and status -- all from the terminal.

**JSON output.** Every command supports `--json` for structured output. Built to be scripted by CI pipelines, shell scripts, and AI agents.

**Local-first.** All state lives as plain files under `~/.schedx/`. No cloud, no daemon, no database. Atomic writes with fsync ensure nothing gets corrupted.

**System-native scheduling.** Integrates with systemd (Linux) and launchd (macOS) for the heartbeat tick. All scheduling decisions happen in schedx itself.

## Real-World Agent Workflows

```bash
# Morning news briefing -- agent reads diverse sources, compiles a fair summary, emails you
schedx add "0 7 * * 1-5" \
  --prompt "Read top stories from Reuters, AP, Al Jazeera, and Ars Technica. Write a balanced 5-minute briefing and email it to me."

# Nightly security review -- two agents audit, a third judges
schedx add "0 2 * * *" \
  --run "claude -p 'Run a full security scan of this machine: open ports, failed logins, unusual processes' > /tmp/audit-claude.md && \
         codex -p 'Review network connections and firewall rules on this host' > /tmp/audit-codex.md && \
         claude -p 'You are a security judge. Review these two audit reports and only alert me if something needs attention: $(cat /tmp/audit-claude.md /tmp/audit-codex.md)'"

# Deploy canary check -- verify health 30 minutes after deploy
schedx add "in 30m" \
  --prompt "Check the /health endpoint and error rates for the last 30 minutes. Did the deploy go clean?"

# Weekly dependency audit
schedx add "0 10 * * 1" \
  --run "cargo audit && npm audit" \
  --name "dep-audit" --tag security
```

## How It Compares

| | cron | launchd | at | schedx |
|---|---|---|---|---|
| **Recurring jobs** | Cron expressions only | Cron + calendar intervals | No | Cron, `every Xm/h/d`, intervals |
| **One-shot tasks** | `@reboot` only | No native "run once at time X" | Yes (single run at a specific time) | Yes (`in 30m`, ISO-8601) |
| **Webhooks** | Manual (wrap curl) | Manual (wrap curl) | Manual (wrap curl) | Built-in (`--webhook`) |
| **Agent prompts** | No | No | No | Built-in (`--prompt`) |
| **Job history** | No | Last exit status only (`launchctl list`) | No | Append-only run history per job |
| **Log capture** | Mailed or redirected manually | stdout/stderr to file (configured per plist) | Mailed | Automatic per-run log files |
| **Pause / resume / skip** | Remove and re-add crontab line | `launchctl enable` / `disable` (no skip) | No | `schedx pause`, `resume`, `skip` |
| **JSON output** | No | No (`launchctl` outputs plists/text) | No | `--json` on every command |
| **Agent-usable** | Agents must parse crontab text, no structured feedback | Agents must generate XML plists, parse unstructured output | Agents must parse queue text | Agents get structured JSON I/O, `--prompt` as first-class action |
| **Cross-platform** | Linux, macOS, BSDs | macOS only | Linux, macOS, BSDs | Linux, macOS |
| **Config format** | Crontab lines | XML plists | Interactive or piped stdin | CLI flags, JSON state files |

**A note on agents:** When an agent has to juggle crontab syntax, launchctl plists, and at queues just to schedule work, that's a lot of scattered context competing for the model's attention. I built schedx because I wanted one tool that brings the time dimension to the agent experience without the noise. It's a single CLI with JSON in and out, prompt scheduling as a first-class action, and queryable history. Reliable and intuitive for agents out of the box.

## More Examples

See [docs/EXAMPLES.md](docs/EXAMPLES.md) for real-world recipes covering DevOps, webhooks, AI agent workflows, one-shot tasks, monitoring, and job lifecycle management.

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
