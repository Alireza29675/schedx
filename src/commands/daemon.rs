use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::engine::dispatcher;
use crate::store::paths;

/// Default dispatch interval in seconds.
const DEFAULT_INTERVAL: u64 = 10;

pub fn execute(interval: Option<u64>) -> Result<()> {
    paths::ensure_dirs()?;

    let interval_secs = interval.unwrap_or(DEFAULT_INTERVAL);
    if interval_secs == 0 {
        bail!("Error: Interval must be greater than zero.");
    }

    let pid_path = paths::schedx_home()?.join("daemon.pid");

    // Check for existing daemon
    if pid_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&pid_path) {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    bail!(
                        "Error: Another schedx daemon is already running (pid {pid}).\n\
                         Stop it first, or remove {}",
                        pid_path.display()
                    );
                }
            }
        }
        // Stale PID file — remove it
        std::fs::remove_file(&pid_path).ok();
    }

    // Write our PID
    let our_pid = std::process::id();
    std::fs::write(&pid_path, our_pid.to_string()).context("failed to write daemon PID file")?;

    // Set up signal handling via an atomic flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Register Ctrl-C / SIGTERM handler
    #[cfg(unix)]
    {
        // SIGTERM
        let r2 = running.clone();
        unsafe {
            libc::signal(libc::SIGTERM, sigterm_handler as libc::sighandler_t);
        }
        RUNNING_FLAG.store(
            std::ptr::from_ref::<AtomicBool>(r2.as_ref()) as usize,
            Ordering::SeqCst,
        );
    }

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("failed to set Ctrl-C handler")?;

    eprintln!(
        "schedx daemon started (pid {our_pid}, interval {interval_secs}s). Press Ctrl-C to stop."
    );

    // Main dispatch loop
    while running.load(Ordering::SeqCst) {
        if let Err(e) = dispatcher::dispatch(Utc::now()) {
            eprintln!("dispatch error: {e:#}");
        }

        // Sleep in short increments to check the running flag promptly
        let mut slept = Duration::ZERO;
        let target = Duration::from_secs(interval_secs);
        while slept < target && running.load(Ordering::SeqCst) {
            let chunk = std::cmp::min(Duration::from_millis(500), target - slept);
            std::thread::sleep(chunk);
            slept += chunk;
        }
    }

    eprintln!("schedx daemon stopped.");

    // Clean up PID file
    std::fs::remove_file(&pid_path).ok();

    Ok(())
}

/// Check if a process is alive (Unix-only: kill -0).
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // SAFETY: kill with signal 0 just checks if process exists.
    #[allow(clippy::cast_possible_wrap)]
    let pid_t = pid as libc::pid_t;
    // SAFETY: kill with signal 0 just checks if process exists.
    unsafe { libc::kill(pid_t, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

// Global pointer for SIGTERM handler to access the AtomicBool.
#[cfg(unix)]
static RUNNING_FLAG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(unix)]
extern "C" fn sigterm_handler(_sig: libc::c_int) {
    let ptr = RUNNING_FLAG.load(Ordering::SeqCst);
    if ptr != 0 {
        let flag = unsafe { &*(ptr as *const AtomicBool) };
        flag.store(false, Ordering::SeqCst);
    }
}
