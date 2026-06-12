//! `schedx up` — reconcile a declarative manifest against the job store.
//!
//! Lock discipline: yaml parsing/validation/env-expansion happen before
//! the lock (no store dependency); the live read, plan, and apply all
//! happen under the ONE state lock, so the plan can never act on phantom
//! state. `--dry-run` reads lock-free (advisory snapshot by design).

use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::Serialize;

use crate::backend;
use crate::engine::lock::FileLock;
use crate::manifest::parse::{ManifestIssue, load_manifest};
use crate::manifest::plan::{Plan, PlanAction, UpdateReason, compute_plan};
use crate::manifest::state::{
    AppliedJob, MANIFEST_STATE_SCHEMA_VERSION, ManifestState, load_manifest_state,
    save_manifest_state,
};
use crate::manifest::{JobSpec, Manifest};
use crate::model::job::{Job, JobState, JobStatus};
use crate::schedule::parser::parse_schedule;
use crate::store::config::load_config;
use crate::store::paths;
use crate::store::state;
use crate::util::id::new_job_id;

/// One job in the `--json` report.
#[derive(Debug, Serialize)]
struct JobRef {
    name: String,
    /// `None` for creations in a dry run (no id generated).
    id: Option<String>,
    kind: String,
    schedule: String,
}

#[derive(Debug, Serialize)]
struct UpReport {
    manifest: String,
    file: String,
    dry_run: bool,
    created: Vec<JobRef>,
    updated: Vec<JobRef>,
    recreated: Vec<JobRef>,
    removed: Vec<JobRef>,
    drift_corrected: Vec<JobRef>,
    unchanged: Vec<JobRef>,
    warnings: Vec<String>,
}

pub fn execute(file: &Path, dry_run: bool, force: bool, json_output: bool) -> Result<()> {
    let lookup = |name: &str| std::env::var(name).ok();
    let manifest = load_manifest(file, &lookup)
        .map_err(|issues| issues_error(&format!("Invalid manifest {}", file.display()), &issues))?;

    paths::ensure_dirs()?;

    // Apply holds the state lock across read + plan + apply; a dry run is
    // an advisory snapshot and deliberately does not contend.
    let _lock = if dry_run {
        None
    } else {
        Some(FileLock::state()?)
    };

    let prior = load_manifest_state(&manifest.name)?;
    if let Some(prior) = &prior {
        if prior.manifest_path != manifest.path && !force {
            bail!(
                "Error: manifest '{}' is already in use by {}.\n\
                 If this is a different set of schedules, give this manifest an explicit unique 'name:'.\n\
                 If you moved the manifest file, re-run with --force to accept the new path.",
                manifest.name,
                prior.manifest_path.display()
            );
        }
    }

    let live = state::load_state()?;
    let config = load_config()?;
    let now = Utc::now();
    let plan = compute_plan(
        &manifest,
        prior.as_ref(),
        &live,
        config.default_timeout_seconds,
        now,
        force,
    )
    .map_err(|issues| issues_error(&format!("Cannot apply {}", file.display()), &issues))?;

    if dry_run {
        let report = build_report(&manifest, file, &plan, &live, true, None);
        print_report(&report, &plan, json_output);
        return Ok(());
    }

    // Generate ids up front so the store mutation and the manifest state
    // file record the same ids.
    let ids: Vec<(String, String)> = plan
        .actions
        .iter()
        .filter_map(|a| match a {
            PlanAction::Create { name, .. } | PlanAction::Recreate { name, .. } => {
                Some((name.clone(), new_job_id()))
            }
            _ => None,
        })
        .collect();
    let new_id_for = |name: &str| -> String {
        ids.iter()
            .find(|(n, _)| n == name)
            .map(|(_, id)| id.clone())
            .expect("id generated for every create/recreate")
    };

    if !plan.is_noop() {
        state::update_state(|s| {
            for action in &plan.actions {
                apply_action(
                    s,
                    action,
                    &manifest,
                    &new_id_for,
                    config.default_timeout_seconds,
                    now,
                )?;
            }
            Ok(())
        })?;
    }

    // Record the applied state (still under the lock; markers on the jobs
    // themselves let reconciliation self-heal if we crash before this).
    save_manifest_state(&build_manifest_state(&manifest, &plan, &new_id_for))?;

    // One dispatcher check for the whole reconcile, not one per job.
    if !plan.is_noop() {
        if let Ok(be) = backend::detect_backend() {
            let _ = be.ensure_dispatcher();
        }
    }

    let report = build_report(&manifest, file, &plan, &live, false, Some(&new_id_for));
    print_report(&report, &plan, json_output);
    Ok(())
}

