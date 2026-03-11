use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{Backend, BackendHealth};

const SERVICE_NAME: &str = "schedx-dispatch.service";
const TIMER_NAME: &str = "schedx-dispatch.timer";

pub struct SystemdBackend {
    unit_dir: PathBuf,
}

impl SystemdBackend {
    pub fn new() -> Result<Self> {
        let unit_dir = systemd_user_unit_dir()?;
        Ok(Self { unit_dir })
    }

    /// Check if systemd user session is available.
    pub fn is_available() -> bool {
        Command::new("systemctl")
            .args(["--user", "is-system-running"])
            .output()
            .is_ok()
    }

    fn schedx_binary_path() -> Result<PathBuf> {
        std::env::current_exe().context("could not determine schedx binary path")
    }

    fn service_path(&self) -> PathBuf {
        self.unit_dir.join(SERVICE_NAME)
    }

    fn timer_path(&self) -> PathBuf {
        self.unit_dir.join(TIMER_NAME)
    }

    fn write_units(&self) -> Result<()> {
        let schedx_bin = Self::schedx_binary_path()?;
        let schedx_path = schedx_bin.display();

        fs::create_dir_all(&self.unit_dir).with_context(|| {
            format!(
                "failed to create systemd unit directory: {}",
                self.unit_dir.display()
            )
        })?;

        let service_content = format!(
            "\
[Unit]
Description=schedx dispatcher tick

[Service]
Type=oneshot
ExecStart={schedx_path} _dispatch
"
        );

        let timer_content = "\
[Unit]
Description=schedx dispatcher timer (every minute)

[Timer]
OnCalendar=*-*-* *:*:00
Persistent=true
AccuracySec=1s
Unit=schedx-dispatch.service

[Install]
WantedBy=timers.target
";

        fs::write(self.service_path(), service_content)
            .context("failed to write schedx-dispatch.service")?;
        fs::write(self.timer_path(), timer_content)
            .context("failed to write schedx-dispatch.timer")?;

        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn reload_and_enable(&self) -> Result<()> {
        run_systemctl(&["daemon-reload"])?;
        run_systemctl(&["enable", "--now", TIMER_NAME])?;
        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn check_linger(&self) -> Result<()> {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        // Check if linger is enabled for this user
        let linger_path = PathBuf::from(format!("/var/lib/systemd/linger/{user}"));
        if !linger_path.exists() {
            bail!(
                "Systemd user lingering is disabled; jobs will not run after reboot until login.\n\
                 Run: sudo loginctl enable-linger {user}"
            );
        }
        Ok(())
    }
}

impl Backend for SystemdBackend {
    fn ensure_dispatcher(&self) -> Result<()> {
        self.check_linger()?;
        self.write_units()?;
        self.reload_and_enable()?;
        Ok(())
    }

    fn remove_dispatcher(&self) -> Result<()> {
        // Best-effort: stop and disable timer, remove unit files
        let _ = run_systemctl(&["stop", TIMER_NAME]);
        let _ = run_systemctl(&["disable", TIMER_NAME]);
        let _ = fs::remove_file(self.service_path());
        let _ = fs::remove_file(self.timer_path());
        let _ = run_systemctl(&["daemon-reload"]);
        Ok(())
    }

    fn check_health(&self) -> Result<BackendHealth> {
        let mut messages = Vec::new();
        let mut healthy = true;

        // Check if timer unit exists
        if !self.timer_path().exists() {
            messages.push("Timer unit file missing".to_string());
            healthy = false;
        }
        if !self.service_path().exists() {
            messages.push("Service unit file missing".to_string());
            healthy = false;
        }

        // Check if timer is active
        if let Ok(output) = run_systemctl(&["is-active", TIMER_NAME]) {
            let status = output.trim().to_string();
            if status == "active" {
                messages.push("Timer is active".to_string());
            } else {
                messages.push(format!("Timer is {status}, expected active"));
                healthy = false;
            }
        } else {
            messages.push("Could not check timer status".to_string());
            healthy = false;
        }

        // Check linger
        if let Err(e) = self.check_linger() {
            messages.push(format!("{e}"));
        } else {
            messages.push("User lingering is enabled".to_string());
        }

        Ok(BackendHealth { healthy, messages })
    }

    fn name(&self) -> &'static str {
        "systemd"
    }
}

fn systemd_user_unit_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".config/systemd/user"))
}

fn run_systemctl(args: &[&str]) -> Result<String> {
    let mut cmd_args = vec!["--user"];
    cmd_args.extend(args);

    let output = Command::new("systemctl")
        .args(&cmd_args)
        .output()
        .context("failed to run systemctl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("systemctl {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
