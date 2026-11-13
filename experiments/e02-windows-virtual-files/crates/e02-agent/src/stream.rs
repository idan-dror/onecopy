//! IStream serving one synthetic file. Stage A: no fault injection.

use crate::{log, stats};
use e02_synth::{Synth, SynthStream};
use serde_json::json;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Mutex;
use windows::Win32::Foundation::{E_NOTIMPL, E_POINTER, S_FALSE, S_OK, STG_E_ACCESSDENIED, STG_E_INVALIDFUNCTION};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, ISequentialStream_Impl, IStream, IStream_Impl, LOCKTYPE, STATFLAG, STATFLAG_NONAME, STATSTG,
    STGC, STGM_READ, STGTY_STREAM, STREAM_SEEK, STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET,
};
use windows::core::{HRESULT, PWSTR, Ref, Result, implement};

/// How many individual reads are logged at the start and kept for the end of a stream.
const LOG_FIRST_READS: u64 = 200;
const LOG_LAST_READS: usize = 200;
/// Every Nth read in the middle is logged as a sample.
const LOG_EVERY_NTH_READ: u64 = 1000;

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
    last_reads: VecDeque<ReadRecord>,
}

// Agile = false: see data_object.rs; T1 keeps every IStream call on the owner STA.
#[implement(IStream, Agile = false)]
pub struct VirtualFileStream {
    stream_id: u64,
    file_index: u32,
    name: String,
    size: u64,
    state: Mutex<State>,
}

impl VirtualFileStream {
    pub fn create(stream_id: u64, file_index: u32, name: &str, size: u64, synth: &Synth) -> IStream {
        stats::with(|s| s.streams_created += 1);
        log::event("stream_create", json!({ "stream_id": stream_id, "file_index": file_index, "advertised_size": size }));
        VirtualFileStream {
            stream_id,
            file_index,
            name: name.to_owned(),
            size,
            state: Mutex::new(State {
                source: synth.stream_at(file_index, 0),
                position: 0,
                max_position: 0,
                eof_read: false,
                read_calls: 0,
                bytes_served: 0,
                min_request: None,
                max_request: None,
                seeks: 0,
                stat_calls: 0,
                last_reads: VecDeque::with_capacity(LOG_LAST_READS),
            }),
        }
        .into()
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
}

impl Drop for VirtualFileStream {
    fn drop(&mut self) {
        let st = self.state.get_mut().unwrap_or_else(|p| p.into_inner());
        // Flush the tail of the read history that was not logged individually.
        let logged_upto = LOG_FIRST_READS;
        let tail_start = st.read_calls.saturating_sub(st.last_reads.len() as u64) + 1;
        for (i, rec) in st.last_reads.iter().enumerate() {
            let n = tail_start + i as u64;
            if n > logged_upto && n % LOG_EVERY_NTH_READ != 0 {
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
            advertised_size: self.size,
            bytes_served: st.bytes_served,
            final_position: st.position,
            max_position: st.max_position,
            read_calls: st.read_calls,
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

impl ISequentialStream_Impl for VirtualFileStream_Impl {
    fn Read(&self, pv: *mut c_void, cb: u32, pcbread: *mut u32) -> HRESULT {
        stats::count("IStream::Read");
        let now = log::now_ms();
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let remaining = self.size.saturating_sub(st.position);
        let n = (cb as u64).min(remaining) as usize;
        if n > 0 {
            if pv.is_null() {
                return E_POINTER;
            }
            // SAFETY: the caller provides a buffer of at least `cb` bytes; n <= cb.
            let buf = unsafe { std::slice::from_raw_parts_mut(pv as *mut u8, n) };
            st.source.fill_next(buf);
        }
        let offset = st.position;
        st.position += n as u64;
        st.max_position = st.max_position.max(st.position);
        if (n as u32) < cb && st.position >= self.size {
            st.eof_read = true;
        }
        st.bytes_served += n as u64;
        st.read_calls += 1;
        st.min_request = Some(st.min_request.map_or(cb, |m| m.min(cb)));
        st.max_request = Some(st.max_request.map_or(cb, |m| m.max(cb)));
        if !pcbread.is_null() {
            unsafe { *pcbread = n as u32 };
        }
        // A short read only happens at end of stream; report it as S_FALSE.
        let hr = if n as u32 == cb { S_OK } else { S_FALSE };
        let rec = ReadRecord { t_ms: now, offset, requested: cb, returned: n as u32, hr: hr.0 };
        let read_no = st.read_calls;
        if read_no <= LOG_FIRST_READS || read_no % LOG_EVERY_NTH_READ == 0 || hr != S_OK {
            self.log_read(read_no, &rec);
        }
        if st.last_reads.len() == LOG_LAST_READS {
            st.last_reads.pop_front();
        }
        st.last_reads.push_back(rec);
        drop(st);
        stats::with(|s| {
            s.read_calls += 1;
            s.bytes_served += n as u64;
            if hr != S_OK {
                s.short_reads += 1;
            }
            s.first_read_ms.get_or_insert(now);
            s.last_read_ms = Some(now);
        });
        hr
    }

    fn Write(&self, _pv: *const c_void, _cb: u32, _pcbwritten: *mut u32) -> HRESULT {
        self.other_call("Write", STG_E_ACCESSDENIED)
    }
}

impl IStream_Impl for VirtualFileStream_Impl {
    fn Seek(&self, dlibmove: i64, dworigin: STREAM_SEEK, plibnewposition: *mut u64) -> Result<()> {
        stats::count("IStream::Seek");
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let base: i128 = match dworigin {
            STREAM_SEEK_SET => 0,
            STREAM_SEEK_CUR => st.position as i128,
            STREAM_SEEK_END => self.size as i128,
            _ => return Err(STG_E_INVALIDFUNCTION.into()),
        };
        let target = base + dlibmove as i128;
        let result = if target < 0 || target > self.size as i128 {
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

    fn SetSize(&self, _libnewsize: u64) -> Result<()> {
        Err(self.other_call("SetSize", E_NOTIMPL).into())
    }

    fn CopyTo(&self, _pstm: Ref<'_, IStream>, _cb: u64, _pcbread: *mut u64, _pcbwritten: *mut u64) -> Result<()> {
        Err(self.other_call("CopyTo", E_NOTIMPL).into())
    }

    fn Commit(&self, _grfcommitflags: &STGC) -> Result<()> {
        Err(self.other_call("Commit", E_NOTIMPL).into())
    }

    fn Revert(&self) -> Result<()> {
        Err(self.other_call("Revert", E_NOTIMPL).into())
    }

    fn LockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: &LOCKTYPE) -> Result<()> {
        Err(self.other_call("LockRegion", E_NOTIMPL).into())
    }

    fn UnlockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: u32) -> Result<()> {
        Err(self.other_call("UnlockRegion", E_NOTIMPL).into())
    }

    fn Stat(&self, pstatstg: *mut STATSTG, grfstatflag: &STATFLAG) -> Result<()> {
        stats::count("IStream::Stat");
        if pstatstg.is_null() {
            return Err(E_POINTER.into());
        }
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
                cbSize: self.size,
                grfMode: STGM_READ,
                ..Default::default()
            };
        }
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        st.stat_calls += 1;
        log::event("stream_stat", json!({ "stream_id": self.stream_id, "flags": grfstatflag.0, "reported_size": self.size }));
        Ok(())
    }

    fn Clone(&self) -> Result<IStream> {
        Err(self.other_call("Clone", E_NOTIMPL).into())
    }
}
