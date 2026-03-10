use anyhow::Result;

use crate::model::job::JobStatus;
use crate::output::format::JobListEntry;
use crate::output::table;
use crate::store::state;

pub fn execute(status: Option<&str>, tag: Option<&str>, json_output: bool) -> Result<()> {
    let status_filter = status
        .map(|s| {
            s.parse::<JobStatus>().map_err(|_| {
                anyhow::anyhow!(
                    "Error: Invalid status filter '{s}'. Use: active, paused, completed"
                )
            })
        })
        .transpose()?;

    let state = state::load_state()?;
    let mut jobs: Vec<_> = state.jobs.values().collect();

    // Apply filters
    if let Some(sf) = status_filter {
        jobs.retain(|j| j.status == sf);
    }
    if let Some(t) = tag {
        jobs.retain(|j| j.tags.iter().any(|jt| jt == t));
    }

    // Sort by creation time (newest first)
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    if json_output {
        let entries: Vec<_> = jobs.iter().map(|j| JobListEntry::from_job(j)).collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("{}", table::format_job_table(&jobs));
    }

    Ok(())
}
