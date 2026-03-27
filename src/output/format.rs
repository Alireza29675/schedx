use chrono::Utc;
use serde::Serialize;

use crate::model::action::Action;
use crate::model::job::Job;
use crate::model::run_record::RunRecord;
use crate::model::schedule::JobSchedule;
use crate::output::time::{format_datetime, format_datetime_with_relative};
use crate::schedule::parser::{latest_due_cron_time, next_cron_time, next_interval_time};
use crate::util::redact;

/// JSON representation for `schedx list --json`.
#[derive(Debug, Serialize)]
pub struct JobListEntry {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
    pub schedule_input: String,
    pub next_run: Option<String>,
    pub next_run_readable: Option<String>,
    #[serde(rename = "type")]
    pub action_type: String,
    pub tags: Vec<String>,
    pub skip_remaining: u32,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_at_readable: Option<String>,
}

impl JobListEntry {
    pub fn from_job(job: &Job) -> Self {
        let now = Utc::now();
        let next_run = compute_next_run(job);
        Self {
            id: job.id.clone(),
            name: job.name.clone(),
            status: job.status.to_string(),
            schedule_input: job.schedule_input.clone(),
            next_run: next_run.map(|t| t.to_rfc3339()),
            next_run_readable: next_run.map(|t| format_datetime_with_relative(t, now)),
            action_type: job.action.kind_str().to_string(),
            tags: job.tags.clone(),
            skip_remaining: job.skip_remaining,
            last_run_status: job.last_run.as_ref().map(|r| r.status.to_string()),
            last_run_at: job.last_run.as_ref().map(|r| r.finished_at.to_rfc3339()),
            last_run_at_readable: job
                .last_run
                .as_ref()
                .map(|r| format_datetime_with_relative(r.finished_at, now)),
        }
    }
}

/// JSON representation for `schedx get --json`.
#[derive(Debug, Serialize)]
pub struct JobDetail {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
    pub schedule_input: String,
    pub next_run: Option<String>,
    pub next_run_readable: Option<String>,
    pub action: ActionDetail,
    pub timeout_seconds: u64,
    pub tags: Vec<String>,
    pub skip_remaining: u32,
    pub created_at: String,
    pub created_at_readable: String,
    pub updated_at: String,
    pub updated_at_readable: String,
    pub run_count: u64,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_at_readable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub on_failure_shell: bool,
}

#[derive(Debug, Serialize)]
pub struct ActionDetail {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

impl ActionDetail {
    pub fn from_action(action: &Action) -> Self {
        match action {
            Action::Run {
                command,
                shell,
                workdir,
            } => Self {
                kind: "run".to_string(),
                command: Some(command.clone()),
                shell: Some(*shell),
                workdir: workdir.clone(),
                url: None,
                method: None,
                headers: None,
                text: None,
                agent: None,
            },
            Action::Prompt { text, agent } => Self {
                kind: "prompt".to_string(),
                command: None,
                shell: None,
                workdir: None,
                url: None,
                method: None,
                headers: None,
                text: Some(text.clone()),
                agent: agent.clone(),
            },
            Action::Webhook {
                url,
                method,
                headers,
                ..
            } => {
                let redacted: Vec<String> = headers
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", redact::redact_header_value(k, v)))
                    .collect();
                Self {
                    kind: "webhook".to_string(),
                    command: None,
                    shell: None,
                    workdir: None,
                    url: Some(redact::redact_url(url)),
                    method: Some(method.to_string()),
                    headers: if redacted.is_empty() {
                        None
                    } else {
                        Some(redacted)
                    },
                    text: None,
                    agent: None,
                }
            }
        }
    }
}

impl JobDetail {
    pub fn from_job(job: &Job) -> Self {
        let now = Utc::now();
        let next_run = compute_next_run(job);
        Self {
            id: job.id.clone(),
            name: job.name.clone(),
            status: job.status.to_string(),
            schedule_input: job.schedule_input.clone(),
            next_run: next_run.map(|t| t.to_rfc3339()),
            next_run_readable: next_run.map(|t| format_datetime_with_relative(t, now)),
            action: ActionDetail::from_action(&job.action),
            timeout_seconds: job.timeout_seconds,
            tags: job.tags.clone(),
            skip_remaining: job.skip_remaining,
            created_at: job.created_at.to_rfc3339(),
            created_at_readable: format_datetime_with_relative(job.created_at, now),
            updated_at: job.updated_at.to_rfc3339(),
            updated_at_readable: format_datetime_with_relative(job.updated_at, now),
            run_count: job.run_count,
            last_run_status: job.last_run.as_ref().map(|r| r.status.to_string()),
            last_run_at: job.last_run.as_ref().map(|r| r.finished_at.to_rfc3339()),
            last_run_at_readable: job
                .last_run
                .as_ref()
                .map(|r| format_datetime_with_relative(r.finished_at, now)),
            on_failure: job.on_failure.as_ref().map(|cmd| {
                crate::util::redact::redact_cli_args(
                    &shell_words::split(cmd).unwrap_or_else(|_| vec![cmd.clone()]),
                )
                .join(" ")
            }),
            on_failure_shell: job.on_failure_shell,
        }
    }
}

/// JSON representation for `schedx history --json`.
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub run_id: String,
    pub job_id: String,
    pub trigger: String,
    pub scheduled_for: String,
    pub scheduled_for_readable: String,
    pub started_at: String,
    pub started_at_readable: String,
    pub finished_at: String,
    pub finished_at_readable: String,
    pub status: String,
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_run_id: Option<String>,
}

