use anyhow::{Result, bail};

use crate::backend;

/// (Re)generate the system scheduler backend files and enable the dispatcher.
pub fn execute(backend_name: &str) -> Result<()> {
    let be = backend::detect_backend()?;

    if be.name() != backend_name {
        bail!(
            "Error: Detected backend is '{}', not '{backend_name}'.\n\
             Run without arguments to see the detected backend.",
            be.name()
        );
    }

    println!("Configuring {} backend...", be.name());

    be.ensure_dispatcher()?;
    println!("Dispatcher configured.");

    match be.check_health() {
        Ok(health) => {
            for msg in &health.messages {
                println!("  {msg}");
            }
            if health.healthy {
                println!("Backend is healthy.");
            } else {
                println!("Backend has issues — see messages above.");
            }
        }
        Err(e) => eprintln!("Health check error: {e}"),
    }

    Ok(())
}
