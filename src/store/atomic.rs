use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use super::paths::set_file_permissions;

/// Atomically write `data` to `target` via temp-file + rename.
///
/// 1. Write to a temp file in the same directory.
/// 2. flush + fsync the temp file.
/// 3. rename temp -> target (atomic on POSIX).
/// 4. fsync the parent directory.
pub fn atomic_write(target: &Path, data: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .context("target path has no parent directory")?;

    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;

    let temp_path = parent.join(format!(
        ".{}.tmp",
        target.file_name().map_or_else(
            || "unknown".to_string(),
            |n| n.to_string_lossy().to_string()
        )
    ));

    // Write to temp file
    let mut file = fs::File::create(&temp_path)
        .with_context(|| format!("failed to create temp file: {}", temp_path.display()))?;
    file.write_all(data)
        .with_context(|| format!("failed to write temp file: {}", temp_path.display()))?;
    file.flush()?;
    file.sync_all()
        .with_context(|| "failed to fsync temp file")?;
    drop(file);

    // Set secure permissions before rename
    set_file_permissions(&temp_path)?;

    // Atomic rename
    fs::rename(&temp_path, target).with_context(|| {
        format!(
            "failed to rename {} -> {}",
            temp_path.display(),
            target.display()
        )
    })?;

    // fsync parent directory
    fsync_dir(parent)?;

    Ok(())
}

/// Write JSON value atomically.
pub fn atomic_write_json<T: serde::Serialize>(target: &Path, value: &T) -> Result<()> {
    let data =
        serde_json::to_vec_pretty(value).context("failed to serialize data for atomic write")?;
    atomic_write(target, &data)
}

/// fsync a directory to ensure rename durability.
fn fsync_dir(dir: &Path) -> Result<()> {
    let d = fs::File::open(dir)
        .with_context(|| format!("failed to open dir for fsync: {}", dir.display()))?;
    d.sync_all()
        .with_context(|| format!("failed to fsync dir: {}", dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.json");
        atomic_write(&target, b"{\"hello\":\"world\"}").unwrap();
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("hello"));
    }

    #[test]
    fn atomic_write_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.json");
        atomic_write(&target, b"first").unwrap();
        atomic_write(&target, b"second").unwrap();
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "second");
    }
}