impl HistoryEntry {
    pub fn from_record(record: &RunRecord) -> Self {
        let now = Utc::now();
        Self {
            run_id: record.run_id.clone(),
            job_id: record.job_id.clone(),
            trigger: record.trigger.to_string(),
            scheduled_for: record.scheduled_for.to_rfc3339(),
            scheduled_for_readable: format_datetime(record.scheduled_for),
            started_at: record.started_at.to_rfc3339(),
            started_at_readable: format_datetime_with_relative(record.started_at, now),
            finished_at: record.finished_at.to_rfc3339(),
            finished_at_readable: format_datetime_with_relative(record.finished_at, now),
            status: record.status.to_string(),
            exit_code: record.exit_code,
            failed_run_id: record.failed_run_id.clone(),
        }
    }
}

pub fn compute_next_run(job: &Job) -> Option<chrono::DateTime<chrono::Utc>> {
    compute_next_run_inner(job, true, 0)
}

pub fn compute_next_run_after_skips(
    job: &Job,
    extra_skips: u32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    compute_next_run_inner(job, true, extra_skips)
}

pub fn compute_next_run_ignoring_status(job: &Job) -> Option<chrono::DateTime<chrono::Utc>> {
    compute_next_run_inner(job, false, 0)
}

fn compute_next_run_inner(
    job: &Job,
    require_active: bool,
    extra_skips: u32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    use crate::model::job::JobStatus;

    if require_active && job.status != JobStatus::Active {
        return None;
    }

    match &job.schedule {
        JobSchedule::OneShot { fire_at } => {
            if job.last_scheduled_at.is_none() && job.in_flight.is_none() {
                Some(*fire_at)
            } else {
                None
            }
        }
        JobSchedule::RecurringCron { .. } | JobSchedule::RecurringInterval { .. } => {
            let now = Utc::now();
            let mut anchor = scheduling_anchor(job);
            let mut remaining_skips = job.skip_remaining.saturating_add(extra_skips);

            if job.in_flight.is_some() {
                for missed_due in due_occurrences_after_anchor(job, anchor, now) {
                    anchor = missed_due;
                    remaining_skips = remaining_skips.saturating_sub(1);
                }
            } else if let Some(due_now) = latest_due_after_anchor(job, anchor, now) {
                if remaining_skips == 0 {
                    return Some(due_now);
                }
                remaining_skips -= 1;
                anchor = due_now;
            }

            let mut next = next_occurrence_after(job, anchor)?;
            while remaining_skips > 0 {
                anchor = next;
                next = next_occurrence_after(job, anchor)?;
                remaining_skips -= 1;
            }
            Some(next)
        }
    }
}

