# E-02 Stage A report: minimal feasibility of Windows virtual-file paste

**Experiment:** E-02, Stage A only (spec §0 and §7.0), Phase 0.

**Status:** Stage A is complete, and the corrections from the owner's review are applied. The report is ready for final approval. **Stage B has not started.**

**Dates:** first runs 2026-11-12; corrections and D1 calibration 2026-11-12/13.

**Scope of every run:**

| Item | Value |
|---|---|
| Target | G-01 (an Explorer folder window) |
| Threading | T1 (COM objects bound to the owner STA) |
| Drivers | D2c (automated folder-view *Paste* command), **calibrated against D1** (the operator pressing Ctrl+V) |

## 1. Outcome

| Criterion | Result | Evidence |
|---|---|---|
| SA-1: build and unit tests | **pass**. `cargo build --release --locked` succeeds; `cargo test --release --locked` passes 14 of 14 (7 synthetic-source, 2 descriptor-layout, 5 Explorer close-decision). | §4 |
| SA-2: names, sizes and digests for A-01 to A-06 | **pass, and D2c is now calibrated.** All 36 authoritative runs pass. The D1 runs of A-01, A-04 and A-05 are equivalent to D2c (§6). | §5, §6 |
| SA-3: lifecycle visible; A-07 leaves no formats | **pass**. Every run shows set clipboard → descriptor `GetData` → one `FileContents` `GetData` per file → reads → stream release. All streams are released. The clipboard holds no formats after the agent exits. Since `9964dc9` the release step is an atomic ownership check (§9). | §5, §9 |
| SA-4: A-04 peak minus A-01 peak below 16 MiB | **pass**. OS-tracked peak commit: 1 MiB file 2.19–2.25 MiB; 64 MiB file 2.18–2.37 MiB. The largest difference is 0.12 MiB. | §8 |
| SA-5: log audit clean | **pass**. 564 raw evidence files and 76 committed report files scanned. The audit was negative-tested. | §11 |
| SA-6: report | This document. | — |
| Review item A: D1 calibration | **Done; equivalent.** 6 of 6 D1-vs-D2c pairs match. | §6 |
| Review item B: harness safety | **Done.** 7 findings fixed. 21 of 21 negative tests pass. One real product-relevant defect was found and fixed (§9.3). | §9 |
| Review item C: threading and provenance | T2 is carried into Stage B. `ZoneIdentifier` was researched, not tested; no provenance claim is made. | §10 |

**Conclusion, limited to Stage A.** On Windows 11 25H2 build 26200.9711, Explorer pasted OneCopy-style virtual files correctly in every run. The provider offered:

- `FileGroupDescriptorW` and `FileContents` over `IStream`;
- no `CF_HDROP`;
- a single-STA provider.

Coverage was small files (0 B to 64 MiB), 1 to 3 files per data object, and 1 or 2 destination folders. Provider memory stayed flat and did not grow with file size. Manual Ctrl+V and the automated driver produced identical COM, stream and outcome behavior.

Stage A says **nothing** about any of the following, and native-paste guarantees remain unclaimed (D-024):

- large files, delayed streams or faulting streams;
- cancellation or provider termination during a paste;
- partial-file and false-success behavior;
- provenance;
- targets other than Explorer.

## 2. Review corrections

| Item | What was done |
|---|---|
| **A. D1 calibration** | The operator pasted A-01, A-04 and A-05 manually after step-by-step instructions, each in an agreed hands-off window. Each run was compared with two D2c runs on the same commit across COM sequence, threads, stream reads and lifetime, files and digests, clipboard ownership and cleanup, and errors. No divergence was found. |
| **B. Harness safety** | Reviewed the full lifecycle and fixed 7 issues (§9.1). Added a harness module, a safe cleanup routine, owned-process handling, durable records, interference classification and an Explorer ownership check. Added 21 negative tests (§9.2). The DT-01 test exposed an agent defect that cleared other applications' clipboards; it is fixed (§9.3). |
| **C. Threading and provenance** | The agile-object evidence is preserved, and not treated as T2 proof. T2 stalls and concurrency are carried into Stage B. `CFSTR_ZONEIDENTIFIER` semantics were researched before any test (§10.2). |

