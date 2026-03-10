mod helpers;

use std::fs;

use chrono::{Duration, Utc};
use helpers::TestEnv;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn list_empty() {
    let env = TestEnv::new();
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No jobs found"));
}

#[test]
fn list_empty_json() {
    let env = TestEnv::new();
    env.cmd()
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[test]
fn list_shows_added_jobs() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 5m", "--run", "echo one", "--name", "job-one"])
        .assert()
        .success();

    env.cmd()
        .args(["add", "every 1h", "--run", "echo two", "--name", "job-two"])
        .assert()
        .success();

    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("job-one"))
        .stdout(predicate::str::contains("job-two"));
}

#[test]
fn list_filter_by_status() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 5m", "--run", "echo a", "--name", "active-job"])
        .assert()
        .success();

    env.cmd()
        .args(["list", "--status", "paused"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No jobs found"));
}

#[test]
fn list_filter_by_tag() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo tagged",
            "--tag",
            "ops",
            "--name",
            "ops-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo untagged",
            "--name",
            "other-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["list", "--tag", "ops"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ops-job"))
        .stdout(predicate::str::contains("other-job").not());
}

#[test]
fn list_json_format() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo json",
            "--name",
            "json-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"schedule_input\""))
        .stdout(predicate::str::contains("\"next_run\""))
        .stdout(predicate::str::contains("\"next_run_readable\""));
}

#[test]
fn overdue_jobs_are_labeled_due_not_next() {
    let env = TestEnv::new();

    let add_output = env
        .cmd()
        .args([
            "add",
            "every 1m",
            "--run",
            "echo overdue",
            "--name",
            "overdue-job",
            "--json",
        ])
        .output()
        .expect("failed to add job");
    assert!(add_output.status.success(), "add command should succeed");

    let add_json: Value = serde_json::from_slice(&add_output.stdout).expect("valid add JSON");
    let job_id = add_json["id"]
        .as_str()
        .expect("add JSON should include job id")
        .to_string();

    let jobs_path = env.home().join("jobs.json");
    let jobs_raw = fs::read_to_string(&jobs_path).expect("failed to read jobs.json");
    let mut jobs_json: Value = serde_json::from_str(&jobs_raw).expect("valid jobs.json");

    let old_time = (Utc::now() - Duration::seconds(90)).to_rfc3339();
    jobs_json["jobs"][&job_id]["created_at"] = Value::String(old_time.clone());
    jobs_json["jobs"][&job_id]["updated_at"] = Value::String(old_time);
    jobs_json["jobs"][&job_id]["last_scheduled_at"] = Value::Null;

    fs::write(
        &jobs_path,
        serde_json::to_string_pretty(&jobs_json).expect("serialize jobs.json"),
    )
    .expect("failed to write jobs.json");

    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Due:"));

    env.cmd()
        .args(["get", "overdue-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Due:"));
}
