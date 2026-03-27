use std::process::Command;

use anyhow::{Result, bail};

/// Perform a cargo-based upgrade. Streams cargo output to the terminal.
pub fn execute() -> Result<()> {
    println!("Running: cargo install schedx --locked --force\n");
    let status = Command::new("cargo")
        .args(["install", "schedx", "--locked", "--force"])
        .status()?;
    if !status.success() {
        bail!("cargo install failed with exit code {status}");
    }
    Ok(())
}
