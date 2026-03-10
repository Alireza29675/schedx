mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;

#[test]
fn add_run_job() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 5m",
            "--run",
            "echo hello",
            "--name",
            "test-job",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job test-job"));
}

#[test]
fn add_run_job_json() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo hi",
            "--name",
            "json-test",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""));
}

#[test]
fn add_prompt_job() {
    let env = TestEnv::new();
    // First add an agent
    env.cmd()
        .args([
            "agent",
            "add",
            "test-agent",
            "--bin",
            "echo",
            "--arg",
            "AGENT:",
        ])
        .assert()
        .success();

    env.cmd()
        .args([
            "add",
            "in 30m",
            "--prompt",
            "check system health",
            "--agent",
            "test-agent",
            "--name",
            "health-check",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job health-check"));
}

#[test]
fn add_requires_exactly_one_action() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 5m"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Exactly one action required"));
}

#[test]
fn add_shell_only_with_run() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 5m", "--prompt", "hello", "--shell"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--shell can only be used with --run",
        ));
}

#[test]
fn add_with_tags() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo tagged",
            "--tag",
            "ops",
            "--tag",
            "monitoring",
            "--name",
            "tagged-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["get", "tagged-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ops"))
        .stdout(predicate::str::contains("monitoring"));
}

#[test]
fn add_invalid_schedule() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "garbage", "--run", "echo test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not parse schedule"));
}

#[test]
fn add_cron_schedule() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "0 9 * * 1-5",
            "--run",
            "echo weekday",
            "--name",
            "weekday-job",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job weekday-job"));
}

#[test]
fn add_bare_seconds() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "30s", "--run", "echo quick", "--name", "quick-job"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job quick-job"));
}

#[test]
fn add_in_seconds() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "in 10s",
            "--run",
            "echo fast",
            "--name",
            "fast-job",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schedule_input\": \"in 10s\""));
}

#[test]
fn add_every_seconds() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 15s",
            "--run",
            "echo heartbeat",
            "--name",
            "heartbeat",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"schedule_input\": \"every 15s\"",
        ))
        .stdout(predicate::str::contains("\"next_run\""))
        .stdout(predicate::str::contains("\"next_run_readable\""));
}

#[test]
fn add_json_has_readable_fields() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "echo readable",
            "--name",
            "readable-job",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"next_run_readable\""))
        .stdout(predicate::str::contains("\"created_at_readable\""))
        .stdout(predicate::str::contains("\"updated_at_readable\""));
}

#[test]
fn add_with_timeout() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 1h",
            "--run",
            "sleep 10",
            "--timeout",
            "60",
            "--name",
            "timeout-job",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["get", "timeout-job", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"timeout_seconds\": 60"));
}
