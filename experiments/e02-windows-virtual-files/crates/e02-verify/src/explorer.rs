//! Experiment-owned Explorer window handling (driver D2c and cleanup).
//!
//! Safety rule: the harness may only close an Explorer window that it opened for
//! this run. A window is treated as experiment-owned only if
//!   1. it currently shows the run's unique destination folder,
//!   2. its top-level window did NOT exist in the snapshot taken before the run
//!      (Explorer may open folders as a tab in a user's existing window),
//!   3. it is the same top-level window recorded when the folder was opened, and
//!   4. it hosts no other tab (the user may have opened more tabs in it).
//!
//! Snapshots record only window handles and tab counts, never folder paths.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Snapshot {
    pub top_windows: Vec<TopWindow>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopWindow {
    pub top_hwnd: i64,
    pub tabs: u32,
}

impl Snapshot {
    pub fn tops(&self) -> BTreeSet<i64> {
        self.top_windows.iter().map(|w| w.top_hwnd).collect()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum CloseDecision {
    /// Experiment-owned window with a single tab: safe to close.
    Close,
    /// No window currently shows the destination folder.
    NotFound,
    /// The folder opened as a tab inside a window that existed before the run.
    KeepPreexistingWindow,
    /// The window now hosts other tabs as well.
    KeepSharedWithOtherTabs,
    /// A different top-level window shows the folder than the one recorded at open time.
    KeepUnexpectedWindow,
}

pub fn decide(found_top: Option<i64>, tabs_on_top: u32, expected_top: Option<i64>, preexisting: &BTreeSet<i64>) -> CloseDecision {
    let Some(top) = found_top else { return CloseDecision::NotFound };
    if preexisting.contains(&top) {
        return CloseDecision::KeepPreexistingWindow;
    }
    if expected_top.is_some_and(|e| e != top) {
        return CloseDecision::KeepUnexpectedWindow;
    }
    if tabs_on_top != 1 {
        return CloseDecision::KeepSharedWithOtherTabs;
    }
    CloseDecision::Close
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pre(v: &[i64]) -> BTreeSet<i64> {
        v.iter().copied().collect()
    }

    #[test]
    fn closes_only_new_single_tab_window_matching_expected() {
        assert_eq!(decide(Some(10), 1, Some(10), &pre(&[1, 2])), CloseDecision::Close);
        assert_eq!(decide(Some(10), 1, None, &pre(&[])), CloseDecision::Close);
    }

    #[test]
    fn never_closes_preexisting_user_window_even_if_it_shows_the_folder() {
        assert_eq!(decide(Some(2), 1, Some(2), &pre(&[1, 2])), CloseDecision::KeepPreexistingWindow);
        assert_eq!(decide(Some(2), 3, None, &pre(&[2])), CloseDecision::KeepPreexistingWindow);
    }

    #[test]
    fn keeps_window_that_gained_other_tabs() {
        assert_eq!(decide(Some(10), 2, Some(10), &pre(&[])), CloseDecision::KeepSharedWithOtherTabs);
        assert_eq!(decide(Some(10), 0, Some(10), &pre(&[])), CloseDecision::KeepSharedWithOtherTabs);
    }

    #[test]
    fn keeps_unexpected_window() {
        assert_eq!(decide(Some(11), 1, Some(10), &pre(&[])), CloseDecision::KeepUnexpectedWindow);
    }

    #[test]
    fn not_found() {
        assert_eq!(decide(None, 0, Some(10), &pre(&[10])), CloseDecision::NotFound);
    }
}

#[cfg(windows)]
pub mod win {
    use super::{CloseDecision, Snapshot, TopWindow, decide};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Shell::{IFolderView, IPersistFolder2, IShellBrowser, IShellWindows, SHGetPathFromIDListW, ShellWindows};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE, WM_COMMAND};
    use windows::core::{GUID, Interface};

    type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

    /// SID_STopLevelBrowser (shlguid.h).
    const SID_STOP_LEVEL_BROWSER: GUID = GUID::from_u128(0x4c96be40_915c_11cf_99d3_00aa004ae837);
    /// Explorer folder-view Paste command id (undocumented; experiment-only driver, D-041).
    /// Only ever posted to the run's own, freshly created, empty test folder view.
    const DEFVIEW_PASTE_COMMAND: usize = 0x701A;

    struct ShellWin {
        top: i64,
        view: HWND,
        path: Option<String>,
    }

    fn init_com() -> AnyResult<()> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        Ok(())
    }

