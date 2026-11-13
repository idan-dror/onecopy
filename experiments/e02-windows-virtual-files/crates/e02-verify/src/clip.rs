//! Harness clipboard helpers (Windows only).
//!
//! Ownership rule (checklist W-01/W-02): nothing here empties the clipboard unless its current
//! content is identified as this run's while the clipboard is held open, so no other
//! application can change it between the check and the action. Identification uses a run-unique
//! tag: the replacement helper's own format, or the provider's marker format.
//!
//! Content is never read or recorded except the run tag and the provider's 4-byte drop-effect
//! DWORD (latency probe); other formats are reported by name and size only.

use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND, S_OK};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW, GetClipboardOwner,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{OleGetClipboard, OleInitialize, OleUninitialize, ReleaseStgMedium};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, IsWindow, RegisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WNDCLASSW,
};
use windows::core::{HSTRING, w};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const MARKER: &str = "OneCopyE02Marker";
pub const REPLACEMENT: &str = "OneCopyE02Replacement";
const CF_UNICODETEXT: u32 = 13;
/// Formats the persistence probe reports (by size only).
const PROBED: &[&str] = &[
    "FileGroupDescriptorW", "FileContents", "Preferred DropEffect", MARKER, "ZoneIdentifier", "DataObject",
    "Ole Private Data",
];

fn register(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) }
}

fn open(hwnd: Option<HWND>) -> bool {
    for _ in 0..80 {
        if unsafe { OpenClipboard(hwnd) }.is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn format_names() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut cf = 0u32;
    loop {
        cf = unsafe { EnumClipboardFormats(cf) };
        if cf == 0 {
            break;
        }
        let mut buf = [0u16; 128];
        let len = unsafe { GetClipboardFormatNameW(cf, &mut buf) };
        out.push((cf, if len > 0 { String::from_utf16_lossy(&buf[..len as usize]) } else { format!("cf#{cf}") }));
    }
    out
}

/// Reads a small HGLOBAL format (at most 4 KiB). Must be called with the clipboard open.
fn read_small(cf: u32) -> Option<Vec<u8>> {
    unsafe {
        let h = GetClipboardData(cf).ok()?;
        if h.is_invalid() {
            return None;
        }
        let g = HGLOBAL(h.0);
        let size = GlobalSize(g);
        if size == 0 || size > 4096 {
            return None;
        }
        let p = GlobalLock(g) as *const u8;
        if p.is_null() {
            return None;
        }
        let v = std::slice::from_raw_parts(p, size).to_vec();
        let _ = GlobalUnlock(g);
        Some(v)
    }
}

fn tag_matches(data: &[u8], tag: &str) -> bool {
    let trimmed: &[u8] = match data.iter().position(|&b| b == 0) {
        Some(i) => &data[..i],
        None => data,
    };
    trimmed == tag.as_bytes()
}

/// (owner window present, owner window still exists).
fn owner_state() -> (bool, bool) {
    match unsafe { GetClipboardOwner() } {
        Ok(h) if !h.is_invalid() => (true, unsafe { IsWindow(Some(h)) }.as_bool()),
        _ => (false, false),
    }
}

fn hglobal_from(bytes: &[u8]) -> AnyResult<HGLOBAL> {
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))?;
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            let _ = GlobalFree(Some(h));
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        let _ = GlobalUnlock(h);
        Ok(h)
    }
}

extern "system" fn helper_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn helper_window() -> AnyResult<HWND> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(helper_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: w!("OneCopyE02ClipboardHelper"),
            ..Default::default()
        };
        RegisterClassW(&class);
        Ok(CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("OneCopyE02ClipboardHelper"),
            w!("OneCopy E-02 clipboard helper"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )?)
    }
}

/// LIFE-03: replaces the clipboard with synthetic text, as "another application" would.
/// The content carries this run's tag so the harness can later identify and remove it.
pub fn set_replacement(tag: &str) -> AnyResult<Value> {
    let hwnd = helper_window()?;
    let t0 = Instant::now();
    let opened = open(Some(hwnd));
    let open_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if !opened {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return Err("could not open the clipboard".into());
    }
    let seq_before = unsafe { GetClipboardSequenceNumber() };
    // EmptyClipboard sends WM_DESTROYCLIPBOARD to the current owner and waits for it: when the
    // owner's thread is blocked (T1 stall), this call blocks too. Its latency is a measurement.
    let t1 = Instant::now();
    let emptied = unsafe { EmptyClipboard() }.is_ok();
    let empty_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let mut set_ok = false;
    if emptied {
        let text: Vec<u16> = "E-02 synthetic clipboard replacement".encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: Vec<u8> = text.iter().flat_map(|c| c.to_le_bytes()).collect();
        let tag_bytes: Vec<u8> = tag.bytes().chain(std::iter::once(0)).collect();
        unsafe {
            set_ok = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(hglobal_from(&bytes)?.0))).is_ok()
                && SetClipboardData(register(REPLACEMENT), Some(HANDLE(hglobal_from(&tag_bytes)?.0))).is_ok();
        }
    }
    unsafe {
        let _ = CloseClipboard();
        let _ = DestroyWindow(hwnd);
    }
    Ok(json!({
        "replaced": emptied && set_ok, "open_ms": open_ms, "empty_ms": empty_ms,
        "sequence_before": seq_before, "sequence_after": unsafe { GetClipboardSequenceNumber() },
    }))
}

