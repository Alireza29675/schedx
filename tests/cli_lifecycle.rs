mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;

#[test]
fn pause_and_resume() {
    let env = TestEnv::new();

    // Add a job
    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo hello",
            "--name",
            "lifecycle-job",
        ])
        .assert()
        .success();

    // Pause
    env.cmd()
        .args(["pause", "lifecycle-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Paused job lifecycle-job"));

    // Verify paused status
    env.cmd()
        .args(["get", "lifecycle-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"paused\""));

    // Double-pause should error
    env.cmd()
        .args(["pause", "lifecycle-job"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already paused"));

    // Resume
    env.cmd()
        .args(["resume", "lifecycle-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Resumed job lifecycle-job"));

    // Verify active status
    env.cmd()
        .args(["get", "lifecycle-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"active\""));

    // Double-resume should error
    env.cmd()
        .args(["resume", "lifecycle-job"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already active"));
}

#[test]
fn rm_requires_force() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo rm-test",
            "--name",
            "rm-job",
        ])
        .assert()
        .success();

    // Without --force, should prompt
    env.cmd()
        .args(["rm", "rm-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"));

    // Job still exists
    env.cmd().args(["get", "rm-job"]).assert().success();

    // With --force, should remove
    env.cmd()
        .args(["rm", "rm-job", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed job rm-job"));

    // Job no longer exists
    env.cmd().args(["get", "rm-job"]).assert().failure();
}

#[test]
fn skip_recurring_job() {
    let env = TestEnv::new();

    // Add a recurring job
    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo skip-test",
            "--name",
            "skip-job",
        ])
        .assert()
        .success();

    // Skip 1 run (default)
    env.cmd()
        .args(["skip", "skip-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Skipping next 1 run"))
        .stdout(predicate::str::contains("Next execution:"));

    // Verify skip_remaining in JSON
    env.cmd()
        .args(["get", "skip-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skip_remaining\": 1"));

    // Skip 2 more (cumulative: 3)
    env.cmd()
        .args(["skip", "skip-job", "--times", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Skipping next 3 runs"));

    // Verify skip_remaining is now 3
    env.cmd()
        .args(["get", "skip-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skip_remaining\": 3"));
}

#[test]
fn skip_oneshot_rejected() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "in 30m",
            "--run",
            "echo oneshot",
            "--name",
            "oneshot-job",
        ])
        .assert()
        .success();

    // Skip should be rejected for one-shot jobs
    env.cmd()
        .args(["skip", "oneshot-job"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("one-shot"));
}

#[test]
fn skip_shows_in_list() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo listed",
            "--name",
            "list-skip-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["skip", "list-skip-job", "--times", "2"])
        .assert()
        .success();

    // List should show the skip annotation
    env.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[skipping 2]"));
}

#[test]
fn get_job_not_found() {
    let env = TestEnv::new();
    env.cmd()
        .args(["get", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn get_by_id_and_name() {
    let env = TestEnv::new();

    // Add and capture ID from JSON
    let output = env
        .cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo get-test",
            "--name",
            "get-job",
            "--json",
        ])
        .output()
        .expect("failed to run command");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid JSON");
    let id = json["id"].as_str().expect("no id field");

    // Get by name
    env.cmd()
        .args(["get", "get-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("get-job"));

    // Get by ID
    env.cmd()
        .args(["get", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("get-job"));
}
