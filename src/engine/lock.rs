use std::fs::{self, File};

use anyhow::{Context, Result};
use fs4::fs_std::FileExt;

use crate::store::paths;

/// An RAII lock guard backed by an OS advisory file lock.
/// The lock is released when dropped (or when the process exits).
pub struct FileLock {
    _file: File,
}

impl FileLock {
    /// Acquire the global state lock (blocking).
    pub fn state() -> Result<Self> {
        let path = paths::state_lock_path()?;
        Self::acquire_blocking(&path)
    }

    /// Acquire the dispatch lock (non-blocking).
    /// Returns `None` if already held by another process.
    pub fn dispatch_non_blocking() -> Result<Option<Self>> {
        let path = paths::dispatch_lock_path()?;
        Self::acquire_non_blocking(&path)
    }

    /// Acquire a per-job lock (non-blocking).
    /// Returns `None` if already held (overlap prevention).
    pub fn job_non_blocking(job_id: &str) -> Result<Option<Self>> {
        let path = paths::job_lock_path(job_id)?;
        Self::acquire_non_blocking(&path)
    }

    fn acquire_blocking(path: &std::path::Path) -> Result<Self> {
        ensure_lock_parent(path)?;
        let file = File::create(path)
            .with_context(|| format!("failed to create lock file: {}", path.display()))?;
        file.lock_exclusive()
            .with_context(|| format!("failed to acquire lock: {}", path.display()))?;
        Ok(Self { _file: file })
    }

    fn acquire_non_blocking(path: &std::path::Path) -> Result<Option<Self>> {
        ensure_lock_parent(path)?;
        let file = File::create(path)
            .with_context(|| format!("failed to create lock file: {}", path.display()))?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => {
                // On some systems the error kind differs; check raw OS error
                // On some Unix systems, the error code may differ
                #[cfg(unix)]
                {
                    // EWOULDBLOCK (11) and EAGAIN (11) on Linux/macOS
                    if matches!(e.raw_os_error(), Some(11 | 35)) {
                        return Ok(None);
                    }
                }
                #[cfg(windows)]
                {
                    // ERROR_LOCK_VIOLATION
                    if matches!(e.raw_os_error(), Some(33)) {
                        return Ok(None);
                    }
                }
                Err(e).with_context(|| format!("failed to acquire lock: {}", path.display()))
            }
        }
    }
}

fn ensure_lock_parent(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
