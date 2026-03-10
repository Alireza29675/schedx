use anyhow::Result;

use super::{Backend, BackendHealth};

/// No-op backend for testing and dry-run scenarios.
pub struct NoneBackend;

impl Backend for NoneBackend {
    fn ensure_dispatcher(&self) -> Result<()> {
        // No-op: no system service to manage
        Ok(())
    }

    fn remove_dispatcher(&self) -> Result<()> {
        Ok(())
    }

    fn check_health(&self) -> Result<BackendHealth> {
        Ok(BackendHealth {
            healthy: true,
            messages: vec!["Backend: none (no system scheduler)".to_string()],
        })
    }

    fn name(&self) -> &'static str {
        "none"
    }
}
