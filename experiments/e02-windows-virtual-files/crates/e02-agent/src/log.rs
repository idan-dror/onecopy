//! JSONL event log. Never records clipboard content, user names, machine names
//! or paths outside the run directory.

use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub struct EventLog {
    start: Instant,
    run_id: String,
    case_id: String,
    out: Mutex<BufWriter<File>>,
}

static LOG: OnceLock<EventLog> = OnceLock::new();

pub fn init(path: &Path, run_id: &str, case_id: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let log = EventLog {
        start: Instant::now(),
        run_id: run_id.to_owned(),
        case_id: case_id.to_owned(),
        out: Mutex::new(BufWriter::new(file)),
    };
    LOG.set(log).map_err(|_| std::io::Error::other("event log already initialized"))
}

/// Milliseconds since the agent started (monotonic).
pub fn now_ms() -> f64 {
    LOG.get().map(|l| l.start.elapsed().as_secs_f64() * 1000.0).unwrap_or(0.0)
}

pub fn event(name: &str, fields: Value) {
    let Some(log) = LOG.get() else { return };
    let mut obj = Map::new();
    obj.insert("t_ms".into(), json!((log.start.elapsed().as_secs_f64() * 1000.0 * 1000.0).round() / 1000.0));
    obj.insert("run_id".into(), json!(log.run_id));
    obj.insert("case_id".into(), json!(log.case_id));
    obj.insert("thread_id".into(), json!(current_thread_id()));
    obj.insert("event".into(), json!(name));
    if let Value::Object(extra) = fields {
        obj.extend(extra);
    }
    let line = Value::Object(obj).to_string();
    if let Ok(mut out) = log.out.lock() {
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

#[cfg(windows)]
fn current_thread_id() -> u32 {
    unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
}

#[cfg(not(windows))]
fn current_thread_id() -> u32 {
    0
}
