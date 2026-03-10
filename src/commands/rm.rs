use anyhow::Result;

use crate::commands::get::find_job;
use crate::engine::lock::FileLock;
use crate::store::state;

pub fn execute(id: &str, force: bool) -> Result<()> {
    let job = find_job(id)?;

    if !force {
        println!(
            "Are you sure you want to remove job {} ({})? Use --force to confirm.",
            job.display_name(),
            job.id
        );
        return Ok(());
    }

    let _lock = FileLock::state()?;
    state::update_state(|s| {
        s.jobs.remove(&job.id);
        Ok(())
    })?;

    println!("Removed job {} ({})", job.display_name(), job.id);
    Ok(())
}
