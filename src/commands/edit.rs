use std::io::{IsTerminal, Read};

use anyhow::{Result, bail};
use chrono::Utc;
use serde::Serialize;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::model::action::Action;
use crate::model::job::{Job, JobStatus};
use crate::model::schedule::{JobSchedule, ParsedSchedule};
use crate::schedule::parser::parse_schedule;
use crate::store::state;

/// Input limits (same as add.rs).
const MAX_COMMAND_LEN: usize = 32 * 1024;
const MAX_PROMPT_LEN: usize = 128 * 1024;

/// A single before/after change record.
#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub field: String,
    pub old: String,
    pub new: String,
}

/// JSON output for edit command.
#[derive(Debug, Serialize)]
struct EditResult {
    id: String,
    name: Option<String>,
    changes: Vec<Change>,
}

/// Validated inputs ready to apply under the state lock.
struct ValidatedEdits {
    prompt_text: Option<String>,
    parsed_schedule: Option<ParsedSchedule>,
}

#[allow(clippy::too_many_arguments)]
pub fn execute(
    id: &str,
    name: Option<&str>,
    prompt: Option<&str>,
    prompt_stdin: bool,
    run_cmd: Option<&str>,
    agent: Option<&str>,
    timeout: Option<u64>,
    schedule: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let job = find_job(id)?;
    let validated = validate_inputs(&job, name, prompt, prompt_stdin, run_cmd, agent, schedule)?;

    // Warn if in_flight and schedule is changing
    if validated.parsed_schedule.is_some() && job.in_flight.is_some() {
        eprintln!(
            "Warning: Job {} has a run in flight. \
             The new schedule will take effect after it completes.",
            job.display_name()
        );
    }

    // Check name uniqueness (lockless pre-check)
    if let Some(new_name) = name {
        let current_state = state::load_state()?;
        let conflict = current_state
            .jobs
            .values()
            .any(|j| j.id != job.id && j.name.as_deref() == Some(new_name));
        if conflict {
            bail!(
                "Error: Another job already has the name '{new_name}'. \
                 Use a different name or use the job ID."
            );
        }
    }

    let job_id = job.id.clone();
    let changes = apply_edits(
        &job_id,
        name,
        validated.prompt_text.as_deref(),
        run_cmd,
        agent,
        timeout,
        schedule,
        validated.parsed_schedule.as_ref(),
    )?;

    print_result(
        &job,
        &job_id,
        name,
        &changes,
        validated.parsed_schedule.as_ref(),
        json_output,
    );

    Ok(())
}

/// Validate all inputs before acquiring any lock.
#[allow(clippy::too_many_arguments)]
fn validate_inputs(
    job: &Job,
    name: Option<&str>,
    prompt: Option<&str>,
    prompt_stdin: bool,
    run_cmd: Option<&str>,
    agent: Option<&str>,
    schedule: Option<&str>,
) -> Result<ValidatedEdits> {
    if job.status == JobStatus::Completed || job.status == JobStatus::Archived {
        bail!(
            "Error: Job {} ({}) is {} and cannot be edited.",
            job.display_name(),
            job.id,
            job.status
        );
    }

    let has_changes = name.is_some()
        || prompt.is_some()
        || prompt_stdin
        || run_cmd.is_some()
        || agent.is_some()
        || schedule.is_some();

    if !has_changes {
        bail!(
            "Error: No changes specified.\n\
             Usage: schedx edit <id> [--name NAME] [--prompt TEXT] [--run CMD] \
             [--agent NAME] [--timeout SECS] [--schedule EXPR]"
        );
    }

    // Validate action-type constraints
    if (prompt.is_some() || prompt_stdin) && !matches!(job.action, Action::Prompt { .. }) {
        bail!("Error: --prompt can only be used with prompt jobs.");
    }
    if run_cmd.is_some() && !matches!(job.action, Action::Run { .. }) {
        bail!("Error: --run can only be used with run jobs.");
    }
    if agent.is_some() && !matches!(job.action, Action::Prompt { .. }) {
        bail!("Error: --agent can only be used with prompt jobs.");
    }

    // Read stdin if needed (before lock)
    let prompt_text = if prompt_stdin {
        if std::io::stdin().is_terminal() {
            eprintln!("Reading prompt from stdin... press Ctrl-D when done.");
        }
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("Error: Failed to read stdin: {e}"))?;
        Some(buf)
    } else {
        prompt.map(str::to_string)
    };

    // Validate lengths
    if let Some(ref text) = prompt_text {
        if text.is_empty() {
            bail!("Error: Prompt text cannot be empty.");
        }
        if text.len() > MAX_PROMPT_LEN {
            bail!("Error: Prompt exceeds maximum length of {MAX_PROMPT_LEN} bytes.");
        }
    }
    if let Some(cmd) = run_cmd {
        if cmd.is_empty() {
            bail!("Error: Command cannot be empty.");
        }
        if cmd.len() > MAX_COMMAND_LEN {
            bail!("Error: Command exceeds maximum length of {MAX_COMMAND_LEN} bytes.");
        }
    }

    // Parse schedule if provided (before lock -- can be slow)
    let now = Utc::now();
    let parsed_schedule = schedule.map(|s| parse_schedule(s, now)).transpose()?;

    Ok(ValidatedEdits {
        prompt_text,
        parsed_schedule,
    })
}

