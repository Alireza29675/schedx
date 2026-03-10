/// Schedule parser edge cases and validation tests.
/// Core parsing tests are in the unit test module inside parser.rs.
/// These are integration-level tests exercising the CLI error messages.
mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;

#[test]
fn schedule_every_zero_rejected() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 0h", "--run", "echo test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not parse schedule"));
}

#[test]
fn schedule_every_minutes_max() {
    let env = TestEnv::new();
    // 59m is valid
    env.cmd()
        .args([
            "add",
            "every 59m",
            "--run",
            "echo test",
            "--name",
            "max-min",
        ])
        .assert()
        .success();

    // 60m is invalid
    env.cmd()
        .args(["add", "every 60m", "--run", "echo test"])
        .assert()
        .failure();
}

#[test]
fn schedule_in_creates_oneshot() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "in 4h",
            "--run",
            "echo oneshot",
            "--name",
            "oneshot-job",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schedule_input\": \"in 4h\""));
}

#[test]
fn schedule_iso_datetime() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "2099-12-31T23:59:59Z",
            "--run",
            "echo future",
            "--name",
            "future-job",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job future-job"));
}

#[test]
fn schedule_past_datetime_rejected() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "2020-01-01T00:00:00Z", "--run", "echo past"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("in the past"));
}

#[test]
fn schedule_invalid_string() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "tomorrow", "--run", "echo test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not parse schedule"));
}

#[test]
fn schedule_cron_weekday() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "0 9 * * 1-5",
            "--run",
            "echo weekday",
            "--name",
            "weekday",
        ])
        .assert()
        .success();
}

#[test]
fn schedule_cron_invalid() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "99 99 99 99 99", "--run", "echo test"])
        .assert()
        .failure();
}

#[test]
fn schedule_bare_seconds_oneshot() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "10s", "--run", "echo quick", "--name", "quick"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job quick"));
}

#[test]
fn schedule_bare_minutes_oneshot() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "5m", "--run", "echo fivemin", "--name", "fivemin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created job fivemin"));
}

#[test]
fn schedule_every_seconds_recurring() {
    let env = TestEnv::new();
    env.cmd()
        .args([
            "add",
            "every 30s",
            "--run",
            "echo tick",
            "--name",
            "ticker",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"schedule_input\": \"every 30s\"",
        ));
}

#[test]
fn schedule_every_zero_seconds_rejected() {
    let env = TestEnv::new();
    env.cmd()
        .args(["add", "every 0s", "--run", "echo test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not parse schedule"));
}
