# Releasing schedx

## Versioning

`schedx` uses semantic versioning with git tags in the form `vMAJOR.MINOR.PATCH`.

- increment `PATCH` for fixes and compatible internal improvements
- increment `MINOR` for new features or user-visible behavior changes while pre-1.0
- increment `MAJOR` for intentionally breaking stable contracts after 1.0

Before tagging:

1. update `Cargo.toml`
2. update `CHANGELOG.md`
3. verify `cargo test --locked`
4. verify `cargo package --locked`

## Release Pipeline

Pushing a tag like `v0.1.0` triggers the release workflow.

The workflow:

1. validates that the tag matches the crate version
2. creates a draft GitHub Release
3. builds release binaries for Linux, macOS, and Windows
4. uploads tarballs and a combined `checksums.txt`
5. attaches `install.sh` and `install.ps1`
6. publishes the crate to crates.io
7. publishes the GitHub Release after the crate publish succeeds

## Install and Update Policy

`schedx` uses explicit updates.

- Cargo users run `cargo install schedx --locked --force`
- direct-download users rerun the installer script
- no silent background auto-update is performed by the CLI

This is the common trust model for reliable OSS CLI tools because upgrades stay visible and reproducible.
