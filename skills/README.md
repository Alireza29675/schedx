# schedx Agent Skills

One canonical skill — [`schedx/SKILL.md`](schedx/SKILL.md) plus its
[`reference.md`](schedx/reference.md) — that teaches AI coding agents how to
drive schedx. As of 2026 the landscape converged on the Anthropic-style
`SKILL.md` folder, so the same skill is read natively by **Claude Code, Codex,
Gemini CLI, Cursor, and opencode**. No per-agent variants.

## Install

```bash
# Cross-agent one-liner (vercel-labs/skills, no schedx needed for this step):
npx skills add Alireza29675/schedx              # every detected agent
npx skills add Alireza29675/schedx -a claude-code   # one agent
#   agent ids: claude-code · codex · cursor · gemini-cli · opencode

# From the binary — no Node, offline, version-matched to your schedx:
schedx setup            # detect your agents and install for each
schedx setup --all      # install for every supported agent
schedx setup --list     # show what's installed and its version
```

Both paths write the **same** skill — there is nothing to copy by hand.

## Where it lands

| Agent | Path |
|-------|------|
| Claude Code | `~/.claude/skills/schedx/` |
| Codex, Gemini CLI, opencode | `~/.agents/skills/schedx/` (the shared skills path they all read) |
| Cursor | `.cursor/skills/schedx/` in the current project (Cursor reads skills per project, so run it inside each repo) |

`schedx setup` stamps each installed file with the schedx version, so the skill
can never drift from the binary. Run `schedx setup --force` after upgrading to
refresh.

> Gemini CLI support is verified against Google's published Gemini CLI skills
> docs (which give `~/.agents/skills/` precedence), not a live binary run.

## Project-level AGENTS.md

To add schedx awareness to a single project without installing a skill, copy the
snippet from [AGENTS-snippet.md](AGENTS-snippet.md) into your project's
`AGENTS.md`.

## Files

| File | Purpose |
|------|---------|
| `schedx/SKILL.md` | The canonical skill — frontmatter + quick command grammar |
| `schedx/reference.md` | Full command/flag reference, loaded on demand by the skill |
| `AGENTS-snippet.md` | Copy-paste snippet for a project's `AGENTS.md` |
