mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn manual_run_captures_output() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo hello-from-run",
            "--name",
            "run-test",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["run", "run-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Running job run-test"));

    // Check logs contain the output
    env.cmd()
        .args(["logs", "run-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-from-run"));
}

#[test]
fn manual_run_increments_count() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo count",
            "--name",
            "count-job",
        ])
        .assert()
        .success();

    env.cmd().args(["run", "count-job"]).assert().success();
    env.cmd().args(["run", "count-job"]).assert().success();

    env.cmd()
        .args(["get", "count-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"run_count\": 2"));
}

#[test]
fn run_nonexistent_job() {
    let env = TestEnv::new();

    env.cmd()
        .args(["run", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn run_with_shell_mode() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo shell | cat",
            "--shell",
            "--name",
            "shell-job",
        ])
        .assert()
        .success();

    env.cmd().args(["run", "shell-job"]).assert().success();

    env.cmd()
        .args(["logs", "shell-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shell"));
}

#[test]
fn logs_with_specific_run_shows_header() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo header-test",
            "--name",
            "header-job",
        ])
        .assert()
        .success();

    env.cmd().args(["run", "header-job"]).assert().success();

    let history_output = env
        .cmd()
        .args(["history", "header-job", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let history: Value = serde_json::from_slice(&history_output).expect("valid history JSON");
    let run_id = history
        .as_array()
        .and_then(|records| records.first())
        .and_then(|record| record.get("run_id"))
        .and_then(Value::as_str)
        .expect("history contains a run_id")
        .to_string();

    env.cmd()
        .args(["logs", "header-job", "--run", &run_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Run: {run_id} at ")))
        .stdout(predicate::str::contains("Logs:"))
        .stdout(predicate::str::contains("header-test"));
}
