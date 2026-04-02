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
use crate::store::paths;
use crate::store::state;
use crate::util::id::new_run_id;

const MAX_WEBHOOK_RESPONSE_LOG_BYTES: u64 = 256 * 1024;

/// Execute a single job run.
///
/// Returns `true` if the schedx process itself should exit 0 (job failure is not schedx failure).
pub fn exec_job(job_id: &str, scheduled_for: DateTime<Utc>, trigger: Trigger) -> Result<bool> {
    exec_job_with_run_id(job_id, scheduled_for, trigger, None)
}

/// Execute a single job run.
///
/// Returns `true` if the schedx process itself should exit 0 (job failure is not schedx failure).
#[allow(clippy::too_many_lines)]
pub fn exec_job_with_run_id(
    job_id: &str,
    scheduled_for: DateTime<Utc>,
    trigger: Trigger,
    scheduled_run_id: Option<&str>,
) -> Result<bool> {
    let job = state::load_job(job_id)?;
    let Some(job) = job else {
        anyhow::bail!("Job not found: {job_id}");
    };

    if job.status != JobStatus::Active {
        return Ok(true);
    }

    if trigger == Trigger::Scheduled {
        let Some(run_id) = scheduled_run_id else {
            anyhow::bail!("Missing --run-id for scheduled execution");
        };
        let claim_matches = job
            .in_flight
            .as_ref()
            .is_some_and(|claim| claim.run_id == run_id && claim.scheduled_for == scheduled_for);
        if !claim_matches {
            return Ok(true);
        }
    }

    // Try to acquire per-job lock (overlap prevention)
    let Some(_job_lock) = FileLock::job_non_blocking(job_id)? else {
        if trigger == Trigger::Manual {
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
                failed_run_id: None,
                error_message: None,
            };
            history::append_record(&record)?;
        }
        return Ok(true);
    };

    let run_id = scheduled_run_id.map_or_else(new_run_id, str::to_string);
    let (log_file, log_path) = logger::create_log_file(job_id, &run_id)?;
    let started_at = Utc::now();

    let result = execute_action(&job, log_file);

    let finished_at = Utc::now();
    let (status, exit_code, error_message) = match &result {
        Ok((code, timed_out)) => {
            if *timed_out {
                (RunStatus::Timeout, *code, None)
            } else if code == &Some(0) {
                (RunStatus::Success, *code, None)
            } else {
                (RunStatus::Failed, *code, None)
            }
        }
        Err(e) => {
            // Write the error to the log file so `schedx logs` shows what went wrong.
            let err_str = format!("{e:#}");
            if let Ok(home) = paths::schedx_home() {
                let abs_log = home.join(&log_path);
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(abs_log)
                {
                    use std::io::Write as _;
                    writeln!(f, "[schedx internal error] {err_str}").ok();
                }
            }
            (RunStatus::InternalError, None, Some(err_str))
        }
    };

    // Update job state
    let _lock = FileLock::state()?;
    state::update_state(|s| {
        if let Some(j) = s.jobs.get_mut(job_id) {
            j.last_scheduled_at = Some(
                j.last_scheduled_at
                    .map_or(scheduled_for, |current| current.max(scheduled_for)),
            );
            if trigger == Trigger::Scheduled
                && j.in_flight
                    .as_ref()
                    .is_some_and(|claim| claim.run_id == run_id)
            {
                j.in_flight = None;
            }
            j.run_count += 1;
            j.updated_at = Utc::now();
            j.last_run = Some(LastRun {
                run_id: run_id.clone(),
                started_at,
                finished_at,
                status,
                exit_code,
                log_path: log_path.clone(),
                error_message: error_message.clone(),
            });

            // Track consecutive failures for alerting
            if status.should_trigger_fallback() {
                j.consecutive_failures = j.consecutive_failures.saturating_add(1);
            } else if status == RunStatus::Success {
                j.consecutive_failures = 0;
            }

            // One-shot completion
            if j.is_one_shot() && !status.is_internal_error() {
                j.status = JobStatus::Completed;
                j.completed_at = Some(Utc::now());
            }
        }
        Ok(())
    })?;

    // Append history record
    let record = RunRecord {
        run_id: run_id.clone(),
        job_id: job_id.to_string(),
        trigger,
        scheduled_for,
        started_at,
        finished_at,
        status,
        exit_code,
        log_path: log_path.clone(),
        failed_run_id: None,
        error_message: error_message.clone(),
    };
    history::append_record(&record)?;

    // Fire fallback if the job failed
    if status.should_trigger_fallback() {
        let _ = spawn_fallback(job_id, &run_id, status, exit_code, &log_path, scheduled_for);
    }

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

