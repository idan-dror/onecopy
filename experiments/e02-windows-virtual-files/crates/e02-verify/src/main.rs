//! E-02 helper tool:
//! - `expect`: expected BLAKE3 digest of a synthetic file
//! - `verify`: destination outcome per case file (size, digest, Zone.Identifier)
//! - `watch`: 100 ms polling observer of a destination directory
//! - `clipboard-formats`: names of the formats currently on the clipboard
//! - `audit`: scans evidence for user/machine names, profile paths and content markers
//! - `explorer-paste` / `explorer-close`: automated driver D2c for an Explorer folder window
//! - Stage B1: `write-synthetic`, `clipboard-replace`, `clipboard-release`, `clipboard-probe`,
//!   `dialog-watch` (read-only UI Automation) and `dialog-act` (one targeted button)
//!
//! Output never contains user names, machine names or clipboard content.

#[cfg(windows)]
mod clip;
#[cfg(windows)]
mod dialogs;
mod explorer;

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
    /// Verify the destination files of a case (complete, partial-prefix, wrong, original kept).
    Verify {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Seed of the pre-existing originals (ERR-08): reports whether each original survived.
        #[arg(long)]
        pre_existing_seed: Option<u64>,
    },
    /// Write one synthetic file (never overwrites; used for pre-existing originals).
    WriteSynthetic {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        index: u32,
        #[arg(long)]
        size: u64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Replace the clipboard with tagged synthetic text, as another application would (LIFE-03).
    ClipboardReplace {
        #[arg(long)]
        tag: String,
    },
    /// Empty the clipboard only if it holds this run's tagged content; report what persists.
    ClipboardRelease {
        #[arg(long)]
        tag: String,
        #[arg(long)]
        expect_sequence: Option<u32>,
        /// Report only; never empty.
        #[arg(long)]
        probe_only: bool,
    },
    /// Measure clipboard access latency for other applications (STREAM-08).
    ClipboardProbe {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        stop_file: PathBuf,
        #[arg(long, default_value_t = 500)]
        interval_ms: u64,
        #[arg(long, default_value_t = 600)]
        timeout_secs: u64,
        #[arg(long)]
        parent_pid: Option<u32>,
    },
    /// Observe Explorer file-operation dialogs (read-only UI Automation).
    DialogWatch {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        state: PathBuf,
        #[arg(long)]
        stop_file: PathBuf,
        /// Comma-separated run tokens (destination folder names, synthetic file names).
        #[arg(long)]
        tokens: String,
        #[arg(long)]
        test_root: String,
        #[arg(long, default_value_t = 250)]
        interval_ms: u64,
        #[arg(long, default_value_t = 900)]
        timeout_secs: u64,
        #[arg(long)]
        parent_pid: Option<u32>,
    },
    /// Invoke one named button in one of this run's Explorer dialogs (experiment only, D-041).
    DialogAct {
        #[arg(long)]
        hwnd: isize,
        /// Comma-separated acceptable button names, in preference order.
        #[arg(long)]
        names: String,
        #[arg(long)]
        tokens: String,
        #[arg(long)]
        test_root: String,
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
        /// Stop observing if this (harness) process exits.
        #[arg(long)]
        parent_pid: Option<u32>,
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
    /// Record the top-level Explorer windows that exist before a run (handles and tab counts only).
    ExplorerSnapshot {
        #[arg(long)]
        out: PathBuf,
    },
    /// Find the window showing `dest` and report whether it is a new (experiment-owned) window.
    ExplorerFind {
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        pre_snapshot: PathBuf,
        #[arg(long, default_value_t = 15)]
        timeout_secs: u64,
    },
    /// Post the folder-view Paste command to the run's own window (driver D2c, experiment only).
    ExplorerPaste {
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        expected_top: i64,
    },
    /// Print a case's file names and sizes as JSON (used by the run scripts).
    CaseFiles {
        #[arg(long)]
        case: PathBuf,
    },
    /// Close the run's window only if it is experiment-owned and has no other tabs.
    ExplorerClose {
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        pre_snapshot: PathBuf,
        #[arg(long)]
        expected_top: Option<i64>,
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
        Command::Verify { case, dest, out, pre_existing_seed } => verify(&case, &dest, &out, pre_existing_seed),
        Command::WriteSynthetic { seed, index, size, out } => write_synthetic(seed, index, size, &out),
        Command::ClipboardReplace { tag } => b1::clipboard_replace(&tag),
        Command::ClipboardRelease { tag, expect_sequence, probe_only } => b1::clipboard_release(&tag, expect_sequence, probe_only),
        Command::ClipboardProbe { out, stop_file, interval_ms, timeout_secs, parent_pid } => {
            b1::clipboard_probe(&out, &stop_file, interval_ms, timeout_secs, parent_pid)
        }
        Command::DialogWatch { out, state, stop_file, tokens, test_root, interval_ms, timeout_secs, parent_pid } => {
            b1::dialog_watch(&out, &state, &stop_file, &split(&tokens), &test_root, interval_ms, timeout_secs, parent_pid)
        }
        Command::DialogAct { hwnd, names, tokens, test_root } => b1::dialog_act(hwnd, &split(&names), &split(&tokens), &test_root),
        Command::Watch { dest, out, stop_file, interval_ms, timeout_secs, parent_pid } => {
            watch(&dest, &out, &stop_file, interval_ms, timeout_secs, parent_pid)
        }
        Command::ClipboardFormats { out } => win::clipboard_formats().and_then(|v| emit(&v, out.as_deref())),
        Command::Audit { dir, cases, out } => audit(&dir, &cases).and_then(|v| emit(&v, out.as_deref())),
        Command::ExplorerSnapshot { out } => explorer_cmd(explorer::win::snapshot).and_then(|v| emit(&v, Some(&out))),
        Command::ExplorerFind { dest, pre_snapshot, timeout_secs } => {
            explorer_cmd(|| explorer::win::find_owned(&dest, &pre_snapshot, timeout_secs)).and_then(|v| emit(&v, None))
        }
        Command::ExplorerPaste { dest, expected_top } => {
            explorer_cmd(|| explorer::win::paste(&dest, expected_top)).and_then(|v| emit(&v, None))
        }
        Command::ExplorerClose { dest, pre_snapshot, expected_top } => {
            explorer_cmd(|| explorer::win::close_owned(&dest, &pre_snapshot, expected_top)).and_then(|v| emit(&v, None))
        }
        Command::CaseFiles { case } => Case::load(&case).map_err(Into::into).and_then(|c| {
            let files: Vec<Value> = c
                .files
                .iter()
                .map(|f| json!({ "name": f.name.replace('/', "\\"), "size": f.size, "advertised_size": f.descriptor_size(),
                                 "faults": f.faults }))
                .collect();
            emit(
                &json!({ "case_id": c.case_id, "description": c.description, "files": files,
                         "threading": c.variants.threading, "large": c.variants.large,
                         "read_timeout_ms": c.variants.read_timeout_ms, "zone_identifier": c.variants.zone_identifier,
                         "harness": c.harness }),
                None,
            )
        }),
    };
    if let Err(e) = result {
        eprintln!("e02-verify: {e}");
        std::process::exit(1);
    }
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn split(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_owned()).filter(|x| !x.is_empty()).collect()
}

#[cfg(windows)]
mod b1 {
    use super::{AnyResult, emit, win};
    use std::path::Path;

    pub fn clipboard_replace(tag: &str) -> AnyResult<()> {
        emit(&crate::clip::set_replacement(tag)?, None)
    }
    pub fn clipboard_release(tag: &str, expect_sequence: Option<u32>, probe_only: bool) -> AnyResult<()> {
        emit(&crate::clip::release(tag, expect_sequence, probe_only)?, None)
    }
    pub fn clipboard_probe(out: &Path, stop: &Path, interval_ms: u64, timeout_secs: u64, parent_pid: Option<u32>) -> AnyResult<()> {
        let parent = parent_pid.map(win::ProcessWatch::open).transpose()?;
        crate::clip::probe(out, stop, parent.as_ref(), interval_ms, timeout_secs)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn dialog_watch(
        out: &Path,
        state: &Path,
        stop: &Path,
        tokens: &[String],
        test_root: &str,
        interval_ms: u64,
        timeout_secs: u64,
        parent_pid: Option<u32>,
    ) -> AnyResult<()> {
        let parent = parent_pid.map(win::ProcessWatch::open).transpose()?;
        crate::dialogs::watch(out, state, stop, parent.as_ref(), tokens, test_root, interval_ms, timeout_secs)
    }
    pub fn dialog_act(hwnd: isize, names: &[String], tokens: &[String], test_root: &str) -> AnyResult<()> {
        emit(&crate::dialogs::act(hwnd, names, tokens, test_root)?, None)
    }
}

#[cfg(not(windows))]
mod b1 {
    use super::AnyResult;
    use std::path::Path;
    pub fn clipboard_replace(_: &str) -> AnyResult<()> {
        Err("Windows only".into())
    }
    pub fn clipboard_release(_: &str, _: Option<u32>, _: bool) -> AnyResult<()> {
        Err("Windows only".into())
    }
    pub fn clipboard_probe(_: &Path, _: &Path, _: u64, _: u64, _: Option<u32>) -> AnyResult<()> {
        Err("Windows only".into())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn dialog_watch(_: &Path, _: &Path, _: &Path, _: &[String], _: &str, _: u64, _: u64, _: Option<u32>) -> AnyResult<()> {
        Err("Windows only".into())
    }
    pub fn dialog_act(_: isize, _: &[String], _: &[String], _: &str) -> AnyResult<()> {
        Err("Windows only".into())
    }
}

/// Writes one synthetic file; refuses to overwrite anything.
fn write_synthetic(seed: u64, index: u32, size: u64, out: &Path) -> AnyResult<()> {
    if size > e02_synth::BOUNDED_CASE_MAX_BYTES {
        return Err("write-synthetic is limited to bounded sizes in Stage B1".into());
    }
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(out)?;
    let mut w = std::io::BufWriter::new(file);
    let mut stream = Synth::new(seed).stream_at(index, 0);
    let mut buf = vec![0u8; 1 << 20];
    let mut remaining = size;
    while remaining > 0 {
        let n = remaining.min(buf.len() as u64) as usize;
        stream.fill_next(&mut buf[..n]);
        w.write_all(&buf[..n])?;
        remaining -= n as u64;
    }
    w.flush()?;
    println!("{}", json!({ "written": size }));
    Ok(())
}

/// True if the file's bytes equal the first `size` bytes of synthetic file `index`.
fn is_synthetic_prefix(path: &Path, synth: &Synth, index: u32, size: u64) -> std::io::Result<bool> {
    Ok(hash_file(path)? == synth.digest(index, size))
}

#[cfg(windows)]
fn explorer_cmd(f: impl FnOnce() -> AnyResult<Value>) -> AnyResult<Value> {
    f()
}

#[cfg(not(windows))]
fn explorer_cmd(_f: impl FnOnce() -> AnyResult<Value>) -> AnyResult<Value> {
    Err("Windows only".into())
}

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

/// Destination state of one expected file (spec §9.1 inputs).
fn file_state(exists: bool, size: Option<u64>, true_size: u64, full_match: Option<bool>, prefix_match: Option<bool>, original: Option<bool>) -> &'static str {
    match (exists, size) {
        (false, _) | (_, None) => "absent",
        _ if original == Some(true) => "original_preserved",
        (_, Some(0)) if true_size > 0 => "empty",
        (_, Some(s)) if s == true_size => if full_match == Some(true) { "complete_correct" } else { "full_size_wrong_content" },
        (_, Some(s)) if s < true_size => if prefix_match == Some(true) { "partial_correct_prefix" } else { "partial_wrong_content" },
        _ => if prefix_match == Some(true) { "oversize_synthetic_continuation" } else { "oversize_wrong_content" },
    }
}

fn verify(case_path: &Path, dest: &Path, out: &Path, pre_existing_seed: Option<u64>) -> AnyResult<()> {
    let case = Case::load(case_path)?;
    let synth = Synth::new(case.seed);
    let original_synth = pre_existing_seed.map(Synth::new);
    let mut files = Vec::new();
    let mut all_match = true;
    for (index, file) in case.files.iter().enumerate() {
        let path = dest.join(file.name.replace('/', "\\"));
        let meta = std::fs::metadata(&path).ok().filter(|m| m.is_file());
        let size = meta.as_ref().map(|m| m.len());
        let digest_match = if size == Some(file.size) {
            let expected = synth.digest(index as u32, file.size);
            Some(hash_file(&path)? == expected)
        } else {
            None
        };
        // Is whatever is there a correct prefix (or synthetic continuation) of the stream?
        let prefix_match = match size {
            Some(s) if s > 0 && s != file.size && s <= e02_synth::BOUNDED_CASE_MAX_BYTES => {
                Some(is_synthetic_prefix(&path, &synth, index as u32, s)?)
            }
            _ => None,
        };
        let original_preserved = match (&original_synth, size) {
            (Some(o), Some(s)) if s == file.size => Some(hash_file(&path)? == o.digest(index as u32, file.size)),
            (Some(_), _) => Some(false),
            _ => None,
        };
        let advertised = case.variants.advertise_size.then(|| file.descriptor_size());
        // A file with the final name that a user could take as complete: its size equals the
        // advertised size, or no size was advertised at all.
        let mistakable = meta.is_some() && advertised.is_none_or(|a| size == Some(a));
        let state = file_state(meta.is_some(), size, file.size, digest_match, prefix_match, original_preserved);
        let ok = digest_match == Some(true);
        all_match &= ok;
        files.push(json!({
            "index": index, "name": file.name, "expected_size": file.size, "advertised_size": advertised,
            "exists": meta.is_some(), "size": size, "digest_match": digest_match, "prefix_match": prefix_match,
            "original_preserved": original_preserved, "state": state, "mistakable_as_complete": mistakable,
            "zone_identifier": if meta.is_some() { zone_identifier(&path) } else { Value::Null },
        }));
    }
    let expected: Vec<String> = case.files.iter().map(|f| f.name.replace('/', "\\")).collect();
    let mut extra: Vec<Value> = Vec::new();
    for (name, size) in list_entries(dest) {
        if name.ends_with('\\') || expected.contains(&name) || name == ".onecopy-e02-run" {
            continue;
        }
        // Temporary or renamed copies: does the content match a prefix of any case file?
        let mut matches_file = None;
        if let Some(s) = size.filter(|s| *s > 0 && *s <= e02_synth::BOUNDED_CASE_MAX_BYTES) {
            for index in 0..case.files.len() {
                if is_synthetic_prefix(&dest.join(&name), &synth, index as u32, s)? {
                    matches_file = Some(index);
                    break;
                }
            }
        }
        extra.push(json!({ "name": name, "size": size, "synthetic_prefix_of_file": matches_file }));
    }
    let result = json!({ "case_id": case.case_id, "all_files_match": all_match, "files": files, "unexpected_entries": extra });
    std::fs::write(out, serde_json::to_string_pretty(&result)?)?;
    println!("{}", json!({ "case_id": case.case_id, "all_files_match": all_match }));
    Ok(())
}

fn watch(dest: &Path, out: &Path, stop_file: &Path, interval_ms: u64, timeout_secs: u64, parent_pid: Option<u32>) -> AnyResult<()> {
    let parent = parent_pid.map(win::ProcessWatch::open).transpose()?;
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
        let parent_gone = parent.as_ref().is_some_and(|p| p.exited());
        if stop_file.exists() || parent_gone || start.elapsed() > Duration::from_secs(timeout_secs) {
            if parent_gone {
                writeln!(writer, "{}", json!({ "t_ms": start.elapsed().as_secs_f64() * 1000.0, "change": "stopped", "reason": "parent_exited" }))?;
            }
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
    pub fn clipboard_formats() -> AnyResult<Value> {
        Err("Windows only".into())
    }
    pub struct ProcessWatch;
    impl ProcessWatch {
        pub fn open(_pid: u32) -> AnyResult<Self> {
            Err("Windows only".into())
        }
        pub fn exited(&self) -> bool {
            false
        }
    }
}

#[cfg(windows)]
mod win {
    use super::AnyResult;
    use serde_json::{Value, json};
    use std::time::Duration;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EnumClipboardFormats, GetClipboardFormatNameW, GetClipboardSequenceNumber, OpenClipboard,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};

    /// Read-only: opens the clipboard, enumerates format names, closes it. Never empties or sets it.
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

    /// Watches a parent process for exit through a SYNCHRONIZE handle (immune to PID reuse
    /// once opened).
    pub struct ProcessWatch(HANDLE);

    impl ProcessWatch {
        pub fn open(pid: u32) -> AnyResult<Self> {
            Ok(Self(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid)? }))
        }
        pub fn exited(&self) -> bool {
            unsafe { WaitForSingleObject(self.0, 0) != WAIT_TIMEOUT }
        }
    }

    impl Drop for ProcessWatch {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}
