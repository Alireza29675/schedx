use anyhow::Result;
use chrono::Utc;

use crate::engine::dispatcher;
use crate::store::paths;

pub fn execute() -> Result<()> {
    paths::ensure_dirs()?;
    dispatcher::dispatch(Utc::now())
}
