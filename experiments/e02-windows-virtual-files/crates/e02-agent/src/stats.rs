//! Aggregated counters for the run summary (no content).

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default, Serialize, Clone)]
pub struct Stats {
    /// Thread that called OleInitialize/OleSetClipboard (the owner STA).
    pub owner_thread_id: u32,
    /// Thread id -> number of COM method calls dispatched on it.
    pub com_call_threads: BTreeMap<u32, u64>,
    /// COM method name -> call count.
    pub calls: BTreeMap<String, u64>,
    pub first_descriptor_get_data_ms: Option<f64>,
    pub first_contents_get_data_ms: Option<f64>,
    pub first_read_ms: Option<f64>,
    pub last_read_ms: Option<f64>,
    pub last_stream_release_ms: Option<f64>,
    /// First time another process replaced our clipboard object (interference), if ever.
    pub clipboard_lost_ms: Option<f64>,
    pub streams_created: u64,
    pub streams_released: u64,
    pub bytes_served: u64,
    pub read_calls: u64,
    pub short_reads: u64,
    /// Reads that returned a failure HRESULT (injected errors, timeouts, aborts, budget).
    pub failed_reads: u64,
    pub peak_private_bytes: u64,
    pub peak_working_set_bytes: u64,
    pub memory_samples: u64,
    /// Lifetime peak commit as tracked by Windows (PeakPagefileUsage).
    pub os_peak_commit_bytes: u64,
    pub os_peak_working_set_bytes: u64,
    pub per_stream: Vec<StreamSummary>,
}

#[derive(Serialize, Clone)]
pub struct StreamSummary {
    pub stream_id: u64,
    pub file_index: u32,
    pub advertised_size: u64,
    pub bytes_served: u64,
    pub final_position: u64,
    /// Furthest position reached by reads (targets may seek back afterwards).
    pub max_position: u64,
    pub read_calls: u64,
    pub failed_reads: u64,
    pub min_request: Option<u32>,
    pub max_request: Option<u32>,
    pub seeks: u64,
    pub stats_calls: u64,
    /// True once a Read at or past the advertised size returned fewer bytes than requested.
    pub eof_read: bool,
}

pub static STATS: Mutex<Option<Stats>> = Mutex::new(None);

pub fn with<R>(f: impl FnOnce(&mut Stats) -> R) -> R {
    let mut guard = STATS.lock().unwrap_or_else(|p| p.into_inner());
    f(guard.get_or_insert_with(Stats::default))
}

pub fn count(method: &str) {
    let tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    with(|s| {
        *s.calls.entry(method.to_owned()).or_default() += 1;
        *s.com_call_threads.entry(tid).or_default() += 1;
    });
}

pub fn snapshot() -> Stats {
    with(|s| s.clone())
}
