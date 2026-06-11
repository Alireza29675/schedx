//! CLI integration tests for `schedx up` — declarative manifest reconciliation.

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

/// `get <name> --json` parsed, asserting the job exists.
fn job_json(env: &TestEnv, name: &str) -> serde_json::Value {
    let output = env
        .cmd()
        .args(["get", name, "--json"])
        .output()
        .expect("failed to run get");
    assert!(
        output.status.success(),
        "get {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("get --json produced invalid JSON")
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
fn up_creates_run_prompt_and_webhook_jobs() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: trio
defaults:
  timeout: 120
  tags: [managed]
jobs:
  runner:
    schedule: every 5m
    run: echo one
    shell: true
    workdir: /tmp
  prompter:
    schedule: every 30m
    prompt: summarize the inbox
    agent: claude
    paused: true
  hooker:
    schedule: 0 9 * * 1-5
    webhook: https://example.com/hook
    method: GET
    timeout: 60
    tags: [own]
",
    );

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("+ create runner [run]"))
        .stdout(predicate::str::contains("+ create prompter [prompt]"))
        .stdout(predicate::str::contains("+ create hooker [webhook]"))
        .stdout(predicate::str::contains("Applied manifest 'trio'"));

    let runner = job_json(&env, "runner");
    assert_eq!(runner["action"]["type"], "run");
    assert_eq!(runner["action"]["command"], "echo one");
    assert_eq!(runner["action"]["shell"], true);
    assert_eq!(runner["action"]["workdir"], "/tmp");
    assert_eq!(runner["schedule_input"], "every 5m");
    assert_eq!(runner["timeout_seconds"], 120, "defaults.timeout applies");
    assert_eq!(runner["tags"], serde_json::json!(["managed"]));
    assert_eq!(runner["status"], "active");

    let prompter = job_json(&env, "prompter");
    assert_eq!(prompter["action"]["type"], "prompt");
    assert_eq!(prompter["action"]["text"], "summarize the inbox");
    assert_eq!(prompter["action"]["agent"], "claude");
    assert_eq!(prompter["status"], "paused", "paused: true lands as paused");
    assert_eq!(prompter["timeout_seconds"], 120);

    let hooker = job_json(&env, "hooker");
    assert_eq!(hooker["action"]["type"], "webhook");
    assert_eq!(hooker["action"]["url"], "https://example.com/hook");
    assert_eq!(hooker["action"]["method"], "GET");
    assert_eq!(hooker["schedule_input"], "0 9 * * 1-5");
    assert_eq!(
        hooker["timeout_seconds"], 60,
        "job timeout overrides default"
    );
    assert_eq!(hooker["tags"], serde_json::json!(["own"]));
}

#[test]
fn second_up_is_a_noop_and_leaves_store_untouched() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: idem
jobs:
  steady:
    schedule: every 1h
    run: echo steady
",
    );

    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    let before = jobs_file_bytes(&env);

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes"));

    let after = jobs_file_bytes(&env);
    assert_eq!(before, after, "a noop up must not rewrite jobs.json");
}

#[test]
fn yaml_edit_updates_job_in_place_keeping_id() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: inplace
jobs:
  worker:
    schedule: every 1h
    run: echo before
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    let id_before = job_json(&env, "worker")["id"].as_str().unwrap().to_string();

    write_yaml(
        dir.path(),
        "
name: inplace
jobs:
  worker:
    schedule: every 1h
    run: echo after
",
    );
    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("~ update worker"));

    let worker = job_json(&env, "worker");
    assert_eq!(worker["id"], id_before.as_str(), "update must keep the id");
    assert_eq!(worker["action"]["command"], "echo after");
}

#[test]
fn removed_yaml_job_is_pruned_and_unmanaged_job_survives() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();

    // An imperatively-added job with a different name must come through
    // the whole up lifecycle byte-for-byte untouched.
    env.cmd()
        .args([
            "add",
            "every 7m",
            "--run",
            "echo bystander",
            "--name",
            "bystander",
        ])
        .assert()
        .success();
    let bystander_before = job_json(&env, "bystander");

    let path = write_yaml(
        dir.path(),
        "
name: prune
jobs:
  keep:
    schedule: every 1h
    run: echo keep
  goner:
    schedule: every 2h
    run: echo goner
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();

    write_yaml(
        dir.path(),
        "
name: prune
jobs:
  keep:
    schedule: every 1h
    run: echo keep
",
    );
    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("- remove goner"));

    env.cmd().args(["get", "goner"]).assert().failure();
    env.cmd().args(["get", "keep"]).assert().success();

    let bystander_after = job_json(&env, "bystander");
    for key in [
        "id",
        "status",
        "schedule_input",
        "action",
        "tags",
        "timeout_seconds",
        "created_at",
        "updated_at",
    ] {
        assert_eq!(
            bystander_before[key], bystander_after[key],
            "unmanaged job field '{key}' changed during the up lifecycle"
        );
    }
}

