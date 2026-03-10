use anyhow::{Result, bail};
use chrono::Utc;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::model::job::JobStatus;
use crate::output::format::compute_next_run;
use crate::output::time::format_datetime_with_relative;
use crate::schedule::parser::next_cron_time;
use crate::store::state;

/// Compute the effective next execution time after `n` skips from `start`.
fn next_run_after_skips(job: &crate::model::job::Job, n: u32) -> Option<chrono::DateTime<Utc>> {
    use crate::model::schedule::JobSchedule;

    if n == 0 {
        return compute_next_run(job);
    }

    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            let mut cursor = job.last_scheduled_at.unwrap_or(job.created_at);
            // Advance cursor n+1 times (n skips + 1 for the actual next run)
            for _ in 0..=n {
                match next_cron_time(expr, cursor) {
                    Ok(Some(t)) => cursor = t,
                    _ => return None,
                }
            }
            Some(cursor)
        }
        JobSchedule::RecurringInterval { every_seconds } => {
            let anchor = job.last_scheduled_at.unwrap_or(job.created_at);
            let secs = i64::try_from(*every_seconds).unwrap_or(i64::MAX);
            let total_skip = i64::from(n + 1);
            Some(anchor + chrono::Duration::seconds(secs * total_skip))
        }
        JobSchedule::OneShot { .. } => {
            // Can't skip one-shot jobs
            None
        }
    }
}

pub fn execute(id: &str, times: u32, json_output: bool) -> Result<()> {
    let job = find_job(id)?;

    if job.status != JobStatus::Active {
        bail!(
            "Job {} ({}) is not active (status: {}). Only active recurring jobs can be skipped.",
            job.display_name(),
            job.id,
            job.status
        );
    }

    if job.is_one_shot() {
        bail!(
            "Job {} ({}) is a one-shot job and cannot be skipped.\n\
             Use 'schedx pause' instead to prevent it from running.",
            job.display_name(),
            job.id
        );
    }

    if times == 0 {
        bail!("Error: --times must be greater than zero.");
    }

    // Compute the effective next execution time (after the skips)
    let new_total = job.skip_remaining + times;
    let effective_next = next_run_after_skips(&job, new_total);

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(&job.id) {
            j.skip_remaining += times;
            j.updated_at = Utc::now();
        }
        Ok(())
    })?;

    if json_output {
        let updated_job = find_job(&job.id)?;
        let output = crate::output::format::JobDetail::from_job(&updated_job);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let now = Utc::now();
        let next_str = effective_next.map_or_else(
            || "unknown".to_string(),
            |t| format_datetime_with_relative(t, now),
        );
        let times_word = if new_total == 1 { "run" } else { "runs" };
        println!(
            "Skipping next {new_total} {times_word} of job {} ({}). Next execution: {next_str}",
            job.display_name(),
            job.id,
        );
    }

    Ok(())
}
