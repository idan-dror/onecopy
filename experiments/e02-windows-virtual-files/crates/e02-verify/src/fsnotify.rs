//! Change notifications for one destination directory (ReadDirectoryChangesW).
//!
//! Polling every 100 ms can miss a file that Explorer creates and deletes quickly (seen in the
//! first Stage B1 error runs). This observer reports every create, delete, rename and size
//! change of a directory entry. It records names only (synthetic) and never reads content.

use serde_json::{Value, json};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::core::HSTRING;

/// Starts a background thread; events arrive on the returned channel. The thread ends with the
/// process (the read blocks in the kernel).
pub fn spawn(dir: &Path, start: Instant) -> Result<Receiver<Value>, Box<dyn std::error::Error>> {
    let handle = unsafe {
        CreateFileW(
            &HSTRING::from(dir.as_os_str()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )?
    };
    let raw = handle.0 as usize;
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let handle = windows::Win32::Foundation::HANDLE(raw as *mut _);
        // DWORD-aligned buffer, as ReadDirectoryChangesW requires.
        let mut buf = vec![0u32; 16 * 1024];
        loop {
            let mut returned = 0u32;
            let r = unsafe {
                ReadDirectoryChangesW(
                    handle,
                    buf.as_mut_ptr().cast(),
                    (buf.len() * 4) as u32,
                    false,
                    FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME | FILE_NOTIFY_CHANGE_SIZE,
                    Some(&mut returned),
                    None,
                    None,
                )
            };
            let t_ms = start.elapsed().as_secs_f64() * 1000.0;
            if let Err(e) = r {
                let _ = tx.send(json!({ "t_ms": t_ms, "change": "fs_error", "hr": format!("{:#010x}", e.code().0) }));
                return;
            }
            if returned == 0 {
                let _ = tx.send(json!({ "t_ms": t_ms, "change": "fs_overflow" }));
                continue;
            }
            let base = buf.as_ptr() as *const u8;
            let mut offset = 0usize;
            loop {
                // SAFETY: the kernel filled `returned` bytes with FILE_NOTIFY_INFORMATION records.
                let info = unsafe { &*(base.add(offset) as *const FILE_NOTIFY_INFORMATION) };
                let len = info.FileNameLength as usize / 2;
                let name_ptr = unsafe { (base.add(offset) as *const FILE_NOTIFY_INFORMATION as *const u8).add(12) as *const u16 };
                let name = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(name_ptr, len) });
                let change = match info.Action {
                    FILE_ACTION_ADDED => "fs_added",
                    FILE_ACTION_REMOVED => "fs_removed",
                    FILE_ACTION_MODIFIED => "fs_modified",
                    FILE_ACTION_RENAMED_OLD_NAME => "fs_renamed_from",
                    FILE_ACTION_RENAMED_NEW_NAME => "fs_renamed_to",
                    _ => "fs_other",
                };
                if tx.send(json!({ "t_ms": t_ms, "change": change, "name": name })).is_err() {
                    return;
                }
                if info.NextEntryOffset == 0 {
                    break;
                }
                offset += info.NextEntryOffset as usize;
            }
        }
    });
    Ok(rx)
}
