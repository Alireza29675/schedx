use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};

use anyhow::{Context, Result};

use crate::model::run_record::RunRecord;

use super::paths;

/// Append a run record to the history file (JSONL format).
pub fn append_record(record: &RunRecord) -> Result<()> {
    let path = paths::history_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut line = serde_json::to_string(record).context("failed to serialize run record")?;
    line.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open history file: {}", path.display()))?;

    paths::set_file_permissions(&path)?;
    file.write_all(line.as_bytes())
        .context("failed to write history record")?;
    file.flush()?;
    Ok(())
}

/// Load all history records, optionally filtered by job ID.
pub fn load_records(job_id: Option<&str>, limit: Option<usize>) -> Result<Vec<RunRecord>> {
    let path = paths::history_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)
        .with_context(|| format!("failed to open history file: {}", path.display()))?;
    let reader = std::io::BufReader::new(file);

    let mut records: Vec<RunRecord> = Vec::new();
    for line in reader.lines() {
        let line = line.context("failed to read history line")?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<RunRecord>(trimmed) {
            Ok(record) => {
                if let Some(jid) = job_id {
                    if record.job_id == jid {
                        records.push(record);
                    }
                } else {
                    records.push(record);
                }
            }
            Err(_) => {
                // Skip malformed lines gracefully
                continue;
            }
        }
    }

    // Return most recent records first
    records.reverse();

    if let Some(limit) = limit {
        records.truncate(limit);
    }

    Ok(records)
}
