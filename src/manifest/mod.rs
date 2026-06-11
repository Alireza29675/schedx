//! Declarative manifest support (`schedx.yaml`).
//!
//! This module turns a yaml file into a validated, normalized,
//! env-expanded [`Manifest`] value. Reconciliation against the store
//! (`schedx up` / `schedx down`) happens elsewhere.

pub mod env;
pub mod parse;
pub mod plan;
pub mod state;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::action::Action;

/// A validated, normalized, env-expanded manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Resolved manifest name (explicit `name:` or derived from the directory).
    pub name: String,
    /// Absolute canonicalized path of the yaml file.
    pub path: PathBuf,
    /// Job name -> desired spec.
    pub jobs: BTreeMap<String, JobSpec>,
}

/// Desired state for one job, post-defaults, post-`${VAR}` expansion, validated.
///
/// Serialized into the manifest state file as the applied spec, so drift
/// classification is a structural compare against what was last applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    /// Raw schedule string, validated parseable (re-parsed at apply time).
    pub schedule_input: String,
    /// The action, built via the shared `action_input` builders.
    pub action: Action,
    /// `None` = "use config default" (resolved later, at apply).
    pub timeout_seconds: Option<u64>,
    pub tags: Vec<String>,
    /// Tri-state: `Some(true)` = ensure paused, `Some(false)` = ensure active,
    /// `None` = don't manage paused state.
    pub paused: Option<bool>,
    pub on_failure: Option<String>,
    pub on_failure_shell: bool,
}
