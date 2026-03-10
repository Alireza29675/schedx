use anyhow::Result;

use crate::commands::get::find_job;
use crate::output::format::HistoryEntry;
use crate::output::table;
use crate::store::history;

pub fn execute(id: Option<&str>, limit: usize, json_output: bool) -> Result<()> {
    // If an ID is given, resolve it to ensure it exists
    let job_id = id.map(|i| find_job(i).map(|j| j.id)).transpose()?;

    let records = history::load_records(job_id.as_deref(), Some(limit))?;

    if json_output {
        let entries: Vec<_> = records.iter().map(HistoryEntry::from_record).collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("{}", table::format_history_table(&records));
    }

    Ok(())
}