/// Append a structured failure line to `~/.schedx/failures.log`.
fn append_failures_log(
    job_id: &str,
    job_name: &str,
    run_id: &str,
    status: RunStatus,
    exit_code: Option<i32>,
    log_path: &str,
) {
    let Ok(home) = paths::schedx_home() else {
        return;
    };
    let path = home.join("failures.log");
    let now = Utc::now().to_rfc3339();
    let display_name = if job_name.is_empty() {
        job_id
    } else {
        job_name
    };
    let exit_str = exit_code.map_or_else(String::new, |c| format!(" exit_code={c}"));
    let line = format!(
        "[{now}] FAILED job=\"{display_name}\" id={job_id} run={run_id} status={status}{exit_str} log={log_path}\n"
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| {
            use std::io::Write as _;
            f.write_all(line.as_bytes())
        });
}

/// Spawn `schedx _exec-fallback` as a detached subprocess after a failed run.
fn spawn_fallback(
    job_id: &str,
    run_id: &str,
    status: RunStatus,
    exit_code: Option<i32>,
    log_path: &str,
    scheduled_for: DateTime<Utc>,
) -> Result<()> {
    // Always append to failures.log
    let job_name = state::load_job(job_id)?
        .and_then(|j| j.name.clone())
        .unwrap_or_default();
    append_failures_log(job_id, &job_name, run_id, status, exit_code, log_path);

    // Resolve the absolute log path
    let abs_log_path = paths::schedx_home().map_or_else(
        |_| log_path.to_string(),
        |h| h.join(log_path).to_string_lossy().to_string(),
    );

    // Spawn _exec-fallback as detached process
    let schedx_bin = std::env::current_exe().context("could not determine schedx binary path")?;
    let exit_code_str = exit_code.map_or_else(String::new, |c| c.to_string());
    let mut cmd = Command::new(schedx_bin);
    cmd.args([
        "_exec-fallback",
        job_id,
        "--failed-run-id",
        run_id,
        "--failed-status",
        status.as_str(),
        "--failed-exit-code",
        &exit_code_str,
        "--failed-log-path",
        &abs_log_path,
        "--failed-scheduled-for",
        &scheduled_for.to_rfc3339(),
    ]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    detach_fallback_process(&mut cmd);
    cmd.spawn()
        .with_context(|| format!("failed to spawn _exec-fallback for job {job_id}"))?;
    Ok(())
}

#[cfg(unix)]
fn detach_fallback_process(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn detach_fallback_process(_cmd: &mut Command) {}

/// Execute a fallback command for a failed job. Called from the `_exec-fallback` hidden command.
pub fn exec_fallback(
    job_id: &str,
    failed_run_id: &str,
    failed_status: &str,
    failed_exit_code: &str,
    failed_log_path: &str,
    failed_scheduled_for: &str,
) -> Result<bool> {
    let config = load_config()?;

    // Resolve fallback command: per-job > global config > None
    let job = state::load_job(job_id)?;
    let (fallback_cmd, fallback_shell) = if let Some(ref j) = job {
        if j.on_failure.is_some() {
            (j.on_failure.clone(), j.on_failure_shell)
        } else {
            (config.on_failure.clone(), config.on_failure_shell)
        }
    } else {
        (config.on_failure.clone(), config.on_failure_shell)
    };

    let Some(command) = fallback_cmd else {
        return Ok(true);
    };

    // Concurrency check: count active fallback lock files
    let locks_dir = paths::locks_dir()?;
    let active_fallbacks = std::fs::read_dir(&locks_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.starts_with("fallback-"))
                })
                .count()
        })
        .unwrap_or(0);

    if active_fallbacks >= usize::try_from(config.max_concurrent_fallbacks).unwrap_or(10) {
        eprintln!(
            "Fallback skipped for job {job_id}: max concurrent fallbacks ({}) reached.",
            config.max_concurrent_fallbacks
        );
        return Ok(true);
    }

    // Acquire a fallback lock
    let fallback_lock_path = locks_dir.join(format!("fallback-{failed_run_id}.lock"));
    let _fallback_lock = FileLock::acquire_non_blocking_path(&fallback_lock_path)?;

    let run_id = new_run_id();
    let (log_file, log_rel_path) = logger::create_log_file(job_id, &run_id)?;
    let started_at = Utc::now();

    // Build stripped environment
    let job_name = job
        .as_ref()
        .and_then(|j| j.name.clone())
        .unwrap_or_default();

    let env_vars = [
        ("PATH", std::env::var("PATH").unwrap_or_default()),
        ("HOME", std::env::var("HOME").unwrap_or_default()),
        ("SHELL", std::env::var("SHELL").unwrap_or_default()),
        ("TERM", std::env::var("TERM").unwrap_or_default()),
        ("SCHEDX_FAILED_JOB_ID", job_id.to_string()),
        ("SCHEDX_FAILED_JOB_NAME", job_name),
        ("SCHEDX_FAILED_RUN_ID", failed_run_id.to_string()),
        ("SCHEDX_FAILED_STATUS", failed_status.to_string()),
        ("SCHEDX_FAILED_EXIT_CODE", failed_exit_code.to_string()),
        ("SCHEDX_FAILED_LOG_PATH", failed_log_path.to_string()),
        (
            "SCHEDX_FAILED_SCHEDULED_FOR",
            failed_scheduled_for.to_string(),
        ),
    ];

    // Execute the fallback command
    let result = execute_fallback_command(&command, fallback_shell, &env_vars, log_file);

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

    // Parse the scheduled_for timestamp for the record
    let scheduled_for_dt = chrono::DateTime::parse_from_rfc3339(failed_scheduled_for)
        .map(|dt| dt.to_utc())
        .unwrap_or(started_at);

    // Record in history
    let record = RunRecord {
        run_id,
        job_id: job_id.to_string(),
        trigger: Trigger::Fallback,
        scheduled_for: scheduled_for_dt,
        started_at,
        finished_at,
        status,
        exit_code,
        log_path: log_rel_path,
        failed_run_id: Some(failed_run_id.to_string()),
        error_message: None,
    };
    history::append_record(&record)?;

    Ok(true)
}

