use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::paths;

/// Create a log file for a run, returning the file handle and relative path.
pub fn create_log_file(job_id: &str, run_id: &str) -> Result<(File, String)> {
    let job_dir = paths::job_log_dir(job_id)?;
    fs::create_dir_all(&job_dir)?;

    let filename = format!("{run_id}.log");
    let abs_path = job_dir.join(&filename);

    let file = File::create(&abs_path)
        .with_context(|| format!("failed to create log file: {}", abs_path.display()))?;
    paths::set_file_permissions(&abs_path)?;

    let rel_path = format!("logs/{job_id}/{filename}");
    Ok((file, rel_path))
}

/// Read the latest log for a job.
pub fn read_latest_log(job_id: &str, lines: Option<usize>) -> Result<String> {
    let job_dir = paths::job_log_dir(job_id)?;
    if !job_dir.exists() {
        return Ok("No logs found for this job.".to_string());
    }

    let mut entries: Vec<_> = fs::read_dir(&job_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();

    if entries.is_empty() {
        return Ok("No logs found for this job.".to_string());
    }

    // Sort by name (contains timestamp) descending to get latest
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));

    read_log_file(&entries[0].path(), lines)
}

/// Read a specific run's log.
pub fn read_run_log(job_id: &str, run_id: &str, lines: Option<usize>) -> Result<String> {
    let job_dir = paths::job_log_dir(job_id)?;
    let log_path = job_dir.join(format!("{run_id}.log"));

    if !log_path.exists() {
        anyhow::bail!("Log file not found: {}", log_path.display());
    }

    read_log_file(&log_path, lines)
}

fn read_log_file(path: &PathBuf, max_lines: Option<usize>) -> Result<String> {
    let file =
        File::open(path).with_context(|| format!("failed to open log file: {}", path.display()))?;

    if let Some(n) = max_lines {
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start = all_lines.len().saturating_sub(n);
        Ok(all_lines[start..].join("\n"))
    } else {
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Ok(content)
    }
}

/// Clean up log files older than `retention_days`.
pub fn cleanup_old_logs(retention_days: u32) -> Result<()> {
    let logs_dir = paths::logs_dir()?;
    if !logs_dir.exists() {
        return Ok(());
    }

    let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(retention_days));

    for job_entry in fs::read_dir(&logs_dir)?.filter_map(Result::ok) {
        if !job_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }

        for log_entry in fs::read_dir(job_entry.path())?.filter_map(Result::ok) {
            let path = log_entry.path();
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    let modified_dt: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified_dt < cutoff {
                        fs::remove_file(&path).ok();
                    }
                }
            }
        }

        // Remove empty job log directories
        if fs::read_dir(job_entry.path())
            .map(|mut d| d.next().is_none())
            .unwrap_or(false)
        {
            fs::remove_dir(job_entry.path()).ok();
        }
    }

    Ok(())
}
