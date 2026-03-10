use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::engine::lock::FileLock;
use crate::engine::logger;
use crate::model::action::Action;
use crate::model::job::{Job, JobStatus};
use crate::model::run_record::{LastRun, RunRecord, RunStatus, Trigger};
use crate::store::config::load_config;
use crate::store::history;
use crate::store::state;
use crate::util::id::new_run_id;

const MAX_WEBHOOK_RESPONSE_LOG_BYTES: u64 = 256 * 1024;

/// Execute a single job run.
///
/// Returns `true` if the sched process itself should exit 0 (job failure is not sched failure).
pub fn exec_job(job_id: &str, scheduled_for: DateTime<Utc>, trigger: Trigger) -> Result<bool> {
    let job = state::load_job(job_id)?;
    let Some(job) = job else {
        anyhow::bail!("Job not found: {job_id}");
    };

    if job.status != JobStatus::Active {
        return Ok(true);
    }

    // Try to acquire per-job lock (overlap prevention)
    let Some(_job_lock) = FileLock::job_non_blocking(job_id)? else {
        // Another instance is running this job — skip
        let record = RunRecord {
            run_id: new_run_id(),
            job_id: job_id.to_string(),
            trigger,
            scheduled_for,
            started_at: Utc::now(),
            finished_at: Utc::now(),
            status: RunStatus::SkippedOverlap,
            exit_code: None,
            log_path: String::new(),
        };
        history::append_record(&record)?;
        return Ok(true);
    };

    let run_id = new_run_id();
    let (log_file, log_path) = logger::create_log_file(job_id, &run_id)?;
    let started_at = Utc::now();

    let result = execute_action(&job, log_file);

    let finished_at = Utc::now();
    let (status, exit_code) = match &result {
        Ok((code, timed_out)) => {
            if *timed_out {
                (RunStatus::Timeout, *code)
            } else if code == &Some(0) {
                (RunStatus::Success, *code)
            } else {
                (RunStatus::Failed, *code)
            }
        }
        Err(_) => (RunStatus::InternalError, None),
    };

    // Update job state
    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(job_id) {
            j.last_scheduled_at = Some(scheduled_for);
            j.run_count += 1;
            j.updated_at = Utc::now();
            j.last_run = Some(LastRun {
                run_id: run_id.clone(),
                started_at,
                finished_at,
                status,
                exit_code,
                log_path: log_path.clone(),
            });

            // One-shot completion
            if j.is_one_shot() && !status.is_internal_error() {
                j.status = JobStatus::Completed;
            }
        }
        Ok(())
    })?;

    // Append history record
    let record = RunRecord {
        run_id,
        job_id: job_id.to_string(),
        trigger,
        scheduled_for,
        started_at,
        finished_at,
        status,
        exit_code,
        log_path,
    };
    history::append_record(&record)?;

    Ok(!status.is_internal_error())
}

/// Execute the job action, returning `(exit_code, timed_out)`.
fn execute_action(job: &Job, log_file: std::fs::File) -> Result<(Option<i32>, bool)> {
    match &job.action {
        Action::Run {
            command,
            shell,
            workdir,
        } => execute_run_action(
            command,
            *shell,
            workdir.as_deref(),
            job.timeout_seconds,
            log_file,
        ),
        Action::Prompt { text, agent } => {
            execute_prompt_action(text, agent.as_deref(), job.timeout_seconds, log_file)
        }
        Action::Webhook {
            url,
            method,
            headers,
            body,
        } => execute_webhook_action(
            url,
            *method,
            headers,
            body.as_deref(),
            job.timeout_seconds,
            log_file,
        ),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn execute_run_action(
    command: &str,
    shell: bool,
    workdir: Option<&str>,
    timeout_seconds: u64,
    log_file: std::fs::File,
) -> Result<(Option<i32>, bool)> {
    let mut cmd = if shell {
        let mut c = Command::new("/bin/sh");
        c.args(["-lc", command]);
        c
    } else {
        let argv = shell_words::split(command)
            .with_context(|| format!("failed to parse command: {command}"))?;
        if argv.is_empty() {
            anyhow::bail!("empty command");
        }
        let mut c = Command::new(&argv[0]);
        if argv.len() > 1 {
            c.args(&argv[1..]);
        }
        c
    };

    if let Some(dir) = workdir {
        cmd.current_dir(dir);
    }

    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file.try_clone()?;
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));
    isolate_child_process(&mut cmd);

    let mut child = cmd.spawn().context("failed to spawn command")?;

    // Timeout watchdog
    let timeout = Duration::from_secs(timeout_seconds);
    match wait_with_timeout(&mut child, timeout) {
        Ok(status) => Ok((status.code(), false)),
        Err(err) if is_timeout_error(&err) => {
            kill_process(&mut child);
            Ok((None, true))
        }
        Err(err) => Err(err),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn execute_prompt_action(
    prompt_text: &str,
    agent_name: Option<&str>,
    timeout_seconds: u64,
    log_file: std::fs::File,
) -> Result<(Option<i32>, bool)> {
    let config = load_config()?;
    let agent_key = agent_name
        .map(std::string::ToString::to_string)
        .or(config.default_agent.clone())
        .context(
            "No agent specified and no default agent configured.\n\
             Run: schedx agent add <name> --bin <path>",
        )?;

    let profile = config.agents.get(&agent_key).with_context(|| {
        format!(
            "Agent '{agent_key}' not found in config.\n\
             Run: schedx agent list"
        )
    })?;

    let mut cmd = Command::new(&profile.bin);
    cmd.args(&profile.args);

    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file.try_clone()?;
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));
    isolate_child_process(&mut cmd);

    if profile.prompt_stdin {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.arg(prompt_text);
        cmd.stdin(Stdio::null());
    }

    let mut child = cmd.spawn().context("failed to spawn agent")?;

    if profile.prompt_stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt_text.as_bytes()).ok();
            drop(stdin);
        }
    }

    let timeout = Duration::from_secs(timeout_seconds);
    match wait_with_timeout(&mut child, timeout) {
        Ok(status) => Ok((status.code(), false)),
        Err(err) if is_timeout_error(&err) => {
            kill_process(&mut child);
            Ok((None, true))
        }
        Err(err) => Err(err),
    }
}

