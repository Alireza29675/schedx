//! Pure reconciliation planning: desired manifest vs recorded state vs
//! live store.
//!
//! `compute_plan` has no I/O — callers load everything (under the state
//! lock, for a real apply) and pass it in, which makes the entire
//! reconcile table executable as fast unit tests.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::manifest::parse::ManifestIssue;
use crate::manifest::state::ManifestState;
use crate::manifest::{JobSpec, Manifest};
use crate::model::job::{Job, JobState, JobStatus};
use crate::schedule::parser::parse_schedule;

/// What `up` will do, in apply order.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanAction {
    /// New declared job: insert with a fresh id.
    Create { name: String, spec: JobSpec },
    /// Owned live job differs from desired: mutate in place (id, run
    /// history identity, and counters preserved).
    Update {
        name: String,
        job_id: String,
        spec: JobSpec,
        /// Desired schedule string differs from the live one — apply
        /// resets `last_scheduled_at` to avoid a catch-up burst.
        schedule_changed: bool,
        reason: UpdateReason,
    },
    /// Declared job needs a fresh id: the recorded live job is gone
    /// (imperative `rm`), or a completed one-shot's spec changed
    /// (`old_job_id` to remove first).
    Recreate {
        name: String,
        spec: JobSpec,
        old_job_id: Option<String>,
    },
    /// Owned job no longer declared: orphan prune.
    Remove { name: String, job_id: String },
    /// Desired and live agree.
    Unchanged { name: String, job_id: String },
}

/// Why an owned job is being updated — reporting only, the mutation is
/// identical (desired wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateReason {
    /// The yaml changed since the last apply.
    SpecChanged,
    /// The yaml is unchanged but the live job was edited imperatively.
    Drift,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub actions: Vec<PlanAction>,
    pub warnings: Vec<String>,
}

impl Plan {
    /// True when applying this plan would not touch the store.
    pub fn is_noop(&self) -> bool {
        self.actions
            .iter()
            .all(|a| matches!(a, PlanAction::Unchanged { .. }))
    }
}

/// Compute the reconcile plan. Collision and schedule issues are
/// collected (all of them) and abort the whole plan — nothing partial.
pub fn compute_plan(
    manifest: &Manifest,
    prior: Option<&ManifestState>,
    live: &JobState,
    default_timeout_seconds: u64,
    now: DateTime<Utc>,
) -> Result<Plan, Vec<ManifestIssue>> {
    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // Ownership resolution, belt-and-braces: recorded entries are
    // validated against the live job's marker; live jobs carrying our
    // marker but missing from the record (state file lost or torn between
    // the two writes) are recovered by their job name.
    let owned = resolve_owned(&manifest.name, prior, live);

    let mut actions = Vec::new();

    for (name, spec) in &manifest.jobs {
        if let Some(job_id) = owned.get(name) {
            let job = &live.jobs[job_id];
            if let Some(flight) = &job.in_flight {
                if !matches(spec, job, name, &manifest.name, default_timeout_seconds) {
                    warnings.push(format!(
                        "job '{name}' ({job_id}) has run {} in flight; updating anyway",
                        flight.run_id
                    ));
                }
            }
            actions.push(plan_for_owned(
                name,
                job_id,
                spec,
                job,
                prior,
                &manifest.name,
                default_timeout_seconds,
                now,
                &mut issues,
            ));
        } else if let Some(holder) = live.jobs.values().find(|j| j.name.as_deref() == Some(name)) {
            // Name taken by a job we don't own — never adopt silently.
            let message = match &holder.managed_by {
                Some(other) => format!(
                    "job name '{name}' is managed by manifest '{other}' ({})",
                    holder.id
                ),
                None => format!(
                    "job name '{name}' already exists and is not managed by manifest '{}' ({}); \
                     rename it, `schedx rm` it, or pick another name",
                    manifest.name, holder.id
                ),
            };
            issues.push(ManifestIssue::new(format!("jobs.{name}"), message));
        } else {
            validate_schedule(spec, name, now, &mut issues);
            let was_recorded = prior.is_some_and(|p| p.jobs.contains_key(name));
            if was_recorded {
                // Recorded but gone live (imperative rm): recreate.
                actions.push(PlanAction::Recreate {
                    name: name.clone(),
                    spec: spec.clone(),
                    old_job_id: None,
                });
            } else {
                actions.push(PlanAction::Create {
                    name: name.clone(),
                    spec: spec.clone(),
                });
            }
        }
    }

    // Owned jobs no longer declared: orphan prune.
    for (name, job_id) in &owned {
        if !manifest.jobs.contains_key(name) {
            if let Some(flight) = &live.jobs[job_id].in_flight {
                warnings.push(format!(
                    "job '{name}' ({job_id}) has run {} in flight; removing anyway",
                    flight.run_id
                ));
            }
            actions.push(PlanAction::Remove {
                name: name.clone(),
                job_id: job_id.clone(),
            });
        }
    }

    if issues.is_empty() {
        Ok(Plan { actions, warnings })
    } else {
        Err(issues)
    }
}

