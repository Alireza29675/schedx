# schedx AGENTS.md Snippet

Copy the section below into your project's `AGENTS.md` file to teach AI coding agents how to use schedx in your project.

---

## schedx

This project uses [schedx](https://github.com/Alireza29675/schedx) for scheduling recurring jobs, AI agent prompts, and webhooks.

**Quick reference:**
- `schedx add "<schedule>" --run|--prompt|--webhook` — create a scheduled job
- `schedx list [--json]` — list active jobs
- `schedx run <id|name>` — trigger a job now
- `schedx logs <id|name>` — view job output
- `schedx history [--json]` — view run history

**Schedule formats:** cron (`0 9 * * 1-5`), intervals (`every 5m`), one-shot (`in 30m`), ISO-8601 timestamps.

**Action types:** `--run` (shell command), `--prompt` (AI agent), `--webhook` (HTTP request).

For full CLI reference: https://github.com/Alireza29675/schedx/blob/main/docs/REFERENCE.md
