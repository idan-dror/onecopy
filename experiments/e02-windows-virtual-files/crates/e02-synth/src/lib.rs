//! E-02 synthetic byte source and case model.
//!
//! Byte `n` of file `i` in a run with seed `s` is the IETF ChaCha20 keystream at
//! byte offset `n`, with key `BLAKE3.derive_key("onecopy e02 synth v1", s)` and a
//! 96-bit nonce whose first four bytes are `i` (little endian). The stream is
//! incompressible, reproducible and seekable, so no content is ever stored:
//! expected digests are recomputed by regenerating the bytes.
//!
//! The 32-bit ChaCha20 block counter limits one (key, nonce) stream to 256 GiB,
//! which covers every E-02 case (largest: 100 GiB).

pub mod fault;

pub use fault::{Fault, FilePlan, ReadAction, ReadOutcome};

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Domain-separation context for key derivation. Changing it changes every byte.
pub const KEY_CONTEXT: &str = "onecopy e02 synth v1";

/// Largest stream supported by one ChaCha20 (key, nonce) pair: 2^32 blocks of 64 bytes.
pub const MAX_STREAM_BYTES: u64 = (1u64 << 32) * 64;

/// Largest total case size without `large = true`. Large-file cases belong to Stage B2, which
/// needs separate owner review and a separately agreed hands-off window.
pub const BOUNDED_CASE_MAX_BYTES: u64 = 256 << 20;

/// `OleFlushClipboard` (exit mode `flush`) is refused for cases larger than this.
pub const FLUSH_MAX_CASE_BYTES: u64 = 64 << 20;

/// Deterministic generator for one run seed.
#[derive(Clone)]
pub struct Synth {
    key: [u8; 32],
}

impl Synth {
    pub fn new(seed: u64) -> Self {
        Self { key: blake3::derive_key(KEY_CONTEXT, &seed.to_le_bytes()) }
    }

    /// Creates a cipher positioned at `offset` within file `file_index`.
    pub fn stream_at(&self, file_index: u32, offset: u64) -> SynthStream {
        assert!(offset <= MAX_STREAM_BYTES, "offset beyond ChaCha20 stream limit");
        let mut nonce = [0u8; 12];
        nonce[..4].copy_from_slice(&file_index.to_le_bytes());
        let mut cipher = ChaCha20::new(&self.key.into(), &nonce.into());
        cipher.seek(offset);
        SynthStream { cipher, position: offset }
    }

    /// Fills `buf` with the bytes of file `file_index` starting at `offset`.
    pub fn fill(&self, file_index: u32, offset: u64, buf: &mut [u8]) {
        self.stream_at(file_index, offset).fill_next(buf);
    }

    /// BLAKE3 digest of the first `size` bytes of file `file_index`.
    pub fn digest(&self, file_index: u32, size: u64) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        let mut stream = self.stream_at(file_index, 0);
        let mut buf = vec![0u8; 1 << 20];
        let mut remaining = size;
        while remaining > 0 {
            let n = remaining.min(buf.len() as u64) as usize;
            stream.fill_next(&mut buf[..n]);
            hasher.update(&buf[..n]);
            remaining -= n as u64;
        }
        hasher.finalize()
    }
}

/// Sequential reader over one synthetic file; avoids re-keying per read.
pub struct SynthStream {
    cipher: ChaCha20,
    position: u64,
}

impl SynthStream {
    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn seek(&mut self, offset: u64) {
        assert!(offset <= MAX_STREAM_BYTES, "offset beyond ChaCha20 stream limit");
        self.cipher.seek(offset);
        self.position = offset;
    }

    /// Writes the next `buf.len()` keystream bytes into `buf`.
    pub fn fill_next(&mut self, buf: &mut [u8]) {
        buf.fill(0);
        self.cipher.apply_keystream(buf);
        self.position += buf.len() as u64;
    }
}

