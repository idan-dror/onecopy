//! IDataObject offering synthetic virtual files:
//! CFSTR_FILEDESCRIPTORW (HGLOBAL), CFSTR_FILECONTENTS (IStream per lindex),
//! CFSTR_PREFERREDDROPEFFECT (DWORD copy) and a private run marker. No CF_HDROP.

use crate::{descriptor, log, stats, stream::VirtualFileStream};
use e02_synth::{Case, Synth};
use serde_json::json;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use windows::Win32::Foundation::{
    DATA_S_SAMEFORMATETC, DV_E_FORMATETC, DV_E_LINDEX, DV_E_TYMED, E_INVALIDARG, E_POINTER, HGLOBAL, OLE_E_ADVISENOTSUPPORTED,
    S_OK,
};
use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, STGMEDIUM,
    STGMEDIUM_0, TYMED_HGLOBAL, TYMED_ISTREAM,
};
use windows::Win32::System::DataExchange::{GetClipboardFormatNameW, RegisterClipboardFormatW};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::ReleaseStgMedium;
use windows::Win32::UI::Shell::{
    CFSTR_FILECONTENTS, CFSTR_FILEDESCRIPTORW, CFSTR_PREFERREDDROPEFFECT, SHCreateStdEnumFmtEtc,
};
use windows::core::{BOOL, HRESULT, PCWSTR, Ref, Result, implement, w};

/// DROPEFFECT_COPY.
const DROPEFFECT_COPY: u32 = 1;

pub struct Formats {
    pub descriptor: u16,
    pub contents: u16,
    pub preferred_effect: u16,
    pub marker: u16,
}

impl Formats {
    pub fn register() -> Result<Self> {
        let reg = |name: PCWSTR| -> Result<u16> {
            let id = unsafe { RegisterClipboardFormatW(name) };
            if id == 0 { Err(windows::core::Error::from_thread()) } else { Ok(id as u16) }
        };
        Ok(Self {
            descriptor: reg(CFSTR_FILEDESCRIPTORW)?,
            contents: reg(CFSTR_FILECONTENTS)?,
            preferred_effect: reg(CFSTR_PREFERREDDROPEFFECT)?,
            marker: reg(w!("OneCopyE02Marker"))?,
        })
    }
}

/// Human-readable clipboard format name for logs (format names are not content).
pub fn format_name(cf: u16) -> String {
    if cf >= 0xC000 {
        let mut buf = [0u16; 128];
        let len = unsafe { GetClipboardFormatNameW(cf as u32, &mut buf) };
        if len > 0 {
            return String::from_utf16_lossy(&buf[..len as usize]);
        }
    }
    match cf {
        1 => "CF_TEXT".into(),
        13 => "CF_UNICODETEXT".into(),
        15 => "CF_HDROP".into(),
        _ => format!("cf#{cf}"),
    }
}

pub struct Shared {
    pub case: Case,
    pub synth: Synth,
    pub formats: Formats,
    pub run_id: String,
    next_stream_id: AtomicU64,
}

impl Shared {
    pub fn new(case: Case, formats: Formats, run_id: String) -> Arc<Self> {
        let synth = Synth::new(case.seed);
        Arc::new(Self { case, synth, formats, run_id, next_stream_id: AtomicU64::new(1) })
    }
}

#[implement(IDataObject)]
pub struct VirtualFileDataObject {
    shared: Arc<Shared>,
}

impl VirtualFileDataObject {
    pub fn create(shared: Arc<Shared>) -> IDataObject {
        VirtualFileDataObject { shared }.into()
    }

    fn offered(&self) -> [FORMATETC; 4] {
        let f = &self.shared.formats;
        let fe = |cf: u16, tymed: i32| FORMATETC {
            cfFormat: cf,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: tymed as u32,
        };
        [
            fe(f.descriptor, TYMED_HGLOBAL.0),
            fe(f.contents, TYMED_ISTREAM.0),
            fe(f.preferred_effect, TYMED_HGLOBAL.0),
            fe(f.marker, TYMED_HGLOBAL.0),
        ]
    }

    /// Validates a request; returns the resolved file index for contents requests.
    fn check(&self, fe: &FORMATETC) -> std::result::Result<Option<u32>, HRESULT> {
        let f = &self.shared.formats;
        let n = self.shared.case.files.len() as i32;
        let wants = |t: i32| fe.tymed & (t as u32) != 0;
        if fe.cfFormat == f.contents {
            if !wants(TYMED_ISTREAM.0) {
                return Err(DV_E_TYMED);
            }
            // lindex -1 is accepted only for single-file objects.
            let index = if fe.lindex == -1 && n == 1 { 0 } else { fe.lindex };
            if index < 0 || index >= n {
                return Err(DV_E_LINDEX);
            }
            Ok(Some(index as u32))
        } else if [f.descriptor, f.preferred_effect, f.marker].contains(&fe.cfFormat) {
            if !wants(TYMED_HGLOBAL.0) { Err(DV_E_TYMED) } else { Ok(None) }
        } else {
            Err(DV_E_FORMATETC)
        }
    }
}

fn hglobal_medium(h: HGLOBAL) -> STGMEDIUM {
    STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: h }, pUnkForRelease: ManuallyDrop::new(None) }
}

fn hglobal_from_bytes(bytes: &[u8]) -> Result<HGLOBAL> {
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))?;
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        let _ = GlobalUnlock(h);
        Ok(h)
    }
}