## 3. Environment

The values below come from each run record. Run records contain no user or machine names.

| Item | Value |
|---|---|
| OS | Windows 11 Home 25H2, build 26200.9711 |
| Explorer | Version 10.0.26100.8875. `SeparateProcess` = 0. Every test folder opened as a **new top-level window with one tab** (recorded per run since `897004c`). |
| Clipboard History, cloud clipboard | Not configured (registry values absent) |
| Microsoft Defender | Version 4.18.26100.4. Real-time protection on and unchanged. |
| Test root | `D:\OneCopyE02` on NTFS, carrying the `.onecopy-e02-testroot` marker. Adopted once: it contained only an empty `dest` folder. Run folders are `dest\<case>-<timestamp>-<8 hex>\paste-target-N`. |
| Toolchain | rustc 1.99.1 (MSVC). Crates: windows 0.62.2, windows-core 0.62.2, windows-implement 0.60.2, chacha20 0.10.2, blake3 1.8.7. |
| Hardware | AMD Ryzen 7 5700X3D, 31.9 GiB RAM |

## 4. Exact commands

All commands run from `experiments/e02-windows-virtual-files`.

```powershell
cargo build --release --locked                                    # Finished `release` profile
cargo test  --release --locked                                    # 2 + 7 + 5 passed, 0 failed

# Harness safety (B): non-desktop tests (no clipboard, no Explorer), then desktop tests in a hands-off window
.\scripts\test-harness-safety.ps1                                 # 17/17 (reports/stage-a/safety/harness-safety-20261112T215652.json)
.\scripts\test-harness-safety.ps1 -IncludeDesktop                 # 21/21 at 9964dc9 (harness-safety-20261112T220359.json)

# D2c regression at 9964dc9 (hands-off window)
foreach ($c in 'A-01','A-02','A-03','A-04','A-05','A-06','A-01','A-04','A-05') {
  $p = 1; if ($c -eq 'A-06') { $p = 2 }; .\scripts\run-stage-a.ps1 -Case $c -Driver ExplorerCommand -Pastes $p }

# D1 calibration at 9964dc9 (the operator presses Ctrl+V; nothing is simulated)
.\scripts\run-stage-a.ps1 -Case A-01 -Driver Manual
.\scripts\run-stage-a.ps1 -Case A-04 -Driver Manual
.\scripts\run-stage-a.ps1 -Case A-05 -Driver Manual

node scripts/compare-drivers.mjs reports/stage-a reports/stage-a/calibration 9964dc9   # all_equivalent: true
node scripts/summarize-stage-a.mjs reports/stage-a                                     # 36/36 pass
.\target\release\e02-verify.exe audit --dir evidence --cases cases\A-01.toml ... --cases cases\A-06.toml   # clean, 564 files
.\target\release\e02-verify.exe audit --dir reports  --cases cases\A-01.toml ... --cases cases\A-06.toml   # clean, 76 files
```

**Operator procedure for D1.** The procedure was given to the operator before each case:

1. A File Explorer window titled `paste-target-1` opens.
2. Click once in its empty area.
3. Press Ctrl+V once.
4. Keep hands off until the window closes itself.
5. Do not copy anything during the test.

## 5. Results: authoritative runs (36)

| Set | Commit | Driver | Runs | Result |
|---|---|---|---|---|
| T1 matrix, A-01 to A-06 × 3 (A-06 pastes into 2 folders) | `d85b1c5`, `e1657ad` | D2c | 18 | 18 pass |
| Memory-peak runs, A-01 and A-04 × 3 | `1c2b07c`, `af648cc` | D2c | 6 | 6 pass |
| Regression after the safety fixes: A-01 to A-06, plus A-01, A-04 and A-05 again | `9964dc9` | D2c | 9 | 9 pass |
| D1 calibration: A-01, A-04, A-05 | `9964dc9` | D1 | 3 | 3 pass |

