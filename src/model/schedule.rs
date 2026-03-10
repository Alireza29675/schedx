use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parsed schedule representation stored in job state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobSchedule {
    RecurringCron { expr: String },
    RecurringInterval { every_seconds: u64 },
    OneShot { fire_at: DateTime<Utc> },
}

impl JobSchedule {
    pub fn is_one_shot(&self) -> bool {
        matches!(self, Self::OneShot { .. })
    }
}

/// Result of schedule parsing, includes human-readable description.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParsedSchedule {
    RecurringCron {
        expr: String,
        human: String,
    },
    RecurringInterval {
        every_seconds: u64,
        human: String,
    },
    OneShot {
        fire_at: DateTime<Utc>,
        human: String,
    },
}

impl ParsedSchedule {
    pub fn to_job_schedule(&self) -> JobSchedule {
        match self {
            Self::RecurringCron { expr, .. } => JobSchedule::RecurringCron { expr: expr.clone() },
            Self::RecurringInterval { every_seconds, .. } => JobSchedule::RecurringInterval {
                every_seconds: *every_seconds,
            },
            Self::OneShot { fire_at, .. } => JobSchedule::OneShot { fire_at: *fire_at },
        }
    }

    #[allow(dead_code)]
    pub fn fire_at(&self) -> Option<&DateTime<Utc>> {
        match self {
            Self::OneShot { fire_at, .. } => Some(fire_at),
            Self::RecurringCron { .. } | Self::RecurringInterval { .. } => None,
        }
    }
}
