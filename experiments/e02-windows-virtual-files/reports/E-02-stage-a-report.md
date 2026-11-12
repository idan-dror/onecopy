# E-02 Stage A report — minimal feasibility (Windows virtual-file paste)

**Experiment:** E-02, Stage A only (spec §0, §7.0), Phase 0.
**Status:** complete, for owner review. **Stage B has not started** and waits for that review.
**Date:** 2026-11-12. **Target:** G-01 (Explorer folder window). **Threading:** T1 (STA-bound objects).
**Driver:** D2c (automated folder-view *Paste* command). **D1 manual calibration has not been performed yet**, so every SA-2 result below is *pass, pending D1 calibration*.

## 1. Outcome

| Criterion | Result | Evidence |
|---|---|---|
| SA-1: build and tests | **pass**. `cargo build --release --locked` succeeded; `cargo test --release --locked` passed 9 of 9 (7 synthetic-source tests, 2 descriptor-layout tests). | §3 |
| SA-2: names, sizes and digests for A-01–A-06 | **pass (D2c), pending D1 calibration**. 24 of 24 runs; every file matched its advertised name and size, and every BLAKE3 digest matched. | §4 |
| SA-3: lifecycle visible; A-07 leaves no formats | **pass**. Set clipboard → descriptor `GetData` → per-file `FileContents` `GetData` → reads → stream release, in every run. All streams created were released. After the agent exited, the clipboard held **no formats** in all 24 runs. | §5 |
| SA-4: A-04 peak minus A-01 peak < 16 MiB | **pass**. OS-tracked peak commit was 2.18–2.23 MiB for A-04 (64 MiB file) and 2.24 MiB for A-01 (1 MiB file): a difference of ≤ 0. | §6 |
| SA-5: log audit clean | **pass**. Evidence: 337 files scanned. Committed reports: 49 files scanned (including this report). No user name, machine name, profile path or synthetic content marker found. The audit was negative-tested and detects each category. | §7 |
| SA-6: report with commands, environment, classified results | This document. | — |

**Conclusion (Stage A scope only).** On this Windows 11 25H2 build, Explorer pasted OneCopy-style virtual files correctly:

- no `CF_HDROP` was offered;
- `FileGroupDescriptorW` plus `FileContents` were served over `IStream`;
- the provider was a single STA;
- files were small (0 B–64 MiB), one to three per data object, and pasted into one or two folders;
- provider memory stayed flat and independent of file size.

**This says nothing yet** about large files, delayed or faulting streams, cancellation, provider termination, partial-file or false-success behavior, provenance, or other targets. Those are Stage B questions, and native-paste guarantees remain unclaimed (D-024).

## 2. Environment (from each run record; no user or machine names)

| Item | Value |
|---|---|
| OS | Windows 11 Home 25H2, build 26200.9711 |
| Explorer | 10.0.26100.8875; folder windows in the same process (`SeparateProcess` = 0) |
| Clipboard History / cloud clipboard | Not configured (registry values absent; Windows default) |
| Microsoft Defender | 4.18.26100.4, real-time protection **on** (unchanged) |
| Destination | `D:\OneCopyE02\dest\<run>\p<n>` on NTFS; deleted after each verified run |
| Toolchain | rustc 1.99.1 (MSVC); windows 0.62.2, windows-core 0.62.2, windows-implement 0.60.2; chacha20 0.10.2; blake3 1.8.7 |
| Hardware | AMD Ryzen 7 5700X3D, 31.9 GiB RAM |
| Commits | `d85b1c5` and `e1657ad` (18-run T1 matrix); `1c2b07c` and `af648cc` (6 memory-peak runs). The agent's paste path is identical across them; later commits only add instrumentation and harness handling. The source tree was clean for every run (`experiment_tree_dirty` = false). |

## 3. Exact commands

Run from `experiments/e02-windows-virtual-files`:

