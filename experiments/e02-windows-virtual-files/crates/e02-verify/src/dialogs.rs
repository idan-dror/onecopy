//! Explorer file-operation dialogs: read-only observation, and one narrowly targeted button
//! invocation (experiment-only automation, D-041).
//!
//! Observation: every poll lists visible top-level windows that belong to an `explorer.exe`
//! process and have a dialog class (`OperationStatusWindow`, `#32770`). Windows that existed
//! when the watcher started are ignored. For new ones, UI Automation element names, control
//! types and automation ids are read, sanitized and logged.
//!
//! A dialog is "ours" if its owner chain leads to one of the run's own Explorer windows, or its
//! text mentions a run token (synthetic file name, run folder). The UI language is not assumed:
//! buttons are matched by Win32 control id (automation id, e.g. "2" = IDCANCEL), not by label.
//!
//! Invocation (`act`) re-validates the window first: it must still exist, belong to
//! `explorer.exe`, have an allowed class, and be ours. Otherwise nothing is invoked.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RPC_E_CHANGED_MODE};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationInvokePattern, TreeScope_Subtree, UIA_ButtonControlTypeId,
    UIA_HyperlinkControlTypeId, UIA_InvokePatternId, UIA_ProgressBarControlTypeId, UIA_SplitButtonControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GA_ROOTOWNER, GW_OWNER, GetAncestor, GetClassNameW, GetWindow, GetWindowTextW, GetWindowThreadProcessId,
    IsWindow, IsWindowVisible,
};
use windows::core::{BOOL, PWSTR};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

const DIALOG_CLASSES: &[&str] = &["OperationStatusWindow", "#32770"];
const MAX_ELEMENTS: i32 = 400;

#[derive(Clone, Debug)]
pub struct Win {
    pub hwnd: isize,
    pub class: String,
    pub title: String,
    pub owner: isize,
    pub root_owner: isize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Button {
    pub name: String,
    /// Win32 control id as UIA automation id ("1" = IDOK, "2" = IDCANCEL, ...); title-bar
    /// buttons have non-numeric ids.
    pub id: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogText {
    pub names: Vec<String>,
    pub buttons: Vec<Button>,
    pub progress_bar: bool,
}

/// Replaces identifying strings; keeps synthetic names and run folders.
pub struct Sanitizer {
    needles: Vec<(String, &'static str)>,
}

impl Sanitizer {
    pub fn new(test_root: &str) -> Self {
        let mut needles = Vec::new();
        let mut add = |v: Option<String>, label: &'static str| {
            if let Some(v) = v.filter(|v| v.len() >= 3) {
                needles.push((v.to_lowercase(), label));
            }
        };
        add(Some(test_root.trim_end_matches('\\').to_owned()), "<testroot>");
        add(std::env::var("USERPROFILE").ok(), "<profile>");
        add(std::env::var("USERNAME").ok(), "<user>");
        add(std::env::var("COMPUTERNAME").ok(), "<machine>");
        Self { needles }
    }

    pub fn clean(&self, s: &str) -> String {
        let mut out = s.to_owned();
        for (needle, label) in &self.needles {
            loop {
                let lower = out.to_lowercase();
                let Some(i) = lower.find(needle.as_str()) else { break };
                // Lowercasing can change byte lengths for some scripts; fall back to dropping the text.
                if lower.len() != out.len() {
                    return "<unsanitizable>".into();
                }
                out.replace_range(i..i + needle.len(), label);
            }
        }
        out.chars().filter(|c| !c.is_control()).take(300).collect()
    }
}

fn process_is_explorer(pid: u32) -> bool {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        ok && String::from_utf16_lossy(&buf[..len as usize]).to_lowercase().ends_with("\\explorer.exe")
    }
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn title_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Window texts of a window's child controls (Win32 WM_GETTEXT; works for hidden windows).
fn child_texts(hwnd: HWND) -> Vec<String> {
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        out.push(hwnd);
        BOOL((out.len() < 64) as i32)
    }
    let mut kids: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(Some(hwnd), Some(cb), LPARAM(&mut kids as *mut Vec<HWND> as isize));
    }
    kids.into_iter().map(title_of).filter(|s| !s.trim().is_empty()).collect()
}

/// Titles and child-control texts along a dialog's owner chain (up to four levels). Explorer's
/// copy error box names no file, but it is owned by the (possibly hidden) progress window of
/// the same operation, whose text does.
pub fn owner_chain_texts(w: &Win) -> (Vec<String>, Vec<String>) {
    let mut texts = Vec::new();
    let mut classes = Vec::new();
    let mut h = w.owner;
    for _ in 0..4 {
        if h == 0 || h == w.hwnd {
            break;
        }
        let hw = HWND(h as *mut _);
        classes.push(class_of(hw));
        texts.push(title_of(hw));
        texts.extend(child_texts(hw));
        h = unsafe { GetWindow(hw, GW_OWNER) }.map(|o| o.0 as isize).unwrap_or(0);
    }
    (texts, classes)
}

