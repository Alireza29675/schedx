use std::io::{self, IsTerminal, Write as _};

use anyhow::{Result, bail};

use crate::cli::AgentCommands;
use crate::engine::lock::FileLock;
use crate::model::config::AgentProfile;
use crate::store::config;
use crate::util::detect::{KNOWN_CLI_AGENTS, is_binary_on_path};
use crate::util::redact;

pub fn execute(command: &AgentCommands, json_output: bool) -> Result<()> {
    match command {
        AgentCommands::Add {
            name,
            bin,
            arg,
            prompt_stdin,
        } => add_agent(name, bin, arg, *prompt_stdin),
        AgentCommands::Rm { name } => remove_agent(name),
        AgentCommands::List => list_agents(json_output),
        AgentCommands::Default { name } => set_default(name),
        AgentCommands::Detect { force } => detect_agents(*force, json_output),
    }
}

fn add_agent(name: &str, bin: &str, args: &[String], prompt_stdin: bool) -> Result<()> {
    let _lock = FileLock::state()?;
    config::update_config(|c| {
        c.agents.insert(
            name.to_string(),
            AgentProfile {
                bin: bin.to_string(),
                args: args.to_vec(),
                prompt_stdin,
            },
        );
        Ok(())
    })?;

    println!("Added agent '{name}'");
    Ok(())
}

fn remove_agent(name: &str) -> Result<()> {
    let _lock = FileLock::state()?;
    config::update_config(|c| {
        if c.agents.remove(name).is_none() {
            bail!("Error: Agent '{name}' not found.");
        }
        // Clear default if it was this agent
        if c.default_agent.as_deref() == Some(name) {
            c.default_agent = None;
        }
        Ok(())
    })?;

    println!("Removed agent '{name}'");
    Ok(())
}

fn list_agents(json_output: bool) -> Result<()> {
    let cfg = config::load_config()?;

    if json_output {
        let entries: Vec<_> = cfg
            .agents
            .iter()
            .map(|(name, profile)| {
                serde_json::json!({
                    "name": name,
                    "bin": profile.bin,
                    "args": redact::redact_cli_args(&profile.args),
                    "prompt_stdin": profile.prompt_stdin,
                    "is_default": cfg.default_agent.as_deref() == Some(name.as_str()),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if cfg.agents.is_empty() {
        println!("No agents configured.");
        println!("Add one: schedx agent add <name> --bin <path>");
    } else {
        for (name, profile) in &cfg.agents {
            let default_marker = if cfg.default_agent.as_deref() == Some(name.as_str()) {
                " (default)"
            } else {
                ""
            };
            println!(
                "{name}{default_marker}: {} {}",
                profile.bin,
                redact::redact_cli_args(&profile.args).join(" ")
            );
        }
    }

    Ok(())
}

fn set_default(name: &str) -> Result<()> {
    let _lock = FileLock::state()?;
    config::update_config(|c| {
        if !c.agents.contains_key(name) {
            bail!(
                "Error: Agent '{name}' not found. Add it first: schedx agent add {name} --bin <path>"
            );
        }
        c.default_agent = Some(name.to_string());
        Ok(())
    })?;

    println!("Default agent set to '{name}'");
    Ok(())
}

struct DetectResult {
    name: &'static str,
    description: &'static str,
    status: &'static str,
}

/// The outcome of detecting and registering agent profiles.
struct Detection {
    results: Vec<DetectResult>,
    chosen_default: Option<String>,
    existing_default: Option<String>,
}

pub fn detect_agents(force: bool, json_output: bool) -> Result<()> {
    // The interactive default-agent prompt only makes sense for human runs.
    let detection = run_detection(force, !json_output)?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&detect_value(&detection)).unwrap()
        );
    } else {
        print_detect_human(
            &detection.results,
            detection.chosen_default.as_deref(),
            detection.existing_default.as_deref(),
        );
    }
    Ok(())
}

/// Detect + register agents and return the JSON value WITHOUT printing, so a
/// caller like `schedx setup --json` can fold it into one combined document
/// instead of emitting a second top-level JSON object.
pub fn detect_agents_json(force: bool) -> Result<serde_json::Value> {
    Ok(detect_value(&run_detection(force, false)?))
}

fn detect_value(detection: &Detection) -> serde_json::Value {
    let agents: Vec<_> = detection
        .results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "description": r.description,
                "status": r.status,
            })
        })
        .collect();
    let default_agent = detection
        .chosen_default
        .as_deref()
        .or(detection.existing_default.as_deref());
    serde_json::json!({ "agents": agents, "default_agent": default_agent })
}

