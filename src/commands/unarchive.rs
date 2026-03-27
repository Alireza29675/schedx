use anyhow::Result;
use chrono::Utc;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::model::job::JobStatus;
use crate::store::state;

pub fn execute(id: &str) -> Result<()> {
    let job = find_job(id)?;

    if job.status != JobStatus::Archived {
        anyhow::bail!(
            "Job {} ({}) is not archived (current status: {}).",
            job.display_name(),
            job.id,
            job.status
        );
    }

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(&job.id) {
            j.status = JobStatus::Completed;
            j.updated_at = Utc::now();
        }
        Ok(())
    })?;

    println!("Unarchived job {} ({})", job.display_name(), job.id);
    Ok(())
}
