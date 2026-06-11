//! CLI integration tests for `schedx down` — bringing a declarative
//! manifest's jobs back out of the store.

mod helpers;

use std::path::{Path, PathBuf};

use helpers::TestEnv;
use predicates::prelude::*;

/// Write a manifest into `dir` and return its path (always passed to
/// `up`/`down` via `-f` as an absolute path — no cwd games).
fn write_yaml(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("schedx.yaml");
    std::fs::write(&path, content).expect("failed to write manifest");
    path
}

/// Total job count, archived included.
fn job_count(env: &TestEnv) -> usize {
    let output = env
        .cmd()
        .args(["list", "--all", "--json"])
        .output()
        .expect("failed to run list");
    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("list --json produced invalid JSON");
    v.as_array().expect("list --json must be an array").len()
}

fn jobs_file_bytes(env: &TestEnv) -> Vec<u8> {
    std::fs::read(env.home().join("jobs.json")).expect("jobs.json should exist")
}

fn manifest_state_path(env: &TestEnv, manifest: &str) -> PathBuf {
    env.home()
        .join("manifests")
        .join(format!("{manifest}.json"))
}

#[test]
fn down_removes_managed_jobs_only_and_second_down_is_noop() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();

    env.cmd()
        .args([
            "add",
            "every 9m",
            "--run",
            "echo survivor",
            "--name",
            "survivor",
        ])
        .assert()
        .success();

    let path = write_yaml(
        dir.path(),
        "
name: teardown
jobs:
  alpha:
    schedule: every 1h
    run: echo a
  beta:
    schedule: every 2h
    run: echo b
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    assert_eq!(job_count(&env), 3);

    env.cmd()
        .args(["down", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("removed alpha"))
        .stdout(predicate::str::contains("removed beta"))
        .stdout(predicate::str::contains(
            "Brought down manifest 'teardown': 2 job(s) removed.",
        ));

    env.cmd().args(["get", "alpha"]).assert().failure();
    env.cmd().args(["get", "beta"]).assert().failure();
    env.cmd().args(["get", "survivor"]).assert().success();
    assert!(
        !manifest_state_path(&env, "teardown").exists(),
        "down must delete the manifest state file"
    );

    // Second down: nothing left to do, still exit 0.
    env.cmd()
        .args(["down", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to bring down"));
}

#[test]
fn down_dry_run_changes_nothing() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: rehearsal
jobs:
  actor:
    schedule: every 1h
    run: echo act
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    let before = jobs_file_bytes(&env);

    env.cmd()
        .args(["down", "-f"])
        .arg(&path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("would remove actor"));

    assert_eq!(
        before,
        jobs_file_bytes(&env),
        "dry run must not touch the store"
    );
    env.cmd().args(["get", "actor"]).assert().success();
    assert!(
        manifest_state_path(&env, "rehearsal").exists(),
        "dry run must keep the manifest state file"
    );
}

#[test]
fn down_unknown_manifest_exits_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["down", "--manifest", "nosuch"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn down_works_after_yaml_deleted_via_directory_name() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    // No explicit `name:`: the manifest name derives from the directory.
    let project = dir.path().join("downderived");
    std::fs::create_dir(&project).unwrap();
    let path = write_yaml(
        &project,
        "
jobs:
  orphan-to-be:
    schedule: every 1h
    run: echo bye
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    assert!(manifest_state_path(&env, "downderived").exists());

    // Compose-style: the yaml is gone, but `down -f` pointing into the
    // same directory still resolves the manifest.
    std::fs::remove_file(&path).unwrap();
    env.cmd()
        .args(["down", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("removed orphan-to-be"));

    assert_eq!(job_count(&env), 0);
    assert!(!manifest_state_path(&env, "downderived").exists());
}

#[test]
fn down_json_report_shape() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: jsondown
jobs:
  reportee:
    schedule: every 1h
    run: echo r
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    let id = {
        let output = env
            .cmd()
            .args(["get", "reportee", "--json"])
            .output()
            .expect("failed to run get");
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid JSON");
        v["id"].as_str().expect("job must have an id").to_string()
    };

    let output = env
        .cmd()
        .args(["down", "-f"])
        .arg(&path)
        .arg("--json")
        .output()
        .expect("failed to run down --json");
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("down --json must print valid JSON");
    assert_eq!(report["manifest"], "jsondown");
    assert_eq!(report["dry_run"], false);
    let removed = report["removed"]
        .as_array()
        .expect("removed must be an array");
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0]["name"], "reportee");
    assert_eq!(removed[0]["id"], id.as_str());
    assert_eq!(removed[0]["kind"], "run");
    assert_eq!(removed[0]["schedule"], "every 1h");
}

#[test]
fn down_from_wrong_directory_is_refused_but_manifest_flag_works() {
    let env = TestEnv::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let yaml = "
name: guarddown
jobs:
  precious:
    schedule: every 1h
    run: echo p
";

    let path_a = write_yaml(dir_a.path(), yaml);
    env.cmd().args(["up", "-f"]).arg(&path_a).assert().success();

    // Same manifest name resolved from a different directory: a wrong-spot
    // down is as destructive as a wrong-spot up, so it is refused.
    let path_b = write_yaml(dir_b.path(), yaml);
    env.cmd()
        .args(["down", "-f"])
        .arg(&path_b)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already in use by"));
    env.cmd().args(["get", "precious"]).assert().success();

    // `--manifest <name>` names the exact target and skips the path
    // check by design.
    env.cmd()
        .args(["down", "--manifest", "guarddown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed precious"));
    assert_eq!(job_count(&env), 0);
}
