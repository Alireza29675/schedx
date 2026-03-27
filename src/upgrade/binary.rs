use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::upgrade;

/// Download a release asset and return its bytes.
fn download_asset(version: &str, filename: &str) -> Result<Vec<u8>> {
    let url = upgrade::release_asset_url(version, filename);
    let mut buf = Vec::new();
    let mut reader = ureq::get(&url)
        .header("User-Agent", "schedx-upgrade")
        .call()
        .with_context(|| format!("failed to download {url}"))?
        .into_body()
        .into_reader();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Extract the `schedx` binary from a `.tar.gz` archive.
fn extract_binary(archive_bytes: &[u8]) -> Result<Vec<u8>> {
    let gz = flate2::read::GzDecoder::new(archive_bytes);
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.file_name().is_some_and(|n| n == "schedx") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    bail!("schedx binary not found in archive")
}

/// Verify SHA-256 checksum of `data` against `checksums.txt` content for the given `filename`.
/// Returns the hex digest on success.
fn verify_checksum(data: &[u8], checksums_txt: &str, filename: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = format!("{:x}", hasher.finalize());

    for line in checksums_txt.lines() {
        // Format: "<hash>  <filename>" or "<hash> <filename>"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == filename {
            if parts[0] == digest {
                return Ok(digest);
            }
            bail!(
                "Checksum mismatch for {filename}.\n  Expected: {}\n  Got:      {digest}",
                parts[0]
            );
        }
    }
    bail!("No checksum entry found for {filename} in checksums.txt")
}

/// Replace the binary at `current_path` atomically.
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> Result<()> {
    let dir = current_path
        .parent()
        .context("cannot determine parent directory of current binary")?;
    let tmp = dir.join(".schedx-upgrade.tmp");
    fs::write(&tmp, new_binary).context("failed to write temp binary")?;
    set_executable(&tmp)?;
    fs::rename(&tmp, current_path).context("failed to replace binary (atomic rename)")?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(path, perms).context("failed to set executable permissions")?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Perform a binary upgrade to the given version. Returns `(install_path, checksum_hex)`.
pub fn execute(version: &str) -> Result<(String, String)> {
    let target = upgrade::detect_target()?;
    let archive_name = format!("schedx-{target}.tar.gz");

    let install_path = std::env::current_exe().context("cannot determine current binary path")?;

    // Check write permission
    let parent = install_path
        .parent()
        .context("cannot determine parent directory")?;
    if parent
        .metadata()
        .map(|m| m.permissions().readonly())
        .unwrap_or(true)
    {
        bail!(
            "Cannot write to {}. Try: sudo schedx upgrade",
            install_path.display()
        );
    }

    println!("Downloading {archive_name}...");
    let archive_bytes = download_asset(version, &archive_name)?;

    println!("Downloading checksums.txt...");
    let checksums_bytes = download_asset(version, "checksums.txt")?;
    let checksums_txt = String::from_utf8(checksums_bytes).context("checksums.txt is not UTF-8")?;

    print!("Verifying checksum... ");
    let digest = verify_checksum(&archive_bytes, &checksums_txt, &archive_name)?;
    println!("verified (SHA-256: {digest})");

    println!("Installing to {}...", install_path.display());
    let binary = extract_binary(&archive_bytes)?;
    replace_binary(&install_path, &binary)?;

    Ok((install_path.display().to_string(), digest))
}