Every run finished with all of the following:

- a digest match and no unexpected destination entries;
- every stream released;
- agent exit code 0;
- an empty clipboard after exit;
- COM calls only on the owner STA;
- the observer seeing only final file names.

Per-run records are `reports/stage-a/*.json`; the roll-up is `reports/stage-a/stage-a-rollup.json`.

Caveat: the 24 runs before `9964dc9` used the defective exit-ownership gate (§9.3). It made no difference to them, because no other application changed the clipboard during those runs. Their paste observations are valid.

## 6. D1 vs D2c calibration (review item A)

| Case | D1 run | D2c runs (same commit `9964dc9`) | Equivalent |
|---|---|---|---|
| A-01 (1 MiB) | `A-01-20261113T173143-b143b344` | `…220409-1015744e`, `…220503-0bade5a4` | yes, yes |
| A-04 (64 MiB) | `A-04-20261113T173215-4096de28` | `…220434-87a32a49`, `…220511-d9fc6200` | yes, yes |
| A-05 (3 files) | `A-05-20261113T173233-77e58232` | `…220442-ed1d8ba9`, `…220519-9b74765f` | yes, yes |

| Dimension compared | D1 vs D2c |
|---|---|
| COM call sequence (paste phase; UI polling removed) | **Identical.** A-01 and A-04 had 19 steps; A-05 had 29. See the sequence below. |
| Threads | Identical: all COM calls on the owner STA, one thread |
| Stream reads and lifetime | Identical read counts (5, 257, 40), request size (262,144 B), seek pattern and EOF read. `Stat` was never called. Create-to-release lifetime: 4 and 139 ms identical; A-05 1,079 ms (D1) vs 769 and 818 ms (D2c), consistent with the operator's timing. |
| File sizes, names and digests | Identical; all match |
| Clipboard ownership and cleanup | Identical. The agent held ownership (`S_OK` and owner process ours) until exit, then the atomic release left the clipboard empty. |
| Errors and unexpected behavior | None. The only HRESULTs were `S_FALSE` at EOF and `DV_E_FORMATETC` for formats the agent does not offer. |
| UI polling (not required to match) | Identical for A-01 and A-04 (16 / 9 / 3). A-05 D1 had 19 / 11 / 3 against 16 / 9 / 3, as expected from the operator's mouse and keyboard interaction. |

Explorer's paste-phase sequence for one file was the same under D1 and D2c (`!` marks a format the agent does not offer):

```
get_data Shell IDList Array! → CF_HDROP! → Shell IDList Array! → CF_HDROP! → Shell IDList Array!
set_data TargetCLSID → get_data EnterpriseDataProtectionId! → AsyncFlag! → FileGroupDescriptorW
→ Shell Object Offsets! → ZoneIdentifier! → stream_create → get_data FileContents#0
→ Seek(CUR,0) → Seek(SET,0) → [256 KiB reads … zero-byte EOF read] → Seek(SET,0) → stream_release
→ set_data Performed DropEffect → Logical Performed DropEffect → get_data cf#0!
```

Full data: `reports/stage-a/calibration/d1-vs-d2c.json` and `d1-vs-d2c.md`. **D2c is calibrated for the Stage A operations** (small-file paste into Explorer). It is *not* calibrated for cancel, error or termination paths; Stage B §9.4 calibrates those separately.

## 7. Observed Explorer behavior

Unchanged from the first report:

- Explorer reads 256 KiB at a time.
- `Stat` is called only for zero-length files.
- Unimplemented `IStream` methods are never called.
- It requests `ZoneIdentifier`, `AsyncFlag`, `EnterpriseDataProtectionId` and `Shell Object Offsets`, none of which the agent offers.
- It writes the destination directly under its final name. With files this small, that is weak evidence, settled in Stage B.
- No pasted file had a `Zone.Identifier` stream.

