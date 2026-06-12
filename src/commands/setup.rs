use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};

/// Version embedded in installed skill files for tracking.
const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");

// The one canonical skill, embedded at compile time. Every agent gets the same
// SKILL.md + reference.md — the converged 2026 format they all read natively.
const SKILL_MD: &str = include_str!("../../skills/schedx/SKILL.md");
const REFERENCE_MD: &str = include_str!("../../skills/schedx/reference.md");

/// The five known agent targets for skill installation.
const KNOWN_AGENTS: &[&str] = &["claude", "codex", "cursor", "gemini", "opencode"];

#[allow(clippy::fn_params_excessive_bools)]
pub fn execute(
    agent: Option<&str>,
    all: bool,
    force: bool,
    dry_run: bool,
    list: bool,
    json_output: bool,
) -> Result<()> {
    if list {
        return list_skills(json_output);
    }

    let opts = InstallOpts {
        agent,
        all,
        force,
        dry_run,
        json_output,
    };
    install(&opts)?;

    // After installing skills, also register detected agent profiles.
    if !dry_run {
        if !json_output {
            println!();
        }
        super::agent::detect_agents(force, json_output)?;
    }

    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
struct InstallOpts<'a> {
    agent: Option<&'a str>,
    all: bool,
    force: bool,
    dry_run: bool,
    json_output: bool,
}

/// Resolve which agents to install for, then write skill files.
fn install(opts: &InstallOpts<'_>) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;

    let targets = resolve_targets(opts)?;
    if targets.is_empty() {
        return Ok(());
    }

    let mut results: Vec<SkillResult> = targets
        .iter()
        .map(|&a| install_for_agent(a, &home, opts.force, opts.dry_run))
        .collect();

    // Also report undetected agents when not targeting a specific one.
    if opts.agent.is_none() {
        for &name in KNOWN_AGENTS {
            if !targets.contains(&name) {
                results.push(SkillResult {
                    agent: name.to_string(),
                    path: None,
                    status: "not_detected".to_string(),
                });
            }
        }
    }

    if opts.json_output {
        print_results_json(&results)?;
    } else {
        print_results_human(&results, opts.dry_run, &home);
    }

    Ok(())
}

fn resolve_targets<'a>(opts: &InstallOpts<'a>) -> Result<Vec<&'a str>> {
    if let Some(name) = opts.agent {
        if !KNOWN_AGENTS.contains(&name) {
            bail!(
                "Unknown agent '{name}'. Valid agents: {}",
                KNOWN_AGENTS.join(", ")
            );
        }
        return Ok(vec![name]);
    }

    // --all installs for every supported agent, detected or not (so you can
    // pre-seed agents you haven't installed yet).
    if opts.all {
        return Ok(KNOWN_AGENTS.to_vec());
    }

    let detected: Vec<&str> = KNOWN_AGENTS
        .iter()
        .filter(|a| is_agent_detected(a))
        .copied()
        .collect();

    if detected.is_empty() {
        if opts.json_output {
            println!("[]");
        } else {
            println!("No supported agents detected.");
            println!("Install one of: Claude Code, Codex, Cursor, Gemini CLI, or OpenCode.");
            println!("Or specify an agent directly: schedx setup --agent claude");
            println!("Or install for all supported agents: schedx setup --all");
        }
    }

    Ok(detected)
}

