//! Stop conditions shared by the message loop and by blocking fault actions.
//!
//! In threading variant T1 a stalled `Read` blocks the owner STA, so the window timer that
//! normally checks the stop file cannot run. Every fault sleep therefore polls the same stop
//! conditions itself, so no injected stall can outlive the harness or ignore a stop request.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HANDLE, WAIT_TIMEOUT};
use windows::Win32::System::Threading::WaitForSingleObject;

pub struct StopCondition {
    pub stop_file: PathBuf,
    pub deadline: Instant,
    /// SYNCHRONIZE handle to the harness process, if a parent was given.
    pub parent: Option<HANDLE>,
}

// SAFETY: the handle is only waited on (never closed) while the process runs.
unsafe impl Send for StopCondition {}
unsafe impl Sync for StopCondition {}

static STOP: OnceLock<StopCondition> = OnceLock::new();
static STOPPING: AtomicBool = AtomicBool::new(false);

pub fn install(condition: StopCondition) {
    let _ = STOP.set(condition);
}

/// Which stop conditions hold now: (stop file, deadline, parent exited).
pub fn reasons() -> (bool, bool, bool) {
    let Some(stop) = STOP.get() else { return (false, false, false) };
    let by_file = stop.stop_file.exists();
    let by_deadline = Instant::now() >= stop.deadline;
    let by_parent = stop.parent.is_some_and(|h| unsafe { WaitForSingleObject(h, 0) } != WAIT_TIMEOUT);
    (by_file, by_deadline, by_parent)
}

pub fn stop_requested() -> bool {
    if STOPPING.load(Ordering::Relaxed) {
        return true;
    }
    let (a, b, c) = reasons();
    if a || b || c {
        STOPPING.store(true, Ordering::Relaxed);
        return true;
    }
    false
}

/// Message-loop condition for `GetMessageW`: continue only on a positive result. 0 is WM_QUIT
/// and -1 is an error; `BOOL::as_bool()` would treat -1 as true (HRESULT/return-code audit).
pub fn keep_pumping(result: windows::core::BOOL) -> bool {
    result.0 > 0
}

/// Sleeps up to `ms`, polling the stop conditions every 100 ms. Returns false if interrupted.
pub fn sleep_interruptible(ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        let now = Instant::now();
        if now >= end {
            return true;
        }
        if stop_requested() {
            return false;
        }
        std::thread::sleep((end - now).min(Duration::from_millis(100)));
    }
}

#[cfg(test)]
mod tests {
    use super::keep_pumping;
    use windows::core::BOOL;

    #[test]
    fn message_loop_stops_on_quit_and_on_error() {
        assert!(keep_pumping(BOOL(1)));
        assert!(!keep_pumping(BOOL(0)), "WM_QUIT");
        assert!(!keep_pumping(BOOL(-1)), "error must not keep the loop running");
        assert!(BOOL(-1).as_bool(), "the trap: as_bool() treats -1 as true");
    }
}

#[cfg(test)]
mod stop_tests {
    use super::*;

    #[test]
    fn injected_sleeps_stop_when_the_stop_file_appears() {
        let dir = std::env::temp_dir().join(format!("e02-control-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let stop_file = dir.join("stop");
        std::fs::write(&stop_file, b"stop").unwrap();
        install(StopCondition { stop_file: stop_file.clone(), deadline: Instant::now() + Duration::from_secs(600), parent: None });
        let t0 = Instant::now();
        assert!(!sleep_interruptible(30_000), "a stall must be interrupted by the stop file");
        assert!(t0.elapsed() < Duration::from_secs(2), "interruption took {:?}", t0.elapsed());
        let _ = std::fs::remove_file(&stop_file);
        let _ = std::fs::remove_dir(&dir);
    }
}
