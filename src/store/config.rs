use std::fs;

use anyhow::{Context, Result};

use crate::model::config::AppConfig;

use super::atomic::atomic_write_json;
use super::paths;

/// Load the application config. Returns defaults if file does not exist.
pub fn load_config() -> Result<AppConfig> {
    let path = paths::config_file()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config: AppConfig = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

/// Save the config atomically.
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = paths::config_file()?;
    atomic_write_json(&path, config).context("failed to save config")?;
    Ok(())
}

/// Update config via a closure.
pub fn update_config<F>(f: F) -> Result<AppConfig>
where
    F: FnOnce(&mut AppConfig) -> Result<()>,
{
    let mut config = load_config()?;
    f(&mut config)?;
    save_config(&config)?;
    Ok(config)
}
