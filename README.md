# schedx

`schedx` is a secure scheduler CLI for commands, prompts, and webhooks. It gives agents and humans a single, scriptable interface for recurring work, one-shot jobs, logs, history, and system scheduler integration.

## Status

The project is pre-1.0 and uses semantic versioning. Breaking changes can still happen in minor releases while the CLI and persistence model settle.

## Install

From source with Cargo:

```bash
cargo install schedx --locked
```

From GitHub Releases on macOS or Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
```

From GitHub Releases on Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/Alireza29675/schedx/main/install.ps1 | iex
```

## Update

`schedx` does not perform silent background self-updates.

- `cargo` installs update with `cargo install schedx --locked --force`
- direct installs update by rerunning the installer
- release artifacts are versioned, checksummed, and attached to GitHub Releases

This keeps upgrades explicit and auditable, which is the normal model for reliable CLI tooling.

## Quick Start

```bash
schedx add "every 1h" --run "echo hello"
schedx list
schedx run <job-id>
schedx history
schedx logs <job-id>
```

## Security

- local state is stored under `~/.sched` or `SCHED_HOME`
- config, locks, logs, and backups are created with owner-only permissions on Unix
- insecure `http://` webhooks are blocked by default
- webhook URLs and sensitive headers are redacted in structured output
- agent arguments that look like secrets are redacted when listed

Security reporting guidance lives in [SECURITY.md](SECURITY.md).

## Release Model

- every release is tagged as `vMAJOR.MINOR.PATCH`
- GitHub Actions builds release artifacts for major platforms
- GitHub Releases are the source of truth for direct installs
- crates.io is published from the same tagged release

The detailed process is documented in [docs/RELEASING.md](docs/RELEASING.md).
