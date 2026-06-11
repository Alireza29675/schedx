//! Per-manifest state files (`$SCHEDX_HOME/manifests/<name>.json`).
//!
//! A state file records which jobs a manifest owns and the spec that was
//! last applied for each, enabling drift classification and `down`. It is
//! deliberately redundant with the `managed_by` markers on jobs themselves:
//! the markers are the ground truth of ownership (they live in `jobs.json`,
//! written in the same locked window), and reconciliation re-derives the
//! state file from them when the two disagree.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::manifest::JobSpec;
use crate::store::atomic::atomic_write_json;
use crate::store::paths;

pub const MANIFEST_STATE_SCHEMA_VERSION: u32 = 1;

/// Top-level manifest state file schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestState {
    pub schema_version: u32,
    pub manifest_name: String,
    /// Absolute path of the yaml last applied — the cross-prune guard:
    /// `up`/`down` refuse when the current file resolves elsewhere
    /// (unless `--force` accepts the move).
    pub manifest_path: PathBuf,
    pub applied_at: DateTime<Utc>,
    /// Declared job name -> what was applied for it.
    pub jobs: BTreeMap<String, AppliedJob>,
}

/// One declared job's applied record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedJob {
    /// Id of the live job in `jobs.json`.
    pub job_id: String,
    /// The spec as last applied (post-defaults, post-`${VAR}` expansion).
    pub applied_spec: JobSpec,
    pub applied_at: DateTime<Utc>,
}

/// Load a manifest's state file. `Ok(None)` if it does not exist.
pub fn load_manifest_state(manifest_name: &str) -> Result<Option<ManifestState>> {
    load_manifest_state_at(&paths::manifest_state_file(manifest_name)?)
}

/// Save a manifest's state file atomically (0600, temp+fsync+rename).
pub fn save_manifest_state(state: &ManifestState) -> Result<()> {
    let path = paths::manifest_state_file(&state.manifest_name)?;
    atomic_write_json(&path, state)
        .with_context(|| format!("failed to save manifest state {}", path.display()))
}

/// Delete a manifest's state file. Missing file is not an error
/// (`down` is idempotent).
pub fn delete_manifest_state(manifest_name: &str) -> Result<()> {
    let path = paths::manifest_state_file(manifest_name)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            Err(e).with_context(|| format!("failed to delete manifest state {}", path.display()))
        }
    }
}

fn load_manifest_state_at(path: &Path) -> Result<Option<ManifestState>> {
    if !path.exists() {
        return Ok(None);
    }
    let data =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let state: ManifestState = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::action::Action;

    fn sample_state(dir: &Path) -> (ManifestState, PathBuf) {
        let spec = JobSpec {
            schedule_input: "every 1h".to_string(),
            action: Action::Run {
                command: "echo hi".to_string(),
                shell: false,
                workdir: None,
            },
            timeout_seconds: Some(60),
            tags: vec!["t".to_string()],
            paused: None,
            on_failure: None,
            on_failure_shell: false,
        };
        let now = Utc::now();
        let state = ManifestState {
            schema_version: MANIFEST_STATE_SCHEMA_VERSION,
            manifest_name: "demo".to_string(),
            manifest_path: dir.join("schedx.yaml"),
            applied_at: now,
            jobs: BTreeMap::from([(
                "j1".to_string(),
                AppliedJob {
                    job_id: "abc123".to_string(),
                    applied_spec: spec,
                    applied_at: now,
                },
            )]),
        };
        let path = dir.join("demo.json");
        (state, path)
    }

    #[test]
    fn round_trips_through_json() {
        let dir = tempfile::tempdir().unwrap();
        let (state, path) = sample_state(dir.path());

        atomic_write_json(&path, &state).unwrap();
        let loaded = load_manifest_state_at(&path).unwrap().unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn missing_file_loads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.json");
        assert_eq!(load_manifest_state_at(&missing).unwrap(), None);
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_default() {
        // A torn/corrupt state file must surface, never silently read as
        // "no manifest" (that would let `up` recreate and orphan jobs).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("demo.json");
        std::fs::write(&path, "{not json").unwrap();
        assert!(load_manifest_state_at(&path).is_err());
    }
}
