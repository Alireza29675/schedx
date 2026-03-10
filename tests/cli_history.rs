mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;

#[test]
fn history_empty() {
    let env = TestEnv::new();

    env.cmd()
        .args(["history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No history records found"));
}

#[test]
fn history_after_run() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo hist",
            "--name",
            "hist-job",
        ])
        .assert()
        .success();

    env.cmd().args(["run", "hist-job"]).assert().success();

    env.cmd()
        .args(["history", "hist-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("success"));
}

#[test]
fn history_json() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo json-hist",
            "--name",
            "json-hist-job",
        ])
        .assert()
        .success();

    env.cmd().args(["run", "json-hist-job"]).assert().success();

    env.cmd()
        .args(["history", "json-hist-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"run_id\""))
        .stdout(predicate::str::contains("\"status\""));
}

#[test]
fn history_with_limit() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo limit",
            "--name",
            "limit-job",
        ])
        .assert()
        .success();

    // Run multiple times
    env.cmd().args(["run", "limit-job"]).assert().success();
    env.cmd().args(["run", "limit-job"]).assert().success();
    env.cmd().args(["run", "limit-job"]).assert().success();

    // With limit 1, should show only 1 record
    let output = env
        .cmd()
        .args(["history", "limit-job", "--limit", "1", "--json"])
        .output()
        .expect("failed to run command");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid JSON");
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[test]
fn history_all_jobs() {
    let env = TestEnv::new();

    env.cmd()
        .args(["add", "every 1h", "--run", "echo a", "--name", "job-a"])
        .assert()
        .success();

    env.cmd()
        .args(["add", "every 1h", "--run", "echo b", "--name", "job-b"])
        .assert()
        .success();

    env.cmd().args(["run", "job-a"]).assert().success();
    env.cmd().args(["run", "job-b"]).assert().success();

    // History without job ID shows all
    env.cmd().args(["history", "--json"]).assert().success();

    let output = env
        .cmd()
        .args(["history", "--json"])
        .output()
        .expect("failed to run command");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid JSON");
    assert!(json.as_array().unwrap().len() >= 2);
}
