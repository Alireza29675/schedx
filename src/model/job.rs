use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::action::Action;
use super::run_record::LastRun;
use super::schedule::JobSchedule;

/// Job status lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Active,
    Paused,
    Completed,
    Archived,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            other => Err(format!("unknown status: {other}")),
        }
    }
}

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// A scheduled run that has been claimed by the dispatcher but not fully reconciled yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InFlightRun {
    pub run_id: String,
    pub scheduled_for: DateTime<Utc>,
    pub claimed_at: DateTime<Utc>,
}

/// A scheduled job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: Option<String>,
    pub status: JobStatus,
    pub schedule_input: String,
    pub schedule: JobSchedule,
    pub action: Action,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_scheduled_at: Option<DateTime<Utc>>,
    pub last_run: Option<LastRun>,
    #[serde(default)]
    pub run_count: u64,
    /// Number of upcoming scheduled runs to skip (decremented by dispatcher).
    #[serde(default)]
    pub skip_remaining: u32,
    #[serde(default)]
    pub in_flight: Option<InFlightRun>,
    /// Command to run when this job fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    /// Use shell execution for the failure command.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub on_failure_shell: bool,
    /// When a one-shot job completed (used for archive countdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of consecutive `failed/timeout/internal_error` runs since last success.
    #[serde(default)]
    pub consecutive_failures: u32,
}

impl Job {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    pub fn is_one_shot(&self) -> bool {
        self.schedule.is_one_shot()
    }
}

/// Top-level state file schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobState {
    pub schema_version: u32,
    pub jobs: std::collections::BTreeMap<String, Job>,
}

impl Default for JobState {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            jobs: std::collections::BTreeMap::new(),
        }
    }
}