fn scheduling_anchor(job: &Job) -> chrono::DateTime<chrono::Utc> {
    let mut anchor = job.last_scheduled_at.unwrap_or(job.created_at);
    if let Some(claim) = &job.in_flight {
        if claim.scheduled_for > anchor {
            anchor = claim.scheduled_for;
        }
    }
    anchor
}

fn latest_due_after_anchor(
    job: &Job,
    anchor: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            latest_due_cron_time(expr, anchor, now).ok().flatten()
        }
        JobSchedule::RecurringInterval { every_seconds } => {
            crate::schedule::parser::interval_is_due(*every_seconds, Some(anchor), anchor, now)
        }
        JobSchedule::OneShot { .. } => None,
    }
}

fn next_occurrence_after(
    job: &Job,
    anchor: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match &job.schedule {
        JobSchedule::RecurringCron { expr } => next_cron_time(expr, anchor).ok().flatten(),
        JobSchedule::RecurringInterval { every_seconds } => Some(next_interval_time(
            *every_seconds,
            Some(anchor),
            anchor,
            Utc::now(),
        )),
        JobSchedule::OneShot { .. } => None,
    }
}

fn due_occurrences_after_anchor(
    job: &Job,
    anchor: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<chrono::DateTime<chrono::Utc>> {
    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            let mut due_times = Vec::new();
            let mut cursor = anchor;
            while let Ok(Some(next)) = next_cron_time(expr, cursor) {
                if next > now {
                    break;
                }
                due_times.push(next);
                cursor = next;
            }
            due_times
        }
        JobSchedule::RecurringInterval { every_seconds } => {
            let mut due_times = Vec::new();
            let secs = i64::try_from(*every_seconds).unwrap_or(i64::MAX);
            let mut next = anchor + chrono::Duration::seconds(secs);
            while next <= now {
                due_times.push(next);
                next += chrono::Duration::seconds(secs);
            }
            due_times
        }
        JobSchedule::OneShot { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;
    use crate::model::action::Action;
    use crate::model::job::{InFlightRun, Job, JobStatus};

    fn recurring_job(
        schedule: JobSchedule,
        created_at: chrono::DateTime<chrono::Utc>,
        last_scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
        skip_remaining: u32,
        in_flight: Option<InFlightRun>,
    ) -> Job {
        Job {
            id: "job".to_string(),
            name: Some("job".to_string()),
            status: JobStatus::Active,
            schedule_input: "schedule".to_string(),
            schedule,
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
            skip_remaining,
            in_flight,
            on_failure: None,
            on_failure_shell: false,
            completed_at: None,
        }
    }

    #[test]
    fn next_run_accounts_for_skip_remaining_when_job_is_due() {
        let now = Utc::now();
        let created_at = now - Duration::seconds(25);
        let job = recurring_job(
            JobSchedule::RecurringInterval { every_seconds: 10 },
            created_at,
            None,
            1,
            None,
        );

        let next_run = compute_next_run(&job).expect("next run");
        assert!(next_run > now);
    }

    #[test]
    fn next_run_anchors_on_in_flight_claim() {
        let now = Utc::now();
        let created_at = now - Duration::seconds(40);
        let claim = InFlightRun {
            run_id: "r1".to_string(),
            scheduled_for: now - Duration::seconds(5),
            claimed_at: now - Duration::seconds(5),
        };
        let job = recurring_job(
            JobSchedule::RecurringInterval { every_seconds: 10 },
            created_at,
            Some(now - Duration::seconds(15)),
            0,
            Some(claim),
        );

        let next_run = compute_next_run(&job).expect("next run");
        assert!(next_run > now);
    }

    #[test]
    fn one_shot_with_in_flight_claim_has_no_next_run() {
        let created_at = Utc.with_ymd_and_hms(2026, 3, 11, 22, 0, 0).unwrap();
        let claim = InFlightRun {
            run_id: "r1".to_string(),
            scheduled_for: created_at + Duration::seconds(10),
            claimed_at: created_at + Duration::seconds(10),
        };
        let job = recurring_job(
            JobSchedule::OneShot {
                fire_at: created_at + Duration::seconds(10),
            },
            created_at,
            None,
            0,
            Some(claim),
        );

        assert!(compute_next_run(&job).is_none());
    }
}
