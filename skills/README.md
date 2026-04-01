# schedx Agent Skills

Skill files that teach AI coding agents how to use schedx.

## Automatic installation

```bash
schedx skill install          # Interactive — picks detected agents
schedx skill install --all    # Install for all detected agents
schedx skill install --agent claude   # Install for a specific agent
```

## Manual installation

### Claude Code

```bash
mkdir -p ~/.claude/skills/schedx
cp skills/SKILL.md ~/.claude/skills/schedx/SKILL.md
cp skills/reference.md ~/.claude/skills/schedx/reference.md
```

### Codex (OpenAI)

```bash
mkdir -p ~/.agents/skills/schedx
cp skills/SKILL.md ~/.agents/skills/schedx/SKILL.md
cp skills/reference.md ~/.agents/skills/schedx/reference.md
```

### Cursor

```bash
cp skills/cursor.mdc ~/.cursor/rules/schedx.mdc
```

### Gemini CLI

```bash
mkdir -p ~/.gemini/instructions
cp skills/gemini.md ~/.gemini/instructions/schedx.md
```

Then add to your `~/.gemini/GEMINI.md`:
```
See ~/.gemini/instructions/schedx.md for schedx scheduler CLI usage.
```

### OpenCode

```bash
mkdir -p ~/.config/opencode/instructions
cp skills/opencode.md ~/.config/opencode/instructions/schedx.md
```

Then add to your `~/.config/opencode/opencode.json` instructions array:
```json
{
  "instructions": ["~/.config/opencode/instructions/schedx.md"]
}
```

## Project-level AGENTS.md

To add schedx awareness to a specific project, copy the snippet from [AGENTS-snippet.md](AGENTS-snippet.md) into your project's `AGENTS.md` file.

## Files

| File | Format | For |
|------|--------|-----|
| `SKILL.md` | Agent Skills standard | Claude Code, Codex, GitHub Copilot, VS Code |
| `reference.md` | Supporting reference | Loaded on-demand by SKILL.md |
| `cursor.mdc` | Cursor MDC | Cursor |
| `gemini.md` | Plain markdown | Gemini CLI |
| `opencode.md` | Plain markdown | OpenCode |
| `AGENTS-snippet.md` | Copy-paste snippet | Any agent via project AGENTS.md |
