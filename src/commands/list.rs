use anyhow::Result;

use crate::model::job::JobStatus;
use crate::output::format::JobListEntry;
use crate::output::table;
use crate::store::config::load_config;
use crate::store::state;

pub fn execute(
    status: Option<&str>,
    tag: Option<&str>,
    all: bool,
    json_output: bool,
) -> Result<()> {
    let status_filter = status
        .map(|s| {
            s.parse::<JobStatus>().map_err(|_| {
                anyhow::anyhow!(
                    "Error: Invalid status filter '{s}'. Use: active, paused, completed, archived"
                )
            })
        })
        .transpose()?;

    let state = state::load_state()?;
    let mut jobs: Vec<_> = state.jobs.values().collect();

    // Apply filters
    if let Some(sf) = status_filter {
        jobs.retain(|j| j.status == sf);
    } else if !all {
        // Default: hide archived jobs
        jobs.retain(|j| j.status != JobStatus::Archived);
    }
    if let Some(t) = tag {
        jobs.retain(|j| j.tags.iter().any(|jt| jt == t));
    }

    // Count archived for footer hint (only when not explicitly filtering)
    let archived_count = if status_filter.is_none() && !all {
        state
            .jobs
            .values()
            .filter(|j| j.status == JobStatus::Archived)
            .count()
    } else {
        0
    };

    // Sort by creation time (newest first)
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let threshold = load_config()
        .map(|c| c.consecutive_failure_threshold)
        .unwrap_or(5);

    if json_output {
        let entries: Vec<_> = jobs.iter().map(|j| JobListEntry::from_job(j)).collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("{}", table::format_job_table(&jobs, threshold));
        if archived_count > 0 {
            eprintln!(
                "{archived_count} archived job{} hidden. Use --all to show.",
                if archived_count == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}
