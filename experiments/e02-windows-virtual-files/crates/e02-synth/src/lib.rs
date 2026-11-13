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

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use serde::Deserialize;
use std::path::Path;

/// Domain-separation context for key derivation. Changing it changes every byte.
pub const KEY_CONTEXT: &str = "onecopy e02 synth v1";

/// Largest stream supported by one ChaCha20 (key, nonce) pair: 2^32 blocks of 64 bytes.
pub const MAX_STREAM_BYTES: u64 = (1u64 << 32) * 64;

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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    /// Relative path placed in `FILEDESCRIPTORW.cFileName` (synthetic name only).
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Variants {
    /// T1 = stream objects live in the owner STA (the only Stage A variant).
    pub threading: String,
    /// Set `FD_FILESIZE` with the correct size.
    pub advertise_size: bool,
    /// Set `FD_PROGRESSUI`.
    pub progress_ui: bool,
}

impl Default for Variants {
    fn default() -> Self {
        Self { threading: "T1".into(), advertise_size: true, progress_ui: true }
    }
}

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

    /// Rejects anything Stage A does not support and any unsafe synthetic name.
    pub fn validate(&self) -> Result<(), CaseError> {
        if self.files.is_empty() {
            return Err(CaseError::Invalid("case has no files".into()));
        }
        if self.variants.threading != "T1" {
            return Err(CaseError::Invalid("only threading variant T1 is implemented (Stage A)".into()));
        }
        for file in &self.files {
            validate_relative_name(&file.name)?;
            if file.size > MAX_STREAM_BYTES {
                return Err(CaseError::Invalid(format!("{}: size exceeds stream limit", file.name)));
            }
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
