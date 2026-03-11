use chrono::{DateTime, Utc};

use crate::model::job::{Job, JobStatus};
use crate::model::run_record::RunRecord;
use crate::model::schedule::JobSchedule;
use crate::output::format::{compute_next_run, compute_next_run_ignoring_status};
use crate::output::time::{format_datetime, format_datetime_with_relative, format_duration_short};

/// Build the schedule description line for a job.
fn format_schedule_line(job: &Job) -> String {
    match &job.schedule {
        JobSchedule::OneShot { fire_at } => {
            format!("once at {}", format_datetime(*fire_at))
        }
        JobSchedule::RecurringInterval { every_seconds } => {
            let input = &job.schedule_input;
            format!("{input}  (every {})", format_duration_short(*every_seconds))
        }
        JobSchedule::RecurringCron { expr } => {
            let input = &job.schedule_input;
            if input.starts_with("every ") || input.starts_with("*/") || looks_like_raw_cron(input)
            {
                if input == expr {
                    format!("cron {expr}")
                } else {
                    format!("{input}  ({expr})")
                }
            } else {
                format!("cron {expr}")
            }
        }
    }
}

fn looks_like_raw_cron(s: &str) -> bool {
    s.split_whitespace().count() == 5
}

/// Compute next run time ignoring job status (for paused "would run" display).
fn compute_next_run_raw(job: &Job) -> Option<DateTime<Utc>> {
    compute_next_run_ignoring_status(job)
}

/// Build the next-run / status line for a job.
fn format_status_line(job: &Job, now: DateTime<Utc>) -> String {
    match job.status {
        JobStatus::Completed => {
            if let Some(ref last) = job.last_run {
                format!("Last run: {}", last.status.as_str())
            } else {
                "Completed".to_string()
            }
        }
        JobStatus::Paused => {
            if let Some(next_time) = compute_next_run_raw(job) {
                format!("Paused  (would run {})", format_datetime(next_time))
            } else {
                "Paused".to_string()
            }
        }
        JobStatus::Active => {
            let next = compute_next_run(job);
            if let Some(next_time) = next {
                let skip_suffix = if job.skip_remaining > 0 {
                    format!(" [skipping {}]", job.skip_remaining)
                } else {
                    String::new()
                };
                let label = if next_time <= now { "Due" } else { "Next" };
                format!(
                    "{label}: {}{}",
                    format_datetime_with_relative(next_time, now),
                    skip_suffix
                )
            } else {
                "Active".to_string()
            }
        }
    }
}

/// Format a list of jobs as a human-readable card-style list.
pub fn format_job_table(jobs: &[&Job]) -> String {
    if jobs.is_empty() {
        return "No jobs found.".to_string();
    }

    let now = Utc::now();
    let mut blocks: Vec<String> = Vec::new();

    for job in jobs {
        let name = job.display_name();
        let truncated_name = if name.len() > 30 {
            format!("{}...", &name[..27])
        } else {
            name.to_string()
        };

        let status_str = job.status.to_string();
        let type_str = job.action.kind_str();

        // Line 1: id  name                            status  type
        let line1 = format!(
            "  {:<8} {:<36} {:>9}  {}",
            job.id, truncated_name, status_str, type_str,
        );

        // Line 2: schedule description
        let line2 = format!("           {}", format_schedule_line(job));

        // Line 3: next run / last status
        let line3 = format!("           {}", format_status_line(job, now));

        blocks.push(format!("{line1}\n{line2}\n{line3}"));
    }

    blocks.join("\n\n")
}

/// Format a single job's details for human-readable output.
pub fn format_job_detail(job: &Job) -> String {
    let now = Utc::now();
    let mut lines = Vec::new();
    lines.push(format!("Job: {} ({})", job.display_name(), job.id));
    lines.push(format!("Status: {}", job.status));
    lines.push(format!("Schedule: {}", format_schedule_line(job)));
    lines.push(format!("Type: {}", job.action.kind_str()));
    lines.push(format!("Timeout: {}s", job.timeout_seconds));

    if !job.tags.is_empty() {
        lines.push(format!("Tags: {}", job.tags.join(", ")));
    }

    if let Some(next) = compute_next_run(job) {
        let skip_note = if job.skip_remaining > 0 {
            format!(" [skipping next {}]", job.skip_remaining)
        } else {
            String::new()
        };
        let label = if next <= now { "Due" } else { "Next run" };
        lines.push(format!(
            "{label}: {}{}",
            format_datetime_with_relative(next, now),
            skip_note
        ));
    }

    if job.skip_remaining > 0 {
        lines.push(format!("Skip remaining: {}", job.skip_remaining));
    }

    lines.push(format!(
        "Created: {}",
        format_datetime_with_relative(job.created_at, now)
    ));
    lines.push(format!(
        "Updated: {}",
        format_datetime_with_relative(job.updated_at, now)
    ));
    lines.push(format!("Run count: {}", job.run_count));

    if let Some(ref last) = job.last_run {
        lines.push(format!(
            "Last run: {} at {} ({})",
            last.run_id,
            format_datetime(last.finished_at),
            last.status,
        ));
    }

    lines.join("\n")
}

/// Format history records as a human-readable table.
pub fn format_history_table(records: &[RunRecord]) -> String {
    if records.is_empty() {
        return "No history records found.".to_string();
    }

    let now = Utc::now();
    let mut lines = Vec::new();

    for (i, record) in records.iter().enumerate() {
        if i > 0 {
            lines.push(String::new());
        }
        lines.push(format!(
            "  {:<24} {:<10} {:<10} {}",
            record.run_id, record.job_id, record.status, record.trigger,
        ));
        lines.push(format!(
            "    Finished: {}",
            format_datetime_with_relative(record.finished_at, now)
        ));
    }

    lines.join("\n")
}
