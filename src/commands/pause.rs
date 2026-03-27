use anyhow::Result;
use chrono::Utc;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::model::job::JobStatus;
use crate::store::state;

pub fn execute(id: &str) -> Result<()> {
    let job = find_job(id)?;

    if job.status == JobStatus::Paused {
        anyhow::bail!("Job {} ({}) is already paused.", job.display_name(), job.id);
    }
    if job.status == JobStatus::Completed {
        anyhow::bail!(
            "Job {} ({}) is completed and cannot be paused.",
            job.display_name(),
            job.id
        );
    }
    if job.status == JobStatus::Archived {
        anyhow::bail!(
            "Job {} ({}) is archived. Use `schedx unarchive` to restore it.",
            job.display_name(),
            job.id
        );
    }

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(&job.id) {
            j.status = JobStatus::Paused;
            j.updated_at = Utc::now();
        }
        Ok(())
    })?;

    println!("Paused job {} ({})", job.display_name(), job.id);
    Ok(())
}