## 8. Memory

OS-tracked lifetime peak commit (`PeakPagefileUsage`):

| Case | Runs | Peak commit |
|---|---|---|
| A-01 (1 MiB) | 2.19–2.25 MiB | — |
| A-04 (64 MiB) | 2.18–2.37 MiB | — |
| A-05 | 2.19–2.25 MiB | — |

Peak memory is independent of file size: the largest A-04 minus A-01 difference is 0.12 MiB. Explorer's own memory was not measured (Stage B).

## 9. Harness safety review (item B)

### 9.1 Findings and fixes

| # | Finding in the earlier harness | Fix (commit `897004c`, plus `9964dc9` for #7) |
|---|---|---|
| 1 | Cleanup used `Remove-Item -Recurse` with only a path-prefix check. Windows PowerShell 5.1 can follow junctions during recursive deletion. | `Remove-RunDestination` deletes only the run's expected synthetic files by exact name, re-checking for reparse points immediately before each delete. It removes directories only once they are empty, never uses `-Recurse`, never enters or deletes through reparse points, and leaves and reports anything unexpected. Cleanup runs only after a pass; destinations are kept on fail or inconclusive. |
| 2 | Test root not validated; run IDs unique only to the second. | Test roots must carry a marker and sit on a fixed local drive. They must neither be inside nor contain a protected location (profile, Windows, Program Files, ProgramData, Public), and no reparse point may appear anywhere in the path. Unmarked non-empty directories are never adopted. Run IDs add an 8-hex random suffix and a run marker that cleanup must match. |
| 3 | An aborted script left the agent holding the clipboard (twice observed) and left watchers running. | Everything is in `try/catch/finally`; the stop file is always written. Owned processes are stopped only after an identity check: same process object, PID, start time and image path. The agent and observers watch the harness PID through a `SYNCHRONIZE` handle and exit if it disappears. The agent refuses to start a second instance (named mutex). |
| 4 | Failed runs produced no record (twice observed). | A record is always written, atomically (temporary file, then rename), with `harness.failed_stage`, `harness.exception` and post-cleanup actions. Harness errors and interference are classified as `inconclusive`. |
| 5 | Interference was invisible or aborted the run. | The agent records `clipboard_lost_ms`. A test window that disappears before completion is classified as interference. |
| 6 | `explorer-close` posted `WM_CLOSE` to the top-level window showing the folder. If Explorer had opened the folder as a tab in a pre-existing user window, that whole window would have closed. | Explorer windows are snapshotted before each run (handles and tab counts only). The harness closes a window only if it is new, is the one recorded at open time, and has no other tabs. Otherwise it keeps the window and records why. The decision function is unit-tested. All runs since `897004c` show new single-tab windows. **For runs before `897004c` this cannot be verified retroactively.** |
| 7 | **Agent defect:** see §9.3. | Fixed in `9964dc9`. |

### 9.2 Negative tests: `scripts/test-harness-safety.ps1`, 21 of 21 pass at `9964dc9`

- **Filesystem, FS-01 to FS-13.** These use a private sandbox containing junctions and a sentinel file. They cover:
  - marker creation;
  - refusal of an unmarked directory that holds user data;
  - refusal of drive roots and protected locations;
  - refusal of junction test roots;
  - unique run IDs;
  - exact-file cleanup;
  - unexpected files left and reported;
  - a junction named like an expected file;
  - a paste target replaced by a junction;
  - a run folder without a marker;
  - a mismatched marker;
  - malformed run IDs and names;
  - a test-root marker that disappears;
  - a junction used as the run destination.

  The sentinel survived every test. No real user directory was touched, and no destructive or elevated operation was used.
