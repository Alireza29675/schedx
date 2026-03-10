use anyhow::{Result, bail};

use crate::cli::AgentCommands;
use crate::engine::lock::FileLock;
use crate::model::config::AgentProfile;
use crate::store::config;
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
