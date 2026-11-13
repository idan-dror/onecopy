//! Builds the CFSTR_FILEDESCRIPTORW HGLOBAL (FILEGROUPDESCRIPTORW + FILEDESCRIPTORW[n]).

use e02_synth::Case;
use std::mem::{offset_of, size_of};
use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::UI::Shell::{FD_FILESIZE, FD_PROGRESSUI, FD_UNICODE, FILEDESCRIPTORW, FILEGROUPDESCRIPTORW};
use windows::core::{Error, Result};

pub struct DescriptorInfo {
    pub bytes: usize,
    pub entries: usize,
}

/// Allocates a movable HGLOBAL; ownership passes to the caller (the STGMEDIUM recipient).
pub fn build(case: &Case) -> Result<(HGLOBAL, DescriptorInfo)> {
    let n = case.files.len();
    let header = offset_of!(FILEGROUPDESCRIPTORW, fgd);
    let bytes = header + n * size_of::<FILEDESCRIPTORW>();

    let mut flags = FD_UNICODE.0 as u32;
    if case.variants.advertise_size {
        flags |= FD_FILESIZE.0 as u32;
    }
    if case.variants.progress_ui {
        flags |= FD_PROGRESSUI.0 as u32;
    }

    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes)?;
        let base = GlobalLock(h) as *mut u8;
        if base.is_null() {
            let err = Error::from_thread();
            let _ = GlobalFree(Some(h));
            return Err(err);
        }
        let group = base as *mut FILEGROUPDESCRIPTORW;
        (*group).cItems = n as u32;
        let entries = base.add(header) as *mut FILEDESCRIPTORW;
        for (i, file) in case.files.iter().enumerate() {
            let mut fd = FILEDESCRIPTORW { dwFlags: flags, ..Default::default() };
            if case.variants.advertise_size {
                let size = file.descriptor_size();
                fd.nFileSizeHigh = (size >> 32) as u32;
                fd.nFileSizeLow = size as u32;
            }
            // Names were validated as relative and shorter than MAX_PATH.
            let wide: Vec<u16> = file.name.replace('/', "\\").encode_utf16().collect();
            // FILEDESCRIPTORW is 1-byte packed; assign the whole array by value.
            let mut file_name = [0u16; 260];
            file_name[..wide.len()].copy_from_slice(&wide);
            fd.cFileName = file_name;
            entries.add(i).write_unaligned(fd);
        }
        let _ = GlobalUnlock(h);
        Ok((h, DescriptorInfo { bytes, entries: n }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e02_synth::{Case, CaseFile, Variants};

    fn case(files: &[(&str, u64)], advertise_size: bool) -> Case {
        Case {
            case_id: "T".into(),
            description: "test".into(),
            seed: 1,
            files: files
                .iter()
                .map(|(n, s)| CaseFile { name: (*n).into(), size: *s, advertised_size: None, stat_size: None, faults: vec![] })
                .collect(),
            variants: Variants { advertise_size, ..Default::default() },
            harness: Default::default(),
        }
    }

    /// Reads back the HGLOBAL and checks the FILEGROUPDESCRIPTORW layout byte by byte.
    #[test]
    fn layout_flags_sizes_and_names() {
        assert_eq!(size_of::<FILEDESCRIPTORW>(), 592, "packed FILEDESCRIPTORW size");
        let c = case(&[("a.bin", 5 * (1u64 << 30) + 7), ("dir/b.bin", 0)], true);
        let (h, info) = build(&c).unwrap();
        assert_eq!(info.entries, 2);
        assert_eq!(info.bytes, 4 + 2 * 592);
        unsafe {
            let base = GlobalLock(h) as *const u8;
            let group = base as *const FILEGROUPDESCRIPTORW;
            assert_eq!({ (*group).cItems }, 2);
            let fd0 = (base.add(4) as *const FILEDESCRIPTORW).read_unaligned();
            let fd1 = (base.add(4 + 592) as *const FILEDESCRIPTORW).read_unaligned();
            let flags = { fd0.dwFlags };
            assert_ne!(flags & FD_FILESIZE.0 as u32, 0);
            assert_ne!(flags & FD_UNICODE.0 as u32, 0);
            assert_eq!(({ fd0.nFileSizeHigh } as u64) << 32 | { fd0.nFileSizeLow } as u64, 5 * (1u64 << 30) + 7);
            let name1 = { fd1.cFileName };
            let len = name1.iter().position(|&c| c == 0).unwrap();
            assert_eq!(String::from_utf16_lossy(&name1[..len]), r"dir\b.bin");
            let _ = GlobalUnlock(h);
            let _ = GlobalFree(Some(h));
        }
    }

    #[test]
    fn global_unlock_reports_err_when_the_unlock_succeeds() {
        // HRESULT/return-code audit: GlobalUnlock returns FALSE with NO_ERROR when the lock
        // count reaches zero. windows-rs maps that to Err, so "?" on it would fail spuriously.
        // Every call site ignores the result.
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, 16).unwrap();
            assert!(!GlobalLock(h).is_null());
            let r = GlobalUnlock(h);
            assert!(r.is_err(), "unlocked to zero is reported as an error");
            assert_eq!(r.unwrap_err().code().0, 0, "with a success code");
            let _ = GlobalFree(Some(h));
        }
    }

    #[test]
    fn advertised_size_override_is_written() {
        let mut c = case(&[("a.bin", 100)], true);
        c.files[0].advertised_size = Some(250);
        let (h, _) = build(&c).unwrap();
        unsafe {
            let fd = (GlobalLock(h).cast::<u8>().add(4) as *const FILEDESCRIPTORW).read_unaligned();
            assert_eq!({ fd.nFileSizeLow }, 250);
            let _ = GlobalUnlock(h);
            let _ = GlobalFree(Some(h));
        }
    }

    #[test]
    fn size_flag_absent_when_not_advertised() {
        let (h, _) = build(&case(&[("a.bin", 10)], false)).unwrap();
        unsafe {
            let fd = (GlobalLock(h).cast::<u8>().add(4) as *const FILEDESCRIPTORW).read_unaligned();
            let flags = { fd.dwFlags };
            assert_eq!(flags & FD_FILESIZE.0 as u32, 0);
            assert_eq!({ fd.nFileSizeLow }, 0);
            let _ = GlobalUnlock(h);
            let _ = GlobalFree(Some(h));
        }
    }
}
