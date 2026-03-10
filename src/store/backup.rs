use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;

use super::config::load_config;
use super::paths;

/// Create a timestamped backup of a state file.
pub fn create_backup(source: &Path) -> Result<()> {
    let backups_dir = paths::backups_dir()?;
    fs::create_dir_all(&backups_dir)?;

    let stem = source.file_stem().map_or_else(
        || "unknown".to_string(),
        |s| s.to_string_lossy().to_string(),
    );

    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
    let backup_name = format!("{stem}-{timestamp}.json");
    let backup_path = backups_dir.join(&backup_name);

    fs::copy(source, &backup_path).with_context(|| {
        format!(
            "failed to create backup: {} -> {}",
            source.display(),
            backup_path.display()
        )
    })?;

    paths::set_file_permissions(&backup_path)?;

    // Rotate old backups
    rotate_backups()?;
    Ok(())
}

/// Keep only the latest `backup_count` backup files.
fn rotate_backups() -> Result<()> {
    let config = load_config()?;
    let max = config.backup_count as usize;
    let backups_dir = paths::backups_dir()?;

    if !backups_dir.exists() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&backups_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    if entries.len() <= max {
        return Ok(());
    }

    // Sort by name (contains timestamp) in ascending order
    entries.sort_by_key(std::fs::DirEntry::file_name);

    let to_remove = entries.len() - max;
    for entry in entries.into_iter().take(to_remove) {
        fs::remove_file(entry.path()).ok();
    }

    Ok(())
}
