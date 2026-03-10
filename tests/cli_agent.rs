mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;

#[test]
fn agent_add_and_list() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "agent",
            "add",
            "test-agent",
            "--bin",
            "/usr/bin/echo",
            "--arg",
            "AGENT:",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added agent 'test-agent'"));

    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-agent"));
}

#[test]
fn agent_list_empty() {
    let env = TestEnv::new();

    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No agents configured"));
}

#[test]
fn agent_set_default() {
    let env = TestEnv::new();

    env.cmd()
        .args(["agent", "add", "my-agent", "--bin", "echo"])
        .assert()
        .success();

    env.cmd()
        .args(["agent", "default", "my-agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Default agent set to 'my-agent'"));

    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(default)"));
}

#[test]
fn agent_default_nonexistent() {
    let env = TestEnv::new();

    env.cmd()
        .args(["agent", "default", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn agent_rm() {
    let env = TestEnv::new();

    env.cmd()
        .args(["agent", "add", "rm-agent", "--bin", "echo"])
        .assert()
        .success();

    env.cmd()
        .args(["agent", "rm", "rm-agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed agent 'rm-agent'"));

    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No agents configured"));
}

#[test]
fn agent_rm_nonexistent() {
    let env = TestEnv::new();

    env.cmd()
        .args(["agent", "rm", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn agent_list_json() {
    let env = TestEnv::new();

    env.cmd()
        .args([
            "agent",
            "add",
            "json-agent",
            "--bin",
            "echo",
            "--arg=run",
            "--arg=json",
        ])
        .assert()
        .success();

    env.cmd()
        .args(["agent", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("json-agent"));
}
