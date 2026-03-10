use anyhow::Result;
use chrono::Utc;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::model::job::JobStatus;
use crate::store::state;

pub fn execute(id: &str) -> Result<()> {
    let job = find_job(id)?;

    if job.status == JobStatus::Active {
        anyhow::bail!("Job {} ({}) is already active.", job.display_name(), job.id);
    }
    if job.status == JobStatus::Completed {
        anyhow::bail!(
            "Job {} ({}) is completed and cannot be resumed.",
            job.display_name(),
            job.id
        );
    }

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(&job.id) {
            j.status = JobStatus::Active;
            j.updated_at = Utc::now();
        }
        Ok(())
    })?;

    println!("Resumed job {} ({})", job.display_name(), job.id);
    Ok(())
}
