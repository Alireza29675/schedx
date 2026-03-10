use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{Backend, BackendHealth};

const PLIST_LABEL: &str = "com.schedx.dispatcher";

pub struct LaunchdBackend {
    plist_path: PathBuf,
}

impl LaunchdBackend {
    pub fn new() -> Result<Self> {
        let plist_path = launchd_plist_path()?;
        Ok(Self { plist_path })
    }

    /// Check if launchd is available (macOS only).
    pub fn is_available() -> bool {
        Command::new("launchctl").arg("version").output().is_ok()
    }

    fn sched_binary_path() -> Result<PathBuf> {
        // Try the well-known install location first, then fall back to current_exe
        let well_known = dirs::home_dir().map(|h| h.join(".local/bin/schedx"));

        if let Some(ref p) = well_known {
            if p.exists() {
                return Ok(p.clone());
            }
        }

        std::env::current_exe().context("could not determine schedx binary path")
    }

    fn write_plist(&self) -> Result<()> {
        let sched_bin = Self::sched_binary_path()?;
        let sched_path = sched_bin.display();

        // Ensure the LaunchAgents directory exists
        if let Some(parent) = self.plist_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create LaunchAgents directory: {}",
                    parent.display()
                )
            })?;
        }

        let log_dir = crate::store::paths::sched_home()?;
        let stdout_log = log_dir.join("daemon-stdout.log");
        let stderr_log = log_dir.join("daemon-stderr.log");

        // Use StartInterval for periodic dispatch (every 60 seconds).
        // ThrottleInterval defaults to 10s on macOS, which is fine.
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{PLIST_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{sched_path}</string>
        <string>_dispatch</string>
    </array>
    <key>StartInterval</key>
    <integer>60</integer>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:{home_local_bin}</string>
    </dict>
</dict>
</plist>
"#,
            stdout = stdout_log.display(),
            stderr = stderr_log.display(),
            home_local_bin = dirs::home_dir()
                .map(|h| h.join(".local/bin").display().to_string())
                .unwrap_or_default(),
        );

        fs::write(&self.plist_path, plist_content)
            .with_context(|| format!("failed to write plist: {}", self.plist_path.display()))?;

        Ok(())
    }

    fn load_plist(&self) -> Result<()> {
        // Unload first if already loaded (best-effort)
        let _ = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&self.plist_path)
            .output();

        let output = Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&self.plist_path)
            .output()
            .context("failed to run launchctl load")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("launchctl load failed: {}", stderr.trim());
        }

        Ok(())
    }
}

impl Backend for LaunchdBackend {
    fn ensure_dispatcher(&self) -> Result<()> {
        self.write_plist()?;
        self.load_plist()?;
        Ok(())
    }

    fn remove_dispatcher(&self) -> Result<()> {
        // Best-effort: unload and remove plist
        let _ = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&self.plist_path)
            .output();
        let _ = fs::remove_file(&self.plist_path);
        Ok(())
    }

    fn check_health(&self) -> Result<BackendHealth> {
        let mut messages = Vec::new();
        let mut healthy = true;

        // Check if plist exists
        if self.plist_path.exists() {
            messages.push(format!("Plist: {}", self.plist_path.display()));
        } else {
            messages.push("Plist file missing. Run: schedx repair".to_string());
            healthy = false;
        }

        // Check if the job is loaded via launchctl list
        let output = Command::new("launchctl")
            .args(["list", PLIST_LABEL])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // Parse PID from launchctl list output
                if stdout.contains("\"PID\"") || stdout.contains("PID") {
                    messages.push("Dispatcher is loaded and running".to_string());
                } else {
                    messages.push("Dispatcher is loaded (waiting for next interval)".to_string());
                }
            }
            Ok(_) => {
                messages.push("Dispatcher is not loaded. Run: schedx repair".to_string());
                healthy = false;
            }
            Err(e) => {
                messages.push(format!("Could not check launchctl status: {e}"));
                healthy = false;
            }
        }

        Ok(BackendHealth { healthy, messages })
    }

    fn name(&self) -> &'static str {
        "launchd"
    }
}

fn launchd_plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home
        .join("Library/LaunchAgents")
        .join(format!("{PLIST_LABEL}.plist")))
}