    /// All Explorer folder windows/tabs. Paths are only compared, never emitted.
    fn enumerate() -> AnyResult<Vec<ShellWin>> {
        let mut out = Vec::new();
        unsafe {
            let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
            for i in 0..windows.Count()? {
                let Ok(disp) = windows.Item(&VARIANT::from(i)) else { continue };
                let Ok(provider) = disp.cast::<IServiceProvider>() else { continue };
                let Ok(browser) = provider.QueryService::<IShellBrowser>(&SID_STOP_LEVEL_BROWSER) else { continue };
                let Ok(top) = browser.GetWindow() else { continue };
                let Ok(view) = browser.QueryActiveShellView() else { continue };
                let Ok(view_hwnd) = view.GetWindow() else { continue };
                let path = view
                    .cast::<IFolderView>()
                    .and_then(|fv| fv.GetFolder::<IPersistFolder2>())
                    .and_then(|pf| pf.GetCurFolder())
                    .ok()
                    .and_then(|pidl| {
                        let mut buf = [0u16; 260];
                        let ok = SHGetPathFromIDListW(pidl, &mut buf).as_bool();
                        CoTaskMemFree(Some(pidl as *const _));
                        ok.then(|| {
                            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                            String::from_utf16_lossy(&buf[..len])
                        })
                    });
                out.push(ShellWin { top: top.0 as i64, view: view_hwnd, path });
            }
        }
        Ok(out)
    }

    fn same_path(a: &str, b: &Path) -> bool {
        let norm = |s: &str| s.trim_end_matches('\\').to_lowercase();
        norm(a) == norm(&b.to_string_lossy())
    }

    fn tabs_per_top(all: &[ShellWin]) -> BTreeMap<i64, u32> {
        let mut m = BTreeMap::new();
        for w in all {
            *m.entry(w.top).or_insert(0) += 1;
        }
        m
    }

    /// The (single) shell window showing `dest`, plus the tab count of its top-level window.
    fn find(dest: &Path) -> AnyResult<Option<(i64, HWND, u32, usize)>> {
        let all = enumerate()?;
        let tabs = tabs_per_top(&all);
        let matches: Vec<&ShellWin> = all.iter().filter(|w| w.path.as_deref().is_some_and(|p| same_path(p, dest))).collect();
        Ok(matches.first().map(|w| (w.top, w.view, tabs.get(&w.top).copied().unwrap_or(0), matches.len())))
    }

    pub fn snapshot() -> AnyResult<Value> {
        init_com()?;
        let tabs = tabs_per_top(&enumerate()?);
        let snap = Snapshot { top_windows: tabs.into_iter().map(|(top_hwnd, tabs)| TopWindow { top_hwnd, tabs }).collect() };
        Ok(serde_json::to_value(snap)?)
    }

    fn load_snapshot(path: &Path) -> AnyResult<Snapshot> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
    }

    /// Waits for the window showing `dest` and reports whether its top-level window is new.
    pub fn find_owned(dest: &Path, pre_snapshot: &Path, timeout_secs: u64) -> AnyResult<Value> {
        init_com()?;
        let pre = load_snapshot(pre_snapshot)?.tops();
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if let Some((top, _view, tabs, count)) = find(dest)? {
                return Ok(json!({
                    "found": true, "top_hwnd": top, "tabs_on_top": tabs, "windows_showing_dest": count,
                    "top_preexisting": pre.contains(&top),
                }));
            }
            if Instant::now() > deadline {
                return Ok(json!({ "found": false }));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Posts Paste to the folder view, but only in the window recorded at open time.
    pub fn paste(dest: &Path, expected_top: i64) -> AnyResult<Value> {
        init_com()?;
        let mut last_error = String::from("window not found");
        for attempt in 1..=5 {
            std::thread::sleep(Duration::from_millis(750));
            match find(dest)? {
                Some((top, view, _tabs, _)) if top == expected_top => {
                    match unsafe { PostMessageW(Some(view), WM_COMMAND, WPARAM(DEFVIEW_PASTE_COMMAND), LPARAM(0)) } {
                        Ok(()) => {
                            return Ok(json!({ "posted": "WM_COMMAND", "command_id": DEFVIEW_PASTE_COMMAND, "target": "folder_view", "attempt": attempt }));
                        }
                        Err(e) => last_error = e.to_string(),
                    }
                }
                Some(_) => last_error = "folder shown in an unexpected top-level window; not posting".into(),
                None => last_error = "window not found".into(),
            }
        }
        Err(format!("could not post Paste after 5 attempts: {last_error}").into())
    }

    /// Closes the run's window only when `decide` says it is experiment-owned and unshared.
    pub fn close_owned(dest: &Path, pre_snapshot: &Path, expected_top: Option<i64>) -> AnyResult<Value> {
        init_com()?;
        let pre = load_snapshot(pre_snapshot)?.tops();
        let found = find(dest)?;
        let decision = decide(found.map(|f| f.0), found.map(|f| f.2).unwrap_or(0), expected_top, &pre);
        let mut closed = false;
        if decision == CloseDecision::Close {
            let top = HWND(found.expect("found").0 as isize as *mut core::ffi::c_void);
            unsafe { PostMessageW(Some(top), WM_CLOSE, WPARAM(0), LPARAM(0))? };
            closed = true;
        }
        Ok(json!({ "decision": decision, "closed": closed, "tabs_on_top": found.map(|f| f.2) }))
    }
}