/// Visible top-level Explorer dialog windows.
pub fn explorer_dialogs() -> Vec<Win> {
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        out.push(hwnd);
        BOOL(1)
    }
    let mut all: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut all as *mut Vec<HWND> as isize));
    }
    all.into_iter()
        .filter(|&h| unsafe { IsWindowVisible(h) }.as_bool())
        .filter_map(|h| {
            let class = class_of(h);
            if !DIALOG_CLASSES.contains(&class.as_str()) {
                return None;
            }
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
            if !process_is_explorer(pid) {
                return None;
            }
            let owner = unsafe { GetWindow(h, GW_OWNER) }.map(|o| o.0 as isize).unwrap_or(0);
            let root_owner = unsafe { GetAncestor(h, GA_ROOTOWNER) }.0 as isize;
            Some(Win { hwnd: h.0 as isize, class, title: title_of(h), owner, root_owner })
        })
        .collect()
}

pub struct Uia {
    automation: IUIAutomation,
    owns_com: bool,
}

const BUTTON_TYPES: [windows::Win32::UI::Accessibility::UIA_CONTROLTYPE_ID; 3] =
    [UIA_ButtonControlTypeId, UIA_HyperlinkControlTypeId, UIA_SplitButtonControlTypeId];

impl Uia {
    pub fn new() -> AnyResult<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // W-01: S_FALSE ("already initialized") is success and still needs CoUninitialize.
        // RPC_E_CHANGED_MODE means another apartment model is active: do not uninitialize.
        let owns_com = hr.is_ok();
        if hr.is_err() && hr != RPC_E_CHANGED_MODE {
            return Err(format!("CoInitializeEx failed: {:#010x}", hr.0).into());
        }
        let automation: IUIAutomation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? };
        Ok(Self { automation, owns_com })
    }

    fn elements(&self, hwnd: isize) -> AnyResult<Vec<windows::Win32::UI::Accessibility::IUIAutomationElement>> {
        unsafe {
            let el = self.automation.ElementFromHandle(HWND(hwnd as *mut _))?;
            let cond = self.automation.CreateTrueCondition()?;
            let arr = el.FindAll(TreeScope_Subtree, &cond)?;
            let n = arr.Length()?.min(MAX_ELEMENTS);
            Ok((0..n).filter_map(|i| arr.GetElement(i).ok()).collect())
        }
    }

    pub fn read(&self, hwnd: isize) -> AnyResult<DialogText> {
        let mut out = DialogText::default();
        for e in self.elements(hwnd)? {
            unsafe {
                let name = e.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let ct = e.CurrentControlType().unwrap_or_default();
                if ct == UIA_ProgressBarControlTypeId {
                    out.progress_bar = true;
                }
                if BUTTON_TYPES.contains(&ct) {
                    out.buttons.push(Button {
                        name: name.clone(),
                        id: e.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default(),
                        enabled: e.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false),
                    });
                }
                if !name.trim().is_empty() && !out.names.contains(&name) {
                    out.names.push(name);
                }
            }
        }
        Ok(out)
    }

    /// Invokes the first enabled button whose automation id is in `ids` (in preference order).
    /// Returns (id, name) of the invoked button.
    pub fn invoke(&self, hwnd: isize, ids: &[String]) -> AnyResult<Option<(String, String)>> {
        let elements = self.elements(hwnd)?;
        for wanted in ids {
            for e in &elements {
                unsafe {
                    if !BUTTON_TYPES.contains(&e.CurrentControlType().unwrap_or_default()) {
                        continue;
                    }
                    let id = e.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default();
                    if id != *wanted || !e.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false) {
                        continue;
                    }
                    let name = e.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                    let pattern: IUIAutomationInvokePattern = e.GetCurrentPatternAs(UIA_InvokePatternId)?;
                    pattern.Invoke()?;
                    return Ok(Some((id, name)));
                }
            }
        }
        Ok(None)
    }
}

impl Drop for Uia {
    fn drop(&mut self) {
        if self.owns_com {
            unsafe { CoUninitialize() };
        }
    }
}

/// True if any text contains any token (case-insensitive).
pub fn mentions(texts: &[String], tokens: &[String]) -> bool {
    tokens.iter().filter(|t| t.len() >= 6).any(|t| {
        let t = t.to_lowercase();
        texts.iter().any(|s| s.to_lowercase().contains(&t))
    })
}

