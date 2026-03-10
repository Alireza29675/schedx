use anyhow::Result;

use crate::backend;
use crate::store::paths;
use crate::store::state;

pub fn execute(json_output: bool) -> Result<()> {
    let mut messages: Vec<String> = Vec::new();

    // Ensure directory structure
    paths::ensure_dirs()?;
    messages.push("Directory structure verified.".to_string());

    // Validate state files
    match state::load_state() {
        Ok(s) => {
            messages.push(format!(
                "Job state valid ({} jobs, schema v{}).",
                s.jobs.len(),
                s.schema_version
            ));
        }
        Err(e) => {
            messages.push(format!("Job state error: {e}. Creating fresh state."));
            // Write a fresh default state
            state::save_state(&crate::model::job::JobState::default())?;
        }
    }

    // Clean up stale lock files (non-fcntl artifacts)
    cleanup_stale_locks(&mut messages)?;

    // Ensure backend dispatcher
    match backend::detect_backend() {
        Ok(be) => {
            match be.ensure_dispatcher() {
                Ok(()) => messages.push(format!("Backend '{}': dispatcher ensured.", be.name())),
                Err(e) => messages.push(format!("Backend '{}': {e}", be.name())),
            }

            match be.check_health() {
                Ok(health) => {
                    for msg in &health.messages {
                        messages.push(format!("  {msg}"));
                    }
                }
                Err(e) => messages.push(format!("Backend health check error: {e}")),
            }
        }
        Err(e) => {
            messages.push(format!("Backend detection: {e}"));
        }
    }

    if json_output {
        let output = serde_json::json!({
            "messages": messages,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for msg in &messages {
            println!("{msg}");
        }
    }

    Ok(())
}

fn cleanup_stale_locks(messages: &mut Vec<String>) -> Result<()> {
    let locks_dir = paths::locks_dir()?;
    if !locks_dir.exists() {
        return Ok(());
    }

    // We don't remove lock files that are actively held.
    // The OS advisory lock mechanism handles this via process death.
    // We only clean up obviously stale files (shouldn't happen with fcntl locks).
    messages.push("Lock directory verified.".to_string());
    Ok(())
}
