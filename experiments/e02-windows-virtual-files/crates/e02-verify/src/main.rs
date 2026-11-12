//! E-02 helper tool:
//! - `expect`: expected BLAKE3 digest of a synthetic file
//! - `verify`: destination outcome per case file (size, digest, Zone.Identifier)
//! - `watch`: 100 ms polling observer of a destination directory
//! - `clipboard-formats`: names of the formats currently on the clipboard
//! - `audit`: scans evidence for user/machine names, profile paths and content markers
//! - `explorer-paste` / `explorer-close`: automated driver D2c for an Explorer folder window
//!
//! Output never contains user names, machine names or clipboard content.

use clap::{Parser, Subcommand};
use e02_synth::{Case, Synth};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "E-02 verifier, observer and driver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the expected digest for one synthetic file.
    Expect {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        index: u32,
        #[arg(long)]
        size: u64,
    },
    /// Verify the destination files of a case.
    Verify {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Observe a destination directory until a stop file appears or the timeout expires.
    Watch {
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        stop_file: PathBuf,
        #[arg(long, default_value_t = 100)]
        interval_ms: u64,
        #[arg(long, default_value_t = 900)]
        timeout_secs: u64,
    },
    /// Print the clipboard's current format names as JSON.
    ClipboardFormats {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Scan a directory of evidence/report files for forbidden identifying data.
    Audit {
        #[arg(long)]
        dir: PathBuf,
        /// Case files whose first content bytes must never appear in evidence.
        #[arg(long)]
        cases: Vec<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Post the folder-view Paste command to the Explorer window showing `dest` (driver D2c).
    ExplorerPaste {
        #[arg(long)]
        dest: PathBuf,
        #[arg(long, default_value_t = 15)]
        find_timeout_secs: u64,
    },
    /// Print a case's file names and sizes as JSON (used by the run scripts).
    CaseFiles {
        #[arg(long)]
        case: PathBuf,
    },
    /// Close the Explorer window showing `dest`.
    ExplorerClose {
        #[arg(long)]
        dest: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Expect { seed, index, size } => {
            let digest = Synth::new(seed).digest(index, size);
            println!("{}", json!({ "seed": seed, "index": index, "size": size, "blake3": digest.to_hex().to_string() }));
            Ok(())
        }
        Command::Verify { case, dest, out } => verify(&case, &dest, &out),
        Command::Watch { dest, out, stop_file, interval_ms, timeout_secs } => {
            watch(&dest, &out, &stop_file, interval_ms, timeout_secs)
        }
        Command::ClipboardFormats { out } => win::clipboard_formats().and_then(|v| emit(&v, out.as_deref())),
        Command::Audit { dir, cases, out } => audit(&dir, &cases).and_then(|v| emit(&v, out.as_deref())),
        Command::ExplorerPaste { dest, find_timeout_secs } => win::explorer_paste(&dest, find_timeout_secs),
        Command::ExplorerClose { dest } => win::explorer_close(&dest),
        Command::CaseFiles { case } => Case::load(&case).map_err(Into::into).and_then(|c| {
            let files: Vec<Value> = c.files.iter().map(|f| json!({ "name": f.name.replace('/', "\\"), "size": f.size })).collect();
            emit(&json!({ "case_id": c.case_id, "files": files }), None)
        }),
    };
    if let Err(e) = result {
        eprintln!("e02-verify: {e}");
        std::process::exit(1);
    }
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn emit(value: &Value, out: Option<&Path>) -> AnyResult<()> {
    let text = serde_json::to_string_pretty(value)?;
    match out {
        Some(p) => std::fs::write(p, text)?,
        None => println!("{text}"),
    }
    Ok(())
}

fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// Reads only the ZoneId value and the key names of a Zone.Identifier stream.
/// HostUrl/ReferrerUrl values are not recorded because they may contain paths.
fn zone_identifier(path: &Path) -> Value {
    let mut ads = path.as_os_str().to_owned();
    ads.push(":Zone.Identifier");
    match std::fs::read_to_string(PathBuf::from(ads)) {
        Ok(text) => {
            let mut zone_id = None;
            let mut keys = Vec::new();
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    keys.push(k.trim().to_owned());
                    if k.trim().eq_ignore_ascii_case("ZoneId") {
                        zone_id = v.trim().parse::<i64>().ok();
                    }
                }
            }
            json!({ "present": true, "zone_id": zone_id, "keys": keys })
        }
        Err(_) => json!({ "present": false }),
    }
}

fn list_entries(root: &Path) -> BTreeMap<String, Option<u64>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Option<u64>>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('/', "\\");
            match entry.metadata() {
                Ok(m) if m.is_dir() => {
                    out.insert(format!("{rel}\\"), None);
                    walk(&path, root, out);
                }
                Ok(m) => {
                    out.insert(rel, Some(m.len()));
                }
                Err(_) => {
                    out.insert(rel, None);
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn verify(case_path: &Path, dest: &Path, out: &Path) -> AnyResult<()> {
    let case = Case::load(case_path)?;
    let synth = Synth::new(case.seed);
    let mut files = Vec::new();
    let mut all_match = true;
    for (index, file) in case.files.iter().enumerate() {
        let path = dest.join(file.name.replace('/', "\\"));
        let meta = std::fs::metadata(&path).ok();
        let size = meta.as_ref().map(|m| m.len());
        let digest_match = if size == Some(file.size) {
            let expected = synth.digest(index as u32, file.size);
            Some(hash_file(&path)? == expected)
        } else {
            None
        };
        let ok = digest_match == Some(true);
        all_match &= ok;
        files.push(json!({
            "index": index, "name": file.name, "expected_size": file.size,
            "exists": meta.is_some(), "size": size, "digest_match": digest_match,
            "zone_identifier": if meta.is_some() { zone_identifier(&path) } else { Value::Null },
        }));
    }
    let expected: Vec<String> = case.files.iter().map(|f| f.name.replace('/', "\\")).collect();
    let extra: Vec<Value> = list_entries(dest)
        .into_iter()
        .filter(|(name, _)| !name.ends_with('\\') && !expected.contains(name))
        .map(|(name, size)| json!({ "name": name, "size": size }))
        .collect();
    let result = json!({ "case_id": case.case_id, "all_files_match": all_match, "files": files, "unexpected_entries": extra });
    std::fs::write(out, serde_json::to_string_pretty(&result)?)?;
    println!("{}", json!({ "case_id": case.case_id, "all_files_match": all_match }));
    Ok(())
}

fn watch(dest: &Path, out: &Path, stop_file: &Path, interval_ms: u64, timeout_secs: u64) -> AnyResult<()> {
    let start = Instant::now();
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out)?);
    let mut previous: BTreeMap<String, Option<u64>> = BTreeMap::new();
    loop {
        let current = list_entries(dest);
        if current != previous {
            let t_ms = start.elapsed().as_secs_f64() * 1000.0;
            for (name, size) in &current {
                if previous.get(name) != Some(size) {
                    writeln!(writer, "{}", json!({ "t_ms": t_ms, "change": "upsert", "name": name, "size": size }))?;
                }
            }
            for name in previous.keys().filter(|n| !current.contains_key(*n)) {
                writeln!(writer, "{}", json!({ "t_ms": t_ms, "change": "removed", "name": name }))?;
            }
            writer.flush()?;
            previous = current;
        }
        if stop_file.exists() || start.elapsed() > Duration::from_secs(timeout_secs) {
            break;
        }
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
    Ok(())
}

fn audit(dir: &Path, cases: &[PathBuf]) -> AnyResult<Value> {
    let env_needle = |key: &str| std::env::var(key).ok().filter(|v| v.len() >= 3).map(|v| v.to_lowercase());
    let user = env_needle("USERNAME");
    let machine = env_needle("COMPUTERNAME");
    let profile = env_needle("USERPROFILE");
    let mut markers: Vec<Vec<u8>> = Vec::new();
    for c in cases {
        let case = Case::load(c)?;
        let synth = Synth::new(case.seed);
        for (i, f) in case.files.iter().enumerate() {
            if f.size >= 16 {
                let mut prefix = [0u8; 16];
                synth.fill(i as u32, 0, &mut prefix);
                let hex: String = prefix.iter().map(|b| format!("{b:02x}")).collect();
                markers.push(prefix.to_vec());
                markers.push(hex.into_bytes());
            }
        }
    }
    let mut counts = BTreeMap::from([("user_name_hits", 0u64), ("machine_name_hits", 0), ("profile_path_hits", 0), ("content_marker_hits", 0)]);
    let mut scanned = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)?.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let bytes = std::fs::read(&p)?;
            scanned += 1;
            let lower = String::from_utf8_lossy(&bytes).to_lowercase();
            let lower_unescaped = lower.replace("\\\\", "\\");
            let hit = |needle: &Option<String>| needle.as_ref().is_some_and(|n| lower_unescaped.contains(n.as_str()));
            *counts.get_mut("user_name_hits").unwrap() += hit(&user) as u64;
            *counts.get_mut("machine_name_hits").unwrap() += hit(&machine) as u64;
            *counts.get_mut("profile_path_hits").unwrap() += hit(&profile) as u64;
            let marker_hit = markers.iter().any(|m| bytes.windows(m.len()).any(|w| w == m.as_slice()));
            *counts.get_mut("content_marker_hits").unwrap() += marker_hit as u64;
        }
    }
    let clean = counts.values().all(|&c| c == 0);
    Ok(json!({ "files_scanned": scanned, "content_markers_checked": markers.len(), "counts": counts, "clean": clean }))
}