/// One test case, loaded from `cases/<id>.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub description: String,
    pub seed: u64,
    pub files: Vec<CaseFile>,
    #[serde(default)]
    pub variants: Variants,
    /// Instructions for the run script only; the provider ignores them.
    #[serde(default)]
    pub harness: Harness,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    /// Relative path placed in `FILEDESCRIPTORW.cFileName` (synthetic name only).
    pub name: String,
    /// True size of the synthetic file (the expected destination content).
    pub size: u64,
    /// `FD_FILESIZE` value in the descriptor when it must differ from `size` (wrong size).
    #[serde(default)]
    pub advertised_size: Option<u64>,
    /// `IStream::Stat` `cbSize` when it must differ from `size` (DESC-06).
    #[serde(default)]
    pub stat_size: Option<u64>,
    #[serde(default)]
    pub faults: Vec<Fault>,
}

impl CaseFile {
    pub fn plan(&self) -> FilePlan {
        FilePlan::new(self.size, self.faults.clone())
    }

    /// Size written into the descriptor (when `FD_FILESIZE` is set).
    pub fn descriptor_size(&self) -> u64 {
        self.advertised_size.unwrap_or(self.size)
    }
}

/// COM threading variant (spec §3.2, §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Threading {
    /// Data object and streams bound to the owner STA (`Agile = false`).
    T1,
    /// Data object bound to the STA; streams agile (free-threaded marshaler), so cross-process
    /// `Read` calls run on RPC threads and a blocked `Read` does not occupy the STA.
    T2,
    /// Data object and streams agile.
    T2A,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Variants {
    pub threading: Threading,
    /// Set `FD_FILESIZE`.
    pub advertise_size: bool,
    /// Set `FD_PROGRESSUI`.
    pub progress_ui: bool,
    /// Variant RT: a stalled `Read` returns a timeout error after this many ms.
    pub read_timeout_ms: Option<u64>,
    /// Delay before returning the descriptor from `GetData` (STREAM-05).
    pub descriptor_delay_ms: u64,
    /// Delay before returning a contents stream from `GetData` (STREAM-06).
    pub contents_getdata_delay_ms: u64,
    /// Observation only: answer `ZoneIdentifier` requests with this DWORD (for example 3).
    /// No Mark-of-the-Web claim follows from offering it (D-033, checklist W-06).
    pub zone_identifier: Option<u32>,
    /// Hard cap on bytes served by the whole provider process (render-all guard). Reads beyond
    /// it fail. Default: 4 x the case's stream bytes + 1 MiB.
    pub max_total_bytes_served: Option<u64>,
    /// Must be true for cases above `BOUNDED_CASE_MAX_BYTES` (Stage B2 only).
    pub large: bool,
}

impl Default for Variants {
    fn default() -> Self {
        Self {
            threading: Threading::T1,
            advertise_size: true,
            progress_ui: true,
            read_timeout_ms: None,
            descriptor_delay_ms: 0,
            contents_getdata_delay_ms: 0,
            zone_identifier: None,
            max_total_bytes_served: None,
            large: false,
        }
    }
}

/// Run-script instructions (not used by the provider).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Harness {
    /// Number of destination folders pasted into.
    pub pastes: u32,
    /// Paste into all folders at once instead of one after another.
    pub concurrent: bool,
    /// Provider exit mode: clear | leave | flush.
    pub exit_mode: String,
    /// Responses to Explorer dialogs, in order (cancel, try_again, skip, replace, keep_both).
    pub dialog_actions: Vec<String>,
    /// Kill the provider this many ms after it logs the named fault event (for example
    /// "stall_begin"). Killing is done by the harness, by PID and identity.
    pub kill_after_event: Option<String>,
    pub kill_delay_ms: u64,
    /// Kill the provider after it has placed the clipboard and before the paste (PROC-03).
    pub kill_before_paste: bool,
    /// Replace the clipboard with harness-owned synthetic text after this provider event.
    pub replace_clipboard_after_event: Option<String>,
    pub replace_delay_ms: u64,
    /// Ask the operator to close the destination window after this provider event (CANCEL-04).
    pub close_window_after_event: Option<String>,
    /// Pre-create each destination file from this seed before pasting (ERR-08, DEST-05).
    pub pre_existing_seed: Option<u64>,
    /// Run the clipboard latency probe during the run (STREAM-08).
    pub probe_clipboard: bool,
    /// Overall run timeout.
    pub timeout_secs: u64,
    /// Expected target result: success | failure | cancel | observe.
    pub expect: String,
    /// What the operator must do (shown before the run).
    pub operator_note: String,
}

