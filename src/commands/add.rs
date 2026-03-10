use std::io::Read;

use anyhow::{Result, bail};
use chrono::Utc;

use crate::backend;
use crate::engine::lock::FileLock;
use crate::model::action::{Action, HttpMethod};
use crate::model::job::{Job, JobStatus};
use crate::schedule::parser::parse_schedule;
use crate::store::config::load_config;
use crate::store::paths;
use crate::store::state;
use crate::util::id::new_job_id;

/// Input limits from spec.
const MAX_COMMAND_LEN: usize = 32 * 1024;
const MAX_PROMPT_LEN: usize = 128 * 1024;
const MAX_TAGS: usize = 20;
const MAX_TAG_LEN: usize = 64;

#[allow(clippy::too_many_arguments)]
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

    // Validate tags
    if tags.len() > MAX_TAGS {
        bail!("Error: Maximum {MAX_TAGS} tags per job.");
    }
    for tag in tags {
        if tag.len() > MAX_TAG_LEN {
            bail!("Error: Tag '{tag}' exceeds maximum length of {MAX_TAG_LEN} characters.");
        }
    }

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
        if command.len() > MAX_COMMAND_LEN {
            bail!("Error: Command exceeds maximum length of {MAX_COMMAND_LEN} bytes.");
        }
        return Ok(Action::Run {
            command,
            shell,
            workdir: workdir.map(str::to_string),
        });
    }

    if let Some(text) = prompt_text {
        let prompt = if from_stdin {
            read_stdin()?
        } else {
            text.to_string()
        };
        if prompt.len() > MAX_PROMPT_LEN {
            bail!("Error: Prompt exceeds maximum length of {MAX_PROMPT_LEN} bytes.");
        }
        return Ok(Action::Prompt {
            text: prompt,
            agent: agent.map(str::to_string),
        });
    }

    if let Some(url) = webhook_url {
        // Validate URL
        let parsed = url::Url::parse(url)
            .map_err(|e| anyhow::anyhow!("Error: Invalid webhook URL '{url}': {e}"))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            bail!("Error: Only http:// and https:// webhook URLs are supported.");
        }

        // Security check: HTTP requires config flag
        if parsed.scheme() == "http" {
            let config = load_config()?;
            if !config.allow_insecure_http {
                bail!(
                    "Error: HTTP webhooks are blocked by default for security.\n\
                     To allow: schedx config allow_insecure_http true"
                );
            }
        }

        let http_method = method
            .map(str::parse::<HttpMethod>)
            .transpose()
            .map_err(|e| anyhow::anyhow!("Error: {e}"))?
            .unwrap_or_default();

        let parsed_headers = parse_headers(headers)?;

        return Ok(Action::Webhook {
            url: url.to_string(),
            method: http_method,
            headers: parsed_headers,
            body: body.map(str::to_string),
        });
    }

    unreachable!("validation ensures exactly one action is set");
}

fn parse_headers(headers: &[String]) -> Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    for h in headers {
        let (key, value) = h.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("Error: Invalid header format '{h}'. Expected 'Key: Value'.")
        })?;
        result.push((key.trim().to_string(), value.trim().to_string()));
    }
    Ok(result)
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