```powershell
cargo build --release --locked          # Finished `release` profile
cargo test --release --locked           # 2 passed (e02-agent) + 7 passed (e02-synth)

# T1 matrix: three repetitions of A-01..A-06 (A-06 pastes into two folders)
foreach ($rep in 1..3) { foreach ($c in 'A-01','A-02','A-03','A-04','A-05','A-06') {
  $p = 1; if ($c -eq 'A-06') { $p = 2 }
  .\scripts\run-stage-a.ps1 -Case $c -Driver ExplorerCommand -Pastes $p -TimeoutSecs 60 } }
.\scripts\run-stage-a.ps1 -Case A-06 -Driver ExplorerCommand -Pastes 2 -TimeoutSecs 60   # replaced one harness-error run

# Memory-peak runs (OS-tracked peak commit)
foreach ($rep in 1..3) { foreach ($c in 'A-01','A-04') {
  .\scripts\run-stage-a.ps1 -Case $c -Driver ExplorerCommand -TimeoutSecs 60 } }

node scripts/summarize-stage-a.mjs reports/stage-a                # roll-up: reports/stage-a/stage-a-rollup.json
.\target\release\e02-verify.exe audit --dir evidence --cases cases\A-01.toml ... --cases cases\A-06.toml
.\target\release\e02-verify.exe audit --dir reports  --cases cases\A-01.toml ... --cases cases\A-06.toml
```

## 4. Results (24 classified runs)

| Case | What | Runs | Result | Completion (s; Explorer open to final size) | Bytes served per paste | Reads per file | Peak private, MiB (sampled) |
|---|---|---|---|---|---|---|---|
| A-01 | 1 file, 1 MiB | 6 | 6 pass | 1.77–1.88 | 1,048,576 | 4 × 256 KiB + 1 zero-byte EOF read | 1.93–2.24 |
| A-02 | 1 file, 0 B | 3 | 3 pass | 1.81–1.89 | 0 | 0 reads; 1 `Stat` call | 1.97–2.02 |
| A-03 | 1 file, 4 KiB | 3 | 3 pass | 1.84–2.52 | 4,096 | 1 + EOF read | 1.98–2.01 |
| A-04 | 1 file, 64 MiB | 6 | 6 pass | 1.78–1.87 | 67,108,864 | 256 × 256 KiB + EOF read | 1.92–2.24 |
| A-05 | 3 files (1 KiB, 1 MiB, 8 MiB) | 3 | 3 pass | 2.02–3.04 | 9,438,208 | One stream per `lindex` (3 streams) | 2.23–2.24 |
| A-06 | 1 MiB pasted into 2 folders | 3 | 3 pass | 1.75–1.86 per folder | 2 × 1,048,576 | One new stream per `GetData` (2 streams) | 2.23–2.25 |

Every run ended with: digest match, no unexpected destination entries, all streams released, agent exit code 0, clipboard empty after exit, and no interference detected. Per-run records are in `reports/stage-a/*.json`; the roll-up is `reports/stage-a/stage-a-rollup.json`.

Completion time is dominated by Explorer opening the window plus the driver's settle delay. **It is not a throughput measurement**; BASE-06 is a Stage B case.

## 5. Observed Explorer behavior (aggregated from event logs)

- **Threading (T1):** every COM call ran on the owner STA thread, a single thread per run (`com_calls_on_owner_sta_only` = true).
- **Formats Explorer requested per paste:**
  - **Served:** `Preferred DropEffect` (many times, including for menu and paste-enable checks), `FileGroupDescriptorW` (once per paste), `FileContents` (once per file).
  - **Requested but not offered** (`DV_E_FORMATETC`): `Shell IDList Array`, `CF_HDROP`, `Shell Object Offsets`, `EnterpriseDataProtectionId`, `AsyncFlag`, **`ZoneIdentifier`**, `Net Resource`, and a `GetData` with format 0.
- **`SetData` from Explorer:** `DropDescription` (UI feedback), `TargetCLSID`, `Performed DropEffect` = 1 (copy), and `Logical Performed DropEffect` = 1.
- **Stream access pattern:** `Seek(CUR, 0)` → `Seek(SET, 0)` → 256 KiB `Read`s → a final zero-byte `Read` (provider returns `S_FALSE`) → `Seek(SET, 0)` again after reading.
  - `Stat` was called only for the zero-length file.
  - `CopyTo`, `Clone` and the other unimplemented methods were never called.
- **Destination naming:** the 100 ms observer only ever saw the final file names, never a temporary name. For files this small that is **weak evidence** (most completed within one polling interval). Stage B's large-file and fault cases settle it.
- **Provenance:** **no `Zone.Identifier`** on any pasted file. Explorer *asks* the data object for a `ZoneIdentifier` format, which this provider does not offer. That is a concrete lead for SEC-01/SEC-02 in Stage B (§9). It is not tested here.