/// Declared name -> owned live job id. Belt-and-braces: recorded entries
/// are validated against live markers; marker-only jobs (state file lost
/// or torn) are recovered by job name. Shared with `down`.
pub fn resolve_owned(
    manifest_name: &str,
    prior: Option<&ManifestState>,
    live: &JobState,
) -> BTreeMap<String, String> {
    let mut owned = BTreeMap::new();

    if let Some(prior) = prior {
        for (name, applied) in &prior.jobs {
            if let Some(job) = live.jobs.get(&applied.job_id) {
                if job.managed_by.as_deref() == Some(manifest_name) {
                    owned.insert(name.clone(), applied.job_id.clone());
                }
            }
        }
    }

    // Marker scan recovers ownership the record missed.
    for job in live.jobs.values() {
        if job.managed_by.as_deref() == Some(manifest_name) {
            if let Some(job_name) = &job.name {
                owned.entry(job_name.clone()).or_insert(job.id.clone());
            }
        }
    }

    owned
}

#[allow(clippy::too_many_arguments)]
fn plan_for_owned(
    name: &str,
    job_id: &str,
    spec: &JobSpec,
    job: &Job,
    prior: Option<&ManifestState>,
    manifest_name: &str,
    default_timeout_seconds: u64,
    now: DateTime<Utc>,
    issues: &mut Vec<ManifestIssue>,
) -> PlanAction {
    if matches(spec, job, name, manifest_name, default_timeout_seconds) {
        return PlanAction::Unchanged {
            name: name.to_string(),
            job_id: job_id.to_string(),
        };
    }

    validate_schedule(spec, name, now, issues);

    // A completed one-shot cannot be revived in place (`completed` is
    // terminal); a changed spec means a fresh job. Its run history
    // survives in run-history.jsonl.
    if job.status == JobStatus::Completed {
        return PlanAction::Recreate {
            name: name.to_string(),
            spec: spec.clone(),
            old_job_id: Some(job_id.to_string()),
        };
    }

    let reason = match prior.and_then(|p| p.jobs.get(name)) {
        Some(applied) if applied.applied_spec == *spec => UpdateReason::Drift,
        _ => UpdateReason::SpecChanged,
    };
    PlanAction::Update {
        name: name.to_string(),
        job_id: job_id.to_string(),
        spec: spec.clone(),
        schedule_changed: spec.schedule_input != job.schedule_input,
        reason,
    }
}

/// Does the live job already match the desired spec?
fn matches(
    spec: &JobSpec,
    job: &Job,
    declared_name: &str,
    manifest_name: &str,
    default_timeout_seconds: u64,
) -> bool {
    let status_ok = if job.is_one_shot() && job.status == JobStatus::Completed {
        // A completed one-shot already ran; re-running `up` with an
        // unchanged spec must not re-fire it.
        true
    } else {
        match spec.paused {
            Some(true) => job.status == JobStatus::Paused,
            Some(false) => job.status == JobStatus::Active,
            // Unspecified: runtime pause is the operator's, but archived
            // contradicts "declared to exist".
            None => matches!(job.status, JobStatus::Active | JobStatus::Paused),
        }
    };

    status_ok
        && job.schedule_input == spec.schedule_input
        && job.action == spec.action
        && job.timeout_seconds == spec.timeout_seconds.unwrap_or(default_timeout_seconds)
        && job.tags == spec.tags
        && job.on_failure == spec.on_failure
        && job.on_failure_shell == spec.on_failure_shell
        && job.name.as_deref() == Some(declared_name)
        && job.managed_by.as_deref() == Some(manifest_name)
}