/// Acquire lock, re-read state, and apply field-level mutations.
#[allow(clippy::too_many_arguments)]
fn apply_edits(
    job_id: &str,
    name: Option<&str>,
    prompt_text: Option<&str>,
    run_cmd: Option<&str>,
    agent: Option<&str>,
    timeout: Option<u64>,
    schedule: Option<&str>,
    parsed_schedule: Option<&ParsedSchedule>,
) -> Result<Vec<Change>> {
    let mut changes: Vec<Change> = Vec::new();

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        let j = s.jobs.get_mut(job_id).ok_or_else(|| {
            anyhow::anyhow!("Error: Job '{job_id}' not found (deleted between read and edit).")
        })?;

        if let Some(new_name) = name {
            let old = j.name.clone().unwrap_or_default();
            if old != new_name {
                changes.push(Change {
                    field: "name".to_string(),
                    old,
                    new: new_name.to_string(),
                });
                j.name = Some(new_name.to_string());
            }
        }

        if let Some(new_text) = prompt_text {
            if let Action::Prompt { ref mut text, .. } = j.action {
                if text.as_str() != new_text {
                    changes.push(Change {
                        field: "prompt".to_string(),
                        old: text.clone(),
                        new: new_text.to_string(),
                    });
                    new_text.clone_into(text);
                }
            }
        }

        if let Some(new_agent) = agent {
            if let Action::Prompt {
                agent: ref mut current_agent,
                ..
            } = j.action
            {
                let old = current_agent.clone().unwrap_or_default();
                if old != new_agent {
                    changes.push(Change {
                        field: "agent".to_string(),
                        old,
                        new: new_agent.to_string(),
                    });
                    *current_agent = Some(new_agent.to_string());
                }
            }
        }

        if let Some(new_cmd) = run_cmd {
            if let Action::Run {
                ref mut command, ..
            } = j.action
            {
                if command.as_str() != new_cmd {
                    changes.push(Change {
                        field: "command".to_string(),
                        old: command.clone(),
                        new: new_cmd.to_string(),
                    });
                    new_cmd.clone_into(command);
                }
            }
        }

        if let Some(new_timeout) = timeout {
            if j.timeout_seconds != new_timeout {
                changes.push(Change {
                    field: "timeout".to_string(),
                    old: format!("{}s", j.timeout_seconds),
                    new: format!("{new_timeout}s"),
                });
                j.timeout_seconds = new_timeout;
            }
        }

        if let Some(parsed) = parsed_schedule {
            let sched_input = schedule.unwrap();
            if j.schedule_input != sched_input {
                changes.push(Change {
                    field: "schedule".to_string(),
                    old: j.schedule_input.clone(),
                    new: sched_input.to_string(),
                });
                sched_input.clone_into(&mut j.schedule_input);
                j.schedule = parsed.to_job_schedule();
                // Reset anchor to avoid catchup bursts
                j.last_scheduled_at = Some(Utc::now());
            }
        }

        if !changes.is_empty() {
            j.updated_at = Utc::now();
        }

        Ok(())
    })?;

    Ok(changes)
}

/// Print the edit result in human or JSON format.
fn print_result(
    job: &Job,
    job_id: &str,
    name: Option<&str>,
    changes: &[Change],
    parsed_schedule: Option<&ParsedSchedule>,
    json_output: bool,
) {
    if changes.is_empty() {
        println!("No changes.");
        return;
    }

    // Sub-minute interval tip
    if let Some(parsed) = parsed_schedule {
        let new_schedule = parsed.to_job_schedule();
        if matches!(
            new_schedule,
            JobSchedule::RecurringInterval { every_seconds } if every_seconds < 60
        ) {
            eprintln!(
                "Tip: Sub-minute intervals need 'schedx daemon --interval 10' for precise timing.\n\
                 The system scheduler (launchd/systemd) dispatches at most once per minute."
            );
        }
    }

    let display_name = name
        .map(str::to_string)
        .or_else(|| job.name.clone())
        .unwrap_or_else(|| job_id.to_string());

    if json_output {
        let result = EditResult {
            id: job_id.to_string(),
            name: Some(display_name),
            changes: changes.to_vec(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&result) {
            println!("{json}");
        }
    } else {
        println!("Edited job: {display_name} ({job_id})\n");
        for change in changes {
            format_change(change);
        }
    }
}

fn format_change(change: &Change) {
    let label = match change.field.as_str() {
        "prompt" => "Prompt",
        "command" => "Command",
        "agent" => "Agent",
        "name" => "Name",
        "timeout" => "Timeout",
        "schedule" => "Schedule",
        other => other,
    };

    println!("  {label}:");

    if change.old.contains('\n') || change.new.contains('\n') {
        for line in change.old.lines() {
            println!("    - {line}");
        }
        for line in change.new.lines() {
            println!("    + {line}");
        }
    } else {
        println!("    - {}", change.old);
        println!("    + {}", change.new);
    }
    println!();
}
