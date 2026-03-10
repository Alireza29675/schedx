use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a completed (or skipped) run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Success,
    Failed,
    Timeout,
    SkippedOverlap,
    InternalError,
}

impl RunStatus {
    pub fn is_internal_error(self) -> bool {
        self == Self::InternalError
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::SkippedOverlap => "skipped_overlap",
            Self::InternalError => "internal_error",
        }
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Record of a single run attempt, stored in `run-history.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub job_id: String,
    pub trigger: Trigger,
    pub scheduled_for: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub log_path: String,
}

/// What triggered a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Scheduled,
    Manual,
}

impl std::str::FromStr for Trigger {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scheduled" => Ok(Self::Scheduled),
            "manual" => Ok(Self::Manual),
            other => Err(format!("unknown trigger: {other}")),
        }
    }
}

impl std::fmt::Display for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scheduled => write!(f, "scheduled"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

/// Summary of the last run, stored inline in job state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastRun {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub log_path: String,
}
