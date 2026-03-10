use anyhow::Result;

use crate::commands::get::find_job;
use crate::engine::logger;
use crate::output::time::format_datetime_with_relative;
use crate::store::history;

fn print_run_header(job_id: &str, run_id: &str) -> Result<()> {
    let mut header = format!("Run: {run_id}");

    if let Some(record) = history::load_records(Some(job_id), None)?
        .into_iter()
        .find(|r| r.run_id == run_id)
    {
        header = format!(
            "Run: {run_id} at {} ({})",
            format_datetime_with_relative(record.finished_at, chrono::Utc::now()),
            record.status,
        );
    }

    println!("{header}");
    println!("Logs:");
    Ok(())
}

pub fn execute(id: &str, run_id: Option<&str>, lines: Option<usize>) -> Result<()> {
    let job = find_job(id)?;

    let content = match run_id {
        Some(rid) => logger::read_run_log(&job.id, rid, lines)?,
        None => logger::read_latest_log(&job.id, lines)?,
    };

    if let Some(rid) = run_id {
        print_run_header(&job.id, rid)?;
    }

    print!("{content}");
    if !content.ends_with('\n') {
        println!();
    }

    Ok(())
}
