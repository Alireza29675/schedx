pub mod launchd;
pub mod none;
pub mod systemd;

use anyhow::{Result, bail};

/// Backend trait for managing the system-level dispatch timer.
#[allow(dead_code)]
pub trait Backend {
    /// Ensure the dispatch timer/service is installed and enabled.
    fn ensure_dispatcher(&self) -> Result<()>;

    /// Remove the dispatch timer/service.
    fn remove_dispatcher(&self) -> Result<()>;

    /// Check whether the backend is healthy and the dispatcher is running.
    fn check_health(&self) -> Result<BackendHealth>;

    /// Backend name for display.
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BackendHealth {
    pub healthy: bool,
    pub messages: Vec<String>,
}

/// Detect and return the appropriate backend.
pub fn detect_backend() -> Result<Box<dyn Backend>> {
    // Check env override first
    if let Ok(val) = std::env::var("SCHED_BACKEND") {
        return match val.as_str() {
            "none" => Ok(Box::new(none::NoneBackend)),
            "systemd" => Ok(Box::new(systemd::SystemdBackend::new()?)),
            "launchd" => Ok(Box::new(launchd::LaunchdBackend::new()?)),
            other => bail!(
                "Error: Unknown backend '{other}'.\n\
                 Supported backends: 'systemd', 'launchd', 'none'"
            ),
        };
    }

    // Auto-detect: Linux -> systemd, macOS -> launchd
    if cfg!(target_os = "linux") && systemd::SystemdBackend::is_available() {
        return Ok(Box::new(systemd::SystemdBackend::new()?));
    }

    if cfg!(target_os = "macos") && launchd::LaunchdBackend::is_available() {
        return Ok(Box::new(launchd::LaunchdBackend::new()?));
    }

    // Fallback: inform user
    bail!(
        "Error: No supported scheduling backend detected.\n\
         On Linux, ensure systemd is available.\n\
         On macOS, launchd should be available by default.\n\
         Set SCHED_BACKEND=none for manual/daemon mode."
    )
}