/// Schedules are validated only for jobs the plan will write — an
/// unchanged completed one-shot's past date must not brick the manifest.
fn validate_schedule(
    spec: &JobSpec,
    name: &str,
    now: DateTime<Utc>,
    issues: &mut Vec<ManifestIssue>,
) {
    if let Err(e) = parse_schedule(&spec.schedule_input, now) {
        let message = format!("{e:#}");
        issues.push(ManifestIssue::new(
            format!("jobs.{name}"),
            message.strip_prefix("Error: ").unwrap_or(&message),
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::manifest::state::{AppliedJob, MANIFEST_STATE_SCHEMA_VERSION, ManifestState};
    use crate::model::action::Action;

    const DEFAULT_TIMEOUT: u64 = 300;

    fn spec(schedule: &str, command: &str) -> JobSpec {
        JobSpec {
            schedule_input: schedule.to_string(),
            action: Action::Run {
                command: command.to_string(),
                shell: false,
                workdir: None,
            },
            timeout_seconds: None,
            tags: vec![],
            paused: None,
            on_failure: None,
            on_failure_shell: false,
        }
    }

    /// A live job exactly as `up` would have created it from `spec`.
    fn live_job(id: &str, name: &str, manifest: &str, spec: &JobSpec) -> Job {
        let now = Utc::now();
        Job {
            id: id.to_string(),
            name: Some(name.to_string()),
            status: match spec.paused {
                Some(true) => JobStatus::Paused,
                _ => JobStatus::Active,
            },
            schedule_input: spec.schedule_input.clone(),
            schedule: parse_schedule(&spec.schedule_input, now)
                .map(|p| p.to_job_schedule())
                .unwrap_or(crate::model::schedule::JobSchedule::RecurringInterval {
                    every_seconds: 3600,
                }),
            action: spec.action.clone(),
            timeout_seconds: spec.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT),
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
            managed_by: Some(manifest.to_string()),
        }
    }

    fn manifest(name: &str, jobs: Vec<(&str, JobSpec)>) -> Manifest {
        Manifest {
            name: name.to_string(),
            path: PathBuf::from("/proj/schedx.yaml"),
            jobs: jobs.into_iter().map(|(n, s)| (n.to_string(), s)).collect(),
        }
    }

    fn live(jobs: Vec<Job>) -> JobState {
        JobState {
            schema_version: 2,
            jobs: jobs.into_iter().map(|j| (j.id.clone(), j)).collect(),
        }
    }

    fn recorded(manifest_name: &str, entries: Vec<(&str, &str, JobSpec)>) -> ManifestState {
        let now = Utc::now();
        ManifestState {
            schema_version: MANIFEST_STATE_SCHEMA_VERSION,
            manifest_name: manifest_name.to_string(),
            manifest_path: PathBuf::from("/proj/schedx.yaml"),
            applied_at: now,
            jobs: entries
                .into_iter()
                .map(|(name, id, s)| {
                    (
                        name.to_string(),
                        AppliedJob {
                            job_id: id.to_string(),
                            applied_spec: s,
                            applied_at: now,
                        },
                    )
                })
                .collect(),
        }
    }

    fn plan(
        m: &Manifest,
        prior: Option<&ManifestState>,
        l: &JobState,
    ) -> Result<Plan, Vec<ManifestIssue>> {
        compute_plan(m, prior, l, DEFAULT_TIMEOUT, Utc::now())
    }

    #[test]
    fn empty_store_creates_everything() {
        let m = manifest(
            "demo",
            vec![
                ("a", spec("every 1h", "echo a")),
                ("b", spec("every 2h", "echo b")),
            ],
        );
        let p = plan(&m, None, &live(vec![])).unwrap();
        assert_eq!(p.actions.len(), 2);
        assert!(
            p.actions
                .iter()
                .all(|a| matches!(a, PlanAction::Create { .. }))
        );
    }

    #[test]
    fn second_up_is_a_noop() {
        let s = spec("every 1h", "echo a");
        let job = live_job("id1", "a", "demo", &s);
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert!(p.is_noop(), "expected noop, got {:?}", p.actions);
    }

    #[test]
    fn yaml_change_updates_in_place_as_spec_changed() {
        let applied = spec("every 1h", "echo a");
        let job = live_job("id1", "a", "demo", &applied);
        let desired = spec("every 30m", "echo a");
        let m = manifest("demo", vec![("a", desired)]);
        let prior = recorded("demo", vec![("a", "id1", applied)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert_eq!(p.actions.len(), 1);
        match &p.actions[0] {
            PlanAction::Update {
                job_id,
                schedule_changed,
                reason,
                ..
            } => {
                assert_eq!(job_id, "id1");
                assert!(schedule_changed);
                assert_eq!(*reason, UpdateReason::SpecChanged);
            }
            other => panic!("expected update, got {other:?}"),
        }
    }

    #[test]
    fn imperative_edit_is_drift() {
        let applied = spec("every 1h", "echo a");
        let mut job = live_job("id1", "a", "demo", &applied);
        // Imperative `edit` changed the command after the last apply.
        job.action = Action::Run {
            command: "echo TAMPERED".to_string(),
            shell: false,
            workdir: None,
        };
        let m = manifest("demo", vec![("a", applied.clone())]);
        let prior = recorded("demo", vec![("a", "id1", applied)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        match &p.actions[0] {
            PlanAction::Update {
                reason,
                schedule_changed,
                ..
            } => {
                assert_eq!(*reason, UpdateReason::Drift);
                assert!(!schedule_changed);
            }
            other => panic!("expected drift update, got {other:?}"),
        }
    }

    #[test]
    fn recorded_but_imperatively_removed_is_recreated() {
        let s = spec("every 1h", "echo a");
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        let p = plan(&m, Some(&prior), &live(vec![])).unwrap();
        assert_eq!(
            p.actions,
            vec![PlanAction::Recreate {
                name: "a".to_string(),
                spec: spec("every 1h", "echo a"),
                old_job_id: None
            }]
        );
    }

    #[test]
    fn undeclared_owned_job_is_pruned() {
        let s = spec("every 1h", "echo a");
        let gone = spec("every 2h", "echo gone");
        let keep = live_job("id1", "a", "demo", &s);
        let orphan = live_job("id2", "old", "demo", &gone);
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s), ("old", "id2", gone)]);
        let p = plan(&m, Some(&prior), &live(vec![keep, orphan])).unwrap();
        assert!(p.actions.contains(&PlanAction::Remove {
            name: "old".to_string(),
            job_id: "id2".to_string()
        }));
        assert!(p.actions.contains(&PlanAction::Unchanged {
            name: "a".to_string(),
            job_id: "id1".to_string()
        }));
    }

    #[test]
    fn unmanaged_jobs_are_never_touched() {
        let s = spec("every 1h", "echo a");
        let mut foreign = live_job("idF", "theirs", "x", &spec("every 5m", "echo f"));
        foreign.managed_by = None;
        let m = manifest("demo", vec![("a", s)]);
        let p = plan(&m, None, &live(vec![foreign])).unwrap();
        assert!(
            !p.actions
                .iter()
                .any(|a| matches!(a, PlanAction::Remove { job_id, .. } if job_id == "idF")),
            "unmanaged job must never be pruned"
        );
        assert_eq!(p.actions.len(), 1); // just the create
    }

    #[test]
    fn unmanaged_name_collision_is_a_hard_error() {
        let mut holder = live_job("idF", "a", "x", &spec("every 5m", "echo f"));
        holder.managed_by = None;
        let m = manifest("demo", vec![("a", spec("every 1h", "echo a"))]);
        let issues = plan(&m, None, &live(vec![holder])).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].context, "jobs.a");
        assert!(
            issues[0]
                .message
                .contains("already exists and is not managed by manifest 'demo' (idF)"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn cross_manifest_collision_names_the_owner() {
        let holder = live_job("idO", "a", "other", &spec("every 5m", "echo o"));
        let m = manifest("demo", vec![("a", spec("every 1h", "echo a"))]);
        let issues = plan(&m, None, &live(vec![holder])).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0]
                .message
                .contains("is managed by manifest 'other' (idO)"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn completed_one_shot_with_unchanged_spec_is_unchanged() {
        let s = spec("2020-01-01T00:00:00Z", "echo once");
        let mut job = live_job("id1", "a", "demo", &s);
        job.status = JobStatus::Completed;
        job.schedule = crate::model::schedule::JobSchedule::OneShot {
            fire_at: Utc::now(),
        };
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        // The past ISO date must NOT brick the manifest (validated only
        // for jobs being written; this one is unchanged).
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert!(p.is_noop(), "got {:?}", p.actions);
    }

    #[test]
    fn completed_one_shot_with_changed_spec_is_recreated() {
        let applied = spec("2020-01-01T00:00:00Z", "echo once");
        let mut job = live_job("id1", "a", "demo", &applied);
        job.status = JobStatus::Completed;
        job.schedule = crate::model::schedule::JobSchedule::OneShot {
            fire_at: Utc::now(),
        };
        let desired = spec("every 1h", "echo once");
        let m = manifest("demo", vec![("a", desired.clone())]);
        let prior = recorded("demo", vec![("a", "id1", applied)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert_eq!(
            p.actions,
            vec![PlanAction::Recreate {
                name: "a".to_string(),
                spec: desired,
                old_job_id: Some("id1".to_string())
            }]
        );
    }

    #[test]
    fn invalid_schedule_on_a_new_job_is_an_issue() {
        let m = manifest("demo", vec![("a", spec("whenever", "echo a"))]);
        let issues = plan(&m, None, &live(vec![])).unwrap_err();
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0].message.contains("Could not parse schedule"),
            "got: {}",
            issues[0].message
        );
    }

    #[test]
    fn paused_true_on_active_job_updates() {
        let mut s = spec("every 1h", "echo a");
        let job = live_job("id1", "a", "demo", &s); // active
        s.paused = Some(true);
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert!(
            matches!(&p.actions[0], PlanAction::Update { .. }),
            "got {:?}",
            p.actions
        );
    }

    #[test]
    fn unspecified_pause_leaves_runtime_pause_alone() {
        let s = spec("every 1h", "echo a"); // paused: None
        let mut job = live_job("id1", "a", "demo", &s);
        job.status = JobStatus::Paused; // operator paused it imperatively
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert!(
            p.is_noop(),
            "runtime pause must not be drift: {:?}",
            p.actions
        );
    }

    #[test]
    fn archived_managed_job_is_drift_even_without_paused() {
        let s = spec("every 1h", "echo a");
        let mut job = live_job("id1", "a", "demo", &s);
        job.status = JobStatus::Archived;
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        let p = plan(&m, Some(&prior), &live(vec![job])).unwrap();
        assert!(
            matches!(
                &p.actions[0],
                PlanAction::Update {
                    reason: UpdateReason::Drift,
                    ..
                }
            ),
            "got {:?}",
            p.actions
        );
    }

    #[test]
    fn lost_state_file_recovers_ownership_from_markers() {
        let s = spec("every 1h", "echo a");
        let job = live_job("id1", "a", "demo", &s);
        let m = manifest("demo", vec![("a", s)]);
        // No prior state at all — markers alone must resolve ownership.
        let p = plan(&m, None, &live(vec![job])).unwrap();
        assert_eq!(
            p.actions,
            vec![PlanAction::Unchanged {
                name: "a".to_string(),
                job_id: "id1".to_string()
            }]
        );
    }

    #[test]
    fn timeout_default_resolution_round_trips() {
        let s = spec("every 1h", "echo a"); // timeout None -> default
        let job = live_job("id1", "a", "demo", &s);
        assert_eq!(job.timeout_seconds, DEFAULT_TIMEOUT);
        let m = manifest("demo", vec![("a", s.clone())]);
        let prior = recorded("demo", vec![("a", "id1", s)]);
        assert!(plan(&m, Some(&prior), &live(vec![job])).unwrap().is_noop());
    }

    #[test]
    fn in_flight_update_and_remove_warn() {
        let s = spec("every 1h", "echo a");
        let gone = spec("every 2h", "echo old");
        let mut updating = live_job("id1", "a", "demo", &s);
        updating.in_flight = Some(crate::model::job::InFlightRun {
            run_id: "r1".to_string(),
            scheduled_for: Utc::now(),
            claimed_at: Utc::now(),
        });
        let mut removing = live_job("id2", "old", "demo", &gone);
        removing.in_flight = Some(crate::model::job::InFlightRun {
            run_id: "r2".to_string(),
            scheduled_for: Utc::now(),
            claimed_at: Utc::now(),
        });
        let desired = spec("every 30m", "echo a");
        let m = manifest("demo", vec![("a", desired)]);
        let prior = recorded("demo", vec![("a", "id1", s), ("old", "id2", gone)]);
        let p = plan(&m, Some(&prior), &live(vec![updating, removing])).unwrap();
        assert_eq!(p.warnings.len(), 2, "got {:?}", p.warnings);
        assert!(
            p.warnings
                .iter()
                .any(|w| w.contains("r1") && w.contains("updating anyway"))
        );
        assert!(
            p.warnings
                .iter()
                .any(|w| w.contains("r2") && w.contains("removing anyway"))
        );
    }
}