impl IDataObject_Impl for VirtualFileDataObject_Impl {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> Result<STGMEDIUM> {
        stats::count("IDataObject::GetData");
        let fe = unsafe { pformatetcin.as_ref() }.ok_or(windows::core::Error::from(E_POINTER))?;
        let name = format_name(fe.cfFormat);
        let checked = self.check(fe);
        let result: Result<STGMEDIUM> = match checked {
            Err(hr) => Err(hr.into()),
            Ok(Some(index)) => {
                let now = log::now_ms();
                stats::with(|s| {
                    s.first_contents_get_data_ms.get_or_insert(now);
                });
                let file = &self.shared.case.files[index as usize];
                let id = self.shared.next_stream_id.fetch_add(1, Ordering::Relaxed);
                let stream = VirtualFileStream::create(id, index, &file.name, file.size, &self.shared.synth);
                Ok(STGMEDIUM {
                    tymed: TYMED_ISTREAM.0 as u32,
                    u: STGMEDIUM_0 { pstm: ManuallyDrop::new(Some(stream)) },
                    pUnkForRelease: ManuallyDrop::new(None),
                })
            }
            Ok(None) if fe.cfFormat == self.shared.formats.descriptor => {
                let now = log::now_ms();
                stats::with(|s| {
                    s.first_descriptor_get_data_ms.get_or_insert(now);
                });
                descriptor::build(&self.shared.case).map(|(h, info)| {
                    log::event("descriptor_built", json!({ "entries": info.entries, "bytes": info.bytes }));
                    hglobal_medium(h)
                })
            }
            Ok(None) if fe.cfFormat == self.shared.formats.preferred_effect => {
                hglobal_from_bytes(&DROPEFFECT_COPY.to_le_bytes()).map(hglobal_medium)
            }
            Ok(None) => hglobal_from_bytes(self.shared.run_id.as_bytes()).map(hglobal_medium),
        };
        log::event(
            "get_data",
            json!({
                "format": name, "lindex": fe.lindex, "tymed_requested": fe.tymed, "aspect": fe.dwAspect,
                "ok": result.is_ok(),
                "hr": result.as_ref().err().map(|e| format!("{:#010x}", e.code().0)),
            }),
        );
        result
    }

    fn GetDataHere(&self, pformatetc: *const FORMATETC, _pmedium: *mut STGMEDIUM) -> Result<()> {
        stats::count("IDataObject::GetDataHere");
        let name = unsafe { pformatetc.as_ref() }.map(|fe| format_name(fe.cfFormat));
        log::event("get_data_here", json!({ "format": name, "hr": format!("{:#010x}", DV_E_FORMATETC.0) }));
        Err(DV_E_FORMATETC.into())
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        stats::count("IDataObject::QueryGetData");
        let Some(fe) = (unsafe { pformatetc.as_ref() }) else { return E_POINTER };
        let hr = match self.check(fe) {
            Ok(_) => S_OK,
            Err(hr) => hr,
        };
        log::event(
            "query_get_data",
            json!({ "format": format_name(fe.cfFormat), "lindex": fe.lindex, "tymed": fe.tymed, "hr": format!("{:#010x}", hr.0) }),
        );
        hr
    }

    fn GetCanonicalFormatEtc(&self, _pformatectin: *const FORMATETC, pformatetcout: *mut FORMATETC) -> HRESULT {
        stats::count("IDataObject::GetCanonicalFormatEtc");
        if !pformatetcout.is_null() {
            unsafe { (*pformatetcout).ptd = std::ptr::null_mut() };
        }
        DATA_S_SAMEFORMATETC
    }

    fn SetData(&self, pformatetc: *const FORMATETC, pmedium: *const STGMEDIUM, frelease: BOOL) -> Result<()> {
        stats::count("IDataObject::SetData");
        let Some(fe) = (unsafe { pformatetc.as_ref() }) else { return Err(E_INVALIDARG.into()) };
        // Shell communication formats carry a DWORD in an HGLOBAL; log only that value.
        let mut dword: Option<u32> = None;
        if let Some(medium) = unsafe { pmedium.as_ref() } {
            if medium.tymed == TYMED_HGLOBAL.0 as u32 {
                unsafe {
                    let h = medium.u.hGlobal;
                    if GlobalSize(h) >= 4 {
                        let p = GlobalLock(h) as *const u32;
                        if !p.is_null() {
                            dword = Some(p.read_unaligned());
                            let _ = GlobalUnlock(h);
                        }
                    }
                }
            }
            if frelease.as_bool() {
                unsafe { ReleaseStgMedium(pmedium as *mut STGMEDIUM) };
            }
        }
        log::event(
            "set_data",
            json!({ "format": format_name(fe.cfFormat), "tymed": fe.tymed, "dword": dword, "release": frelease.as_bool() }),
        );
        Ok(())
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> Result<IEnumFORMATETC> {
        stats::count("IDataObject::EnumFormatEtc");
        log::event("enum_format_etc", json!({ "direction": dwdirection }));
        // DATADIR_GET = 1; nothing is offered for DATADIR_SET.
        if dwdirection != 1 {
            return Err(windows::Win32::Foundation::E_NOTIMPL.into());
        }
        unsafe { SHCreateStdEnumFmtEtc(&self.offered()) }
    }

    fn DAdvise(&self, _pformatetc: *const FORMATETC, _advf: u32, _padvsink: Ref<'_, IAdviseSink>) -> Result<u32> {
        stats::count("IDataObject::DAdvise");
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _dwconnection: u32) -> Result<()> {
        stats::count("IDataObject::DUnadvise");
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        stats::count("IDataObject::EnumDAdvise");
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}
