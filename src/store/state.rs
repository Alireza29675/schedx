use std::fs;

use anyhow::{Context, Result};

use crate::model::job::{Job, JobState};
use crate::store::backup;

use super::atomic::atomic_write_json;
use super::paths;

/// Load the job state from disk. Returns default state if file does not exist.
pub fn load_state() -> Result<JobState> {
    let path = paths::jobs_file()?;
    if !path.exists() {
        return Ok(JobState::default());
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let state: JobState = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(state)
}

/// Save the job state to disk atomically, creating a backup first.
pub fn save_state(state: &JobState) -> Result<()> {
    let path = paths::jobs_file()?;

    // Backup existing file before overwrite
    if path.exists() {
        backup::create_backup(&path)?;
    }

    atomic_write_json(&path, state).context("failed to save job state")?;
    Ok(())
}

/// Load a single job by ID from state.
pub fn load_job(job_id: &str) -> Result<Option<Job>> {
    let state = load_state()?;
    Ok(state.jobs.get(job_id).cloned())
}

/// Update state via a closure that receives mutable access.
/// Handles load -> modify -> save atomically (under lock held by caller).
pub fn update_state<F>(f: F) -> Result<JobState>
where
    F: FnOnce(&mut JobState) -> Result<()>,
{
    let mut state = load_state()?;
    f(&mut state)?;
    save_state(&state)?;
    Ok(state)
}