impl Default for Harness {
    fn default() -> Self {
        Self {
            pastes: 1,
            concurrent: false,
            exit_mode: "clear".into(),
            dialog_actions: Vec::new(),
            kill_after_event: None,
            kill_delay_ms: 0,
            kill_before_paste: false,
            replace_clipboard_after_event: None,
            replace_delay_ms: 0,
            close_window_after_event: None,
            pre_existing_seed: None,
            probe_clipboard: false,
            timeout_secs: 90,
            expect: "success".into(),
            operator_note: String::new(),
        }
    }
}

const DIALOG_ACTIONS: &[&str] = &["cancel", "try_again", "skip", "replace", "keep_both"];
const EXIT_MODES: &[&str] = &["clear", "leave", "flush"];
const EXPECTS: &[&str] = &["success", "failure", "cancel", "observe"];
const EVENTS: &[&str] = &["stall_begin", "mark", "error_returned", "eof_returned"];

#[derive(Debug)]
pub enum CaseError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Invalid(String),
}

impl std::fmt::Display for CaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaseError::Io(e) => write!(f, "cannot read case file: {e}"),
            CaseError::Parse(e) => write!(f, "invalid case TOML: {e}"),
            CaseError::Invalid(msg) => write!(f, "invalid case: {msg}"),
        }
    }
}

impl std::error::Error for CaseError {}

impl Case {
    pub fn load(path: &Path) -> Result<Self, CaseError> {
        let text = std::fs::read_to_string(path).map_err(CaseError::Io)?;
        let case: Case = toml::from_str(&text).map_err(CaseError::Parse)?;
        case.validate()?;
        Ok(case)
    }

    /// Total bytes the streams deliver (sizes adjusted for premature EOF and overrun).
    pub fn stream_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.plan().stream_len()).sum()
    }

    /// Provider-wide byte budget (render-all guard).
    pub fn byte_budget(&self) -> u64 {
        self.variants.max_total_bytes_served.unwrap_or(4 * self.stream_bytes() + (1 << 20))
    }

    /// Rejects unsafe names, inconsistent fault plans and unbounded cases.
    pub fn validate(&self) -> Result<(), CaseError> {
        let bad = |m: String| Err(CaseError::Invalid(m));
        if self.files.is_empty() {
            return bad("case has no files".into());
        }
        if !self.case_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') || self.case_id.len() > 16 {
            return bad("case_id must be short and [A-Za-z0-9-]".into());
        }
        let mut total = 0u64;
        for file in &self.files {
            validate_relative_name(&file.name)?;
            let plan = file.plan();
            if plan.stream_len() > MAX_STREAM_BYTES || file.descriptor_size() > MAX_STREAM_BYTES {
                return bad(format!("{}: size exceeds stream limit", file.name));
            }
            plan.validate().map_err(|e| CaseError::Invalid(format!("{}: {e}", file.name)))?;
            total += plan.stream_len().max(file.descriptor_size());
        }
        if total > BOUNDED_CASE_MAX_BYTES && !self.variants.large {
            return bad("case exceeds the bounded size; large cases are Stage B2 only".into());
        }
        let h = &self.harness;
        if !(1..=4).contains(&h.pastes) {
            return bad("harness.pastes must be 1..=4".into());
        }
        if !EXIT_MODES.contains(&h.exit_mode.as_str()) {
            return bad(format!("unknown exit_mode {}", h.exit_mode));
        }
        if h.exit_mode == "flush" && total > FLUSH_MAX_CASE_BYTES {
            return bad("exit_mode flush is refused for cases above 64 MiB".into());
        }
        if let Some(a) = h.dialog_actions.iter().find(|a| !DIALOG_ACTIONS.contains(&a.as_str())) {
            return bad(format!("unknown dialog action {a}"));
        }
        if !EXPECTS.contains(&h.expect.as_str()) {
            return bad(format!("unknown expect {}", h.expect));
        }
        for e in [&h.kill_after_event, &h.replace_clipboard_after_event, &h.close_window_after_event].into_iter().flatten() {
            if !EVENTS.contains(&e.as_str()) {
                return bad(format!("unknown trigger event {e}"));
            }
        }
        if h.timeout_secs == 0 || h.timeout_secs > 1200 {
            return bad("harness.timeout_secs must be 1..=1200".into());
        }
        if self.byte_budget() < self.stream_bytes() {
            return bad("max_total_bytes_served is below one copy of the case".into());
        }
        Ok(())
    }
}

