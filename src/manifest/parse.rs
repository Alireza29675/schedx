//! yaml -> [`Manifest`] parsing, normalization, and validation.
//!
//! Mirrors the `add` command's semantics (via the shared
//! `commands::action_input` builders) but with yaml field names in
//! messages, and is stricter where the CLI is lenient: `method`,
//! `headers`, and `body` are rejected on non-webhook jobs instead of
//! being silently ignored — declarative files deny surprises.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;
use serde::Deserialize;

use crate::commands::action_input::{
    build_prompt_action, build_run_action, build_webhook_action, parse_method, validate_on_failure,
    validate_tags,
};
use crate::manifest::{JobSpec, Manifest, env};
use crate::schedule::parser::parse_schedule;

/// One validation problem, located by `context` (e.g. `jobs.backup-home`
/// or `manifest`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestIssue {
    pub context: String,
    pub message: String,
}

impl ManifestIssue {
    fn new(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    name: Option<String>,
    defaults: Option<RawDefaults>,
    jobs: BTreeMap<String, RawJob>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDefaults {
    timeout: Option<u64>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawJob {
    schedule: String,
    run: Option<String>,
    prompt: Option<String>,
    webhook: Option<String>,
    shell: Option<bool>,
    workdir: Option<String>,
    agent: Option<String>,
    method: Option<String>,
    headers: Option<BTreeMap<String, String>>,
    body: Option<String>,
    timeout: Option<u64>,
    tags: Option<Vec<String>>,
    paused: Option<bool>,
    on_failure: Option<String>,
    on_failure_shell: Option<bool>,
}

/// Load, expand, and validate a manifest file.
///
/// Issues are collected across all jobs rather than stopping at the
/// first failure.
pub fn load_manifest(
    path: &Path,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Manifest, Vec<ManifestIssue>> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        vec![ManifestIssue::new(
            "manifest",
            format!("failed to read {}: {e}", path.display()),
        )]
    })?;
    let canonical = path.canonicalize().map_err(|e| {
        vec![ManifestIssue::new(
            "manifest",
            format!("failed to resolve {}: {e}", path.display()),
        )]
    })?;

    let raw: RawManifest = serde_norway::from_str(&text)
        .map_err(|e| vec![ManifestIssue::new("manifest", e.to_string())])?;

    let mut issues = Vec::new();
    let name = resolve_name(raw.name.as_deref(), &canonical, &mut issues);
    let defaults = raw.defaults.unwrap_or_default();

    let mut jobs = BTreeMap::new();
    for (job_name, raw_job) in raw.jobs {
        let context = format!("jobs.{job_name}");
        if job_name.is_empty() {
            issues.push(ManifestIssue::new(context, "job name must not be empty"));
            continue;
        }
        match build_job(&raw_job, &defaults, lookup, &context) {
            Ok(spec) => {
                jobs.insert(job_name, spec);
            }
            Err(mut job_issues) => issues.append(&mut job_issues),
        }
    }

    if issues.is_empty() {
        Ok(Manifest {
            name,
            path: canonical,
            jobs,
        })
    } else {
        Err(issues)
    }
}

/// Resolve the manifest name: explicit `name:` (strictly validated) or
/// derived from the yaml's parent directory, sanitized. The name becomes
/// a state filename later, hence the strictness.
fn resolve_name(
    explicit: Option<&str>,
    canonical: &Path,
    issues: &mut Vec<ManifestIssue>,
) -> String {
    if let Some(name) = explicit {
        if is_valid_manifest_name(name) {
            return name.to_string();
        }
        issues.push(ManifestIssue::new(
            "manifest",
            format!("invalid manifest name '{name}': must match [A-Za-z0-9][A-Za-z0-9._-]{{0,63}}"),
        ));
        return String::new();
    }

    let dir_name = canonical
        .parent()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Same shape as the explicit-name rule: leading char must be
    // alphanumeric (no hidden state files from `.config`, no
    // flag-lookalikes from `-foo`), capped at 64.
    let sanitized: String = dir_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .skip_while(|c| !c.is_ascii_alphanumeric())
        .take(64)
        .collect();
    if sanitized.is_empty() {
        issues.push(ManifestIssue::new(
            "manifest",
            "cannot derive a manifest name from the file's directory; set 'name:' explicitly",
        ));
    }
    sanitized
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`
fn is_valid_manifest_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Expand, apply defaults, and validate one job.
#[allow(clippy::too_many_lines)]
fn build_job(
    raw: &RawJob,
    defaults: &RawDefaults,
    lookup: &dyn Fn(&str) -> Option<String>,
    context: &str,
) -> Result<JobSpec, Vec<ManifestIssue>> {
    let mut issues = Vec::new();

    // ${VAR} expansion over every user-provided string value
    // (header values, not keys). Tags are merged with defaults first
    // (fill-if-absent, no merging) so default tags expand too.
    let schedule = expand_value(&raw.schedule, lookup, context, &mut issues);
    let run = expand_opt(raw.run.as_deref(), lookup, context, &mut issues);
    let prompt = expand_opt(raw.prompt.as_deref(), lookup, context, &mut issues);
    let webhook = expand_opt(raw.webhook.as_deref(), lookup, context, &mut issues);
    let workdir = expand_opt(raw.workdir.as_deref(), lookup, context, &mut issues);
    let agent = expand_opt(raw.agent.as_deref(), lookup, context, &mut issues);
    let method = expand_opt(raw.method.as_deref(), lookup, context, &mut issues);
    let body = expand_opt(raw.body.as_deref(), lookup, context, &mut issues);
    let on_failure = expand_opt(raw.on_failure.as_deref(), lookup, context, &mut issues);
    let headers: Vec<(String, String)> = raw
        .headers
        .iter()
        .flatten()
        .map(|(k, v)| (k.clone(), expand_value(v, lookup, context, &mut issues)))
        .collect();
    let tags: Vec<String> = raw
        .tags
        .clone()
        .or_else(|| defaults.tags.clone())
        .unwrap_or_default()
        .iter()
        .map(|t| expand_value(t, lookup, context, &mut issues))
        .collect();
    if !issues.is_empty() {
        return Err(issues);
    }

    // Validation, mirroring `add` but with yaml field names.
    let action_count =
        u8::from(run.is_some()) + u8::from(prompt.is_some()) + u8::from(webhook.is_some());
    if action_count != 1 {
        issues.push(ManifestIssue::new(
            context,
            "exactly one action required: run, prompt, or webhook",
        ));
    }
    let combos: [(bool, &str); 6] = [
        (
            raw.shell.is_some() && run.is_none(),
            "shell can only be used with run",
        ),
        (
            raw.workdir.is_some() && run.is_none(),
            "workdir can only be used with run",
        ),
        (
            raw.agent.is_some() && prompt.is_none(),
            "agent can only be used with prompt",
        ),
        (
            raw.method.is_some() && webhook.is_none(),
            "method can only be used with webhook",
        ),
        (
            raw.headers.is_some() && webhook.is_none(),
            "headers can only be used with webhook",
        ),
        (
            raw.body.is_some() && webhook.is_none(),
            "body can only be used with webhook",
        ),
    ];
    for (violated, message) in combos {
        if violated {
            issues.push(ManifestIssue::new(context, message));
        }
    }
    if raw.on_failure_shell.is_some() && on_failure.is_none() {
        issues.push(ManifestIssue::new(
            context,
            "on_failure_shell can only be used with on_failure",
        ));
    }
    if let Err(e) = validate_on_failure(on_failure.as_deref()) {
        issues.push(ManifestIssue::new(
            context,
            strip_error_prefix(&format!("{e:#}")),
        ));
    }
    if let Err(e) = validate_tags(&tags) {
        issues.push(ManifestIssue::new(
            context,
            strip_error_prefix(&format!("{e:#}")),
        ));
    }
    // Validation only; apply-time re-parses.
    if let Err(e) = parse_schedule(&schedule, Utc::now()) {
        issues.push(ManifestIssue::new(
            context,
            strip_error_prefix(&format!("{e:#}")),
        ));
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    // Build the action via the shared builders.
    let action_result = if let Some(command) = run {
        build_run_action(command, raw.shell.unwrap_or(false), workdir)
    } else if let Some(text) = prompt {
        build_prompt_action(text, agent)
    } else {
        let url = webhook.expect("exactly one action validated above");
        parse_method(method.as_deref()).and_then(|m| build_webhook_action(&url, m, headers, body))
    };
    let action = match action_result {
        Ok(action) => action,
        Err(e) => {
            issues.push(ManifestIssue::new(
                context,
                strip_error_prefix(&format!("{e:#}")),
            ));
            return Err(issues);
        }
    };

    Ok(JobSpec {
        schedule_input: schedule,
        action,
        timeout_seconds: raw.timeout.or(defaults.timeout),
        tags,
        paused: raw.paused,
        on_failure,
        on_failure_shell: raw.on_failure_shell.unwrap_or(false),
    })
}

/// Expand one string value, recording an issue on failure (and
/// returning a placeholder the caller must not use once issues exist).
fn expand_value(
    value: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    context: &str,
    issues: &mut Vec<ManifestIssue>,
) -> String {
    match env::expand(value, lookup) {
        Ok(expanded) => expanded,
        Err(e) => {
            let message = if env::is_var_name(&e) {
                format!("undefined environment variable '{e}'")
            } else {
                e
            };
            issues.push(ManifestIssue::new(context, message));
            String::new()
        }
    }
}

fn expand_opt(
    value: Option<&str>,
    lookup: &dyn Fn(&str) -> Option<String>,
    context: &str,
    issues: &mut Vec<ManifestIssue>,
) -> Option<String> {
    value.map(|v| expand_value(v, lookup, context, issues))
}

/// The shared helpers prefix messages with "Error: "; the issue
/// formatter re-adds context, so strip it here. Applied consistently to
/// every reused helper message.
fn strip_error_prefix(message: &str) -> String {
    message
        .strip_prefix("Error: ")
        .unwrap_or(message)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::action::{Action, HttpMethod};

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn test_env(name: &str) -> Option<String> {
        match name {
            "TOKEN" => Some("s3cr3t".to_string()),
            "TARGET" => Some("/backups".to_string()),
            "ENV" => Some("prod".to_string()),
            _ => None,
        }
    }

    fn write_manifest(dir: &Path, yaml: &str) -> PathBuf {
        let path = dir.join("schedx.yaml");
        std::fs::write(&path, yaml).unwrap();
        path
    }

    #[test]
    fn happy_path_all_three_action_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            r#"
name: demo
jobs:
  backup:
    schedule: every 1h
    run: rsync -a /src ${TARGET}
    shell: true
    workdir: /tmp
    on_failure: notify-send failed
    on_failure_shell: true
  remind:
    schedule: every 30m
    prompt: check the inbox
    agent: claude
    paused: true
  notify:
    schedule: 0 9 * * 1-5
    webhook: https://example.com/hook
    headers:
      x-token: Bearer ${TOKEN}
      a-key: one
    body: '{"env":"${ENV}"}'
    timeout: 60
    tags: [reporting]
"#,
        );

        let manifest = load_manifest(&path, &test_env).unwrap();
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.path, path.canonicalize().unwrap());
        assert_eq!(manifest.jobs.len(), 3);

        assert_eq!(
            manifest.jobs["backup"],
            JobSpec {
                schedule_input: "every 1h".to_string(),
                action: Action::Run {
                    command: "rsync -a /src /backups".to_string(),
                    shell: true,
                    workdir: Some("/tmp".to_string()),
                },
                timeout_seconds: None,
                tags: vec![],
                paused: None,
                on_failure: Some("notify-send failed".to_string()),
                on_failure_shell: true,
            }
        );
        assert_eq!(
            manifest.jobs["remind"],
            JobSpec {
                schedule_input: "every 30m".to_string(),
                action: Action::Prompt {
                    text: "check the inbox".to_string(),
                    agent: Some("claude".to_string()),
                },
                timeout_seconds: None,
                tags: vec![],
                paused: Some(true),
                on_failure: None,
                on_failure_shell: false,
            }
        );
        // Headers come out in sorted (BTreeMap) order; method defaults to POST.
        assert_eq!(
            manifest.jobs["notify"],
            JobSpec {
                schedule_input: "0 9 * * 1-5".to_string(),
                action: Action::Webhook {
                    url: "https://example.com/hook".to_string(),
                    method: HttpMethod::Post,
                    headers: vec![
                        ("a-key".to_string(), "one".to_string()),
                        ("x-token".to_string(), "Bearer s3cr3t".to_string()),
                    ],
                    body: Some(r#"{"env":"prod"}"#.to_string()),
                },
                timeout_seconds: Some(60),
                tags: vec!["reporting".to_string()],
                paused: None,
                on_failure: None,
                on_failure_shell: false,
            }
        );
    }

    #[test]
    fn defaults_fill_if_absent_and_job_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: defs
defaults:
  timeout: 120
  tags: [team, nightly]
jobs:
  inherit:
    schedule: every 1h
    run: echo hi
  override:
    schedule: every 2h
    run: echo bye
    timeout: 30
    tags: [own]
",
        );

        let manifest = load_manifest(&path, &no_env).unwrap();
        assert_eq!(manifest.jobs["inherit"].timeout_seconds, Some(120));
        assert_eq!(manifest.jobs["inherit"].tags, vec!["team", "nightly"]);
        assert_eq!(manifest.jobs["override"].timeout_seconds, Some(30));
        assert_eq!(manifest.jobs["override"].tags, vec!["own"]);
    }

    #[test]
    fn unknown_top_level_key_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(dir.path(), "name: x\nbogus: 1\njobs: {}\n");

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "manifest");
        assert!(
            issues[0].message.contains("unknown field"),
            "got: {}",
            issues[0].message
        );
        assert!(
            issues[0].message.contains("bogus"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn unknown_job_key_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: every 1h
    run: echo hi
    interval: 5m
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "manifest");
        assert!(
            issues[0].message.contains("unknown field"),
            "got: {}",
            issues[0].message
        );
        assert!(
            issues[0].message.contains("interval"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn two_actions_is_an_issue() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: every 1h
    run: echo hi
    prompt: also this
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.j1");
        assert_eq!(
            issues[0].message,
            "exactly one action required: run, prompt, or webhook"
        );
    }

    #[test]
    fn agent_without_prompt_is_an_issue() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: every 1h
    run: echo hi
    agent: claude
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.j1");
        assert_eq!(issues[0].message, "agent can only be used with prompt");
    }

    #[test]
    fn method_without_webhook_is_an_issue() {
        // Stricter than the CLI, which silently ignores --method on
        // non-webhook jobs: declarative files deny surprises.
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: every 1h
    run: echo hi
    method: GET
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.j1");
        assert_eq!(issues[0].message, "method can only be used with webhook");
    }

    #[test]
    fn missing_env_var_is_an_issue() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  broken:
    schedule: every 1h
    run: echo ${MISSING_VAR}
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.broken");
        assert_eq!(
            issues[0].message,
            "undefined environment variable 'MISSING_VAR'"
        );
    }

    #[test]
    fn issues_collected_across_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  bad-schedule:
    schedule: whenever
    run: echo hi
  two-actions:
    schedule: every 1h
    run: echo hi
    prompt: also this
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|i| i.context == "jobs.bad-schedule"));
        assert!(issues.iter().any(|i| i.context == "jobs.two-actions"));
    }

    #[test]
    fn dollar_brace_escape_reaches_command_literally() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: every 1h
    run: 'echo $${HOME}'
",
        );

        let manifest = load_manifest(&path, &no_env).unwrap();
        match &manifest.jobs["j1"].action {
            Action::Run { command, .. } => assert_eq!(command, "echo ${HOME}"),
            other => panic!("expected run action, got {other:?}"),
        }
    }

    #[test]
    fn invalid_schedule_preserves_parser_wording() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: x
jobs:
  j1:
    schedule: whenever
    run: echo hi
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.j1");
        assert!(
            issues[0].message.contains("Could not parse schedule"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn invalid_explicit_manifest_name_is_an_issue() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_manifest(
            dir.path(),
            "
name: 'bad name!'
jobs:
  j1:
    schedule: every 1h
    run: echo hi
",
        );

        let issues = load_manifest(&path, &no_env).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "manifest");
        assert!(
            issues[0]
                .message
                .contains("invalid manifest name 'bad name!'"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn manifest_name_derived_from_directory_is_sanitized() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("My App!");
        std::fs::create_dir(&project).unwrap();
        let path = write_manifest(
            &project,
            "
jobs:
  j1:
    schedule: every 1h
    run: echo hi
",
        );

        let manifest = load_manifest(&path, &no_env).unwrap();
        assert_eq!(manifest.name, "My-App-");
    }

    #[test]
    fn derived_name_strips_leading_non_alphanumerics() {
        // `.config` must not derive a hidden state filename, and `-foo`
        // must not derive a flag-lookalike — same leading-char rule as
        // explicit names.
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join(".config");
        std::fs::create_dir(&project).unwrap();
        let path = write_manifest(
            &project,
            "
jobs:
  j1:
    schedule: every 1h
    run: echo hi
",
        );

        let manifest = load_manifest(&path, &no_env).unwrap();
        assert_eq!(manifest.name, "config");
    }
}
