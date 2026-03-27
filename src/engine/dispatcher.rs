use std::process::Command;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};

use crate::engine::lock::FileLock;
use crate::engine::logger;
use crate::model::job::{InFlightRun, Job, JobStatus};
use crate::model::run_record::{LastRun, RunRecord, RunStatus, Trigger};
use crate::model::schedule::JobSchedule;
use crate::schedule::parser::{interval_is_due, latest_due_cron_time, next_cron_time};
use crate::store::config::load_config;
use crate::store::history;
use crate::store::state;
use crate::store::state::load_state;
use crate::util::id::new_run_id;

const STALE_CLAIM_GRACE_SECONDS: i64 = 10;

struct SpawnRequest {
    job_id: String,
    run_id: String,
    scheduled_for: DateTime<Utc>,
}

/// Run one dispatch tick: find all due jobs and launch `_exec` for each.
pub fn dispatch(now: DateTime<Utc>) -> Result<()> {
    let Some(_dispatch_lock) = FileLock::dispatch_non_blocking()? else {
        return Ok(());
    };

    let _ = recover_stale_claims(now)?;

    let loaded = load_state()?;
    let config = load_config()?;

    for job_id in loaded.jobs.keys() {
        process_job(job_id, now)?;
    }

    archive_completed_jobs(now, config.archive_after_hours)?;
    logger::cleanup_old_logs(config.log_retention_days)?;
    Ok(())
}

/// Transition completed one-shot jobs to archived after the configured timeout.
fn archive_completed_jobs(now: DateTime<Utc>, archive_after_hours: u64) -> Result<()> {
    if archive_after_hours == 0 {
        return Ok(());
    }

    let cutoff = now
        - Duration::hours(
            i64::try_from(archive_after_hours)
                .unwrap_or(i64::MAX)
                .min(8760),
        );

    let _state_lock = FileLock::state()?;
    let mut job_state = load_state()?;
    let mut changed = false;

    for job in job_state.jobs.values_mut() {
        if job.status != JobStatus::Completed {
            continue;
        }
        let anchor = job.completed_at.unwrap_or(job.updated_at);
        if anchor <= cutoff {
            job.status = JobStatus::Archived;
            job.updated_at = now;
            changed = true;
        }
    }

    if changed {
        state::save_state(&job_state)?;
    }

    Ok(())
}

/// Recover abandoned scheduled claims.
pub fn recover_stale_claims(now: DateTime<Utc>) -> Result<usize> {
    let loaded = load_state()?;
    let mut recovered = 0usize;

    for job_id in loaded.jobs.keys() {
        let Some(record) = maybe_recover_stale_claim(job_id, now)? else {
            continue;
        };
        history::append_record(&record)?;
        recovered += 1;
    }

    Ok(recovered)
}

fn maybe_recover_stale_claim(job_id: &str, now: DateTime<Utc>) -> Result<Option<RunRecord>> {
    let _state_lock = FileLock::state()?;
    let mut job_state = load_state()?;
    let Some(job) = job_state.jobs.get_mut(job_id) else {
        return Ok(None);
    };
    let Some(claim) = job.in_flight.clone() else {
        return Ok(None);
    };

    if now - claim.claimed_at < Duration::seconds(STALE_CLAIM_GRACE_SECONDS) {
        return Ok(None);
    }

    let Some(_job_lock) = FileLock::job_non_blocking(job_id)? else {
        return Ok(None);
    };

    let record = internal_error_record(job_id, &claim, now);
    clear_claim_from_job(job, &record);
    state::save_state(&job_state)?;
    Ok(Some(record))
}

fn process_job(job_id: &str, now: DateTime<Utc>) -> Result<()> {
    let (spawn_request, overlap_records) = {
        let _state_lock = FileLock::state()?;
        let mut job_state = load_state()?;
        let Some(job) = job_state.jobs.get_mut(job_id) else {
            return Ok(());
        };

        if job.status != JobStatus::Active {
            return Ok(());
        }

        let mut overlap_records = Vec::new();
        let spawn_request = if let Some(claim) = job.in_flight.clone() {
            if FileLock::job_non_blocking(job_id)?.is_some() {
                None
            } else {
                for due_time in due_occurrences_after(job, claim.scheduled_for, now)? {
                    if job.skip_remaining > 0 {
                        job.skip_remaining -= 1;
                    } else {
                        overlap_records.push(skipped_overlap_record(job_id, due_time, now));
                    }
                    job.last_scheduled_at = Some(due_time);
                    job.updated_at = now;
                }
                None
            }
        } else if let Some(scheduled_for) = compute_latest_due(job, now)? {
            if job.skip_remaining > 0 {
                job.skip_remaining -= 1;
                job.last_scheduled_at = Some(scheduled_for);
                job.updated_at = now;
                None
            } else {
                let run_id = new_run_id();
                job.in_flight = Some(InFlightRun {
                    run_id: run_id.clone(),
                    scheduled_for,
                    claimed_at: now,
                });
                job.updated_at = now;
                Some(SpawnRequest {
                    job_id: job.id.clone(),
                    run_id,
                    scheduled_for,
                })
            }
        } else {
            None
        };

        if spawn_request.is_some() || !overlap_records.is_empty() {
            state::save_state(&job_state)?;
        }

        (spawn_request, overlap_records)
    };

    for record in overlap_records {
        history::append_record(&record)?;
    }

    if let Some(request) = spawn_request {
        if let Err(err) = spawn_exec(&request) {
            if let Some(record) =
                clear_claim_with_internal_error(&request.job_id, &request.run_id, now)?
            {
                history::append_record(&record)?;
            }
            eprintln!("dispatch spawn error for job {}: {err:#}", request.job_id);
        }
    }

    Ok(())
}