/// The post-apply state record: every declared job with the id it
/// resolved to and the spec that was just applied.
fn build_manifest_state(
    manifest: &Manifest,
    plan: &Plan,
    new_id_for: &dyn Fn(&str) -> String,
) -> ManifestState {
    let applied_at = Utc::now();
    ManifestState {
        schema_version: MANIFEST_STATE_SCHEMA_VERSION,
        manifest_name: manifest.name.clone(),
        manifest_path: manifest.path.clone(),
        applied_at,
        jobs: manifest
            .jobs
            .iter()
            .map(|(name, spec)| {
                let job_id = plan
                    .actions
                    .iter()
                    .find_map(|a| match a {
                        PlanAction::Update {
                            name: n, job_id, ..
                        }
                        | PlanAction::Unchanged { name: n, job_id }
                            if n == name =>
                        {
                            Some(job_id.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| new_id_for(name));
                (
                    name.clone(),
                    AppliedJob {
                        job_id,
                        applied_spec: spec.clone(),
                        applied_at,
                    },
                )
            })
            .collect(),
    }
}

/// Apply one plan action inside the `update_state` closure — the whole
/// plan lands in one atomic `jobs.json` write.
fn apply_action(
    s: &mut JobState,
    action: &PlanAction,
    manifest: &Manifest,
    new_id_for: &dyn Fn(&str) -> String,
    default_timeout_seconds: u64,
    now: chrono::DateTime<Utc>,
) -> Result<()> {
    match action {
        PlanAction::Create { name, spec } => {
            let job = build_job(
                name,
                spec,
                manifest,
                &new_id_for(name),
                default_timeout_seconds,
                now,
            )?;
            s.jobs.insert(job.id.clone(), job);
        }
        PlanAction::Recreate {
            name,
            spec,
            old_job_id,
        } => {
            if let Some(old) = old_job_id {
                s.jobs.remove(old);
            }
            let job = build_job(
                name,
                spec,
                manifest,
                &new_id_for(name),
                default_timeout_seconds,
                now,
            )?;
            s.jobs.insert(job.id.clone(), job);
        }
        PlanAction::Update {
            name,
            job_id,
            spec,
            schedule_changed,
            ..
        } => {
            let job = s
                .jobs
                .get_mut(job_id)
                .with_context(|| format!("job {job_id} disappeared during apply"))?;
            if *schedule_changed {
                let parsed = parse_schedule(&spec.schedule_input, now)?;
                job.schedule_input.clone_from(&spec.schedule_input);
                job.schedule = parsed.to_job_schedule();
                // Avoid a catch-up burst firing the new schedule against
                // old timestamps (same rule as `edit`).
                job.last_scheduled_at = Some(now);
            }
            job.action = spec.action.clone();
            job.timeout_seconds = spec.timeout_seconds.unwrap_or(default_timeout_seconds);
            job.tags.clone_from(&spec.tags);
            job.on_failure.clone_from(&spec.on_failure);
            job.on_failure_shell = spec.on_failure_shell;
            job.name = Some(name.clone());
            job.managed_by = Some(manifest.name.clone());
            job.status = match spec.paused {
                Some(true) => JobStatus::Paused,
                Some(false) => JobStatus::Active,
                // Runtime pause stays the operator's; archived contradicts
                // "declared to exist" and is restored.
                None if job.status == JobStatus::Archived => JobStatus::Active,
                None => job.status,
            };
            job.updated_at = now;
        }
        PlanAction::Remove { job_id, .. } => {
            s.jobs.remove(job_id);
        }
        PlanAction::Unchanged { .. } => {}
    }
    Ok(())
}

/// A fresh job exactly as `add` would create it, plus the ownership marker.
fn build_job(
    name: &str,
    spec: &JobSpec,
    manifest: &Manifest,
    job_id: &str,
    default_timeout_seconds: u64,
    now: chrono::DateTime<Utc>,
) -> Result<Job> {
    let parsed = parse_schedule(&spec.schedule_input, now)?;
    Ok(Job {
        id: job_id.to_string(),
        name: Some(name.to_string()),
        status: match spec.paused {
            Some(true) => JobStatus::Paused,
            _ => JobStatus::Active,
        },
        schedule_input: spec.schedule_input.clone(),
        schedule: parsed.to_job_schedule(),
        action: spec.action.clone(),
        timeout_seconds: spec.timeout_seconds.unwrap_or(default_timeout_seconds),
        tags: spec.tags.clone(),
        created_at: now,
        updated_at: now,
        last_scheduled_at: None,
        last_run: None,
        run_count: 0,
        skip_remaining: 0,
        in_flight: None,
        on_failure: spec.on_failure.clone(),
        on_failure_shell: spec.on_failure_shell,
        completed_at: None,
        consecutive_failures: 0,
        managed_by: Some(manifest.name.clone()),
    })
}

fn issues_error(headline: &str, issues: &[ManifestIssue]) -> anyhow::Error {
    let mut msg = format!("Error: {headline}:");
    for issue in issues {
        msg.push_str(&format!("\n  {}: {}", issue.context, issue.message));
    }
    anyhow::anyhow!(msg)
}

fn job_ref(name: &str, id: Option<String>, spec: &JobSpec) -> JobRef {
    JobRef {
        name: name.to_string(),
        id,
        kind: spec.action.kind_str().to_string(),
        schedule: spec.schedule_input.clone(),
    }
}

fn build_report(
    manifest: &Manifest,
    file: &Path,
    plan: &Plan,
    live: &JobState,
    dry_run: bool,
    new_id_for: Option<&dyn Fn(&str) -> String>,
) -> UpReport {
    let mut report = UpReport {
        manifest: manifest.name.clone(),
        file: file.display().to_string(),
        dry_run,
        created: vec![],
        updated: vec![],
        recreated: vec![],
        removed: vec![],
        drift_corrected: vec![],
        unchanged: vec![],
        warnings: plan.warnings.clone(),
    };

    for action in &plan.actions {
        match action {
            PlanAction::Create { name, spec } => {
                let id = new_id_for.map(|f| f(name));
                report.created.push(job_ref(name, id, spec));
            }
            PlanAction::Recreate { name, spec, .. } => {
                let id = new_id_for.map(|f| f(name));
                report.recreated.push(job_ref(name, id, spec));
            }
            PlanAction::Update {
                name,
                job_id,
                spec,
                reason,
                ..
            } => {
                let entry = job_ref(name, Some(job_id.clone()), spec);
                match reason {
                    UpdateReason::SpecChanged => report.updated.push(entry),
                    UpdateReason::Drift => report.drift_corrected.push(entry),
                }
            }
            PlanAction::Remove { name, job_id } => {
                let (kind, schedule) = live
                    .jobs
                    .get(job_id)
                    .map(|j| (j.action.kind_str().to_string(), j.schedule_input.clone()))
                    .unwrap_or_default();
                report.removed.push(JobRef {
                    name: name.clone(),
                    id: Some(job_id.clone()),
                    kind,
                    schedule,
                });
            }
            PlanAction::Unchanged { name, job_id } => {
                if let Some(spec) = manifest.jobs.get(name) {
                    report
                        .unchanged
                        .push(job_ref(name, Some(job_id.clone()), spec));
                }
            }
        }
    }

    report
}

fn print_report(report: &UpReport, plan: &Plan, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(report) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("Error: failed to serialize report: {e}"),
        }
        return;
    }

    if plan.is_noop() {
        println!(
            "No changes. {} job(s) up to date for manifest '{}'.",
            report.unchanged.len(),
            report.manifest
        );
        return;
    }

    let verb = if report.dry_run { "would " } else { "" };
    for j in &report.created {
        println!("+ {verb}create {} [{}] ({})", j.name, j.kind, id_str(j));
    }
    for j in &report.recreated {
        println!("↻ {verb}recreate {} [{}] ({})", j.name, j.kind, id_str(j));
    }
    for j in &report.updated {
        println!("~ {verb}update {} [{}] ({})", j.name, j.kind, id_str(j));
    }
    for j in &report.drift_corrected {
        println!(
            "! {verb}correct drift on {} [{}] ({})",
            j.name,
            j.kind,
            id_str(j)
        );
    }
    for j in &report.removed {
        println!("- {verb}remove {} [{}] ({})", j.name, j.kind, id_str(j));
    }
    for w in &report.warnings {
        eprintln!("Warning: {w}");
    }
    let summary = format!(
        "{} created, {} updated, {} drift-corrected, {} recreated, {} removed, {} unchanged",
        report.created.len(),
        report.updated.len(),
        report.drift_corrected.len(),
        report.recreated.len(),
        report.removed.len(),
        report.unchanged.len()
    );
    if report.dry_run {
        println!(
            "Plan for manifest '{}' (dry run): {summary}.",
            report.manifest
        );
    } else {
        println!("Applied manifest '{}': {summary}.", report.manifest);
    }
}

fn id_str(j: &JobRef) -> String {
    j.id.clone().unwrap_or_else(|| "new".to_string())
}
