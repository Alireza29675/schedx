mod helpers;

use std::path::Path;

use helpers::TestEnv;
use predicates::prelude::*;

#[cfg(unix)]
fn create_fake_binary(dir: &Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    let bin_path = dir.join(name);
    std::fs::write(&bin_path, "#!/bin/sh\ntrue\n").unwrap();
    std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Build a PATH that includes `bin_dir` and essential system dirs for `which` to work.
fn test_path(bin_dir: &Path) -> String {
    format!("{}:/usr/bin:/bin", bin_dir.display())
}

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

#[test]
#[cfg(unix)]
fn agent_detect_no_agents() {
    let env = TestEnv::new();
    let empty_dir = tempfile::tempdir().unwrap();

    env.cmd()
        .env("PATH", test_path(empty_dir.path()))
        .args(["agent", "detect", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"not_found\""));
}

#[test]
#[cfg(unix)]
fn agent_detect_single_agent() {
    let env = TestEnv::new();
    let bin_dir = tempfile::tempdir().unwrap();
    create_fake_binary(bin_dir.path(), "claude");

    env.cmd()
        .env("PATH", test_path(bin_dir.path()))
        .args(["agent", "detect", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"added\""))
        .stdout(predicate::str::contains("\"default_agent\": \"claude\""));

    // Verify it's actually in the config.
    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("(default)"));
}

#[test]
#[cfg(unix)]
fn agent_detect_idempotent() {
    let env = TestEnv::new();
    let bin_dir = tempfile::tempdir().unwrap();
    create_fake_binary(bin_dir.path(), "claude");

    // First run: added.
    env.cmd()
        .env("PATH", test_path(bin_dir.path()))
        .args(["agent", "detect", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"added\""));

    // Second run: already_registered.
    env.cmd()
        .env("PATH", test_path(bin_dir.path()))
        .args(["agent", "detect", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"already_registered\""));
}

#[test]
#[cfg(unix)]
fn agent_detect_force() {
    let env = TestEnv::new();
    let bin_dir = tempfile::tempdir().unwrap();
    create_fake_binary(bin_dir.path(), "claude");

    // Manually add claude with custom args.
    env.cmd()
        .args(["agent", "add", "claude", "--bin", "claude", "--arg=custom"])
        .assert()
        .success();

    // Force detect should overwrite.
    env.cmd()
        .env("PATH", test_path(bin_dir.path()))
        .args(["agent", "detect", "--force", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"updated\""));

    // Verify the profile was overwritten (should have -p, not custom).
    env.cmd()
        .args(["agent", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"-p\""))
        .stdout(predicate::str::contains("\"--enable-auto-mode\""));
}

#[test]
#[cfg(unix)]
fn agent_detect_preserves_custom_agents() {
    let env = TestEnv::new();
    let bin_dir = tempfile::tempdir().unwrap();
    create_fake_binary(bin_dir.path(), "claude");

    // Add a custom agent that's not in the known list.
    env.cmd()
        .args(["agent", "add", "my-custom", "--bin", "custom-bin"])
        .assert()
        .success();

    // Detect should not touch the custom agent.
    env.cmd()
        .env("PATH", test_path(bin_dir.path()))
        .args(["agent", "detect", "--force", "--json"])
        .assert()
        .success();

    env.cmd()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-custom"));
}
