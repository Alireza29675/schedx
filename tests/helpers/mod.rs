use std::path::PathBuf;

use tempfile::TempDir;

/// Create a fresh test environment with its own `SCHED_HOME`.
pub struct TestEnv {
    pub dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("failed to create temp dir"),
        }
    }

    pub fn home(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    /// Get a Command preconfigured with `SCHED_HOME` and `SCHED_BACKEND=none`.
    #[allow(deprecated)]
    pub fn cmd(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("schedx").expect("binary not found");
        cmd.env("SCHED_HOME", self.home());
        cmd.env("SCHED_BACKEND", "none");
        cmd
    }
}
