use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Agent profile for prompt action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub bin: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub prompt_stdin: bool,
}

/// Application configuration persisted to `config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub agents: BTreeMap<String, AgentProfile>,
    #[serde(default)]
    pub default_agent: Option<String>,
    #[serde(default = "default_backup_count")]
    pub backup_count: u32,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
    #[serde(default = "default_timeout_seconds")]
    pub default_timeout_seconds: u64,
    #[serde(default)]
    pub allow_insecure_http: bool,
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Global on-failure command (applies to jobs without their own `on_failure`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    /// Use shell execution for the global failure command.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub on_failure_shell: bool,
    /// Maximum concurrent fallback processes.
    #[serde(default = "default_max_concurrent_fallbacks")]
    pub max_concurrent_fallbacks: u32,
    /// Hours after completion before a one-shot job is auto-archived (0 = disabled).
    #[serde(default = "default_archive_after_hours")]
    pub archive_after_hours: u64,
}

fn default_backup_count() -> u32 {
    10
}

fn default_log_retention_days() -> u32 {
    30
}

fn default_timeout_seconds() -> u64 {
    300
}

fn default_backend() -> String {
    "auto".to_string()
}

fn default_max_concurrent_fallbacks() -> u32 {
    10
}

fn default_archive_after_hours() -> u64 {
    48
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            agents: BTreeMap::new(),
            default_agent: None,
            backup_count: default_backup_count(),
            log_retention_days: default_log_retention_days(),
            default_timeout_seconds: default_timeout_seconds(),
            allow_insecure_http: false,
            backend: default_backend(),
            on_failure: None,
            on_failure_shell: false,
            max_concurrent_fallbacks: default_max_concurrent_fallbacks(),
            archive_after_hours: default_archive_after_hours(),
        }
    }
}