/// Synthetic names must be plain relative names: `cFileName` is `WCHAR[MAX_PATH]`
/// and hostile-name testing (SEC-04) is blocked outside a disposable VM.
pub fn validate_relative_name(name: &str) -> Result<(), CaseError> {
    let bad = |why: &str| Err(CaseError::Invalid(format!("unsafe synthetic name {name:?}: {why}")));
    if name.is_empty() {
        return bad("empty");
    }
    if name.encode_utf16().count() >= 260 {
        return bad("does not fit cFileName[MAX_PATH]");
    }
    if name.starts_with(['\\', '/']) || name.contains(':') {
        return bad("absolute or stream syntax");
    }
    for part in name.split(['\\', '/']) {
        if part.is_empty() || part == "." || part == ".." {
            return bad("empty or dot component");
        }
        if part.ends_with(['.', ' ']) {
            return bad("trailing dot or space");
        }
        if !part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
            return bad("only [A-Za-z0-9._-] allowed in Stage A");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_across_instances() {
        let a = Synth::new(7);
        let b = Synth::new(7);
        let mut x = vec![0u8; 4096];
        let mut y = vec![0u8; 4096];
        a.fill(3, 12345, &mut x);
        b.fill(3, 12345, &mut y);
        assert_eq!(x, y);
    }

    #[test]
    fn seeds_and_indices_differ() {
        let mut x = vec![0u8; 64];
        let mut y = vec![0u8; 64];
        Synth::new(1).fill(0, 0, &mut x);
        Synth::new(2).fill(0, 0, &mut y);
        assert_ne!(x, y);
        Synth::new(1).fill(1, 0, &mut y);
        assert_ne!(x, y);
    }

    #[test]
    fn random_access_matches_sequential() {
        let synth = Synth::new(42);
        let mut whole = vec![0u8; 10_000];
        synth.fill(0, 0, &mut whole);
        for (offset, len) in [(0usize, 1usize), (1, 63), (63, 2), (64, 64), (4097, 1000), (9_999, 1)] {
            let mut part = vec![0u8; len];
            synth.fill(0, offset as u64, &mut part);
            assert_eq!(part, whole[offset..offset + len], "offset {offset}");
        }
    }

    #[test]
    fn stream_reads_match_fill() {
        let synth = Synth::new(9);
        let mut expected = vec![0u8; 5000];
        synth.fill(2, 100, &mut expected);
        let mut stream = synth.stream_at(2, 100);
        let mut got = vec![0u8; 5000];
        for chunk in got.chunks_mut(333) {
            stream.fill_next(chunk);
        }
        assert_eq!(got, expected);
        assert_eq!(stream.position(), 5100);
    }

    #[test]
    fn digest_matches_manual_hash() {
        let synth = Synth::new(5);
        let size = 3 * (1 << 20) + 17;
        let mut buf = vec![0u8; size];
        synth.fill(0, 0, &mut buf);
        assert_eq!(synth.digest(0, size as u64), blake3::hash(&buf));
    }

    #[test]
    fn output_is_not_trivially_compressible() {
        let mut buf = vec![0u8; 1 << 16];
        Synth::new(11).fill(0, 0, &mut buf);
        let mut counts = [0u32; 256];
        for b in &buf {
            counts[*b as usize] += 1;
        }
        // Expect ~256 occurrences per byte value; reject gross bias.
        assert!(counts.iter().all(|&c| (128..=384).contains(&c)));
    }

    fn parse(text: &str) -> Result<Case, CaseError> {
        let case: Case = toml::from_str(text).map_err(CaseError::Parse)?;
        case.validate()?;
        Ok(case)
    }

    #[test]
    fn stage_a_case_still_parses_with_defaults() {
        let c = parse("case_id = \"A-01\"\ndescription = \"x\"\nseed = 1\n[[files]]\nname = \"a.bin\"\nsize = 1048576\n").unwrap();
        assert_eq!(c.variants.threading, Threading::T1);
        assert_eq!(c.harness.pastes, 1);
        assert_eq!(c.harness.exit_mode, "clear");
        assert_eq!(c.byte_budget(), 4 * 1048576 + 1048576);
    }

    #[test]
    fn fault_case_parses() {
        let c = parse(r#"
case_id = "ERR-01"
description = "x"
seed = 2
[variants]
threading = "T2"
read_timeout_ms = 30000
[harness]
dialog_actions = ["cancel"]
expect = "failure"
[[files]]
name = "a.bin"
size = 1048576
[[files.faults]]
kind = "error"
at_offset = 524288
hresult = 0x80004005
"#).unwrap();
        assert_eq!(c.variants.threading, Threading::T2);
        assert_eq!(c.files[0].faults, vec![Fault::Error { at_offset: 524288, hresult: 0x8000_4005, once: false }]);
    }

    #[test]
    fn unbounded_and_unsafe_cases_are_rejected() {
        let big = "case_id = \"B\"\ndescription = \"x\"\nseed = 1\n[[files]]\nname = \"a.bin\"\nsize = 1073741824\n";
        assert!(parse(big).is_err(), "1 GiB needs large = true (Stage B2)");
        let flush = "case_id = \"P\"\ndescription = \"x\"\nseed = 1\n[harness]\nexit_mode = \"flush\"\n[[files]]\nname = \"a.bin\"\nsize = 134217728\n";
        assert!(parse(flush).is_err(), "flush above 64 MiB is refused");
        let success_hr = "case_id = \"E\"\ndescription = \"x\"\nseed = 1\n[[files]]\nname = \"a.bin\"\nsize = 10\n[[files.faults]]\nkind = \"error\"\nat_offset = 1\nhresult = 1\n";
        assert!(parse(success_hr).is_err(), "S_FALSE is not an injectable failure");
        let unknown = "case_id = \"E\"\ndescription = \"x\"\nseed = 1\n[[files]]\nname = \"a.bin\"\nsize = 10\n[[files.faults]]\nkind = \"explode\"\n";
        assert!(parse(unknown).is_err());
    }

    #[test]
    fn name_validation() {
        for ok in ["e02-A-01-f0.bin", "dir\\file.bin", "a/b/c.bin"] {
            assert!(validate_relative_name(ok).is_ok(), "{ok}");
        }
        for bad in ["", "..\\x", "a\\..\\b", "\\abs", "C:\\x", "x.bin:ads", "trail.", "sp ", "bad name.bin", "a\\\\b"] {
            assert!(validate_relative_name(bad).is_err(), "{bad}");
        }
        assert!(validate_relative_name(&"a".repeat(260)).is_err());
    }
}
