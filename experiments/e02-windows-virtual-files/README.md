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

- **Synthetic data only.**
- **Test root.** It must be marker-identified (`D:\OneCopyE02`, marker `.onecopy-e02-testroot`) and sit outside protected locations, with no reparse points in its path.
- **Run folders.** Each run gets a unique `dest\<case>-<timestamp>-<hex>\paste-target-N` folder with a run marker.
- **Cleanup.** Deletes only the expected synthetic files, by exact name, and only after a pass. It never deletes recursively and never deletes through reparse points.
- **Clipboard.** A run **replaces the clipboard**, so run desktop batches only in an agreed hands-off window.
  - The agent releases the clipboard only if it still owns it: it opens the clipboard, re-checks ownership, then empties it.
  - It never empties another application's clipboard.
- **Processes.** The agent and observers exit if the harness process disappears. Only experiment-owned processes are stopped, after an identity check.
- **Explorer windows.** An Explorer window is closed only if it was created for this run and has no other tabs.
- **Out of scope here:** networking, encryption, services, installers and settings changes.
- **Blocked:** SEC-04 (disposable VM only) and DEST-01..04 (VHD creation deferred).

## Build and test

```powershell
cargo build --release --locked
cargo test --release --locked
.\scripts\test-harness-safety.ps1                  # non-desktop safety tests (no clipboard, no Explorer)
.\scripts\test-harness-safety.ps1 -IncludeDesktop  # adds clipboard tests: hands-off window only
```

## Run Stage A cases (hands-off window)

```powershell
.\scripts\run-stage-a.ps1 -Case A-01 -Driver ExplorerCommand           # D2c, calibrated against D1 for Stage A
.\scripts\run-stage-a.ps1 -Case A-06 -Driver ExplorerCommand -Pastes 2
.\scripts\run-stage-a.ps1 -Case A-01 -Driver Manual                    # D1: operator presses Ctrl+V in 'paste-target-1'
node scripts/summarize-stage-a.mjs reports/stage-a
node scripts/compare-drivers.mjs reports/stage-a reports/stage-a/calibration <commit-prefix>
.\target\release\e02-verify.exe audit --dir evidence --cases cases\A-01.toml --cases cases\A-04.toml
```

Every run writes a durable record to `reports/stage-a/<run>.json`, including failed and inconclusive runs. Raw evidence stays in the git-ignored `evidence/` directory. The Stage A report is `reports/E-02-stage-a-report.md`.
