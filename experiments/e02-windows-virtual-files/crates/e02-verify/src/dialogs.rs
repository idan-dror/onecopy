//! Explorer file-operation dialogs: read-only observation, and one narrowly targeted button
//! invocation (experiment-only automation, D-041).
//!
//! Observation: every poll lists visible top-level windows that belong to an `explorer.exe`
//! process and have a dialog class (`OperationStatusWindow`, `#32770`). Windows that existed
//! when the watcher started are ignored. For new ones, UI Automation element names are read,
//! sanitized (test root, profile, user and machine names replaced) and logged.
//!
//! Invocation (`act`) re-validates the window first: it must still exist, belong to
//! `explorer.exe`, have an allowed class, and its text must contain one of this run's tokens
//! (unique destination folder or synthetic file name). Otherwise nothing is invoked.

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
    UIA_HyperlinkControlTypeId, UIA_InvokePatternId, UIA_SplitButtonControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible};
use windows::core::{BOOL, PWSTR};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

const DIALOG_CLASSES: &[&str] = &["OperationStatusWindow", "#32770"];
const MAX_ELEMENTS: i32 = 400;

/// (element names, button-like elements as (name, enabled)).
pub type DialogText = (Vec<String>, Vec<(String, bool)>);

#[derive(Clone, Debug)]
pub struct Win {
    pub hwnd: isize,
    pub class: String,
    pub title: String,
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
            Some(Win { hwnd: h.0 as isize, class, title: title_of(h) })
        })
        .collect()
}

pub struct Uia {
    automation: IUIAutomation,
    owns_com: bool,
}

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

    /// (all element names, button-like elements as (name, enabled)).
    pub fn read(&self, hwnd: isize) -> AnyResult<DialogText> {
        unsafe {
            let el = self.automation.ElementFromHandle(HWND(hwnd as *mut _))?;
            let cond = self.automation.CreateTrueCondition()?;
            let arr = el.FindAll(TreeScope_Subtree, &cond)?;
            let n = arr.Length()?.min(MAX_ELEMENTS);
            let mut names = Vec::new();
            let mut buttons = Vec::new();
            for i in 0..n {
                let Ok(e) = arr.GetElement(i) else { continue };
                let name = e.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let ct = e.CurrentControlType().unwrap_or_default();
                if [UIA_ButtonControlTypeId, UIA_HyperlinkControlTypeId, UIA_SplitButtonControlTypeId].contains(&ct) {
                    buttons.push((name.clone(), e.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false)));
                }
                if !name.trim().is_empty() && !names.contains(&name) {
                    names.push(name);
                }
            }
            Ok((names, buttons))
        }
    }

    /// Invokes the first enabled button-like element whose name equals one of `names`
    /// (case-insensitive). Returns the invoked name.
    pub fn invoke(&self, hwnd: isize, names: &[String]) -> AnyResult<Option<String>> {
        unsafe {
            let el = self.automation.ElementFromHandle(HWND(hwnd as *mut _))?;
            let cond = self.automation.CreateTrueCondition()?;
            let arr = el.FindAll(TreeScope_Subtree, &cond)?;
            let n = arr.Length()?.min(MAX_ELEMENTS);
            for wanted in names {
                for i in 0..n {
                    let Ok(e) = arr.GetElement(i) else { continue };
                    let ct = e.CurrentControlType().unwrap_or_default();
                    if ![UIA_ButtonControlTypeId, UIA_HyperlinkControlTypeId, UIA_SplitButtonControlTypeId].contains(&ct) {
                        continue;
                    }
                    let name = e.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                    if !name.trim().eq_ignore_ascii_case(wanted.trim()) || !e.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false) {
                        continue;
                    }
                    let pattern: IUIAutomationInvokePattern = e.GetCurrentPatternAs(UIA_InvokePatternId)?;
                    pattern.Invoke()?;
                    return Ok(Some(name));
                }
            }
            Ok(None)
        }
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

