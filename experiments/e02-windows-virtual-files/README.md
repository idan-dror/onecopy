# E-02 — Windows clipboard virtual-file paste (experiment workspace)

**Phase 0 feasibility experiment.** Isolated and throwaway; not OneCopy product code. Specification: [`docs/experiments/E-02_WINDOWS_VIRTUAL_FILE_PASTE.md`](../../docs/experiments/E-02_WINDOWS_VIRTUAL_FILE_PASTE.md). **Current scope: Stage A only** (small files, lifecycle, correctness, memory). Stage B starts only after owner review.

## What it does

`e02-agent` runs an STA owner thread (threading variant T1). On it, the agent places an `IDataObject` on the Windows clipboard that offers:

- `FileGroupDescriptorW` (`HGLOBAL`);
- `FileContents` (one new `IStream` per `GetData`, per `lindex`);
- `Preferred DropEffect` (copy);
- a private `OneCopyE02Marker` format.

It does **not** offer `CF_HDROP`. Stream bytes come from a deterministic, seekable ChaCha20 keystream (`e02-synth`), so no content is stored and expected digests are recomputed.

Every COM call is logged to `evidence/<run>/events.jsonl` without content. The agent samples its own memory every second. On exit it clears the clipboard if its object is still current. It never calls `OleFlushClipboard`.

`e02-verify` provides the supporting commands:

- `verify`: sizes, BLAKE3 digests and the `Zone.Identifier` zone ID only.
- `watch`: a 100 ms destination observer.
- `clipboard-formats`
- `audit`: scans for user names, machine names, profile paths and content markers, and reports counts only.
- `explorer-paste` / `explorer-close`: driver **D2c**, which posts Explorer's folder-view *Paste* command (`WM_COMMAND` 0x701A, undocumented) to the window showing the test folder. There are no global keystrokes. This is experiment-only automation (D-041).

## Safety

- Synthetic data only.
- Destinations live under `D:\OneCopyE02\dest\<run>\` (not the user profile) and are deleted after verification.
- Running a case **replaces the current clipboard contents**.
- No networking, encryption, services, installers or settings changes.
- Blocked here: SEC-04 (disposable VM only) and DEST-01..04 (VHD creation deferred).

## Build and test

```powershell
cargo build --release --locked
cargo test --release --locked
```

## Run Stage A cases

```powershell
# Automated driver D2c (pending D1 calibration):
.\scripts\run-stage-a.ps1 -Case A-01 -Driver ExplorerCommand
.\scripts\run-stage-a.ps1 -Case A-06 -Driver ExplorerCommand -Pastes 2
# Ground truth D1 (operator presses Ctrl+V in the Explorer window that opens):
.\scripts\run-stage-a.ps1 -Case A-01 -Driver Manual
# Log audit (A-08):
.\target\release\e02-verify.exe audit --dir evidence --cases cases\A-01.toml --cases cases\A-04.toml
```

Each run writes a sanitized record to `reports/stage-a/<run>.json`. Raw evidence stays in the git-ignored `evidence/` directory.
