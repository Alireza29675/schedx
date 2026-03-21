# schedx

Scheduler CLI for recurring jobs, agent prompts, and webhooks.

One tool to schedule everything -- cron jobs, one-shot tasks, webhook calls, and AI agent prompts. Local-first, file-based, and designed to be used by both humans and agents.

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

```bash
# Schedule a recurring command
schedx add "every 1h" --run "echo hello"

# Schedule a one-shot reminder
schedx add "in 30m" --run "say 'break time'"

# Fire a webhook on a cron schedule
schedx add "0 9 * * 1-5" \
  --webhook https://hooks.slack.com/services/T00/B00/xxx \
  --method POST \
  --header "Content-Type: application/json" \
  --body '{"text":"Good morning"}'

# Ask an AI agent something every Friday
schedx add "0 16 * * 5" \
  --prompt "Summarize this week's open pull requests"

# Manage jobs
schedx list
schedx history
schedx logs <job-id>
schedx pause <job-id>
schedx resume <job-id>
schedx rm <job-id>
```

## What It Does

**Three action types, one interface.** Schedule shell commands (`--run`), AI agent prompts (`--prompt`), and HTTP webhooks (`--webhook`) using the same CLI.

**Flexible scheduling.** Cron expressions, human intervals (`every 5m`, `every 2h`), one-shot timers (`in 30m`, `in 2h`), and exact ISO-8601 timestamps.

**Full job lifecycle.** Add, pause, resume, skip, and remove jobs. View run history, logs, and status -- all from the terminal.

**JSON output.** Every command supports `--json` for structured output. Built to be scripted by CI pipelines, shell scripts, and AI agents.

**Local-first.** All state lives as plain files under `~/.schedx/`. No cloud, no daemon, no database. Atomic writes with fsync ensure nothing gets corrupted.

**System-native scheduling.** Integrates with systemd (Linux) and launchd (macOS) for the heartbeat tick. All scheduling decisions happen in schedx itself.

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

**A note on agents:** When an agent has to juggle crontab syntax, launchctl plists, and at queues just to schedule work, that's a lot of scattered context competing for the model's attention. I built schedx because I wanted one tool that brings the time dimension to the agent experience without the noise. It's a single CLI with JSON in and out, prompt scheduling as a first-class action, and queryable history. I've been using it daily for a few months now -- it's reliable and intuitive for agents out of the box.

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