/// Why a dialog is attributed to this run, if it is: its owner chain reaches one of the run's
/// own Explorer windows, its own text names a run token, or its owner chain's text does.
pub fn ours_by(w: &Win, text: &DialogText, tokens: &[String], owners: &[isize], owner_texts: &[String]) -> Option<&'static str> {
    if owners.iter().any(|o| *o != 0 && (*o == w.root_owner || *o == w.owner)) {
        Some("owner")
    } else if mentions(&text.names, tokens) || mentions(std::slice::from_ref(&w.title), tokens) {
        Some("token")
    } else if mentions(owner_texts, tokens) {
        Some("owner_text")
    } else {
        None
    }
}

fn read_owners(path: Option<&Path>) -> Vec<isize> {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Vec<i64>>(s.trim_start_matches('\u{feff}')).ok())
        .map(|v| v.into_iter().map(|x| x as isize).collect())
        .unwrap_or_default()
}

#[derive(Clone)]
struct Tracked {
    class: String,
    title: String,
    text: DialogText,
    ours: Option<&'static str>,
    first_ms: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn watch(
    out: &Path,
    state: &Path,
    stop_file: &Path,
    parent: Option<&crate::win::ProcessWatch>,
    tokens: &[String],
    owners_file: Option<&Path>,
    test_root: &str,
    interval_ms: u64,
    timeout_secs: u64,
) -> AnyResult<()> {
    let uia = Uia::new()?;
    let san = Sanitizer::new(test_root);
    let start = Instant::now();
    let preexisting: Vec<isize> = explorer_dialogs().into_iter().map(|w| w.hwnd).collect();
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out)?);
    writeln!(writer, "{}", json!({ "t_ms": 0.0, "event": "start", "preexisting_dialogs": preexisting.len() }))?;
    let mut tracked: BTreeMap<isize, Tracked> = BTreeMap::new();
    let mut last_read: BTreeMap<isize, Instant> = BTreeMap::new();
    let clean_text = |t: &DialogText| DialogText {
        names: t.names.iter().map(|s| san.clean(s)).collect(),
        buttons: t.buttons.iter().map(|b| Button { name: san.clean(&b.name), id: san.clean(&b.id), enabled: b.enabled }).collect(),
        progress_bar: t.progress_bar,
    };
    loop {
        let t_ms = start.elapsed().as_secs_f64() * 1000.0;
        let owners = read_owners(owners_file);
        let now: Vec<Win> = explorer_dialogs().into_iter().filter(|w| !preexisting.contains(&w.hwnd)).collect();
        for w in &now {
            let first = !tracked.contains_key(&w.hwnd);
            let due = first || last_read.get(&w.hwnd).is_none_or(|t| t.elapsed() >= Duration::from_millis(1000));
            if !due {
                continue;
            }
            last_read.insert(w.hwnd, Instant::now());
            let raw = uia.read(w.hwnd).unwrap_or_default();
            let (owner_texts, owner_classes) = owner_chain_texts(w);
            let ours = ours_by(w, &raw, tokens, &owners, &owner_texts);
            let text = clean_text(&raw);
            let title = san.clean(&w.title);
            match tracked.get_mut(&w.hwnd) {
                None => {
                    writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "appear", "hwnd": w.hwnd, "class": w.class, "title": title,
                        "owner": w.owner, "root_owner": w.root_owner, "owner_classes": owner_classes,
                        "owner_texts": owner_texts.iter().map(|s| san.clean(s)).collect::<Vec<_>>(),
                        "ours": ours.is_some(), "ours_by": ours,
                        "progress_bar": text.progress_bar, "names": text.names, "buttons": text.buttons }))?;
                    tracked.insert(w.hwnd, Tracked { class: w.class.clone(), title, text, ours, first_ms: t_ms });
                }
                Some(tr) => {
                    let ours = tr.ours.or(ours);
                    if tr.text != text || tr.title != title || tr.ours != ours {
                        writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "change", "hwnd": w.hwnd, "title": title,
                            "ours": ours.is_some(), "ours_by": ours, "progress_bar": text.progress_bar,
                            "names": text.names, "buttons": text.buttons }))?;
                        tr.text = text;
                        tr.title = title;
                        tr.ours = ours;
                    }
                }
            }
        }
        let gone: Vec<isize> = tracked.keys().copied().filter(|h| !now.iter().any(|w| w.hwnd == *h)).collect();
        for h in gone {
            let tr = tracked.remove(&h).unwrap();
            last_read.remove(&h);
            writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "gone", "hwnd": h, "class": tr.class, "ours": tr.ours.is_some(),
                "progress_bar": tr.text.progress_bar, "lifetime_ms": t_ms - tr.first_ms }))?;
        }
        writer.flush()?;
        let snapshot: Vec<Value> = tracked
            .iter()
            .map(|(h, tr)| {
                let enabled: Vec<&Button> = tr.text.buttons.iter().filter(|b| b.enabled).collect();
                json!({ "hwnd": h, "class": tr.class, "title": tr.title, "ours": tr.ours.is_some(), "ours_by": tr.ours,
                        "progress_bar": tr.text.progress_bar,
                        "button_ids": enabled.iter().map(|b| b.id.clone()).collect::<Vec<_>>(),
                        "buttons": enabled.iter().map(|b| b.name.clone()).collect::<Vec<_>>() })
            })
            .collect();
        let tmp = state.with_extension("tmp");
        // The run script may hold the state file open while reading; a failed replace is retried next poll.
        if std::fs::write(&tmp, serde_json::to_vec(&json!({ "t_ms": t_ms, "dialogs": snapshot }))?).is_ok() {
            let _ = std::fs::rename(&tmp, state);
        }
        if stop_file.exists() || parent.is_some_and(|p| p.exited()) || start.elapsed() > Duration::from_secs(timeout_secs) {
            break;
        }
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
    Ok(())
}