#[test]
fn imperative_edit_is_drift_corrected() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: driftman
jobs:
  watcher:
    schedule: every 1h
    run: echo stable
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();

    env.cmd()
        .args(["edit", "watcher", "--run", "echo TAMPERED"])
        .assert()
        .success();

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("correct drift on watcher"));

    assert_eq!(
        job_json(&env, "watcher")["action"]["command"],
        "echo stable",
        "drift correction must restore the declared command"
    );
}

#[test]
fn imperatively_removed_managed_job_is_recreated() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: phoenix
jobs:
  reborn:
    schedule: every 1h
    run: echo reborn
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();

    env.cmd()
        .args(["rm", "reborn", "--force"])
        .assert()
        .success();
    env.cmd().args(["get", "reborn"]).assert().failure();

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("recreate reborn"));

    let reborn = job_json(&env, "reborn");
    assert_eq!(reborn["action"]["command"], "echo reborn");
    assert_eq!(reborn["status"], "active");
}

#[test]
fn unmanaged_name_collision_rejected_and_store_untouched() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();

    env.cmd()
        .args(["add", "every 5m", "--run", "echo mine", "--name", "taken"])
        .assert()
        .success();
    let before = jobs_file_bytes(&env);

    let path = write_yaml(
        dir.path(),
        "
name: grabby
jobs:
  taken:
    schedule: every 1h
    run: echo theirs
",
    );
    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "already exists and is not managed",
        ));

    assert_eq!(
        before,
        jobs_file_bytes(&env),
        "a refused up must not touch the store"
    );
    assert_eq!(
        job_json(&env, "taken")["action"]["command"],
        "echo mine",
        "the unmanaged job must never be adopted"
    );
}

#[test]
fn cross_manifest_collision_names_the_owning_manifest() {
    let env = TestEnv::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    let path_a = write_yaml(
        dir_a.path(),
        "
name: mani-a
jobs:
  shared-x:
    schedule: every 1h
    run: echo a
",
    );
    env.cmd().args(["up", "-f"]).arg(&path_a).assert().success();

    let path_b = write_yaml(
        dir_b.path(),
        "
name: mani-b
jobs:
  shared-x:
    schedule: every 1h
    run: echo b
",
    );
    env.cmd()
        .args(["up", "-f"])
        .arg(&path_b)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is managed by manifest 'mani-a'"));

    assert_eq!(
        job_json(&env, "shared-x")["action"]["command"],
        "echo a",
        "manifest A's job must be left alone"
    );
}

// NOTE — contract deviation, deliberate: schedule validation happens at
// PLAN time (so a completed one-shot's past date can't brick an unchanged
// manifest — see src/manifest/parse.rs), while action-shape validation
// happens at PARSE time. A manifest mixing a bad schedule with a
// two-action job therefore reports ONLY the parse-stage issue and exits 2,
// not 4. The two tests below pin each stage's collect-all behavior
// separately.

#[test]
fn all_schedule_issues_reported_and_nothing_applied() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: badsched
jobs:
  bad-one:
    schedule: whenever
    run: echo one
  bad-two:
    schedule: someday
    run: echo two
  fine:
    schedule: every 1h
    run: echo fine
",
    );

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .code(4)
        .stderr(predicate::str::contains("Could not parse schedule"))
        .stderr(predicate::str::contains("jobs.bad-one"))
        .stderr(predicate::str::contains("jobs.bad-two"));

    assert_eq!(
        job_count(&env),
        0,
        "no partial apply: even the valid job stays out"
    );
    assert!(!manifest_state_path(&env, "badsched").exists());
}

#[test]
fn all_parse_issues_reported_and_nothing_applied() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: badshape
jobs:
  two-actions:
    schedule: every 1h
    run: echo hi
    prompt: also this
  lonely-agent:
    schedule: every 1h
    run: echo hi
    agent: claude
",
    );

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "exactly one action required: run, prompt, or webhook",
        ))
        .stderr(predicate::str::contains(
            "agent can only be used with prompt",
        ));

    assert_eq!(job_count(&env), 0);
    assert!(!manifest_state_path(&env, "badshape").exists());
}

#[test]
fn http_webhook_blocked_and_nothing_applied() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: insecure
jobs:
  hook:
    schedule: every 1h
    webhook: http://example.com/hook
",
    );

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(
            "HTTP webhooks are blocked by default",
        ));

    assert_eq!(job_count(&env), 0);
    assert!(!manifest_state_path(&env, "insecure").exists());
}

#[test]
fn env_var_interpolation_lands_in_webhook_header() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    // Header key is deliberately non-sensitive so `get --json` shows the
    // value unredacted.
    let path = write_yaml(
        dir.path(),
        "
name: envman
jobs:
  notify:
    schedule: every 1h
    webhook: https://example.com/hook
    headers:
      x-env-marker: Bearer ${SCHEDX_TEST_UP_TOKEN}
",
    );

    env.cmd()
        .env("SCHEDX_TEST_UP_TOKEN", "s3cr3t")
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success();

    let headers = job_json(&env, "notify")["action"]["headers"].clone();
    assert_eq!(headers, serde_json::json!(["x-env-marker: Bearer s3cr3t"]));
}

