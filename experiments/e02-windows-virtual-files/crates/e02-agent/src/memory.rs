//! Self-sampling of process memory (GetProcessMemoryInfo), every second.

use crate::{log, stats};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::GetCurrentProcess;

#[derive(Clone, Copy)]
pub struct Sample {
    pub private_bytes: u64,
    pub working_set_bytes: u64,
}

pub fn sample() -> Option<Sample> {
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
            counters.cb,
        )
        .ok()?;
    }
    Some(Sample { private_bytes: counters.PrivateUsage as u64, working_set_bytes: counters.WorkingSetSize as u64 })
}

pub fn record(reason: &str) {
    if let Some(s) = sample() {
        stats::with(|st| {
            st.peak_private_bytes = st.peak_private_bytes.max(s.private_bytes);
            st.peak_working_set_bytes = st.peak_working_set_bytes.max(s.working_set_bytes);
            st.memory_samples += 1;
        });
        log::event("memory", json!({ "reason": reason, "private_bytes": s.private_bytes, "working_set_bytes": s.working_set_bytes }));
    }
}

pub struct Sampler {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Sampler {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                record("periodic");
                // Sleep in short steps so shutdown is prompt.
                for _ in 0..10 {
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Self { stop, handle: Some(handle) }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        record("final");
    }
}
