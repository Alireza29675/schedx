use chrono::Utc;
use serde::Serialize;

use crate::model::action::Action;
use crate::model::job::Job;
use crate::model::run_record::RunRecord;
use crate::output::time::{format_datetime, format_datetime_with_relative};
use crate::schedule::parser::next_cron_time;
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
        }
    }
}

pub fn compute_next_run(job: &Job) -> Option<chrono::DateTime<chrono::Utc>> {
    use crate::model::job::JobStatus;
    use crate::model::schedule::JobSchedule;
    use crate::schedule::parser::next_interval_time;

    if job.status != JobStatus::Active {
        return None;
    }

    match &job.schedule {
        JobSchedule::RecurringCron { expr } => {
            let after = job.last_scheduled_at.unwrap_or(job.created_at);
            next_cron_time(expr, after).ok().flatten()
        }
        JobSchedule::RecurringInterval { every_seconds } => Some(next_interval_time(
            *every_seconds,
            job.last_scheduled_at,
            job.created_at,
            chrono::Utc::now(),
        )),
        JobSchedule::OneShot { fire_at } => {
            if job.last_scheduled_at.is_none() {
                Some(*fire_at)
            } else {
                None
            }
        }
    }
}