#[test]
fn missing_env_var_fails_naming_it_and_applies_nothing() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: envless
jobs:
  notify:
    schedule: every 1h
    webhook: https://example.com/hook
    headers:
      x-env-marker: Bearer ${SCHEDX_TEST_UP_TOKEN}
",
    );

    env.cmd()
        .env_remove("SCHEDX_TEST_UP_TOKEN")
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "undefined environment variable 'SCHEDX_TEST_UP_TOKEN'",
        ));

    assert_eq!(job_count(&env), 0);
    assert!(!manifest_state_path(&env, "envless").exists());
}

#[test]
fn env_var_rotation_reconciles_on_next_up() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: rotator
jobs:
  notify:
    schedule: every 1h
    webhook: https://example.com/hook
    headers:
      x-env-marker: Bearer ${SCHEDX_TEST_UP_TOKEN}
",
    );

    env.cmd()
        .env("SCHEDX_TEST_UP_TOKEN", "old-value")
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success();

    // Same yaml, new env value: the expanded spec differs, so up updates.
    env.cmd()
        .env("SCHEDX_TEST_UP_TOKEN", "new-value")
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("~ update notify"));

    let headers = job_json(&env, "notify")["action"]["headers"].clone();
    assert_eq!(
        headers,
        serde_json::json!(["x-env-marker: Bearer new-value"])
    );
}

#[test]
fn dry_run_creates_nothing() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: ghost
jobs:
  spectre:
    schedule: every 1h
    run: echo boo
",
    );

    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("would create spectre"));

    assert_eq!(job_count(&env), 0, "dry run must not create jobs");
    assert!(
        !manifest_state_path(&env, "ghost").exists(),
        "dry run must not record manifest state"
    );
}

#[test]
fn json_report_shape_on_create() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: jsonman
jobs:
  solo:
    schedule: every 1h
    run: echo solo
",
    );

    let output = env
        .cmd()
        .args(["up", "-f"])
        .arg(&path)
        .arg("--json")
        .output()
        .expect("failed to run up --json");
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("up --json must print valid JSON");
    assert_eq!(report["manifest"], "jsonman");
    assert_eq!(report["dry_run"], false);
    let created = report["created"]
        .as_array()
        .expect("created must be an array");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0]["name"], "solo");
    assert_eq!(created[0]["kind"], "run");
    assert_eq!(created[0]["schedule"], "every 1h");
    assert!(
        created[0]["id"].as_str().is_some_and(|id| !id.is_empty()),
        "a real apply must report the generated id"
    );
}

#[test]
fn same_name_from_different_directory_is_refused_until_forced() {
    let env = TestEnv::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let yaml = "
name: guard
jobs:
  job-x:
    schedule: every 1h
    run: echo x
";

    let path_a = write_yaml(dir_a.path(), yaml);
    let path_b = write_yaml(dir_b.path(), yaml);

    env.cmd().args(["up", "-f"]).arg(&path_a).assert().success();

    // Same manifest name from another directory: refuse.
    env.cmd()
        .args(["up", "-f"])
        .arg(&path_b)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already in use by"));

    // --force accepts the move and updates the recorded path.
    env.cmd()
        .args(["up", "-f"])
        .arg(&path_b)
        .arg("--force")
        .assert()
        .success();

    // After the force, the new directory is the recorded home: a plain
    // up from there works normally.
    env.cmd()
        .args(["up", "-f"])
        .arg(&path_b)
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes"));
}

#[test]
fn lost_state_file_self_heals_without_duplicates() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();
    let path = write_yaml(
        dir.path(),
        "
name: healer
jobs:
  patient:
    schedule: every 1h
    run: echo ok
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();

    let state_path = manifest_state_path(&env, "healer");
    std::fs::remove_file(&state_path).expect("state file should exist after up");

    // Ownership is recovered from the managed_by markers on the jobs
    // themselves: no recreation, no duplicates.
    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes"));

    assert_eq!(job_count(&env), 1, "self-heal must not duplicate jobs");
    assert!(state_path.exists(), "up must re-record the manifest state");
}

#[test]
fn paused_tristate_respects_runtime_state() {
    let env = TestEnv::new();
    let dir = tempfile::tempdir().unwrap();

    // paused: true -> paused.
    let path = write_yaml(
        dir.path(),
        "
name: tristate
jobs:
  toggler:
    schedule: every 1h
    run: echo t
    paused: true
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    assert_eq!(job_json(&env, "toggler")["status"], "paused");

    // paused: false -> active.
    write_yaml(
        dir.path(),
        "
name: tristate
jobs:
  toggler:
    schedule: every 1h
    run: echo t
    paused: false
",
    );
    env.cmd().args(["up", "-f"]).arg(&path).assert().success();
    assert_eq!(job_json(&env, "toggler")["status"], "active");

    // Key removed -> tri-state None: the current runtime status is
    // respected and the manifest is a noop.
    write_yaml(
        dir.path(),
        "
name: tristate
jobs:
  toggler:
    schedule: every 1h
    run: echo t
",
    );
    env.cmd()
        .args(["up", "-f"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes"));
    assert_eq!(job_json(&env, "toggler")["status"], "active");
}
