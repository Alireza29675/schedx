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

The release workflow runs in two cases:

- a push to `main` where `Cargo.toml` changed to a new version
- a direct push of a tag like `v0.1.0`

On a version-bump push to `main`, GitHub Actions creates the matching `vMAJOR.MINOR.PATCH` tag for the pushed commit and continues the release pipeline using that tag name.

The workflow:

1. computes the release version from `Cargo.toml`
2. on `main`, checks whether the previous `main` commit had a different version
3. ensures the matching git tag exists
4. creates a draft GitHub Release
5. builds release binaries for Linux, macOS, and Windows
6. uploads tarballs and a combined `checksums.txt`
7. attaches `install.sh` and `install.ps1`
8. publishes the crate to crates.io unless that version already exists there
9. publishes the GitHub Release after the crate publish step succeeds or is skipped

## Install and Update Policy

`schedx` uses explicit updates.

- Cargo users run `cargo install schedx --locked --force`
- direct-download users rerun the installer script
- no silent background auto-update is performed by the CLI

This is the common trust model for reliable OSS CLI tools because upgrades stay visible and reproducible.
