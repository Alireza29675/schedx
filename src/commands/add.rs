use std::io::Read;

use anyhow::{Result, bail};
use chrono::Utc;

use crate::backend;
use crate::commands::action_input::{
    build_prompt_action, build_run_action, build_webhook_action, parse_header_lines, parse_method,
    validate_on_failure, validate_tags,
};
use crate::engine::lock::FileLock;
use crate::model::action::Action;
use crate::model::job::{Job, JobStatus};
use crate::schedule::parser::parse_schedule;
use crate::store::config::load_config;
use crate::store::paths;
use crate::store::state;
use crate::util::id::new_job_id;

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::fn_params_excessive_bools
)]
pub fn execute(
    schedule_input: &str,
    run_cmd: Option<&str>,
    prompt_text: Option<&str>,
    webhook_url: Option<&str>,
    name: Option<&str>,
    tags: &[String],
    timeout: Option<u64>,
    workdir: Option<&str>,
    agent: Option<&str>,
    from_stdin: bool,
    shell: bool,
    method: Option<&str>,
    headers: &[String],
    body: Option<&str>,
    on_failure: Option<&str>,
    on_failure_shell: bool,
    json_output: bool,
) -> Result<()> {
    paths::ensure_dirs()?;

    // Validate exactly one action
    let action_count = u8::from(run_cmd.is_some())
        + u8::from(prompt_text.is_some())
        + u8::from(webhook_url.is_some());
    if action_count != 1 {
        bail!(
            "Error: Exactly one action required: --run, --prompt, or --webhook.\n\
             Example: schedx add 'every 1h' --run 'echo hello'"
        );
    }

    // Validate flag combinations
    if shell && run_cmd.is_none() {
        bail!("Error: --shell can only be used with --run.");
    }
    if from_stdin && run_cmd.is_none() && prompt_text.is_none() {
        bail!("Error: --stdin can only be used with --run or --prompt.");
    }
    if agent.is_some() && prompt_text.is_none() {
        bail!("Error: --agent can only be used with --prompt.");
    }
    if workdir.is_some() && run_cmd.is_none() {
        bail!("Error: --workdir can only be used with --run.");
    }
    if on_failure_shell && on_failure.is_none() {
        bail!("Error: --on-failure-shell can only be used with --on-failure.");
    }
    validate_on_failure(on_failure)?;
    validate_tags(tags)?;

    // Build action (with possible stdin reading)
    let action = build_action(
        run_cmd,
        prompt_text,
        webhook_url,
        from_stdin,
        shell,
        workdir,
        agent,
        method,
        headers,
        body,
    )?;

    // Parse schedule
    let now = Utc::now();
    let parsed = parse_schedule(schedule_input, now)?;

    // Load config for default timeout
    let config = load_config()?;
    let timeout_seconds = timeout.unwrap_or(config.default_timeout_seconds);

    let job_id = new_job_id();
    let job = Job {
        id: job_id.clone(),
        name: name.map(str::to_string),
        status: JobStatus::Active,
        schedule_input: schedule_input.to_string(),
        schedule: parsed.to_job_schedule(),
        action,
        timeout_seconds,
        tags: tags.to_vec(),
        created_at: now,
        updated_at: now,
        last_scheduled_at: None,
        last_run: None,
        run_count: 0,
        skip_remaining: 0,
        in_flight: None,
        on_failure: on_failure.map(str::to_string),
        on_failure_shell,
        completed_at: None,
        consecutive_failures: 0,
    };

    // Save under lock
    let _lock = FileLock::state()?;
    state::update_state(|s| {
        s.jobs.insert(job_id.clone(), job.clone());
        Ok(())
    })?;

    // Ensure backend dispatcher is running
    let backend_active = if let Ok(be) = backend::detect_backend() {
        let ok = be.ensure_dispatcher().is_ok();
        ok && be.name() != "none"
    } else {
        false
    };

    // Output
    if json_output {
        let output = crate::output::format::JobDetail::from_job(&job);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let display_name = job.display_name().to_string();
        let next_run = crate::output::format::compute_next_run(&job);
        let next_str = next_run.map_or_else(
            || "unknown".to_string(),
            |t| crate::output::time::format_datetime_with_relative(t, now),
        );
        println!("Created job {display_name} ({job_id}). Next run: {next_str}");

        if !backend_active {
            eprintln!(
                "Tip: No scheduling backend active. Run 'schedx daemon &' in the background,\n\
                 or 'schedx repair' to set up the system scheduler."
            );
        } else if is_sub_minute_interval(&job) {
            eprintln!(
                "Tip: Sub-minute intervals need 'schedx daemon --interval 10' for precise timing.\n\
                 The system scheduler (launchd/systemd) dispatches at most once per minute."
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_action(
    run_cmd: Option<&str>,
    prompt_text: Option<&str>,
    webhook_url: Option<&str>,
    from_stdin: bool,
    shell: bool,
    workdir: Option<&str>,
    agent: Option<&str>,
    method: Option<&str>,
    headers: &[String],
    body: Option<&str>,
) -> Result<Action> {
    if let Some(cmd) = run_cmd {
        let command = if from_stdin {
            read_stdin()?
        } else {
            cmd.to_string()
        };
        return build_run_action(command, shell, workdir.map(str::to_string));
    }

    if let Some(text) = prompt_text {
        let prompt = if from_stdin {
            read_stdin()?
        } else {
            text.to_string()
        };
        return build_prompt_action(prompt, agent.map(str::to_string));
    }

    if let Some(url) = webhook_url {
        let http_method = parse_method(method)?;
        let parsed_headers = parse_header_lines(headers)?;
        return build_webhook_action(url, http_method, parsed_headers, body.map(str::to_string));
    }

    unreachable!("validation ensures exactly one action is set");
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| anyhow::anyhow!("Error: Failed to read stdin: {e}"))?;
    Ok(buf)
}

fn is_sub_minute_interval(job: &Job) -> bool {
    matches!(
        job.schedule,
        crate::model::schedule::JobSchedule::RecurringInterval { every_seconds } if every_seconds < 60
    )
}
