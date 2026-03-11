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
5. generates a `schedx.rb` Homebrew formula artifact for the tagged source archive
6. attaches `install.sh`, `install.ps1`, and the Homebrew formula
7. publishes the crate to crates.io unless that version already exists there
8. publishes the GitHub Release after the crate publish step succeeds or is skipped

## Homebrew

To get `brew install schedx`, the formula must be merged into `homebrew/core`. That is an external repository and cannot be completed from this repository alone.

The release workflow generates a ready-to-review `schedx.rb` formula artifact for each tag. The normal path is:

1. tag and push the release
2. download the generated `schedx.rb` from the GitHub Release
3. submit that formula in a PR to `Homebrew/homebrew-core`
4. after merge, users can install with `brew install schedx`

## Install and Update Policy

`schedx` uses explicit updates.

- Cargo users run `cargo install schedx --locked --force`
- direct-download users rerun the installer script
- no silent background auto-update is performed by the CLI

This is the common trust model for reliable OSS CLI tools because upgrades stay visible and reproducible.