#[allow(clippy::too_many_lines)]
fn execute_webhook_action(
    url: &str,
    method: crate::model::action::HttpMethod,
    headers: &[(String, String)],
    body: Option<&str>,
    timeout_seconds: u64,
    mut log_file: std::fs::File,
) -> Result<(Option<i32>, bool)> {
    use std::io::Write as _;

    use crate::model::action::HttpMethod;

    let config = load_config()?;

    // Security: validate URL scheme
    let parsed = url::Url::parse(url).context("invalid webhook URL")?;
    if parsed.scheme() == "http" && !config.allow_insecure_http {
        anyhow::bail!(
            "HTTP webhooks are blocked by default.\n\
             To allow: schedx config allow_insecure_http true"
        );
    }
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        anyhow::bail!("Only http:// and https:// webhook URLs are supported.");
    }

    // Build agent with timeout
    let agent_config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_seconds)))
        .build();
    let agent: ureq::Agent = agent_config.into();

    // Enforce max body size
    if let Some(body_str) = body {
        if body_str.len() > 1_048_576 {
            anyhow::bail!("Webhook body exceeds maximum size of 1 MiB.");
        }
    }

    // Dispatch by method and body presence
    let response = match (method, body) {
        (HttpMethod::Get, _) => {
            let mut req = agent.get(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.call()
        }
        (HttpMethod::Delete, None) => {
            let mut req = agent.delete(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.call()
        }
        (HttpMethod::Post, Some(body_str)) => {
            let mut req = agent.post(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.header("Content-Type", "application/json")
                .send(body_str)
        }
        (HttpMethod::Post, None) => {
            let mut req = agent.post(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.send("")
        }
        (HttpMethod::Put, Some(body_str)) => {
            let mut req = agent.put(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.header("Content-Type", "application/json")
                .send(body_str)
        }
        (HttpMethod::Put, None) => {
            let mut req = agent.put(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.send("")
        }
        (HttpMethod::Patch, Some(body_str)) => {
            let mut req = agent.patch(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.header("Content-Type", "application/json")
                .send(body_str)
        }
        (HttpMethod::Patch, None) => {
            let mut req = agent.patch(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.send("")
        }
        (HttpMethod::Delete, Some(_)) => {
            // DELETE typically has no body; ignore body
            let mut req = agent.delete(url);
            for (key, value) in headers {
                req = req.header(key, value);
            }
            req.call()
        }
    };

    match response {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let mut body_reader = resp
                .body_mut()
                .with_config()
                .limit(MAX_WEBHOOK_RESPONSE_LOG_BYTES)
                .reader();
            let mut body_bytes = Vec::new();
            let body_read_error = body_reader.read_to_end(&mut body_bytes).err();
            let body_text = String::from_utf8_lossy(&body_bytes);

            writeln!(log_file, "HTTP {status}").ok();
            writeln!(log_file, "{body_text}").ok();
            if let Some(err) = body_read_error {
                writeln!(log_file, "[response body truncated or unreadable: {err}]").ok();
            }

            if (200..300).contains(&status) {
                Ok((Some(0), false))
            } else {
                Ok((Some(i32::from(status)), false))
            }
        }
        Err(e) => {
            writeln!(log_file, "Webhook error: {e}").ok();
            let is_timeout =
                e.to_string().contains("timed out") || e.to_string().contains("timeout");
            if is_timeout {
                Ok((None, true))
            } else {
                Ok((Some(1), false))
            }
        }
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if start.elapsed() >= timeout {
            anyhow::bail!("timeout");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn is_timeout_error(err: &anyhow::Error) -> bool {
    err.to_string() == "timeout"
}

fn kill_process(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        if let Ok(pid) = i32::try_from(child.id()) {
            // Best-effort kill of the isolated process group.
            if pid > 0 {
                let _ = unsafe { libc::kill(-pid, libc::SIGKILL) };
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn isolate_child_process(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn isolate_child_process(_cmd: &mut Command) {}