fn print_results_json(results: &[SkillResult]) -> Result<()> {
    let json: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "agent": r.agent,
                "path": r.path,
                "status": r.status,
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn print_results_human(results: &[SkillResult], dry_run: bool, home: &Path) {
    let action = if dry_run {
        "Would install"
    } else {
        "Installing"
    };
    println!("{action} schedx skills...");

    for r in results {
        let icon = match r.status.as_str() {
            "installed" | "would_install" => "+",
            "already_installed" => "~",
            "not_detected" => "-",
            _ => "!",
        };
        let path_str = r
            .path
            .as_deref()
            .map(|p| format!("  {p}"))
            .unwrap_or_default();
        let status_label = match r.status.as_str() {
            "installed" => "installed",
            "would_install" => "would install",
            "already_installed" => "already installed",
            "not_detected" => "not detected",
            _ => &r.status,
        };
        println!("  {icon} {:<12} [{status_label}]{path_str}", r.agent);
    }

    let installed_count = results
        .iter()
        .filter(|r| r.status == "installed" || r.status == "would_install")
        .count();
    if installed_count > 0 && !dry_run {
        println!();
        println!("{installed_count} skill(s) installed. Your agents now know how to use schedx.");
        println!("Skills are version-matched to schedx v{SKILL_VERSION}.");
    }

    // Post-install notes for agents that need manual config.
    if !dry_run {
        for r in results {
            if r.status == "installed" || r.status == "already_installed" {
                print_post_install_note(&r.agent, r.path.as_deref(), home);
            }
        }
    }
}

fn install_for_agent(agent: &str, home: &Path, force: bool, dry_run: bool) -> SkillResult {
    let (target_files, target_dir) = skill_target(agent, home);

    let primary = target_dir.join(target_files[0].0);
    if primary.exists() && !force {
        return SkillResult {
            agent: agent.to_string(),
            path: Some(display_path(&primary, home)),
            status: "already_installed".to_string(),
        };
    }

    if dry_run {
        return SkillResult {
            agent: agent.to_string(),
            path: Some(display_path(&primary, home)),
            status: "would_install".to_string(),
        };
    }

    if let Err(e) = fs::create_dir_all(&target_dir) {
        return SkillResult {
            agent: agent.to_string(),
            path: None,
            status: format!("error: {e}"),
        };
    }

    for (filename, content) in &target_files {
        let path = target_dir.join(filename);
        // Append (don't prepend) the version marker: SKILL.md leads with YAML
        // frontmatter that the native skill format requires at the very top, so
        // a comment before it would hide the skill's name/description.
        let versioned = format!("{content}\n<!-- schedx-skill v{SKILL_VERSION} -->\n");
        if let Err(e) = fs::write(&path, versioned) {
            return SkillResult {
                agent: agent.to_string(),
                path: Some(display_path(&path, home)),
                status: format!("error: {e}"),
            };
        }
    }

    SkillResult {
        agent: agent.to_string(),
        path: Some(display_path(&primary, home)),
        status: "installed".to_string(),
    }
}

/// Return `(files_to_write, target_directory)` for each agent.
/// Where each agent reads a native `SKILL.md` skill folder, as of 2026.
///
/// The landscape converged on one Anthropic-style `SKILL.md` directory:
/// - Claude Code reads `~/.claude/skills/`.
/// - Codex, Gemini CLI, and opencode all read the shared `~/.agents/skills/`
///   path (Gemini gives it precedence over `~/.gemini/skills/`).
/// - Cursor has no global skills dir; it reads `.cursor/skills/` per project,
///   so we install into the current working directory.
///
/// Every target gets the same two files — no per-agent variants to drift.
fn skill_target(agent: &str, home: &Path) -> (Vec<(&'static str, &'static str)>, PathBuf) {
    let files = vec![("SKILL.md", SKILL_MD), ("reference.md", REFERENCE_MD)];
    let dir = match agent {
        "claude" => home.join(".claude/skills/schedx"),
        "codex" | "gemini" | "opencode" => home.join(".agents/skills/schedx"),
        // Cursor reads skills per project — install into the current directory,
        // resolved to an absolute path so the report shows where it landed.
        "cursor" => std::env::current_dir()
            .unwrap_or_default()
            .join(".cursor/skills/schedx"),
        _ => return (vec![], home.to_path_buf()),
    };
    (files, dir)
}

/// Check if an agent binary is available on `PATH`.
fn is_agent_detected(agent: &str) -> bool {
    match agent {
        "claude" | "codex" | "gemini" | "opencode" => crate::util::detect::is_binary_on_path(agent),
        "cursor" => cursor_detected(),
        _ => false,
    }
}

/// Cursor doesn't have a single CLI binary to check.
/// Check for the `~/.cursor` directory or the `cursor` binary.
fn cursor_detected() -> bool {
    if let Some(home) = dirs::home_dir() {
        if home.join(".cursor").exists() {
            return true;
        }
    }
    Command::new("which")
        .arg("cursor")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn display_path(path: &Path, home: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(home) {
        format!("~/{}", rel.display())
    } else {
        path.display().to_string()
    }
}

fn print_post_install_note(agent: &str, path: Option<&str>, _home: &Path) {
    // Claude, Codex, Gemini CLI, and opencode all auto-discover the native
    // SKILL.md from their skills directory — no config edit, no note needed.
    // Cursor is the exception: it has no global skills dir, so the skill lands
    // in the current project and must be installed per project.
    if agent == "cursor" {
        if let Some(p) = path {
            println!();
            println!("Note for Cursor:");
            println!("  Installed into this project at {p}.");
            println!("  Cursor reads skills per project — run `schedx setup --agent cursor`");
            println!("  in each project where you want schedx available.");
        }
    }
}

/// List installed skills and their versions.
fn list_skills(json_output: bool) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;

    let mut entries: Vec<SkillResult> = Vec::new();

    for &agent in KNOWN_AGENTS {
        let (files, dir) = skill_target(agent, &home);
        if files.is_empty() {
            continue;
        }
        let primary = dir.join(files[0].0);
        if primary.exists() {
            let version = read_skill_version(&primary);
            let outdated = version.as_deref() != Some(SKILL_VERSION);
            let status = if outdated {
                format!(
                    "installed (v{}, current: v{SKILL_VERSION})",
                    version.as_deref().unwrap_or("unknown")
                )
            } else {
                format!("installed (v{SKILL_VERSION})")
            };
            entries.push(SkillResult {
                agent: agent.to_string(),
                path: Some(display_path(&primary, &home)),
                status,
            });
        } else {
            entries.push(SkillResult {
                agent: agent.to_string(),
                path: None,
                status: "not installed".to_string(),
            });
        }
    }

    if json_output {
        print_results_json(&entries)?;
    } else {
        println!("{:<12} {:<40} Path", "Agent", "Status");
        for r in &entries {
            println!(
                "{:<12} {:<40} {}",
                r.agent,
                r.status,
                r.path.as_deref().unwrap_or("-")
            );
        }
    }

    Ok(())
}

fn read_skill_version(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let prefix = "<!-- schedx-skill v";
    let start = content.find(prefix)? + prefix.len();
    let end = content[start..].find(" -->")?;
    Some(content[start..start + end].to_string())
}

struct SkillResult {
    agent: String,
    path: Option<String>,
    status: String,
}