#[cfg(not(windows))]
mod win {
    use super::AnyResult;
    use serde_json::Value;
    use std::path::Path;
    pub fn clipboard_formats() -> AnyResult<Value> {
        Err("Windows only".into())
    }
    pub fn explorer_paste(_: &Path, _: u64) -> AnyResult<()> {
        Err("Windows only".into())
    }
    pub fn explorer_close(_: &Path) -> AnyResult<()> {
        Err("Windows only".into())
    }
}

#[cfg(windows)]
mod win {
    use super::AnyResult;
    use serde_json::{Value, json};
    use std::path::Path;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider,
    };
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EnumClipboardFormats, GetClipboardFormatNameW, GetClipboardSequenceNumber, OpenClipboard,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Shell::{
        IFolderView, IPersistFolder2, IShellBrowser, IShellWindows, SHGetPathFromIDListW, ShellWindows,
    };
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE, WM_COMMAND};
    use windows::core::{GUID, Interface};

    /// SID_STopLevelBrowser (shlguid.h).
    const SID_STOP_LEVEL_BROWSER: GUID = GUID::from_u128(0x4c96be40_915c_11cf_99d3_00aa004ae837);
    /// Explorer folder-view Paste command id (undocumented; experiment-only driver, D-041).
    /// Posted to a freshly created, empty test folder, so a wrong id cannot affect user files.
    const DEFVIEW_PASTE_COMMAND: usize = 0x701A;

    pub fn clipboard_formats() -> AnyResult<Value> {
        unsafe {
            let mut opened = false;
            for _ in 0..40 {
                if OpenClipboard(None).is_ok() {
                    opened = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            if !opened {
                return Err("could not open the clipboard".into());
            }
            let mut names = Vec::new();
            let mut cf = 0u32;
            loop {
                cf = EnumClipboardFormats(cf);
                if cf == 0 {
                    break;
                }
                let mut buf = [0u16; 128];
                let len = GetClipboardFormatNameW(cf, &mut buf);
                names.push(if len > 0 { String::from_utf16_lossy(&buf[..len as usize]) } else { format!("cf#{cf}") });
            }
            let _ = CloseClipboard();
            let has = |n: &str| names.iter().any(|x| x == n);
            Ok(json!({
                "sequence": GetClipboardSequenceNumber(),
                "formats": names.clone(),
                "has_onecopy_marker": has("OneCopyE02Marker"),
                "has_file_descriptor": has("FileGroupDescriptorW"),
                "has_file_contents": has("FileContents"),
            }))
        }
    }

    struct ExplorerWindow {
        top: HWND,
        view: HWND,
    }

    fn same_path(a: &str, b: &Path) -> bool {
        let norm = |s: &str| s.trim_end_matches('\\').to_lowercase();
        norm(a) == norm(&b.to_string_lossy())
    }

    fn find_explorer(dest: &Path) -> AnyResult<Option<ExplorerWindow>> {
        unsafe {
            let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
            let count = windows.Count()?;
            for i in 0..count {
                let Ok(disp) = windows.Item(&VARIANT::from(i)) else { continue };
                let Ok(provider) = disp.cast::<IServiceProvider>() else { continue };
                let Ok(browser) = provider.QueryService::<IShellBrowser>(&SID_STOP_LEVEL_BROWSER) else { continue };
                let Ok(view) = browser.QueryActiveShellView() else { continue };
                let Ok(folder_view) = view.cast::<IFolderView>() else { continue };
                let Ok(folder) = folder_view.GetFolder::<IPersistFolder2>() else { continue };
                let Ok(pidl) = folder.GetCurFolder() else { continue };
                let mut buf = [0u16; 260];
                let ok = SHGetPathFromIDListW(pidl, &mut buf).as_bool();
                CoTaskMemFree(Some(pidl as *const _));
                if !ok {
                    continue;
                }
                let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                if same_path(&String::from_utf16_lossy(&buf[..len]), dest) {
                    // IShellBrowser and IShellView inherit IOleWindow; call GetWindow through their own proxies.
                    let top = browser.GetWindow()?;
                    let view_hwnd = view.GetWindow()?;
                    return Ok(Some(ExplorerWindow { top, view: view_hwnd }));
                }
            }
            Ok(None)
        }
    }

    fn wait_for_explorer(dest: &Path, timeout_secs: u64) -> AnyResult<ExplorerWindow> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if let Some(w) = find_explorer(dest)? {
                return Ok(w);
            }
            if Instant::now() > deadline {
                return Err("no Explorer window shows the destination folder".into());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    pub fn explorer_paste(dest: &Path, find_timeout_secs: u64) -> AnyResult<()> {
        let w = wait_for_explorer(dest, find_timeout_secs)?;
        // Give the view a moment to finish initial enumeration.
        std::thread::sleep(Duration::from_millis(750));
        unsafe { PostMessageW(Some(w.view), WM_COMMAND, WPARAM(DEFVIEW_PASTE_COMMAND), LPARAM(0))? };
        println!("{}", json!({ "posted": "WM_COMMAND", "command_id": DEFVIEW_PASTE_COMMAND, "target": "folder_view" }));
        Ok(())
    }

    pub fn explorer_close(dest: &Path) -> AnyResult<()> {
        let w = wait_for_explorer(dest, 5)?;
        unsafe { PostMessageW(Some(w.top), WM_CLOSE, WPARAM(0), LPARAM(0))? };
        println!("{}", json!({ "posted": "WM_CLOSE" }));
        Ok(())
    }
}
