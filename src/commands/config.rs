use anyhow::{Result, bail};

use crate::engine::lock::FileLock;
use crate::store::config;

/// Valid config keys for v0.1.
const VALID_KEYS: &[&str] = &[
    "default_timeout_seconds",
    "backup_count",
    "log_retention_days",
    "default_agent",
    "allow_insecure_http",
];

pub fn execute(key: Option<&str>, value: Option<&str>, json_output: bool) -> Result<()> {
    let cfg = config::load_config()?;

    match (key, value) {
        // Show all config
        (None, _) => {
            if json_output {
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            } else {
                println!("default_timeout_seconds = {}", cfg.default_timeout_seconds);
                println!("backup_count = {}", cfg.backup_count);
                println!("log_retention_days = {}", cfg.log_retention_days);
                println!(
                    "default_agent = {}",
                    cfg.default_agent.as_deref().unwrap_or("(none)")
                );
                println!("allow_insecure_http = {}", cfg.allow_insecure_http);
                println!("backend = {}", cfg.backend);
            }
            Ok(())
        }
        // Read one key
        (Some(k), None) => {
            validate_key(k)?;
            let val = get_config_value(&cfg, k)?;
            if json_output {
                println!("{}", serde_json::json!({ k: val }));
            } else {
                println!("{k} = {val}");
            }
            Ok(())
        }
        // Set a key
        (Some(k), Some(v)) => {
            validate_key(k)?;
            let _lock = FileLock::state()?;
            config::update_config(|c| set_config_value(c, k, v))?;
            println!("Set {k} = {v}");
            Ok(())
        }
    }
}

fn validate_key(key: &str) -> Result<()> {
    if !VALID_KEYS.contains(&key) {
        bail!(
            "Error: Unknown config key '{key}'.\nValid keys: {}",
            VALID_KEYS.join(", ")
        );
    }
    Ok(())
}

fn get_config_value(cfg: &crate::model::config::AppConfig, key: &str) -> Result<String> {
    Ok(match key {
        "default_timeout_seconds" => cfg.default_timeout_seconds.to_string(),
        "backup_count" => cfg.backup_count.to_string(),
        "log_retention_days" => cfg.log_retention_days.to_string(),
        "default_agent" => cfg.default_agent.as_deref().unwrap_or("(none)").to_string(),
        "allow_insecure_http" => cfg.allow_insecure_http.to_string(),
        _ => bail!("Unknown key: {key}"),
    })
}

fn set_config_value(
    cfg: &mut crate::model::config::AppConfig,
    key: &str,
    value: &str,
) -> Result<()> {
    match key {
        "default_timeout_seconds" => {
            cfg.default_timeout_seconds = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Error: '{value}' is not a valid integer."))?;
        }
        "backup_count" => {
            let v: u32 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Error: '{value}' is not a valid integer."))?;
            if v < 1 {
                bail!("Error: backup_count must be >= 1.");
            }
            cfg.backup_count = v;
        }
        "log_retention_days" => {
            let v: u32 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Error: '{value}' is not a valid integer."))?;
            if v < 1 {
                bail!("Error: log_retention_days must be >= 1.");
            }
            cfg.log_retention_days = v;
        }
        "default_agent" => {
            cfg.default_agent = Some(value.to_string());
        }
        "allow_insecure_http" => {
            cfg.allow_insecure_http = match value {
                "true" | "1" | "yes" => true,
                "false" | "0" | "no" => false,
                _ => bail!("Error: '{value}' is not a valid boolean. Use true/false."),
            };
        }
        _ => bail!("Unknown key: {key}"),
    }
    Ok(())
}