/// Releases the clipboard after a run, only if its content is this run's. Also reports which
/// OneCopy formats are still available (PROC-05/07 persistence), by size only.
pub fn release(tag: &str, expect_sequence: Option<u32>, probe_only: bool) -> AnyResult<Value> {
    if !open(None) {
        return Err("could not open the clipboard".into());
    }
    let inspect = || {
        let sequence = unsafe { GetClipboardSequenceNumber() };
        let formats = format_names();
        let has = |n: &str| formats.iter().any(|(_, x)| x == n);
        let (owner_present, owner_alive) = owner_state();
        let mut identified: Option<&str> = None;
        if has(REPLACEMENT) && read_small(register(REPLACEMENT)).is_some_and(|d| tag_matches(&d, tag)) {
            identified = Some("replacement_tag");
        }
        // Only probe data when no live owner could be asked to render it (no render-all risk).
        let mut available = serde_json::Map::new();
        if !owner_alive {
            for name in PROBED {
                if has(name) {
                    let cf = register(name);
                    let size = unsafe {
                        GetClipboardData(cf).ok().filter(|h| !h.is_invalid()).map(|h| GlobalSize(HGLOBAL(h.0)) as u64)
                    };
                    available.insert((*name).into(), json!({ "rendered": size.is_some(), "size": size }));
                }
            }
            // After the provider process dies, the system drops its delayed-render formats and
            // leaves only OLE's private formats pointing at the dead process. They are this
            // run's only if nothing else touched the clipboard since the provider set it.
            let ole_only = !formats.is_empty() && formats.iter().all(|(_, n)| n == "DataObject" || n == "Ole Private Data");
            if identified.is_none() && ole_only && expect_sequence.is_some_and(|s| sequence >= s && sequence - s <= 2) {
                identified = Some("stale_ole_formats_and_sequence");
            }
            if identified.is_none() && has(MARKER) {
                match read_small(register(MARKER)) {
                    Some(d) if tag_matches(&d, tag) => identified = Some("marker_data"),
                    Some(_) => {}
                    None if expect_sequence == Some(sequence) => identified = Some("marker_and_sequence"),
                    None => {}
                }
            }
        }
        let mut cleared = false;
        if identified.is_some() && !probe_only {
            cleared = unsafe { EmptyClipboard() }.is_ok();
        }
        json!({
            "sequence": sequence, "formats": formats.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>(),
            "owner_present": owner_present, "owner_alive": owner_alive,
            "identified_as_this_run": identified, "cleared": cleared, "available": available,
            "refused_reason": if identified.is_none() && (has(MARKER) || has(REPLACEMENT)) {
                Some(if owner_alive { "a live owner holds the clipboard" } else { "tag not matched" })
            } else { None },
        })
    };
    let result = inspect();
    unsafe {
        let _ = CloseClipboard();
    }
    Ok(result)
}

/// STREAM-08: measures how long other clipboard users wait while the provider is busy.
/// Each sample: Win32 format enumeration, then OLE `QueryGetData` and `GetData` for the
/// provider's 4-byte drop-effect DWORD (never file contents).
pub fn probe(out: &Path, stop_file: &Path, parent: Option<&crate::win::ProcessWatch>, interval_ms: u64, timeout_secs: u64) -> AnyResult<()> {
    use std::io::Write;
    unsafe { OleInitialize(None)? };
    let start = Instant::now();
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out)?);
    let effect = register("Preferred DropEffect") as u16;
    while !stop_file.exists() && !parent.is_some_and(|p| p.exited()) && start.elapsed() < Duration::from_secs(timeout_secs) {
        let t_ms = start.elapsed().as_secs_f64() * 1000.0;
        let a = Instant::now();
        let count = if open(None) {
            let n = format_names().len();
            unsafe {
                let _ = CloseClipboard();
            }
            Some(n)
        } else {
            None
        };
        let enum_ms = a.elapsed().as_secs_f64() * 1000.0;
        let available = unsafe { IsClipboardFormatAvailable(effect as u32) }.is_ok();
        let (mut q_ms, mut q_hr, mut g_ms, mut g_ok) = (None, None, None, None);
        if available {
            let b = Instant::now();
            if let Ok(obj) = unsafe { OleGetClipboard() } {
                let fe = FORMATETC { cfFormat: effect, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 };
                // W-01: QueryGetData answers with S_OK or S_FALSE; record the raw code.
                let hr = unsafe { obj.QueryGetData(&fe) };
                q_ms = Some(b.elapsed().as_secs_f64() * 1000.0);
                q_hr = Some(format!("{:#010x}", hr.0));
                if hr == S_OK {
                    let c = Instant::now();
                    let got = unsafe { obj.GetData(&fe) };
                    g_ms = Some(c.elapsed().as_secs_f64() * 1000.0);
                    g_ok = Some(got.is_ok());
                    if let Ok(mut medium) = got {
                        unsafe { ReleaseStgMedium(&mut medium) };
                    }
                }
            }
        }
        writeln!(
            writer,
            "{}",
            json!({ "t_ms": t_ms, "enum_ms": enum_ms, "formats": count, "drop_effect_offered": available,
                    "ole_query_ms": q_ms, "ole_query_hr": q_hr, "ole_get_ms": g_ms, "ole_get_ok": g_ok })
        )?;
        writer.flush()?;
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
    unsafe { OleUninitialize() };
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::tag_matches;

    #[test]
    fn tag_match_requires_exact_run_tag() {
        assert!(tag_matches(b"ERR-01-20261113T120000-0011aabb\0", "ERR-01-20261113T120000-0011aabb"));
        assert!(tag_matches(b"ERR-01-20261113T120000-0011aabb", "ERR-01-20261113T120000-0011aabb"));
        assert!(!tag_matches(b"ERR-01-20261113T120000-0011aabb-extra", "ERR-01-20261113T120000-0011aabb"));
        assert!(!tag_matches(b"ERR-01-20261113T120000", "ERR-01-20261113T120000-0011aabb"));
        assert!(!tag_matches(b"", "x"));
    }
}