- **Processes, PR-01 to PR-04.** An identity mismatch is never stopped; an owned process is stopped; observers exit with their parent; an injected failure before the agent starts still yields a durable inconclusive record.
- **Desktop, DT-01 to DT-04** (hands-off window):
  - after losing ownership, the agent does not clear another application's clipboard;
  - if the harness disappears, the agent exits and releases only its own clipboard;
  - a second agent is refused;
  - an injected failure after the agent took the clipboard yields a durable record and a released clipboard.

### 9.3 Defect found by DT-01, and its impact

**Symptom (`harness-safety-20261112T215901.json`, DT-01 failed).** Another process replaced the clipboard. The agent still judged itself the owner and, at exit, **emptied that application's clipboard**.

**Root cause.** windows-rs exposes `OleIsCurrentClipboard` as `Result<()>`, which maps the success code `S_FALSE` ("not on the clipboard") to `Ok(())`. `.is_ok()` was therefore always true. `OleSetClipboard(NULL)` is documented to empty the clipboard regardless of who owns it.

**Fix (`9964dc9`):**

1. Call `OleIsCurrentClipboard` through a raw binding that keeps `S_OK` and `S_FALSE` distinct.
2. Additionally require that the clipboard owner window belongs to this process.
3. At exit, open the clipboard, which blocks other writers. Re-check ownership while it is open. Call `EmptyClipboard` only if the clipboard is still ours. Never call `OleSetClipboard(NULL)`.

DT-01 now passes. Exit traces show `S_OK` before release and `S_FALSE` after.

**Impact on earlier runs:**

- The `still_current` field and clipboard-loss detection in the earlier run records are unreliable.
- The interrupted run `A-01-20261112T213208` (operator interference) may have emptied the operator's clipboard at exit.

**Product relevance.** "Never clear another application's clipboard" is a requirement for the OneCopy agent itself. The same `S_FALSE` mapping pitfall applies to other windows-rs calls that can return `S_FALSE`.

## 10. Threading and provenance (item C)

### 10.1 Threading

- **T1 is proven for Stage A.** STA-bound objects were used, and every COM call was observed on the owner STA in all 36 runs.
- **The agile-object evidence is preserved, not used as proof.** It is in `reports/stage-a/preliminary-agile/`: 18 passing runs whose calls were dispatched on RPC threads because windows-rs defaults to `Agile = true`. It shows only that agile objects *can* serve small, uncontended pastes. It does **not** show that T2 behaves correctly under blocked reads, concurrent streams or reentrancy.
- **Carried into Stage B:**
  - STREAM-08: T1 vs T2 under a 120 s stall while other processes use the clipboard;
  - LIFE-02: concurrent streams;
  - cancellation during a blocked `Read`.

### 10.2 `ZoneIdentifier`: documented semantics (desk research only, nothing tested)

- **Primary source.** The Windows SDK header `um/ShlObj_core.h` (10.0.26100.0) defines:

  ```
  #define CFSTR_ZONEIDENTIFIER  TEXT("ZoneIdentifier")  // DWORD, to be used with CFSTR_FILECONTENTS data transfers
  ```

  windows-rs exposes the same constant.
- **Not documented on Microsoft Learn.** The Shell Clipboard Formats page, accessed 2026-11-12, does not list it.
- **No public semantics found.** There is no documentation of:
  - which DWORD values are meaningful (presumably `URLZONE` values, which is unverified);
  - which targets read it;
  - whether a target writes Mark-of-the-Web (`Zone.Identifier`) because of it;
  - how it interacts with the policy that disables saving zone information (`SaveZoneInformation`).

  No public implementation found in Chromium, Firefox or Thunderbird shows its use.
- **Observed.** Explorer requests it once per paste.
- **Position.** It is a **research lead only**. Even if Explorer turns out to honor it, the zone would be the *source's own assertion*, carried by a target-dependent mechanism. That is **not** trustworthy Mark-of-the-Web protection, and OneCopy must not claim it as such. Per D-033, OneCopy-managed downloads apply provenance themselves (`IAttachmentExecute`). Native-paste provenance stays an observation.
- **Proposed for Stage B (needs owner decision).** An observation-only variant that offers `ZoneIdentifier = 3`. It would record whether a `Zone.Identifier` stream appears, its contents (zone ID only), and whether SmartScreen or attachment prompts change. It would make no product claim either way.