fn clear_claim_with_internal_error(
    job_id: &str,
    run_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<RunRecord>> {
    let _state_lock = FileLock::state()?;
    let mut job_state = load_state()?;
    let Some(job) = job_state.jobs.get_mut(job_id) else {
        return Ok(None);
    };
    let Some(claim) = job.in_flight.clone() else {
        return Ok(None);
    };
    if claim.run_id != run_id {
        return Ok(None);
    }

    let record = internal_error_record(job_id, &claim, now);
    clear_claim_from_job(job, &record);
    state::save_state(&job_state)?;
    Ok(Some(record))
}

fn clear_claim_from_job(job: &mut Job, record: &RunRecord) {
    job.in_flight = None;
    job.updated_at = record.finished_at;
    job.last_run = Some(LastRun {
        run_id: record.run_id.clone(),
        started_at: record.started_at,
        finished_at: record.finished_at,
        status: record.status,
        exit_code: record.exit_code,
        log_path: record.log_path.clone(),
    });
}

fn internal_error_record(
    job_id: &str,
    claim: &InFlightRun,
    finished_at: DateTime<Utc>,
) -> RunRecord {
    RunRecord {
        run_id: claim.run_id.clone(),
        job_id: job_id.to_string(),
        trigger: Trigger::Scheduled,
        scheduled_for: claim.scheduled_for,
        started_at: claim.claimed_at,
        finished_at,
        status: RunStatus::InternalError,
        exit_code: None,
        log_path: String::new(),
        failed_run_id: None,
    }
}

fn skipped_overlap_record(
    job_id: &str,
    scheduled_for: DateTime<Utc>,
    now: DateTime<Utc>,
) -> RunRecord {
    RunRecord {
        run_id: new_run_id(),
        job_id: job_id.to_string(),
        trigger: Trigger::Scheduled,
        scheduled_for,
        started_at: now,
        finished_at: now,
        status: RunStatus::SkippedOverlap,
        exit_code: None,
        log_path: String::new(),
        failed_run_id: None,
    }
}

fn due_occurrences_after(
    job: &Job,
    after: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>> {
    let mut anchor = job.last_scheduled_at.unwrap_or(job.created_at);
    if after > anchor {
        anchor = after;
    }

    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            let mut due_times = Vec::new();
            let mut cursor = anchor;
            while let Some(next) = next_cron_time(expr, cursor)? {
                if next > now {
                    break;
                }
                due_times.push(next);
                cursor = next;
            }
            Ok(due_times)
        }
        JobSchedule::RecurringInterval { every_seconds } => {
            let secs = i64::try_from(*every_seconds).unwrap_or(i64::MAX);
            let mut due_times = Vec::new();
            let mut next = anchor + Duration::seconds(secs);
            while next <= now {
                due_times.push(next);
                next += Duration::seconds(secs);
            }
            Ok(due_times)
        }
        JobSchedule::OneShot { .. } => Ok(Vec::new()),
    }
}

/// Determine the latest due time for a job.
fn compute_latest_due(job: &Job, now: DateTime<Utc>) -> Result<Option<DateTime<Utc>>> {
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

/// Spawn `schedx _exec <job-id> --scheduled-for <ts> --trigger <trigger>` as a detached process.
fn spawn_exec(request: &SpawnRequest) -> Result<()> {
    let schedx_bin = std::env::current_exe().context("could not determine schedx binary path")?;
    let mut cmd = Command::new(schedx_bin);
    cmd.args([
        "_exec",
        &request.job_id,
        "--scheduled-for",
        &request.scheduled_for.to_rfc3339(),
        "--trigger",
        "scheduled",
        "--run-id",
        &request.run_id,
    ]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    detach_exec_process(&mut cmd);
    cmd.spawn()
        .with_context(|| format!("failed to spawn _exec for job {}", request.job_id))?;
    Ok(())
}

#[cfg(unix)]
fn detach_exec_process(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(windows)]
fn detach_exec_process(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;

    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn detach_exec_process(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::model::action::Action;

    fn recurring_interval_job(
        created_at: DateTime<Utc>,
        last_scheduled_at: Option<DateTime<Utc>>,
        in_flight: Option<InFlightRun>,
    ) -> Job {
        Job {
            id: "job".to_string(),
            name: Some("job".to_string()),
            status: JobStatus::Active,
            schedule_input: "every 10s".to_string(),
            schedule: JobSchedule::RecurringInterval { every_seconds: 10 },
            action: Action::Run {
                command: "echo hi".to_string(),
                shell: false,
                workdir: None,
            },
            timeout_seconds: 30,
            tags: Vec::new(),
            created_at,
            updated_at: created_at,
            last_scheduled_at,
            last_run: None,
            run_count: 0,
            skip_remaining: 0,
            in_flight,
            on_failure: None,
            on_failure_shell: false,
            completed_at: None,
        }
    }

    #[test]
    fn due_occurrences_follow_claim_without_duplicates() {
        let created_at = Utc.with_ymd_and_hms(2026, 3, 11, 22, 0, 0).unwrap();
        let claim = InFlightRun {
            run_id: "r1".to_string(),
            scheduled_for: created_at + Duration::seconds(10),
            claimed_at: created_at + Duration::seconds(10),
        };
        let job = recurring_interval_job(created_at, None, Some(claim));
        let due = due_occurrences_after(
            &job,
            created_at + Duration::seconds(10),
            created_at + Duration::seconds(35),
        )
        .expect("due occurrences");
        assert_eq!(
            due,
            vec![
                created_at + Duration::seconds(20),
                created_at + Duration::seconds(30),
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_dispatch_spawn_uses_detached_process_flags() {
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        assert_eq!(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP, 0x0000_0208);
    }
}
