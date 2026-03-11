mod helpers;

use std::fs;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use fs4::fs_std::FileExt;
use helpers::TestEnv;
use serde_json::Value;

fn add_job_json(env: &TestEnv, args: &[&str]) -> Value {
    let output = env
        .cmd()
        .args(args)
        .output()
        .expect("failed to run add command");
    assert!(output.status.success(), "add command should succeed");
    serde_json::from_slice(&output.stdout).expect("valid add JSON")
}

fn read_jobs_json(env: &TestEnv) -> Value {
    serde_json::from_str(&fs::read_to_string(jobs_file(env)).expect("read jobs.json"))
        .expect("valid jobs.json")
}

fn write_jobs_json(env: &TestEnv, value: &Value) {
    fs::write(
        jobs_file(env),
        serde_json::to_string_pretty(value).expect("serialize jobs.json"),
    )
    .expect("write jobs.json");
}

fn jobs_file(env: &TestEnv) -> std::path::PathBuf {
    env.home().join("jobs.json")
}

#[allow(deprecated)]
fn bin_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin("schedx")
}

fn history_json(env: &TestEnv, job: &str) -> Vec<Value> {
    let output = env
        .cmd()
        .args(["history", job, "--limit", "200", "--json"])
        .output()
        .expect("failed to run history command");
    assert!(output.status.success(), "history command should succeed");
    serde_json::from_slice(&output.stdout).expect("valid history JSON")
}

fn get_json(env: &TestEnv, job: &str) -> Value {
    let output = env
        .cmd()
        .args(["get", job, "--json"])
        .output()
        .expect("failed to run get command");
    assert!(output.status.success(), "get command should succeed");
    serde_json::from_slice(&output.stdout).expect("valid get JSON")
}

fn wait_for<F>(timeout: Duration, mut predicate: F)
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("condition not met within {timeout:?}");
}

#[test]
fn dispatch_runs_due_job_once_and_records_logs() {
    let env = TestEnv::new();
    let bin_cmd = format!("{} --version", bin_path().display());

    add_job_json(
        &env,
        &[
            "add",
            "every 1s",
            "--run",
            &bin_cmd,
            "--name",
            "scheduled-version",
            "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));

    env.cmd().args(["_dispatch"]).assert().success();

    wait_for(Duration::from_secs(3), || {
        history_json(&env, "scheduled-version").len() == 1
    });

    let history = history_json(&env, "scheduled-version");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["status"], "success");

    let details = get_json(&env, "scheduled-version");
    assert_eq!(details["run_count"], 1);
    assert_eq!(details["last_run_status"], "success");

    let logs = env
        .cmd()
        .args(["logs", "scheduled-version"])
        .output()
        .expect("failed to read logs");
    assert!(logs.status.success(), "logs command should succeed");
    let logs_text = String::from_utf8(logs.stdout).expect("utf8 logs");
    assert!(
        logs_text.contains("schedx"),
        "expected version output in logs"
    );
}

#[test]
fn dispatch_records_overlap_once_per_missed_due_occurrence() {
    let env = TestEnv::new();
    let bin_cmd = format!("{} --version", bin_path().display());
    let add_json = add_job_json(
        &env,
        &[
            "add",
            "every 10s",
            "--run",
            &bin_cmd,
            "--name",
            "overlap-claim",
            "--json",
        ],
    );
    let job_id = add_json["id"].as_str().expect("job id").to_string();

    let mut jobs = read_jobs_json(&env);
    let now = chrono::Utc::now();
    let created_at = (now - chrono::Duration::seconds(48)).to_rfc3339();
    let claimed_for = (now - chrono::Duration::seconds(38)).to_rfc3339();
    jobs["jobs"][&job_id]["created_at"] = Value::String(created_at.clone());
    jobs["jobs"][&job_id]["updated_at"] = Value::String(created_at);
    jobs["jobs"][&job_id]["last_scheduled_at"] = Value::Null;
    jobs["jobs"][&job_id]["in_flight"] = serde_json::json!({
        "run_id": "claim-overlap",
        "scheduled_for": claimed_for,
        "claimed_at": claimed_for,
    });
    write_jobs_json(&env, &jobs);

    let lock_path = env.home().join("locks").join(format!("job-{job_id}.lock"));
    fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("create lock parent");
    let lock_file = fs::File::create(&lock_path).expect("create lock file");
    lock_file.lock_exclusive().expect("acquire job lock");

    env.cmd().args(["_dispatch"]).assert().success();
    env.cmd().args(["_dispatch"]).assert().success();

    let history = history_json(&env, "overlap-claim");
    let skipped: Vec<_> = history
        .iter()
        .filter(|record| record["status"] == "skipped_overlap")
        .collect();
    assert_eq!(skipped.len(), 3);

    let details = get_json(&env, "overlap-claim");
    assert_eq!(details["run_count"], 0);
}

#[test]
fn repair_recovers_stale_claim_without_incrementing_run_count() {
    let env = TestEnv::new();
    let bin_cmd = format!("{} --version", bin_path().display());
    let add_json = add_job_json(
        &env,
        &[
            "add",
            "every 10s",
            "--run",
            &bin_cmd,
            "--name",
            "stale-claim",
            "--json",
        ],
    );
    let job_id = add_json["id"].as_str().expect("job id").to_string();

    let mut jobs = read_jobs_json(&env);
    let now = chrono::Utc::now();
    let created_at = (now - chrono::Duration::seconds(48)).to_rfc3339();
    let claimed_for = (now - chrono::Duration::seconds(38)).to_rfc3339();
    let claimed_at = (now - chrono::Duration::seconds(38)).to_rfc3339();
    jobs["jobs"][&job_id]["created_at"] = Value::String(created_at.clone());
    jobs["jobs"][&job_id]["updated_at"] = Value::String(created_at);
    jobs["jobs"][&job_id]["last_scheduled_at"] = Value::Null;
    jobs["jobs"][&job_id]["in_flight"] = serde_json::json!({
        "run_id": "claim-stale",
        "scheduled_for": claimed_for,
        "claimed_at": claimed_at,
    });
    write_jobs_json(&env, &jobs);

    env.cmd().args(["repair", "--json"]).assert().success();

    let history = history_json(&env, "stale-claim");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["status"], "internal_error");

    let details = get_json(&env, "stale-claim");
    assert_eq!(details["run_count"], 0);
    assert_eq!(details["last_run_status"], "internal_error");

    let jobs = read_jobs_json(&env);
    assert!(jobs["jobs"][&job_id]["in_flight"].is_null());
}

