//! IStream serving one synthetic file, with the per-file fault plan (spec §3.3).
//!
//! The logic lives in `StreamCore`. Two thin COM wrappers select the threading variant:
//! `StaStream` (`Agile = false`, T1: every call is serialized on the owner STA) and
//! `AgileStream` (default agile objects, T2/T2A: cross-process calls run on RPC threads).

use crate::data_object::Shared;
use crate::{control, log, stats};
use e02_synth::{FilePlan, ReadOutcome, SynthStream, Threading};
use serde_json::json;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use windows::Win32::Foundation::{E_ABORT, E_FAIL, E_NOTIMPL, E_POINTER, S_FALSE, S_OK, STG_E_ACCESSDENIED, STG_E_INVALIDFUNCTION};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, ISequentialStream_Impl, IStream, IStream_Impl, LOCKTYPE, STATFLAG, STATFLAG_NONAME, STATSTG, STGC,
    STGM_READ, STGTY_STREAM, STREAM_SEEK, STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET,
};
use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
use windows::core::{HRESULT, PWSTR, Ref, Result, implement};

/// How many individual reads are logged at the start and kept for the end of a stream.
const LOG_FIRST_READS: u64 = 200;
const LOG_LAST_READS: usize = 200;
/// Every Nth read in the middle is logged as a sample.
const LOG_EVERY_NTH_READ: u64 = 1000;
/// HRESULT_FROM_WIN32(ERROR_TIMEOUT): returned by a read that hit the provider read timeout (RT).
pub const HR_READ_TIMEOUT: HRESULT = HRESULT(0x8007_05B4_u32 as i32);
/// Exit code used when a `self_kill` fault terminates the provider.
pub const SELF_KILL_EXIT_CODE: u32 = 0xE02;

struct ReadRecord {
    t_ms: f64,
    offset: u64,
    requested: u32,
    returned: u32,
    hr: i32,
}

struct State {
    source: SynthStream,
    position: u64,
    max_position: u64,
    eof_read: bool,
    read_calls: u64,
    bytes_served: u64,
    min_request: Option<u32>,
    max_request: Option<u32>,
    seeks: u64,
    stat_calls: u64,
    failed_reads: u64,
    last_reads: VecDeque<ReadRecord>,
}

pub struct StreamCore {
    stream_id: u64,
    file_index: u32,
    name: String,
    /// True file size (`CaseFile::size`).
    size: u64,
    /// Bytes this stream delivers before end of stream (premature EOF / overrun applied).
    stream_len: u64,
    plan: FilePlan,
    shared: Arc<Shared>,
    state: Mutex<State>,
}

/// Creates the stream object for `file_index` in the threading variant of the case.
pub fn create(stream_id: u64, file_index: u32, shared: &Arc<Shared>) -> IStream {
    let file = &shared.case.files[file_index as usize];
    let plan = file.plan();
    let stream_len = plan.stream_len();
    stats::with(|s| s.streams_created += 1);
    log::event(
        "stream_create",
        json!({ "stream_id": stream_id, "file_index": file_index, "advertised_size": file.descriptor_size(),
                "true_size": file.size, "stream_len": stream_len, "faults": file.faults.len(),
                "agile": shared.case.variants.threading != Threading::T1 }),
    );
    let core = StreamCore {
        stream_id,
        file_index,
        name: file.name.clone(),
        size: file.size,
        stream_len,
        plan,
        shared: shared.clone(),
        state: Mutex::new(State {
            source: shared.synth.stream_at(file_index, 0),
            position: 0,
            max_position: 0,
            eof_read: false,
            read_calls: 0,
            bytes_served: 0,
            min_request: None,
            max_request: None,
            seeks: 0,
            stat_calls: 0,
            failed_reads: 0,
            last_reads: VecDeque::with_capacity(LOG_LAST_READS),
        }),
    };
    match shared.case.variants.threading {
        Threading::T1 => StaStream(core).into(),
        Threading::T2 | Threading::T2A => AgileStream(core).into(),
    }
}

impl StreamCore {
    fn fault_event(&self, action: &str, fields: serde_json::Value) {
        let mut v = json!({ "stream_id": self.stream_id, "file_index": self.file_index, "action": action });
        if let (Some(obj), serde_json::Value::Object(extra)) = (v.as_object_mut(), fields) {
            obj.extend(extra);
        }
        log::event("fault", v);
    }

