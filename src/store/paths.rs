use std::path::PathBuf;

use anyhow::{Context, Result};

/// Resolve the schedx home directory.
/// Priority: `SCHEDX_HOME` env var > `~/.schedx`.
pub fn schedx_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("SCHEDX_HOME") {
        return Ok(PathBuf::from(home));
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".schedx"))
}

pub fn jobs_file() -> Result<PathBuf> {
    Ok(schedx_home()?.join("jobs.json"))
}

pub fn config_file() -> Result<PathBuf> {
    Ok(schedx_home()?.join("config.json"))
}

pub fn history_file() -> Result<PathBuf> {
    Ok(schedx_home()?.join("run-history.jsonl"))
}

pub fn backups_dir() -> Result<PathBuf> {
    Ok(schedx_home()?.join("backups"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(schedx_home()?.join("logs"))
}

pub fn locks_dir() -> Result<PathBuf> {
    Ok(schedx_home()?.join("locks"))
}

pub fn job_log_dir(job_id: &str) -> Result<PathBuf> {
    Ok(logs_dir()?.join(job_id))
}

pub fn state_lock_path() -> Result<PathBuf> {
    Ok(locks_dir()?.join("state.lock"))
}

pub fn dispatch_lock_path() -> Result<PathBuf> {
    Ok(locks_dir()?.join("dispatch.lock"))
}

pub fn job_lock_path(job_id: &str) -> Result<PathBuf> {
    Ok(locks_dir()?.join(format!("job-{job_id}.lock")))
}

/// Ensure the full directory hierarchy exists with secure permissions.
pub fn ensure_dirs() -> Result<()> {
    let home = schedx_home()?;
    for dir in [
        home.clone(),
        home.join("backups"),
        home.join("logs"),
        home.join("locks"),
    ] {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create directory: {}", dir.display()))?;
        set_dir_permissions(&dir)?;
    }
    Ok(())
}

/// Set directory permissions to 0700 (owner-only access).
#[cfg(unix)]
fn set_dir_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

/// Set file permissions to 0600 (owner-only read/write).
#[cfg(unix)]
pub fn set_file_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_file_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}