## 11. Privacy and safety checks

- **Audit.** Clean on 564 evidence files and 76 report files: no user name, machine name, profile path or synthetic content marker. The committed `crates`, `scripts` and `cases` directories are also clean. Only the git-ignored `target/` build output contains local paths, and it is never committed.
  - Negative tests: a planted user name and profile path were detected, and so was a real pasted file's content prefix.
  - The audit reports only counts, never the matched values.
- **Data and settings.**
  - Only synthetic data was used.
  - Destinations stayed under the marked test root and were deleted only after a pass; nothing was left in `dest`.
  - No settings were changed, and Defender stayed on.
  - Process Monitor was not used. No elevation was used.
- **Hands-off agreements.** Every desktop-interfering batch ran in a window agreed with the operator. The clipboard was replaced only during those windows.
- **Still blocked.** SEC-04 (needs a disposable VM) and DEST-01 to DEST-04 (VHD creation, deferred).

## 12. Unsupported, inconclusive and not-run items

| Item | Status |
|---|---|
| Run `A-01-20261112T213208` | **Inconclusive (interference).** The operator closed the test window. At the time no record was produced; that harness gap is now fixed. The run may have emptied the operator's clipboard (§9.3). |
| Run `A-06-20261112T212846` | **Inconclusive (harness error):** a stale view handle; no paste was attempted. |
| 18 runs at `6f1b9e1` | **Not T1:** agile objects. Kept as data only. |
| 9 runs at `897004c` | **Superseded:** ran on the pre-fix ownership gate, with a dirty-tree flag caused by an untracked analysis script. Kept in `reports/stage-a/superseded-897004c/`. |
| D2c calibration for cancel, error and termination paths | **Not in Stage A** (Stage B §9.4) |
| Explorer memory, throughput baseline, large files, faults, cancellation, termination, partial-file classification, provenance variants, other targets, T2 | **Stage B** |
| SEC-04, DEST-01 to DEST-04 | **Blocked** (VM; VHD approval) |

## 13. Deviations from the specification, and Stage B carry-over

**Deviations:**

1. **Synthetic source.** It uses the `chacha20` crate instead of `rand_chacha`, for byte-seekable IETF ChaCha20.
2. **D2c is a targeted window message**, not a global keystroke: `WM_COMMAND` 0x701A, an undocumented command ID. It is calibrated equivalent to D1 for Stage A.
3. **T1 needs explicit `Agile = false`.**
4. **OS peak commit** was added alongside the 1 s memory sampler.
5. **A-07** is evaluated in every run.
6. **Harness additions**, not in the specification: the safety module, negative tests, interference classification and Explorer ownership checks.
7. **Clipboard release** uses the atomic open, check and `EmptyClipboard` sequence instead of `OleSetClipboard(NULL)`.

**Stage B carry-over (needs owner approval):**

- T2 stall and concurrency behavior;
- the observation-only `ZoneIdentifier` variant;
- PROC-07, the `leave` exit: what OLE does when the owner process exits without releasing (possible render-all risk for large files);
- hands-off windows of several minutes for the 10 and 100 GiB cases.

## 14. Commits (branch `experiments/e02-stage-a`)

| Commit | Content |
|---|---|
| `6f1b9e1` | Initial workspace |
| `d85b1c5` | T1 fix (`Agile = false`) |
| `e1657ad` | Driver retry |
| `1c2b07c` | OS peak commit |
| `af648cc` | Interference classification |
| `6436d96` | First Stage A evidence and report |
| `897004c` | Harness safety hardening (item B) |
| `9964dc9` | Clipboard-ownership defect fix and D1/D2c comparison tool |

The calibration evidence and this report are in the commits that follow `9964dc9` on this branch.