const FALLBACK_TIMEOUT_SECONDS: u64 = 60;

#[allow(clippy::needless_pass_by_value)]
fn execute_fallback_command(
    command: &str,
    shell: bool,
    env_vars: &[(&str, String)],
    log_file: std::fs::File,
) -> Result<(Option<i32>, bool)> {
    let mut cmd = if shell {
        let mut c = Command::new("/bin/sh");
        c.args(["-lc", command]);
        c
    } else {
        let argv = shell_words::split(command)
            .with_context(|| format!("failed to parse fallback command: {command}"))?;
        if argv.is_empty() {
            anyhow::bail!("empty fallback command");
        }
        let mut c = Command::new(&argv[0]);
        if argv.len() > 1 {
            c.args(&argv[1..]);
        }
        c
    };

    // Stripped environment: clear all, then set only what we allow
    cmd.env_clear();
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let stdout_file = log_file.try_clone()?;
    let stderr_file = log_file.try_clone()?;
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));
    isolate_child_process(&mut cmd);

    let mut child = cmd.spawn().context("failed to spawn fallback command")?;

    let timeout = Duration::from_secs(FALLBACK_TIMEOUT_SECONDS);
    match wait_with_timeout(&mut child, timeout) {
        Ok(status) => Ok((status.code(), false)),
        Err(err) if is_timeout_error(&err) => {
            kill_process(&mut child);
            Ok((None, true))
        }
        Err(err) => Err(err),
    }
}
