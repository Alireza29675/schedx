use std::fs;
use std::io::IsTerminal;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::store::paths;
use crate::upgrade::{self, CURRENT_VERSION};

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCache {
    checked_at: DateTime<Utc>,
    latest_version: String,
}

/// Check interval: 24 hours.
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Max time to wait for a background fetch before giving up on showing a hint.
const FETCH_TIMEOUT: Duration = Duration::from_millis(100);

/// Returns an update hint string if a newer version is known, or None.
/// Never blocks for more than ~100ms. Never errors.
pub fn maybe_hint(json_mode: bool) -> Option<String> {
    if json_mode || !std::io::stderr().is_terminal() {
        return None;
    }

    let cache_path = paths::update_check_file().ok()?;

    // Try reading cached data
    if let Ok(data) = fs::read_to_string(&cache_path) {
        if let Ok(cache) = serde_json::from_str::<UpdateCache>(&data) {
            let age = Utc::now().signed_duration_since(cache.checked_at);
            if age < chrono::TimeDelta::from_std(CHECK_INTERVAL).ok()? {
                // Cache is fresh
                return if upgrade::is_newer(CURRENT_VERSION, &cache.latest_version) {
                    Some(format_hint(&cache.latest_version))
                } else {
                    None
                };
            }
        }
    }

    // Cache is stale or missing -- try a quick background fetch
    let cache_path_clone = cache_path.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = upgrade::fetch_latest_version();
        if let Ok(version) = &result {
            let cache = UpdateCache {
                checked_at: Utc::now(),
                latest_version: version.clone(),
            };
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = fs::write(&cache_path_clone, json);
            }
        }
        let _ = tx.send(result);
    });

    // Wait up to 100ms for the result
    if let Ok(Ok(latest)) = rx.recv_timeout(FETCH_TIMEOUT) {
        if upgrade::is_newer(CURRENT_VERSION, &latest) {
            return Some(format_hint(&latest));
        }
    }

    None
}

fn format_hint(latest: &str) -> String {
    let current = CURRENT_VERSION;
    format!(
        "\n\x1b[2mA new version of schedx is available: v{latest} (current: v{current}). \
         Run `schedx upgrade` to update.\x1b[0m",
    )
}
