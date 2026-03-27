mod helpers;

use std::fs;
use std::thread;
use std::time::Duration;

use helpers::TestEnv;
use predicates::prelude::*;
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

fn wait_for_history(env: &TestEnv, job: &str, count: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if history_json(env, job).len() >= count {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let actual = history_json(env, job);
    panic!(
        "expected at least {} history records, got {}",
        count,
        actual.len()
    );
}

#[test]
fn add_job_with_on_failure_stores_in_json() {
    let env = TestEnv::new();
    let detail = add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "echo hello",
            "--on-failure",
            "echo failed",
            "--name",
            "fallback-test",
            "--json",
        ],
    );
    assert_eq!(detail["on_failure"], "echo failed");
    // on_failure_shell is skipped in JSON when false
    assert!(detail.get("on_failure_shell").is_none() || detail["on_failure_shell"] == false);

    let get = get_json(&env, "fallback-test");
    assert_eq!(get["on_failure"], "echo failed");
}

#[test]
fn add_job_with_on_failure_shell() {
    let env = TestEnv::new();
    let detail = add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "echo hello",
            "--on-failure",
            "echo $SCHEDX_FAILED_JOB_ID",
            "--on-failure-shell",
            "--name",
            "shell-fallback",
            "--json",
        ],
    );
    assert_eq!(detail["on_failure_shell"], true);
}

#[test]
fn on_failure_shell_without_on_failure_fails() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo hello",
            "--on-failure-shell",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--on-failure-shell can only be used with --on-failure",
        ));
}

#[test]
fn job_without_on_failure_has_null_field() {
    let env = TestEnv::new();
    let detail = add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "echo hello",
            "--name",
            "no-fallback",
            "--json",
        ],
    );
    assert!(detail.get("on_failure").is_none() || detail["on_failure"].is_null());
}

#[test]
fn failed_job_triggers_fallback_and_records_history() {
    let env = TestEnv::new();

    // Create a marker file path for the fallback to write
    let marker = env.home().join("fallback-ran.txt");

    add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            "false", // exit code 1
            "--on-failure",
            &format!("echo fallback-executed > {}", marker.display()),
            "--on-failure-shell",
            "--name",
            "fail-job",
            "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));
    env.cmd().args(["_dispatch"]).assert().success();

    // Wait for the main run + fallback
    wait_for_history(&env, "fail-job", 2);

    let history = history_json(&env, "fail-job");
    let main_run = history
        .iter()
        .find(|r| r["trigger"] == "scheduled" || r["trigger"] == "manual")
        .expect("main run in history");
    assert_eq!(main_run["status"], "failed");

    let fallback_run = history
        .iter()
        .find(|r| r["trigger"] == "fallback")
        .expect("fallback run in history");
    assert_eq!(fallback_run["status"], "success");
    assert!(fallback_run["failed_run_id"].is_string());
    assert_eq!(
        fallback_run["failed_run_id"].as_str().unwrap(),
        main_run["run_id"].as_str().unwrap()
    );

    // Verify the marker file was created by the fallback command
    assert!(
        marker.exists(),
        "fallback should have created the marker file"
    );
}

#[test]
fn successful_job_does_not_trigger_fallback() {
    let env = TestEnv::new();

    let marker = env.home().join("should-not-exist.txt");

    add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            "true", // exit code 0
            "--on-failure",
            &format!("touch {}", marker.display()),
            "--on-failure-shell",
            "--name",
            "success-job",
            "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));
    env.cmd().args(["_dispatch"]).assert().success();

    wait_for_history(&env, "success-job", 1);
    // Give extra time for any fallback that might incorrectly fire
    thread::sleep(Duration::from_millis(500));

    let history = history_json(&env, "success-job");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["status"], "success");
    assert!(
        !marker.exists(),
        "fallback should NOT run for successful jobs"
    );
}