    fn other_call(&self, method: &str, result: HRESULT) -> HRESULT {
        stats::count(&format!("IStream::{method}"));
        log::event("stream_other_call", json!({ "stream_id": self.stream_id, "method": method, "hr": format!("{:#010x}", result.0) }));
        result
    }

    fn log_read(&self, n: u64, rec: &ReadRecord) {
        log::event(
            "stream_read",
            json!({
                "stream_id": self.stream_id, "read_no": n, "offset": rec.offset,
                "requested": rec.requested, "returned": rec.returned, "hr": format!("{:#010x}", rec.hr),
            }),
        );
    }

    /// Records one read in the per-stream and global statistics; returns `hr`.
    fn finish_read(&self, st: &mut State, now: f64, offset: u64, cb: u32, n: u32, hr: HRESULT) -> HRESULT {
        st.read_calls += 1;
        st.bytes_served += n as u64;
        st.min_request = Some(st.min_request.map_or(cb, |m| m.min(cb)));
        st.max_request = Some(st.max_request.map_or(cb, |m| m.max(cb)));
        if hr.is_err() {
            st.failed_reads += 1;
        }
        let rec = ReadRecord { t_ms: now, offset, requested: cb, returned: n, hr: hr.0 };
        let read_no = st.read_calls;
        if read_no <= LOG_FIRST_READS || read_no % LOG_EVERY_NTH_READ == 0 || hr != S_OK {
            self.log_read(read_no, &rec);
        }
        if st.last_reads.len() == LOG_LAST_READS {
            st.last_reads.pop_front();
        }
        st.last_reads.push_back(rec);
        stats::with(|s| {
            s.read_calls += 1;
            s.bytes_served += n as u64;
            if hr == S_FALSE {
                s.short_reads += 1;
            }
            if hr.is_err() {
                s.failed_reads += 1;
            }
            s.first_read_ms.get_or_insert(now);
            s.last_read_ms = Some(now);
        });
        hr
    }

    fn read(&self, pv: *mut c_void, cb: u32, pcbread: *mut u32) -> HRESULT {
        stats::count("IStream::Read");
        let started = Instant::now();
        let now = log::now_ms();
        if !pcbread.is_null() {
            unsafe { *pcbread = 0 };
        }
        let fired = &self.shared.fired[self.file_index as usize];
        let claim = |i: usize| !fired[i].swap(true, Ordering::SeqCst);
        let (position, action) = {
            let st = self.state.lock().unwrap_or_else(|p| p.into_inner());
            (st.position, self.plan.plan_read(st.position, cb, &claim))
        };
        for &m in &action.marks {
            let label = match &self.plan.faults[m] {
                e02_synth::Fault::Mark { label, .. } => label.clone(),
                _ => String::new(),
            };
            self.fault_event("mark", json!({ "offset": position, "label": label }));
        }
        let fail = |hr: HRESULT| {
            let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
            self.finish_read(&mut st, now, position, cb, 0, hr)
        };
        if action.latency_ms > 0 && !control::sleep_interruptible(action.latency_ms) {
            self.fault_event("sleep_aborted", json!({ "offset": position }));
            return fail(E_ABORT);
        }
        if let Some((fault, duration_ms)) = action.stall {
            let timeout = self.shared.case.variants.read_timeout_ms;
            self.fault_event(
                "stall_begin",
                json!({ "offset": position, "fault": fault, "duration_ms": duration_ms, "read_timeout_ms": timeout }),
            );
            let limit = timeout.map_or(duration_ms, |t| t.min(duration_ms));
            if !control::sleep_interruptible(limit) {
                self.fault_event("stall_aborted", json!({ "offset": position, "after_ms": started.elapsed().as_millis() as u64 }));
                return fail(E_ABORT);
            }
            if timeout.is_some_and(|t| t < duration_ms) {
                self.fault_event("read_timeout", json!({ "offset": position, "after_ms": started.elapsed().as_millis() as u64 }));
                return fail(HR_READ_TIMEOUT);
            }
            self.fault_event("stall_end", json!({ "offset": position }));
        }
        let (bytes, end_of_stream) = match action.outcome {
            ReadOutcome::Kill { fault } => {
                self.fault_event("self_kill", json!({ "offset": position, "fault": fault }));
                // TerminateProcess on ourselves: equivalent to `taskkill /F` mid-Read (PROC-01).
                unsafe {
                    let _ = TerminateProcess(GetCurrentProcess(), SELF_KILL_EXIT_CODE);
                }
                return fail(E_FAIL);
            }
            ReadOutcome::Fail { fault, hresult } => {
                self.fault_event("error_returned", json!({ "offset": position, "fault": fault, "hr": format!("{hresult:#010x}") }));
                return fail(HRESULT(hresult as i32));
            }
            ReadOutcome::Serve { bytes, end_of_stream } => (bytes, end_of_stream),
        };
        // Render-all guard: a provider-wide byte budget, so no path can stream an unbounded file.
        let budget = self.shared.byte_budget;
        let before = self.shared.total_served.fetch_add(bytes as u64, Ordering::SeqCst);
        if before + bytes as u64 > budget {
            self.shared.total_served.fetch_sub(bytes as u64, Ordering::SeqCst);
            self.fault_event("byte_budget_exceeded", json!({ "offset": position, "budget": budget, "served": before }));
            return fail(E_FAIL);
        }
        if bytes > 0 && pv.is_null() {
            self.shared.total_served.fetch_sub(bytes as u64, Ordering::SeqCst);
            return fail(E_POINTER);
        }
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if st.position != position {
            // A concurrent Seek moved the stream while this read was planned; serve from the plan.
            st.source.seek(position);
        }
        if bytes > 0 {
            // SAFETY: the caller provides a buffer of at least `cb` bytes; bytes <= cb.
            let buf = unsafe { std::slice::from_raw_parts_mut(pv as *mut u8, bytes as usize) };
            st.source.fill_next(buf);
        }
        st.position = position + bytes as u64;
        st.max_position = st.max_position.max(st.position);
        let first_eof = end_of_stream && !st.eof_read;
        if end_of_stream {
            st.eof_read = true;
        }
        if !pcbread.is_null() {
            unsafe { *pcbread = bytes };
        }
        // End of stream is reported as S_FALSE; a short read that is not end of stream is S_OK.
        let hr = if end_of_stream { S_FALSE } else { S_OK };
        let hr = self.finish_read(&mut st, now, position, cb, bytes, hr);
        drop(st);
        if first_eof {
            self.fault_event("eof_returned", json!({ "offset": position + bytes as u64, "advertised": self.shared.case.files[self.file_index as usize].descriptor_size() }));
        }
        if action.throttle_ms > 0.0 {
            let remaining = action.throttle_ms - started.elapsed().as_secs_f64() * 1000.0;
            if remaining > 0.0 {
                let _ = control::sleep_interruptible(remaining.ceil() as u64);
            }
        }
        hr
    }

