use anyhow::Result;
use chrono::DateTime;

use crate::engine::executor;
use crate::model::run_record::Trigger;
use crate::store::paths;

pub fn execute(job_id: &str, scheduled_for: &str, trigger: &str) -> Result<bool> {
    paths::ensure_dirs()?;

    let scheduled_for_dt = DateTime::parse_from_rfc3339(scheduled_for)
        .map_err(|e| anyhow::anyhow!("Invalid --scheduled-for timestamp: {e}"))?
        .to_utc();

    let trigger_val: Trigger = trigger
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid --trigger value: {e}"))?;

    executor::exec_job(job_id, scheduled_for_dt, trigger_val)
}
