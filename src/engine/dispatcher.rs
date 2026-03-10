use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::engine::lock::FileLock;
use crate::engine::logger;
use crate::model::job::JobStatus;
use crate::model::schedule::JobSchedule;
use crate::schedule::parser::{interval_is_due, latest_due_cron_time};
use crate::store::config::load_config;
use crate::store::state;
use crate::store::state::load_state;

/// Run one dispatch tick: find all due jobs and launch `_exec` for each.
pub fn dispatch(now: DateTime<Utc>) -> Result<()> {
    // Acquire dispatch lock (non-blocking)
    let Some(_dispatch_lock) = FileLock::dispatch_non_blocking()? else {
        // Another dispatch is already running — silently exit
        return Ok(());
    };

    let loaded = load_state()?;
    let config = load_config()?;

    for job in loaded
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Active)
    {
        if let Some(scheduled_for) = compute_latest_due(job, now)? {
            if job.skip_remaining > 0 {
                // Skip this run: advance last_scheduled_at and decrement counter
                skip_run(&job.id, scheduled_for)?;
            } else {
                spawn_exec(&job.id, scheduled_for, "scheduled")?;
            }
        }
    }

    // Lightweight cleanup
    logger::cleanup_old_logs(config.log_retention_days)?;

    Ok(())
}

/// Advance a job past a skipped run without executing it.
fn skip_run(job_id: &str, scheduled_for: DateTime<Utc>) -> Result<()> {
    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(job_id) {
            j.last_scheduled_at = Some(scheduled_for);
            j.skip_remaining = j.skip_remaining.saturating_sub(1);
            j.updated_at = Utc::now();
        }
        Ok(())
    })?;
    Ok(())
}

/// Determine the latest due time for a job.
///
/// - Recurring cron: find the latest cron occurrence in `(last_scheduled_at, now]`
/// - Recurring interval: check if `now >= anchor + every_seconds`
/// - One-shot: if active and `fire_at <= now` and never scheduled -> due
fn compute_latest_due(
    job: &crate::model::job::Job,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>> {
    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            let after = job.last_scheduled_at.unwrap_or(job.created_at);
            latest_due_cron_time(expr, after, now)
        }
        JobSchedule::RecurringInterval { every_seconds } => Ok(interval_is_due(
            *every_seconds,
            job.last_scheduled_at,
            job.created_at,
            now,
        )),
        JobSchedule::OneShot { fire_at } => {
            if job.last_scheduled_at.is_none() && *fire_at <= now {
                Ok(Some(*fire_at))
            } else {
                Ok(None)
            }
        }
    }
}

/// Spawn `sched _exec <job-id> --scheduled-for <ts> --trigger <trigger>` as a background process.
fn spawn_exec(job_id: &str, scheduled_for: DateTime<Utc>, trigger: &str) -> Result<()> {
    let sched_bin = std::env::current_exe().context("could not determine sched binary path")?;

    std::process::Command::new(sched_bin)
        .args([
            "_exec",
            job_id,
            "--scheduled-for",
            &scheduled_for.to_rfc3339(),
            "--trigger",
            trigger,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn _exec for job {job_id}"))?;

    Ok(())
}