#[test]
fn one_shot_stale_claim_retries_after_recovery() {
    let env = TestEnv::new();
    let bin_cmd = format!("{} --version", bin_path().display());
    let add_json = add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            &bin_cmd,
            "--name",
            "retry-oneshot",
            "--json",
        ],
    );
    let job_id = add_json["id"].as_str().expect("job id").to_string();

    thread::sleep(Duration::from_millis(1200));

    let mut jobs = read_jobs_json(&env);
    let fire_at = jobs["jobs"][&job_id]["schedule"]["fire_at"]
        .as_str()
        .expect("fire_at")
        .to_string();
    let claimed_at = (chrono::Utc::now() - chrono::Duration::seconds(20)).to_rfc3339();
    jobs["jobs"][&job_id]["in_flight"] = serde_json::json!({
        "run_id": "claim-oneshot",
        "scheduled_for": fire_at,
        "claimed_at": claimed_at,
    });
    jobs["jobs"][&job_id]["last_scheduled_at"] = Value::Null;
    write_jobs_json(&env, &jobs);

    env.cmd().args(["repair"]).assert().success();
    env.cmd().args(["_dispatch"]).assert().success();

    wait_for(Duration::from_secs(3), || {
        history_json(&env, "retry-oneshot").len() >= 2
    });

    let history = history_json(&env, "retry-oneshot");
    assert_eq!(history[0]["status"], "success");
    assert_eq!(history[1]["status"], "internal_error");

    let details = get_json(&env, "retry-oneshot");
    assert_eq!(details["status"], "completed");
    assert_eq!(details["run_count"], 1);
}

#[test]
fn list_and_get_json_reflect_skipped_due_runs() {
    let env = TestEnv::new();
    let bin_cmd = format!("{} --version", bin_path().display());
    let add_json = add_job_json(
        &env,
        &[
            "add",
            "every 10s",
            "--run",
            &bin_cmd,
            "--name",
            "next-run-skip",
            "--json",
        ],
    );
    let job_id = add_json["id"].as_str().expect("job id").to_string();

    let mut jobs = read_jobs_json(&env);
    let now = chrono::Utc::now();
    let created_at = (now - chrono::Duration::seconds(25)).to_rfc3339();
    jobs["jobs"][&job_id]["created_at"] = Value::String(created_at.clone());
    jobs["jobs"][&job_id]["updated_at"] = Value::String(created_at);
    jobs["jobs"][&job_id]["last_scheduled_at"] = Value::Null;
    jobs["jobs"][&job_id]["skip_remaining"] = Value::from(1);
    write_jobs_json(&env, &jobs);

    let details = get_json(&env, "next-run-skip");
    let next_run =
        chrono::DateTime::parse_from_rfc3339(details["next_run"].as_str().expect("next_run"))
            .expect("next run timestamp")
            .to_utc();
    assert!(next_run > chrono::Utc::now());

    let list_output = env
        .cmd()
        .args(["list", "--json"])
        .output()
        .expect("failed to run list");
    assert!(list_output.status.success(), "list should succeed");
    let list_json: Vec<Value> = serde_json::from_slice(&list_output.stdout).expect("valid list");
    let listed = list_json
        .iter()
        .find(|entry| entry["id"] == job_id)
        .expect("job in list output");
    let listed_next =
        chrono::DateTime::parse_from_rfc3339(listed["next_run"].as_str().expect("listed next_run"))
            .expect("listed next run timestamp")
            .to_utc();
    assert!(listed_next > chrono::Utc::now());
}

#[cfg(unix)]
#[test]
fn daemon_stop_keeps_started_exec_alive() {
    let env = TestEnv::new();
    add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            "sleep 2; printf daemon-detach",
            "--shell",
            "--name",
            "daemon-detach",
            "--json",
        ],
    );

    let mut daemon = StdCommand::new(bin_path());
    daemon
        .env("SCHEDX_HOME", env.home())
        .env("SCHEDX_BACKEND", "none")
        .args(["daemon", "--interval", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = daemon.spawn().expect("spawn daemon");

    thread::sleep(Duration::from_millis(1500));

    unsafe {
        let pid = i32::try_from(child.id()).expect("pid should fit in i32");
        libc::kill(pid, libc::SIGINT);
    }

    let status = child.wait().expect("wait for daemon");
    assert!(status.success(), "daemon should exit successfully");

    wait_for(Duration::from_secs(5), || {
        history_json(&env, "daemon-detach")
            .iter()
            .any(|record| record["status"] == "success")
    });

    let logs = env
        .cmd()
        .args(["logs", "daemon-detach"])
        .output()
        .expect("read daemon-detach logs");
    assert!(logs.status.success(), "logs should succeed");
    let logs_text = String::from_utf8(logs.stdout).expect("utf8 logs");
    assert!(logs_text.contains("daemon-detach"));
}
