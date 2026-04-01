# schedx — Claude Code Plugin

Claude Code plugin for [schedx](https://github.com/Alireza29675/schedx), a local-first scheduler CLI for recurring jobs, AI agent prompts, and webhooks.

## What this plugin provides

- **Auto-install** — schedx binary is installed automatically on first session if not already present
- **Skill** — teaches Claude how to use all schedx commands, schedule formats, and configuration
- **Session context** — Claude knows schedx is available without you needing to mention it

## Install

```
/plugin install schedx
```

## What happens

1. On session start, the plugin checks if `schedx` is installed
2. If missing, it downloads and installs the binary to `~/.local/bin/`
3. Claude is informed that schedx is available and how to use it
4. You can now ask Claude to schedule jobs, check status, manage schedules — it knows the CLI

## Examples

Once the plugin is installed, just ask:

- "Schedule a database backup every night at 2am"
- "Add a webhook that pings Slack every 5 minutes"
- "List all my scheduled jobs"
- "Pause the nightly backup job"
- "Set up a daily AI prompt to summarize open PRs"

## Manual install (alternative)

If you prefer not to use the plugin:

```bash
# Install schedx (includes interactive agent setup)
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh

# Or install and set up separately
cargo install schedx --locked
schedx setup
```

## Links

- [schedx on GitHub](https://github.com/Alireza29675/schedx)
- [Full CLI reference](https://github.com/Alireza29675/schedx/blob/main/docs/REFERENCE.md)
- [Real-world examples](https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md)
