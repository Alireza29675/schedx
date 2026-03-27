pub mod binary;
pub mod cargo;
pub mod check;

use std::process::Command;

use anyhow::{Result, bail};

/// The current compiled-in version.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const GITHUB_REPO: &str = "Alireza29675/schedx";

/// How schedx was installed.
#[derive(Debug, PartialEq, Eq)]
pub enum InstallMethod {
    Cargo,
    Binary,
}

/// Detect whether schedx was installed via `cargo install` or as a prebuilt binary.
pub fn detect_install_method() -> InstallMethod {
    let output = Command::new("cargo").args(["install", "--list"]).output();
    match output {
        Ok(out) if out.status.success() => {
            let list = String::from_utf8_lossy(&out.stdout);
            if list.lines().any(|l| l.starts_with("schedx ")) {
                InstallMethod::Cargo
            } else {
                InstallMethod::Binary
            }
        }
        _ => InstallMethod::Binary,
    }
}

/// Detect the target triple for the current platform.
pub fn detect_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => bail!("Unsupported platform: {os}/{arch}"),
    }
}

/// Fetch the latest release tag from GitHub. Returns the version string without the leading `v`.
pub fn fetch_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let body: serde_json::Value = ureq::get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "schedx-upgrade")
        .call()?
        .body_mut()
        .read_json()?;
    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in GitHub release response"))?;
    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Compare two semver version strings. Returns true if `latest` is newer than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = s.split('.').collect();
        let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(latest) > parse(current)
}

/// Build the download URL for a release asset.
pub fn release_asset_url(version: &str, filename: &str) -> String {
    format!("https://github.com/{GITHUB_REPO}/releases/download/v{version}/{filename}")
}