## 6. Memory

- **Sampled private bytes:** 1.92–2.25 MiB across all cases (1 s sampling).
- **OS-tracked lifetime peak commit (`PeakPagefileUsage`):** A-01 2.24 MiB (3 runs); A-04 2.18–2.23 MiB (3 runs).
  - The 64 MiB file therefore needed no more provider memory than the 1 MiB file.
  - One A-04 run shows why the OS peak was added: sampled 1.92 MiB, OS peak 2.18 MiB, because a sub-second paste can fall between samples.
- **Explorer's own memory was not measured** in Stage A; it is a §8 Stage B metric.

## 7. Privacy and safety checks

- **A-08 audit:** clean on 337 evidence files and 49 committed report files.
  - **Negative tests:** a planted user name and profile path were detected (2 and 1 hits). A copy of a real pasted file was detected as a content marker (1 hit).
  - The audit reports counts only, never the matched values.
- Only synthetic data was used.
- Destinations stayed under the test root and were deleted after verification.
- No settings were changed.
- Defender stayed on.
- Process Monitor was not used.
- No elevation was used.
- SEC-04 and DEST-01 to DEST-04 remain **blocked** (VM and VHD approvals).

## 8. Unsupported, inconclusive and not-run items

| Item | Status | Detail |
|---|---|---|
| D1 manual calibration (§9.4) | **Not run** | Needs the operator to press Ctrl+V. Until it runs, D2c results are not equivalence-checked against Ctrl+V. Command: `.\scripts\run-stage-a.ps1 -Case A-01 -Driver Manual` (also A-04, A-05). |
| Run `A-01-20261112T213208` | **Inconclusive (interference)** | The operator accidentally closed the test window before the paste; nothing was pasted. The script then aborted at cleanup, so there is no record. The harness now records this as inconclusive (`af648cc`). |
| Run `A-06-20261112T212846` | **Inconclusive (harness error)** | The Explorer view handle went stale between lookup and post (`0x80070578`); no paste was attempted and there is no record. The driver now re-finds and retries (`e1657ad`); a replacement run passed. |
| 18 preliminary runs at `6f1b9e1` | **Not T1, excluded** | windows-rs objects are agile by default, so Explorer's calls ran on RPC threads (2,680 events off the STA). All 18 passed, but they are kept under `reports/stage-a/preliminary-agile/` as data only. Fixed in `d85b1c5` with `Agile = false`. |
| Explorer memory, throughput baseline, large files, faults, cancellation, termination, provenance, other targets | **Not in Stage A** | Stage B scope |

## 9. Deviations from the specification, and notes for Stage B

**Deviations:**

1. **Generator crate.** The synthetic source uses the `chacha20` crate (IETF ChaCha20, byte-seekable; nonce = file index) instead of `rand_chacha`. The key derivation is unchanged (`BLAKE3.derive_key("onecopy e02 synth v1", seed)`). Reason: direct byte-offset seeking and a standard keystream.
2. **Driver.** The automated driver is **D2c**: `WM_COMMAND` 0x701A posted to the folder view found through `IShellWindows`. The command ID is undocumented. D2c is experiment-only automation (D-041); it uses no global keystrokes, and its target is a freshly created empty folder. The specification allowed it from `b137c37` onwards.
3. **Threading.** windows-rs `#[implement]` defaults to agile objects. T1 now explicitly uses `Agile = false`.
4. **Memory metric.** Added OS-tracked peak commit alongside the 1 s sampler.
5. **A-07.** Evaluated in every run (clipboard after exit), not as a single case.

**Proposed for Stage B (owner decision):**

- **(a) D1 calibration first.** Run the manual Ctrl+V calibration before Stage B relies on D2c.
- **(b) Threading variant T2.** windows-rs' default agile objects are an available mechanism for T2, observed to dispatch calls off the STA. Use them for STREAM-08.
- **(c) Provenance.** Add a variant that offers a `ZoneIdentifier` format, to learn whether Explorer then writes Mark-of-the-Web. The format's contents are undocumented [K], so this would be an observation only.
- **(d) Equipment.** Stage B on this machine needs hands-off windows of several minutes for the 10/100 GiB cases, or a second machine or VM.
