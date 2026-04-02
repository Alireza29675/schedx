use std::process::ExitCode;

use anyhow::Result;
use chrono::{Duration, Utc};

use crate::backend;
use crate::model::run_record::RunStatus;
use crate::store::config::load_config;
use crate::store::history;
use crate::store::state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Ok,
    Warn,
    Error,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK   ",
            Self::Warn => "WARN ",
            Self::Error => "ERROR",
        }
    }
}

impl serde::Serialize for Severity {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        })
    }
}

#[derive(Debug, serde::Serialize)]
struct Finding {
    check: String,
    severity: Severity,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

pub fn execute(json_output: bool) -> Result<ExitCode> {
    let mut findings: Vec<Finding> = Vec::new();

    // Check 1: Backend timer running
    check_backend_health(&mut findings);

    // Check 2: systemd KillMode=process (Linux only)
    #[cfg(target_os = "linux")]
    check_systemd_killmode(&mut findings);

    // Check 3: Jobs with chronic consecutive failures
    check_chronic_failures(&mut findings);

    // Check 4: Recent internal_error runs
    check_recent_internal_errors(&mut findings);

    // Check 5: Stale in-flight runs
    check_stale_in_flight(&mut findings);

    let error_count = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warn_count = findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();

    if json_output {
        let output = serde_json::json!({
            "findings": findings,
            "error_count": error_count,
            "warn_count": warn_count,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "schedx doctor — {} check{}",
            findings.len(),
            if findings.len() == 1 { "" } else { "s" }
        );
        println!();
        for f in &findings {
            let hint_line = f
                .hint
                .as_deref()
                .map(|h| format!("\n           Hint: {h}"))
                .unwrap_or_default();
            println!("  [{}] {}{}", f.severity.label(), f.message, hint_line);
        }
        println!();
        if error_count == 0 && warn_count == 0 {
            println!("All checks passed.");
        } else {
            let parts: Vec<String> = [
                (error_count > 0).then(|| {
                    format!(
                        "{error_count} error{}",
                        if error_count == 1 { "" } else { "s" }
                    )
                }),
                (warn_count > 0).then(|| {
                    format!(
                        "{warn_count} warning{}",
                        if warn_count == 1 { "" } else { "s" }
                    )
                }),
            ]
            .into_iter()
            .flatten()
            .collect();
            println!(
                "{}. Run `schedx repair` to fix automatically fixable issues.",
                parts.join(", ")
            );
        }
    }

    Ok(if error_count > 0 {
        ExitCode::from(2)
    } else if warn_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn check_backend_health(findings: &mut Vec<Finding>) {
    match backend::detect_backend() {
        Ok(be) => match be.check_health() {
            Ok(health) => {
                if health.healthy {
                    findings.push(Finding {
                        check: "backend".to_string(),
                        severity: Severity::Ok,
                        message: format!("Backend '{}' timer is active", be.name()),
                        hint: None,
                    });
                } else {
                    let detail = health.messages.join("; ");
                    findings.push(Finding {
                        check: "backend".to_string(),
                        severity: Severity::Error,
                        message: format!("Backend '{}' is unhealthy: {detail}", be.name()),
                        hint: Some("Run: schedx repair".to_string()),
                    });
                }
            }
            Err(e) => {
                findings.push(Finding {
                    check: "backend".to_string(),
                    severity: Severity::Error,
                    message: format!("Backend health check failed: {e:#}"),
                    hint: Some("Run: schedx repair".to_string()),
                });
            }
        },
        Err(e) => {
            findings.push(Finding {
                check: "backend".to_string(),
                severity: Severity::Error,
                message: format!("No scheduling backend detected: {e:#}"),
                hint: Some("Run: schedx repair".to_string()),
            });
        }
    }
}

#[cfg(target_os = "linux")]
fn check_systemd_killmode(findings: &mut Vec<Finding>) {
    let service_path =
        dirs::home_dir().map(|h| h.join(".config/systemd/user/schedx-dispatch.service"));

    let Some(path) = service_path else {
        findings.push(Finding {
            check: "systemd_killmode".to_string(),
            severity: Severity::Warn,
            message: "Could not determine home directory to check service file".to_string(),
            hint: None,
        });
        return;
    };

    if !path.exists() {
        findings.push(Finding {
            check: "systemd_killmode".to_string(),
            severity: Severity::Warn,
            message: "Service file not found — dispatcher may not be installed".to_string(),
            hint: Some("Run: schedx repair".to_string()),
        });
        return;
    }

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if content.contains("KillMode=process") {
        findings.push(Finding {
            check: "systemd_killmode".to_string(),
            severity: Severity::Ok,
            message: "Service file has KillMode=process".to_string(),
            hint: None,
        });
    } else {
        findings.push(Finding {
            check: "systemd_killmode".to_string(),
            severity: Severity::Error,
            message: "Service file missing KillMode=process — spawned _exec processes may be \
                      killed prematurely (this was the cause of the silent 9-day failure bug)"
                .to_string(),
            hint: Some("Run: schedx repair".to_string()),
        });
    }
}

fn check_chronic_failures(findings: &mut Vec<Finding>) {
    let threshold = load_config()
        .map(|c| c.consecutive_failure_threshold)
        .unwrap_or(5);

    if threshold == 0 {
        return;
    }

    let state = match state::load_state() {
        Ok(s) => s,
        Err(e) => {
            findings.push(Finding {
                check: "chronic_failures".to_string(),
                severity: Severity::Warn,
                message: format!("Could not load job state: {e:#}"),
                hint: None,
            });
            return;
        }
    };

    let mut chronic: Vec<String> = state
        .jobs
        .values()
        .filter(|j| j.consecutive_failures >= threshold)
        .map(|j| {
            format!(
                "'{}' ({} failures)",
                j.display_name(),
                j.consecutive_failures
            )
        })
        .collect();

    if chronic.is_empty() {
        findings.push(Finding {
            check: "chronic_failures".to_string(),
            severity: Severity::Ok,
            message: format!("No jobs with {threshold}+ consecutive failures"),
            hint: None,
        });
    } else {
        chronic.sort();
        for job_desc in chronic {
            findings.push(Finding {
                check: "chronic_failures".to_string(),
                severity: Severity::Warn,
                message: format!("Chronic failure: {job_desc}"),
                hint: Some("Check logs with: schedx logs <job-id>".to_string()),
            });
        }
    }
}

fn check_recent_internal_errors(findings: &mut Vec<Finding>) {
    let cutoff = Utc::now() - Duration::hours(24);

    let records = match history::load_records(None, Some(100)) {
        Ok(r) => r,
        Err(e) => {
            findings.push(Finding {
                check: "internal_errors".to_string(),
                severity: Severity::Warn,
                message: format!("Could not read run history: {e:#}"),
                hint: None,
            });
            return;
        }
    };

    let mut affected_jobs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for record in &records {
        if record.status == RunStatus::InternalError && record.finished_at >= cutoff {
            affected_jobs.insert(record.job_id.clone());
        }
    }

    if affected_jobs.is_empty() {
        findings.push(Finding {
            check: "internal_errors".to_string(),
            severity: Severity::Ok,
            message: "No internal_error runs in the last 24 hours".to_string(),
            hint: None,
        });
    } else {
        let jobs_list = affected_jobs.into_iter().collect::<Vec<_>>().join(", ");
        findings.push(Finding {
            check: "internal_errors".to_string(),
            severity: Severity::Warn,
            message: format!("internal_error runs in last 24h for jobs: {jobs_list}"),
            hint: Some("Check logs with: schedx logs <job-id>".to_string()),
        });
    }
}

fn check_stale_in_flight(findings: &mut Vec<Finding>) {
    let stale_threshold = Duration::minutes(30);
    let now = Utc::now();

    let Ok(state) = state::load_state() else {
        return; // already reported in chronic_failures check
    };

    let mut stale_jobs: Vec<String> = state
        .jobs
        .values()
        .filter_map(|j| {
            j.in_flight
                .as_ref()
                .filter(|claim| now - claim.claimed_at > stale_threshold)
                .map(|claim| {
                    let age_mins = (now - claim.claimed_at).num_minutes();
                    format!("'{}' (in-flight for {age_mins}m)", j.display_name())
                })
        })
        .collect();

    if stale_jobs.is_empty() {
        findings.push(Finding {
            check: "stale_in_flight".to_string(),
            severity: Severity::Ok,
            message: "No stale in-flight runs".to_string(),
            hint: None,
        });
    } else {
        stale_jobs.sort();
        for desc in stale_jobs {
            findings.push(Finding {
                check: "stale_in_flight".to_string(),
                severity: Severity::Warn,
                message: format!("Stale in-flight run: {desc}"),
                hint: Some("Run: schedx repair".to_string()),
            });
        }
    }
}
