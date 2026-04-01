# schedx Complete Reference

This is the detailed reference for schedx. For a quick overview, see [SKILL.md](SKILL.md).

For the full, formatted reference with all commands, flags, defaults, examples, configuration keys, architecture details, and troubleshooting, see:

**https://github.com/Alireza29675/schedx/blob/main/docs/REFERENCE.md**

That document covers:

- Every command with all flags, types, and defaults
- All schedule formats with validation rules and limits
- Configuration keys (backup_count, log_retention_days, default_timeout_seconds, allow_insecure_http, backend, on_failure, archive_after_hours, etc.)
- Agent profile setup for Claude Code, Codex, Gemini CLI, OpenCode
- Data directory layout (~/.schedx/)
- Security model (no-shell-by-default, HTTPS-only webhooks, secret redaction, file permissions)
- Architecture (dispatch/exec flow, locking, atomic writes, catch-up policy)
- Exit codes (0-6) and run statuses (success, failed, timeout, skipped_overlap, internal_error)
- Failure handling (per-job and global on_failure commands)
- Troubleshooting guide

For real-world recipes and multi-step workflows, see:

**https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md**
