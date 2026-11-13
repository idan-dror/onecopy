//! Clipboard ownership checks that cannot mistake another application's clipboard for ours.
//!
//! Defect fixed here: windows-rs exposes `OleIsCurrentClipboard` as `Result<()>`, which maps the
//! success code S_FALSE ("not on the clipboard") to `Ok(())`. `.is_ok()` was therefore always true,
//! and the agent emptied whatever another application had put on the clipboard. We now call the
//! function through a raw binding that preserves S_OK vs S_FALSE, and we additionally require the
//! clipboard owner window to belong to this process (OLE assigns ownership to an internal OLE window
//! in the process that called OleSetClipboard).

use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, S_OK};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardOwner, OpenClipboard};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::{HRESULT, Interface};

#[link(name = "ole32")]
unsafe extern "system" {
    /// Raw binding: returns S_OK if `pdataobj` is on the clipboard, S_FALSE if not.
    #[link_name = "OleIsCurrentClipboard"]
    fn ole_is_current_clipboard_raw(pdataobj: *mut c_void) -> HRESULT;
}

#[derive(Clone, Copy, Debug)]
pub struct Ownership {
    /// Raw HRESULT of OleIsCurrentClipboard (0 = S_OK, 1 = S_FALSE).
    pub ole_hresult: i32,
    /// The clipboard owner window belongs to this process.
    pub owner_is_our_process: bool,
}

impl Ownership {
    pub fn ours(&self) -> bool {
        self.ole_hresult == S_OK.0 && self.owner_is_our_process
    }
}

pub fn owner_is_our_process() -> bool {
    unsafe {
        let Ok(owner) = GetClipboardOwner() else { return false };
        if owner.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(owner, Some(&mut pid));
        pid != 0 && pid == GetCurrentProcessId()
    }
}

pub fn check(object: &IDataObject) -> Ownership {
    let hr = unsafe { ole_is_current_clipboard_raw(object.as_raw()) };
    Ownership { ole_hresult: hr.0, owner_is_our_process: owner_is_our_process() }
}

/// Outcome of a release attempt at exit.
#[derive(Debug)]
pub struct Release {
    pub before: Ownership,
    pub opened: bool,
    pub cleared: bool,
    pub after_ole_hresult: i32,
}

/// Atomically releases the clipboard only if it is still ours: the clipboard is opened (which
/// blocks every other application from changing it), ownership is re-checked while it is open,
/// and only then is it emptied. If ownership was lost, nothing is touched.
pub fn release_if_ours(object: &IDataObject, hwnd: HWND) -> Release {
    let mut opened = false;
    for _ in 0..40 {
        if unsafe { OpenClipboard(Some(hwnd)) }.is_ok() {
            opened = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let before = check(object);
    let mut cleared = false;
    if opened {
        if before.ours() {
            // OLE's clipboard window receives WM_DESTROYCLIPBOARD and releases our data object.
            cleared = unsafe { EmptyClipboard() }.is_ok();
        }
        unsafe {
            let _ = CloseClipboard();
        }
    }
    let after = unsafe { ole_is_current_clipboard_raw(object.as_raw()) };
    Release { before, opened, cleared, after_ole_hresult: after.0 }
}

#[cfg(test)]
mod tests {
    //! HRESULT audit (checklist W-01). These tests never change the clipboard: they only ask
    //! whether an object that was never placed on it is current.
    use super::*;
    use crate::data_object::{self, Formats, Shared};
    use e02_synth::Case;
    use windows::Win32::Foundation::S_FALSE;
    use windows::Win32::System::Ole::{OleInitialize, OleIsCurrentClipboard, OleUninitialize};

    fn tiny_object() -> IDataObject {
        let case: Case = toml::from_str(
            "case_id = \"T\"\ndescription = \"unit test\"\nseed = 1\n[[files]]\nname = \"t.bin\"\nsize = 16\n",
        )
        .unwrap();
        data_object::create(Shared::new(case, Formats::register().unwrap(), "unit-test".into()))
    }

    #[test]
    fn windows_rs_maps_s_false_to_ok() {
        // The library behavior behind the Stage A defect: every success code becomes Ok(()).
        assert!(S_FALSE.ok().is_ok());
        assert!(S_FALSE.is_ok());
    }

    #[test]
    fn raw_check_distinguishes_s_false_where_the_wrapper_cannot() {
        unsafe { OleInitialize(None).unwrap() };
        let object = tiny_object();
        let raw = unsafe { ole_is_current_clipboard_raw(object.as_raw()) };
        assert_eq!(raw, S_FALSE, "an object never placed on the clipboard is not current");
        assert!(unsafe { OleIsCurrentClipboard(&object) }.is_ok(), "the trap: the wrapper reports Ok for S_FALSE");
        assert!(!check(&object).ours(), "the ownership check must say 'not ours'");
        drop(object);
        unsafe { OleUninitialize() };
    }
}
