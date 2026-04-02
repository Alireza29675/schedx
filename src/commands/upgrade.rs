use anyhow::Result;

use crate::backend;
use crate::upgrade::{self, CURRENT_VERSION, InstallMethod};

pub fn execute(force: bool, json_output: bool) -> Result<()> {
    println!("Checking for updates...");
    let latest = upgrade::fetch_latest_version()?;

    if !force && !upgrade::is_newer(CURRENT_VERSION, &latest) {
        if json_output {
            let output = serde_json::json!({
                "status": "up_to_date",
                "current_version": CURRENT_VERSION,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("schedx is already up to date (v{CURRENT_VERSION}).");
        }
        return Ok(());
    }

    println!("Upgrading schedx from v{CURRENT_VERSION} to v{latest}...\n");

    let method = upgrade::detect_install_method();

    match method {
        InstallMethod::Cargo => {
            upgrade::cargo::execute()?;
            regenerate_dispatcher(json_output);
            if json_output {
                let output = serde_json::json!({
                    "previous_version": CURRENT_VERSION,
                    "new_version": latest,
                    "method": "cargo",
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("\nSuccessfully upgraded schedx to v{latest}.");
            }
        }
        InstallMethod::Binary => {
            let (install_path, _digest) = upgrade::binary::execute(&latest)?;
            regenerate_dispatcher(json_output);
            if json_output {
                let output = serde_json::json!({
                    "previous_version": CURRENT_VERSION,
                    "new_version": latest,
                    "method": "binary",
                    "install_path": install_path,
                    "checksum_verified": true,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("\nSuccessfully upgraded schedx to v{latest}.");
            }
        }
    }

    Ok(())
}

/// Regenerate the backend dispatcher service files after upgrade so any fixes
/// (e.g. KillMode=process) take effect immediately without a manual repair step.
fn regenerate_dispatcher(json_output: bool) {
    match backend::detect_backend() {
        Ok(be) => {
            if let Err(e) = be.ensure_dispatcher() {
                if !json_output {
                    eprintln!("Warning: Could not update dispatcher service files: {e:#}");
                }
            } else if !json_output {
                println!("Dispatcher service files updated.");
            }
        }
        Err(_) => {} // No supported backend (e.g. daemon mode) — skip silently
    }
}