#[test]
fn failures_log_written_on_failure() {
    let env = TestEnv::new();

    add_job_json(
        &env,
        &[
            "add", "in 1s", "--run", "false", "--name", "log-fail", "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));
    env.cmd().args(["_dispatch"]).assert().success();

    wait_for_history(&env, "log-fail", 1);

    let failures_log = env.home().join("failures.log");
    // Give a moment for the file to be written
    thread::sleep(Duration::from_millis(300));

    assert!(failures_log.exists(), "failures.log should exist");
    let content = fs::read_to_string(&failures_log).expect("read failures.log");
    assert!(content.contains("FAILED"));
    assert!(content.contains("log-fail"));
    assert!(content.contains("status=failed"));
}

#[test]
fn global_config_fallback_fires_for_jobs_without_per_job() {
    let env = TestEnv::new();

    let marker = env.home().join("global-fallback-ran.txt");

    // Set global fallback
    env.cmd()
        .args([
            "config",
            "on_failure",
            &format!("echo global > {}", marker.display()),
        ])
        .assert()
        .success();
    env.cmd()
        .args(["config", "on_failure_shell", "true"])
        .assert()
        .success();

    // Add a job without --on-failure
    add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            "false",
            "--name",
            "global-test",
            "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));
    env.cmd().args(["_dispatch"]).assert().success();

    wait_for_history(&env, "global-test", 2);

    let history = history_json(&env, "global-test");
    let fallback_run = history.iter().find(|r| r["trigger"] == "fallback");
    assert!(
        fallback_run.is_some(),
        "global fallback should have triggered"
    );

    assert!(
        marker.exists(),
        "global fallback should have created the marker file"
    );
}

#[test]
fn per_job_fallback_overrides_global() {
    let env = TestEnv::new();

    let global_marker = env.home().join("global-marker.txt");
    let local_marker = env.home().join("local-marker.txt");

    // Set global fallback
    env.cmd()
        .args([
            "config",
            "on_failure",
            &format!("touch {}", global_marker.display()),
        ])
        .assert()
        .success();
    env.cmd()
        .args(["config", "on_failure_shell", "true"])
        .assert()
        .success();

    // Add job with its own fallback
    add_job_json(
        &env,
        &[
            "add",
            "in 1s",
            "--run",
            "false",
            "--on-failure",
            &format!("touch {}", local_marker.display()),
            "--on-failure-shell",
            "--name",
            "override-test",
            "--json",
        ],
    );

    thread::sleep(Duration::from_millis(1200));
    env.cmd().args(["_dispatch"]).assert().success();

    wait_for_history(&env, "override-test", 2);
    thread::sleep(Duration::from_millis(500));

    assert!(local_marker.exists(), "per-job fallback should have run");
    assert!(
        !global_marker.exists(),
        "global fallback should NOT run when per-job is set"
    );
}

#[test]
fn backward_compat_jobs_without_on_failure_load_fine() {
    let env = TestEnv::new();

    // Add a regular job (no on_failure)
    add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "echo test",
            "--name",
            "compat-test",
            "--json",
        ],
    );

    // Manually strip on_failure from jobs.json to simulate old format
    let jobs_path = env.home().join("jobs.json");
    let content = fs::read_to_string(&jobs_path).expect("read jobs.json");
    let mut jobs: Value = serde_json::from_str(&content).expect("parse jobs.json");

    // Remove on_failure fields if present
    for (_id, job) in jobs["jobs"].as_object_mut().expect("jobs object") {
        let obj = job.as_object_mut().expect("job object");
        obj.remove("on_failure");
        obj.remove("on_failure_shell");
    }
    fs::write(
        &jobs_path,
        serde_json::to_string_pretty(&jobs).expect("serialize"),
    )
    .expect("write");

    // Should still load fine
    let detail = get_json(&env, "compat-test");
    assert_eq!(detail["name"], "compat-test");
    assert!(detail.get("on_failure").is_none() || detail["on_failure"].is_null());
}

#[test]
fn get_shows_on_failure_in_human_output() {
    let env = TestEnv::new();
    add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "echo test",
            "--on-failure",
            "echo failed",
            "--name",
            "human-fallback",
            "--json",
        ],
    );

    env.cmd()
        .args(["get", "human-fallback"])
        .assert()
        .success()
        .stdout(predicate::str::contains("On failure: echo failed"));
}

#[test]
fn config_on_failure_set_and_read() {
    let env = TestEnv::new();

    env.cmd()
        .args(["config", "on_failure", "echo global failure"])
        .assert()
        .success();

    env.cmd()
        .args(["config", "on_failure"])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo global failure"));

    // Clear it
    env.cmd()
        .args(["config", "on_failure", ""])
        .assert()
        .success();

    env.cmd()
        .args(["config", "on_failure"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(none)"));
}

#[test]
fn manual_run_fires_fallback_on_failure() {
    let env = TestEnv::new();

    let marker = env.home().join("manual-fallback.txt");

    add_job_json(
        &env,
        &[
            "add",
            "every 1h",
            "--run",
            "false",
            "--on-failure",
            &format!("touch {}", marker.display()),
            "--on-failure-shell",
            "--name",
            "manual-fail",
            "--json",
        ],
    );

    env.cmd().args(["run", "manual-fail"]).assert().success();

    // Wait for fallback
    thread::sleep(Duration::from_secs(2));

    let history = history_json(&env, "manual-fail");
    let fallback = history.iter().find(|r| r["trigger"] == "fallback");
    assert!(fallback.is_some(), "manual run should fire fallback");
    assert!(marker.exists(), "fallback should have created marker");
}
