use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};

use crate::cli::SkillCommands;

/// Version embedded in installed skill files for tracking.
const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");

// Skill content embedded at compile time.
const SKILL_MD: &str = include_str!("../../skills/SKILL.md");
const REFERENCE_MD: &str = include_str!("../../skills/reference.md");
const CURSOR_MDC: &str = include_str!("../../skills/cursor.mdc");
const GEMINI_MD: &str = include_str!("../../skills/gemini.md");
const OPENCODE_MD: &str = include_str!("../../skills/opencode.md");

/// The five known agent targets for skill installation.
const KNOWN_AGENTS: &[&str] = &["claude", "codex", "cursor", "gemini", "opencode"];

pub fn execute(command: &SkillCommands, json_output: bool) -> Result<()> {
    match command {
        SkillCommands::Install {
            agent,
            all,
            force,
            dry_run,
        } => {
            let opts = InstallOpts {
                agent: agent.as_deref(),
                all: *all,
                force: *force,
                dry_run: *dry_run,
                json_output,
            };
            install(&opts)
        }
        SkillCommands::List => list_skills(json_output),
    }
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
        print_results_human(&results, opts.dry_run);
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

    let detected: Vec<&str> = KNOWN_AGENTS
        .iter()
        .filter(|a| is_agent_detected(a))
        .copied()
        .collect();

    if detected.is_empty() && !opts.all {
        if opts.json_output {
            println!("[]");
        } else {
            println!("No supported agents detected.");
            println!("Install one of: Claude Code, Codex, Cursor, Gemini CLI, or OpenCode.");
            println!("Or specify an agent directly: schedx skill install --agent claude");
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

fn print_results_human(results: &[SkillResult], dry_run: bool) {
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
            if r.status == "installed" {
                print_post_install_note(&r.agent, r.path.as_deref());
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
        let versioned = format!("<!-- schedx-skill v{SKILL_VERSION} -->\n{content}");
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
fn skill_target(agent: &str, home: &Path) -> (Vec<(&'static str, &'static str)>, PathBuf) {
    match agent {
        "claude" => (
            vec![("SKILL.md", SKILL_MD), ("reference.md", REFERENCE_MD)],
            home.join(".claude/skills/schedx"),
        ),
        "codex" => (
            vec![("SKILL.md", SKILL_MD), ("reference.md", REFERENCE_MD)],
            home.join(".agents/skills/schedx"),
        ),
        "cursor" => (vec![("schedx.mdc", CURSOR_MDC)], home.join(".cursor/rules")),
        "gemini" => (
            vec![("schedx.md", GEMINI_MD)],
            home.join(".gemini/instructions"),
        ),
        "opencode" => (
            vec![("schedx.md", OPENCODE_MD)],
            home.join(".config/opencode/instructions"),
        ),
        _ => (vec![], home.to_path_buf()),
    }
}

/// Check if an agent binary is available on `PATH`.
fn is_agent_detected(agent: &str) -> bool {
    let bin = match agent {
        "claude" | "codex" | "gemini" | "opencode" => agent,
        "cursor" => return cursor_detected(),
        _ => return false,
    };
    Command::new("which")
        .arg(bin)
        .output()
        .is_ok_and(|o| o.status.success())
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

fn print_post_install_note(agent: &str, path: Option<&str>) {
    match agent {
        "gemini" => {
            println!();
            println!("Note for Gemini CLI:");
            println!("  Add to ~/.gemini/GEMINI.md:");
            println!("    See ~/.gemini/instructions/schedx.md for schedx usage.");
        }
        "opencode" => {
            if let Some(p) = path {
                println!();
                println!("Note for OpenCode:");
                println!("  Add to your opencode.json instructions array:");
                println!("    \"instructions\": [\"{p}\"]");
            }
        }
        _ => {}
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
