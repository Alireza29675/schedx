use anyhow::Result;

use crate::output::format::JobDetail;
use crate::output::table;
use crate::store::state;

pub fn execute(id: &str, json_output: bool) -> Result<()> {
    let job = find_job(id)?;

    if json_output {
        let detail = JobDetail::from_job(&job);
        println!("{}", serde_json::to_string_pretty(&detail)?);
    } else {
        println!("{}", table::format_job_detail(&job));
    }

    Ok(())
}

/// Find a job by ID or name.
pub fn find_job(id_or_name: &str) -> Result<crate::model::job::Job> {
    let state = state::load_state()?;

    // Try exact ID match first
    if let Some(job) = state.jobs.get(id_or_name) {
        return Ok(job.clone());
    }

    // Try name match
    let by_name: Vec<_> = state
        .jobs
        .values()
        .filter(|j| j.name.as_deref() == Some(id_or_name))
        .collect();

    match by_name.len() {
        0 => anyhow::bail!("Error: Job '{id_or_name}' not found."),
        1 => Ok(by_name[0].clone()),
        _ => {
            anyhow::bail!("Error: Multiple jobs match name '{id_or_name}'. Use the job ID instead.")
        }
    }
}
