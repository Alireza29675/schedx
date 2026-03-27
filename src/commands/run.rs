use anyhow::Result;
use chrono::Utc;

use crate::commands::get::find_job;
use crate::engine::executor;
use crate::model::job::JobStatus;
use crate::model::run_record::Trigger;

pub fn execute(id: &str) -> Result<()> {
    let job = find_job(id)?;

    if job.status == JobStatus::Completed || job.status == JobStatus::Archived {
        anyhow::bail!(
            "Error: Job '{}' ({}) is {} and cannot be run.",
            job.display_name(),
            job.id,
            job.status
        );
    }

    println!(
        "Running job {} ({}) manually...",
        job.display_name(),
        job.id
    );

    let now = Utc::now();
    let success = executor::exec_job(&job.id, now, Trigger::Manual)?;

    if success {
        println!("Job completed. Check logs: schedx logs {}", job.id);
    } else {
        println!(
            "Job encountered an internal error. Check logs: schedx logs {}",
            job.id
        );
    }

    Ok(())
}