    fn seek(&self, dlibmove: i64, dworigin: STREAM_SEEK, plibnewposition: *mut u64) -> Result<()> {
        stats::count("IStream::Seek");
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let end = self.stream_len.max(self.size);
        let base: i128 = match dworigin {
            STREAM_SEEK_SET => 0,
            STREAM_SEEK_CUR => st.position as i128,
            STREAM_SEEK_END => self.stream_len as i128,
            _ => return Err(STG_E_INVALIDFUNCTION.into()),
        };
        let target = base + dlibmove as i128;
        let result = if target < 0 || target > end as i128 {
            Err(STG_E_INVALIDFUNCTION.into())
        } else {
            let new = target as u64;
            if new != st.position {
                st.source.seek(new);
                st.position = new;
            }
            if !plibnewposition.is_null() {
                unsafe { *plibnewposition = new };
            }
            Ok(())
        };
        st.seeks += 1;
        log::event(
            "stream_seek",
            json!({
                "stream_id": self.stream_id, "origin": dworigin.0, "move": dlibmove,
                "new_position": st.position, "ok": result.is_ok(),
            }),
        );
        result
    }

    fn stat(&self, pstatstg: *mut STATSTG, grfstatflag: &STATFLAG) -> Result<()> {
        stats::count("IStream::Stat");
        if pstatstg.is_null() {
            return Err(E_POINTER.into());
        }
        let reported = self.shared.case.files[self.file_index as usize].stat_size.unwrap_or(self.size);
        let no_name = (grfstatflag.0 & STATFLAG_NONAME.0) != 0;
        let mut name = PWSTR::null();
        if !no_name {
            let wide: Vec<u16> = self.name.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let p = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                    name = PWSTR(p);
                }
            }
        }
        unsafe {
            *pstatstg = STATSTG {
                pwcsName: name,
                r#type: STGTY_STREAM.0 as u32,
                cbSize: reported,
                grfMode: STGM_READ,
                ..Default::default()
            };
        }
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        st.stat_calls += 1;
        log::event("stream_stat", json!({ "stream_id": self.stream_id, "flags": grfstatflag.0, "reported_size": reported }));
        Ok(())
    }
}

