//! `schedx down` — remove the jobs a declarative manifest owns.
//!
//! Works even after the yaml is deleted: the manifest is resolved from
//! the file's `name:` when it exists, the file's directory when it
//! doesn't (compose-style), or `--manifest <name>` directly. Removal is
//! the owned set only — state-file entries validated against live
//! markers, plus marker-only jobs the record missed (self-heal).

use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::engine::lock::FileLock;
use crate::manifest::parse::derive_name_from_dir;
use crate::manifest::plan::resolve_owned;
use crate::manifest::state::{delete_manifest_state, load_manifest_state};
use crate::store::paths;
use crate::store::state;

#[derive(Debug, Serialize)]
struct RemovedJob {
    name: String,
    id: String,
    kind: String,
    schedule: String,
}

#[derive(Debug, Serialize)]
struct DownReport {
    manifest: String,
    dry_run: bool,
    removed: Vec<RemovedJob>,
    warnings: Vec<String>,
}

/// Permissive name-only read of a yaml file — `down` must work on a
/// manifest that no longer validates (that can be exactly why it's being
/// brought down).
#[derive(Debug, Deserialize)]
struct NameOnly {
    name: Option<String>,
}

pub fn execute(
    file: &Path,
    manifest_arg: Option<&str>,
    dry_run: bool,
    force: bool,
    json_output: bool,
) -> Result<()> {
    paths::ensure_dirs()?;

    let _lock = if dry_run {
        None
    } else {
        Some(FileLock::state()?)
    };

    let (name, explicit_target) = resolve_target(file, manifest_arg)?;

    let prior = load_manifest_state(&name)?;

    // Cross-prune guard, mirrored from `up`: a wrong-directory `down` is
    // equally destructive. `--manifest` names the exact target and skips
    // the path comparison by design.
    if !explicit_target && !force {
        if let Some(prior) = &prior {
            let matches_stored = if file.exists() {
                file.canonicalize()
                    .map(|p| p == prior.manifest_path)
                    .unwrap_or(false)
            } else {
                // Yaml already deleted: compare the directory we're
                // operating from with where the manifest lived.
                file.parent()
                    .map(|d| {
                        if d.as_os_str().is_empty() {
                            Path::new(".")
                        } else {
                            d
                        }
                    })
                    .and_then(|d| d.canonicalize().ok())
                    .is_some_and(|d| Some(d.as_path()) == prior.manifest_path.parent())
            };
            if !matches_stored {
                bail!(
                    "Error: manifest '{}' is already in use by {}.\n\
                     If you mean that manifest, re-run with --manifest '{}' or from its directory.\n\
                     If you moved the manifest file, re-run with --force to accept the new path.",
                    name,
                    prior.manifest_path.display(),
                    name
                );
            }
        }
    }

    let live = state::load_state()?;
    let owned = resolve_owned(&name, prior.as_ref(), &live);

    if owned.is_empty() && prior.is_none() {
        if manifest_arg.is_some() {
            bail!("Error: manifest '{name}' not found.");
        }
        println!("Nothing to bring down for manifest '{name}'.");
        return Ok(());
    }

    let mut warnings = Vec::new();
    let removed: Vec<RemovedJob> = owned
        .iter()
        .filter_map(|(job_name, job_id)| {
            live.jobs.get(job_id).map(|job| {
                if let Some(flight) = &job.in_flight {
                    warnings.push(format!(
                        "job '{job_name}' ({job_id}) has run {} in flight; removing anyway",
                        flight.run_id
                    ));
                }
                RemovedJob {
                    name: job_name.clone(),
                    id: job_id.clone(),
                    kind: job.action.kind_str().to_string(),
                    schedule: job.schedule_input.clone(),
                }
            })
        })
        .collect();

    if !dry_run {
        if !removed.is_empty() {
            state::update_state(|s| {
                for job in &removed {
                    s.jobs.remove(&job.id);
                }
                Ok(())
            })?;
        }
        delete_manifest_state(&name)?;
    }

    let report = DownReport {
        manifest: name,
        dry_run,
        removed,
        warnings,
    };
    print_report(&report, json_output);
    Ok(())
}

/// Resolve which manifest to bring down. Returns the name and whether it
/// was targeted explicitly via `--manifest`.
fn resolve_target(file: &Path, manifest_arg: Option<&str>) -> Result<(String, bool)> {
    if let Some(name) = manifest_arg {
        return Ok((name.to_string(), true));
    }

    if file.exists() {
        let text = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("Error: failed to read {}: {e}", file.display()))?;
        let parsed: NameOnly = serde_norway::from_str(&text).unwrap_or(NameOnly { name: None });
        if let Some(name) = parsed.name {
            return Ok((name, false));
        }
        let canonical = file
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Error: failed to resolve {}: {e}", file.display()))?;
        return derived_or_error(&canonical);
    }

    // Yaml gone: derive from the directory the file argument points into,
    // compose-style, so `schedx down` works from the manifest's directory.
    let dir = file.parent().map_or(Path::new("."), |d| {
        if d.as_os_str().is_empty() {
            Path::new(".")
        } else {
            d
        }
    });
    let canonical_dir = dir
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Error: failed to resolve {}: {e}", dir.display()))?;
    derived_or_error(&canonical_dir.join("schedx.yaml"))
}

fn derived_or_error(manifest_path: &Path) -> Result<(String, bool)> {
    let name = derive_name_from_dir(manifest_path);
    if name.is_empty() {
        bail!(
            "Error: cannot derive a manifest name from {}; use --manifest <name>.",
            manifest_path.display()
        );
    }
    Ok((name, false))
}

fn print_report(report: &DownReport, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(report) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("Error: failed to serialize report: {e}"),
        }
        return;
    }

    let verb = if report.dry_run {
        "would remove"
    } else {
        "removed"
    };
    for j in &report.removed {
        println!("- {verb} {} [{}] ({})", j.name, j.kind, j.id);
    }
    for w in &report.warnings {
        eprintln!("Warning: {w}");
    }
    if report.removed.is_empty() {
        println!("Nothing to bring down for manifest '{}'.", report.manifest);
    } else if report.dry_run {
        println!(
            "Plan for manifest '{}' (dry run): {} job(s) would be removed.",
            report.manifest,
            report.removed.len()
        );
    } else {
        println!(
            "Brought down manifest '{}': {} job(s) removed.",
            report.manifest,
            report.removed.len()
        );
    }
}