#[derive(Clone)]
struct Tracked {
    class: String,
    title: String,
    names: Vec<String>,
    buttons: Vec<(String, bool)>,
    ours: bool,
    first_ms: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn watch(
    out: &Path,
    state: &Path,
    stop_file: &Path,
    parent: Option<&crate::win::ProcessWatch>,
    tokens: &[String],
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
    loop {
        let t_ms = start.elapsed().as_secs_f64() * 1000.0;
        let now: Vec<Win> = explorer_dialogs().into_iter().filter(|w| !preexisting.contains(&w.hwnd)).collect();
        for w in &now {
            let due = last_read.get(&w.hwnd).is_none_or(|t| t.elapsed() >= Duration::from_millis(1000));
            if !due {
                continue;
            }
            last_read.insert(w.hwnd, Instant::now());
            let (names, buttons) = uia.read(w.hwnd).unwrap_or_default();
            let names: Vec<String> = names.iter().map(|s| san.clean(s)).collect();
            let buttons: Vec<(String, bool)> = buttons.iter().map(|(s, e)| (san.clean(s), *e)).collect();
            let ours = mentions(&names, tokens) || mentions(std::slice::from_ref(&w.title), tokens);
            let title = san.clean(&w.title);
            match tracked.get_mut(&w.hwnd) {
                None => {
                    writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "appear", "hwnd": w.hwnd, "class": w.class, "title": title,
                        "ours": ours, "names": names, "buttons": buttons }))?;
                    tracked.insert(w.hwnd, Tracked { class: w.class.clone(), title, names, buttons, ours, first_ms: t_ms });
                }
                Some(tr) => {
                    if tr.names != names || tr.buttons != buttons || tr.title != title {
                        writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "change", "hwnd": w.hwnd, "title": title,
                            "ours": ours || tr.ours, "names": names, "buttons": buttons }))?;
                        tr.names = names;
                        tr.buttons = buttons;
                        tr.title = title;
                    }
                    tr.ours |= ours;
                }
            }
        }
        let gone: Vec<isize> = tracked.keys().copied().filter(|h| !now.iter().any(|w| w.hwnd == *h)).collect();
        for h in gone {
            let tr = tracked.remove(&h).unwrap();
            last_read.remove(&h);
            writeln!(writer, "{}", json!({ "t_ms": t_ms, "event": "gone", "hwnd": h, "class": tr.class, "ours": tr.ours,
                "lifetime_ms": t_ms - tr.first_ms }))?;
        }
        writer.flush()?;
        let snapshot: Vec<Value> = tracked
            .iter()
            .map(|(h, tr)| json!({ "hwnd": h, "class": tr.class, "title": tr.title, "ours": tr.ours,
                "buttons": tr.buttons.iter().filter(|b| b.1).map(|b| b.0.clone()).collect::<Vec<_>>() }))
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

/// Invokes a named button in one of this run's dialogs, after re-validating the window.
pub fn act(hwnd: isize, names: &[String], tokens: &[String], test_root: &str) -> AnyResult<Value> {
    let refuse = |why: &str| Ok(json!({ "invoked": null, "refused": why }));
    let h = HWND(hwnd as *mut _);
    if !unsafe { IsWindow(Some(h)) }.as_bool() {
        return refuse("window no longer exists");
    }
    let Some(w) = explorer_dialogs().into_iter().find(|w| w.hwnd == hwnd) else {
        return refuse("not a visible Explorer dialog window");
    };
    let uia = Uia::new()?;
    let (texts, _) = uia.read(hwnd)?;
    if !mentions(&texts, tokens) && !mentions(std::slice::from_ref(&w.title), tokens) {
        return refuse("dialog does not mention this run");
    }
    let san = Sanitizer::new(test_root);
    match uia.invoke(hwnd, names)? {
        Some(name) => Ok(json!({ "invoked": san.clean(&name), "class": w.class, "refused": null })),
        None => refuse("no enabled button with a requested name"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_needs_a_long_enough_run_token() {
        let texts = vec!["Copying 1 item to paste-target-1".to_owned()];
        assert!(mentions(&texts, &["paste-target-1".into()]));
        assert!(!mentions(&texts, &["item".into()]), "short tokens are ignored");
        assert!(!mentions(&texts, &["e02-ERR-01-f0.bin".into()]));
    }

    #[test]
    fn sanitizer_replaces_test_root_case_insensitively() {
        let s = Sanitizer::new("D:\\OneCopyE02");
        assert_eq!(s.clean("Copying to d:\\onecopye02\\dest\\x"), "Copying to <testroot>\\dest\\x");
    }
}