impl Drop for StreamCore {
    fn drop(&mut self) {
        let st = self.state.get_mut().unwrap_or_else(|p| p.into_inner());
        // Flush the tail of the read history that was not logged individually.
        let tail_start = st.read_calls.saturating_sub(st.last_reads.len() as u64) + 1;
        for (i, rec) in st.last_reads.iter().enumerate() {
            let n = tail_start + i as u64;
            if n > LOG_FIRST_READS && n % LOG_EVERY_NTH_READ != 0 && rec.hr == S_OK.0 {
                log::event(
                    "stream_read",
                    json!({
                        "stream_id": self.stream_id, "read_no": n, "offset": rec.offset, "tail": true,
                        "requested": rec.requested, "returned": rec.returned, "hr": format!("{:#010x}", rec.hr),
                        "read_t_ms": rec.t_ms,
                    }),
                );
            }
        }
        let summary = stats::StreamSummary {
            stream_id: self.stream_id,
            file_index: self.file_index,
            advertised_size: self.shared.case.files[self.file_index as usize].descriptor_size(),
            bytes_served: st.bytes_served,
            final_position: st.position,
            max_position: st.max_position,
            read_calls: st.read_calls,
            failed_reads: st.failed_reads,
            min_request: st.min_request,
            max_request: st.max_request,
            seeks: st.seeks,
            stats_calls: st.stat_calls,
            eof_read: st.eof_read,
        };
        log::event("stream_release", serde_json::to_value(&summary).unwrap_or_default());
        let now = log::now_ms();
        stats::with(|s| {
            s.streams_released += 1;
            s.last_stream_release_ms = Some(now);
            s.per_stream.push(summary);
        });
    }
}

/// Generates a COM `IStream` wrapper around `StreamCore` with the given agility.
macro_rules! stream_wrapper {
    ($name:ident, $impl_name:ident, $agile:tt) => {
        #[implement(IStream, Agile = $agile)]
        pub struct $name(StreamCore);

        impl ISequentialStream_Impl for $impl_name {
            fn Read(&self, pv: *mut c_void, cb: u32, pcbread: *mut u32) -> HRESULT {
                self.0.read(pv, cb, pcbread)
            }
            fn Write(&self, _pv: *const c_void, _cb: u32, _pcbwritten: *mut u32) -> HRESULT {
                self.0.other_call("Write", STG_E_ACCESSDENIED)
            }
        }

        impl IStream_Impl for $impl_name {
            fn Seek(&self, dlibmove: i64, dworigin: STREAM_SEEK, plibnewposition: *mut u64) -> Result<()> {
                self.0.seek(dlibmove, dworigin, plibnewposition)
            }
            fn SetSize(&self, _libnewsize: u64) -> Result<()> {
                Err(self.0.other_call("SetSize", E_NOTIMPL).into())
            }
            fn CopyTo(&self, _pstm: Ref<'_, IStream>, _cb: u64, _pcbread: *mut u64, _pcbwritten: *mut u64) -> Result<()> {
                Err(self.0.other_call("CopyTo", E_NOTIMPL).into())
            }
            fn Commit(&self, _grfcommitflags: &STGC) -> Result<()> {
                Err(self.0.other_call("Commit", E_NOTIMPL).into())
            }
            fn Revert(&self) -> Result<()> {
                Err(self.0.other_call("Revert", E_NOTIMPL).into())
            }
            fn LockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: &LOCKTYPE) -> Result<()> {
                Err(self.0.other_call("LockRegion", E_NOTIMPL).into())
            }
            fn UnlockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: u32) -> Result<()> {
                Err(self.0.other_call("UnlockRegion", E_NOTIMPL).into())
            }
            fn Stat(&self, pstatstg: *mut STATSTG, grfstatflag: &STATFLAG) -> Result<()> {
                self.0.stat(pstatstg, grfstatflag)
            }
            fn Clone(&self) -> Result<IStream> {
                Err(self.0.other_call("Clone", E_NOTIMPL).into())
            }
        }
    };
}

// Agile = false: windows-rs objects are agile by default (IAgileObject + free-threaded marshaler),
// which dispatches cross-process calls on RPC threads. T1 needs STA-bound objects.
stream_wrapper!(StaStream, StaStream_Impl, false);
// Agile (default): candidate T2 mechanism. Cross-process calls arrive on RPC threads, so the
// stream state must be (and is) thread-safe.
stream_wrapper!(AgileStream, AgileStream_Impl, true);