/// Invokes one button (by control id) in one of this run's dialogs, after re-validating it.
pub fn act(hwnd: isize, ids: &[String], tokens: &[String], owners: &[isize], test_root: &str) -> AnyResult<Value> {
    let refuse = |why: &str| Ok(json!({ "invoked": null, "refused": why }));
    let h = HWND(hwnd as *mut _);
    if !unsafe { IsWindow(Some(h)) }.as_bool() {
        return refuse("window no longer exists");
    }
    let Some(w) = explorer_dialogs().into_iter().find(|w| w.hwnd == hwnd) else {
        return refuse("not a visible Explorer dialog window");
    };
    let uia = Uia::new()?;
    let text = uia.read(hwnd)?;
    let (owner_texts, _) = owner_chain_texts(&w);
    let Some(why) = ours_by(&w, &text, tokens, owners, &owner_texts) else {
        return refuse("dialog is not attributed to this run (owner or token)");
    };
    let san = Sanitizer::new(test_root);
    match uia.invoke(hwnd, ids)? {
        Some((id, name)) => Ok(json!({ "invoked": id, "name": san.clean(&name), "class": w.class, "ours_by": why, "refused": null })),
        None => refuse("no enabled button with a requested control id"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(owner: isize, root: isize) -> Win {
        Win { hwnd: 50, class: "#32770".into(), title: "x".into(), owner, root_owner: root }
    }

    #[test]
    fn mentions_needs_a_long_enough_run_token() {
        let texts = vec!["Copying 1 item to paste-target-1".to_owned()];
        assert!(mentions(&texts, &["paste-target-1".into()]));
        assert!(!mentions(&texts, &["item".into()]), "short tokens are ignored");
        assert!(!mentions(&texts, &["e02-ERR-01-f0.bin".into()]));
    }

    #[test]
    fn attribution_by_owner_chain_or_token_only() {
        let t = DialogText { names: vec!["Unspecified error".into()], ..Default::default() };
        assert_eq!(ours_by(&win(0, 7), &t, &[], &[7], &[]), Some("owner"));
        assert_eq!(ours_by(&win(7, 9), &t, &[], &[7], &[]), Some("owner"));
        assert_eq!(ours_by(&win(0, 9), &t, &["paste-target-1".into()], &[7], &[]), None, "a foreign dialog");
        assert_eq!(ours_by(&win(0, 0), &t, &[], &[0], &[]), None, "a null owner never matches");
        let named = DialogText { names: vec!["Copying 'e02-ERR-01-f0.bin'".into()], ..Default::default() };
        assert_eq!(ours_by(&win(0, 9), &named, &["e02-ERR-01-f0.bin".into()], &[7], &[]), Some("token"));
        let owner_text = vec!["Copying 'e02-ERR-01-f0.bin'".to_owned()];
        assert_eq!(ours_by(&win(3, 9), &t, &["e02-ERR-01-f0.bin".into()], &[7], &owner_text), Some("owner_text"));
        assert_eq!(ours_by(&win(3, 9), &t, &["e02-ERR-02-f0.bin".into()], &[7], &owner_text), None, "another run's file");
    }

    #[test]
    fn sanitizer_replaces_test_root_case_insensitively() {
        let s = Sanitizer::new("D:\\OneCopyE02");
        assert_eq!(s.clean("Copying to d:\\onecopye02\\dest\\x"), "Copying to <testroot>\\dest\\x");
    }
}