fn run_detection(force: bool, allow_prompt: bool) -> Result<Detection> {
    let cfg = config::load_config()?;

    let mut results: Vec<DetectResult> = Vec::new();
    let mut to_insert: Vec<(&str, AgentProfile)> = Vec::new();

    for agent in KNOWN_CLI_AGENTS {
        if !is_binary_on_path(agent.bin) {
            results.push(DetectResult {
                name: agent.name,
                description: agent.description,
                status: "not_found",
            });
            continue;
        }

        let already_registered = cfg.agents.contains_key(agent.name);
        if already_registered && !force {
            results.push(DetectResult {
                name: agent.name,
                description: agent.description,
                status: "already_registered",
            });
        } else {
            let status = if already_registered {
                "updated"
            } else {
                "added"
            };
            to_insert.push((agent.name, agent.to_profile()));
            results.push(DetectResult {
                name: agent.name,
                description: agent.description,
                status,
            });
        }
    }

    // Determine default agent selection before mutating config.
    let needs_default = cfg.default_agent.is_none();
    let added_names: Vec<&str> = results
        .iter()
        .filter(|r| {
            r.status == "added" || r.status == "updated" || r.status == "already_registered"
        })
        .map(|r| r.name)
        .collect();

    let chosen_default = if needs_default && added_names.len() == 1 {
        Some(added_names[0].to_string())
    } else if needs_default && added_names.len() > 1 && allow_prompt {
        prompt_default_agent(&added_names)
    } else {
        None
    };

    // Apply all changes in a single atomic config update.
    if !to_insert.is_empty() || chosen_default.is_some() {
        let _lock = FileLock::state()?;
        config::update_config(|c| {
            for (name, profile) in &to_insert {
                c.agents.insert((*name).to_string(), profile.clone());
            }
            if let Some(ref default) = chosen_default {
                c.default_agent = Some(default.clone());
            }
            Ok(())
        })?;
    }

    Ok(Detection {
        results,
        chosen_default,
        existing_default: cfg.default_agent,
    })
}

fn prompt_default_agent(candidates: &[&str]) -> Option<String> {
    if !io::stdin().is_terminal() {
        return None;
    }

    eprintln!();
    eprintln!("Multiple agents detected. Select default:");
    for (i, name) in candidates.iter().enumerate() {
        eprintln!("  [{}] {name}", i + 1);
    }
    eprint!("Choice [1-{}]: ", candidates.len());
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let choice: usize = input.trim().parse().ok()?;
    if choice >= 1 && choice <= candidates.len() {
        Some(candidates[choice - 1].to_string())
    } else {
        None
    }
}

fn print_detect_human(
    results: &[DetectResult],
    chosen_default: Option<&str>,
    existing_default: Option<&str>,
) {
    println!("Detecting AI coding agents...");

    for r in results {
        let (icon, label) = match r.status {
            "added" => ("+", "added"),
            "updated" => ("\u{21bb}", "updated"),
            "already_registered" => ("~", "already registered"),
            "not_found" => ("-", "not found"),
            _ => ("?", r.status),
        };
        println!("  {icon} {:<12} {:<28} [{label}]", r.name, r.description);
    }

    let added = results.iter().filter(|r| r.status == "added").count();
    let updated = results.iter().filter(|r| r.status == "updated").count();
    let skipped = results
        .iter()
        .filter(|r| r.status == "already_registered")
        .count();

    println!();
    let mut parts: Vec<String> = Vec::new();
    if added > 0 {
        parts.push(format!("{added} added"));
    }
    if updated > 0 {
        parts.push(format!("{updated} updated"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} already registered"));
    }
    if parts.is_empty() {
        println!("No agents detected.");
        println!("Install one of: Claude Code, Codex, Gemini CLI, or OpenCode.");
    } else {
        println!(
            "{} agent(s): {}.",
            added + updated + skipped,
            parts.join(", ")
        );
    }

    let default_agent = chosen_default.or(existing_default);
    if let Some(name) = default_agent {
        println!("Default agent: {name}");
    }
}
