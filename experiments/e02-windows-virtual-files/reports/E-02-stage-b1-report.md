# E-02 Stage B1 report: fault handling and correctness of Windows virtual-file paste

**Experiment:** E-02, Stage B1 (owner direction, 2026-11-13), Phase 0. B1 covers fault handling and correctness on bounded synthetic files. Stage B2 (1, 10 and 100 GiB, other targets) has **not started**.

**Status:** B1 is implemented and validated, and is ready for owner review. The PR is not merged.

**Date of runs:** 2026-11-13.

**Run scope:**

| Item | Value |
|---|---|
| Target | G-01 (an Explorer folder window) |
| File sizes | ≤ 64 MiB per file (bounded; scale variants are B2) |
| Paste driver | D2c (calibrated in Stage A) |
| Dialog driver | D2u (UI Automation by control id), calibrated against D1 (operator clicks) |

**The question B1 answers:** is native Windows file paste safe and reliable under interruption and failure, not merely under ideal conditions?

## 1. Outcome

**Answer:** **partly.** Explorer handles stream errors, provider termination and cancellation safely: it shows an error and leaves no file (O1). However, B1 found **two unsafe target behaviors** that a provider must design around, and **one T1 defect class** that rules out STA-bound streams.

**Unsafe target behaviors and T1 defects:**

| # | Finding | Repeats | Severity |
|---|---|---|---|
| **F1** | **A stream that ends early is accepted as a successful copy.** Explorer keeps a truncated file under its final name, shows no error and sets `Performed DropEffect` = copy. It does not compare the bytes received with `FD_FILESIZE`. | DESC-03 and DESC-04: **6 of 6 O4** | **False success** |
| **F2** | **"Copy and Replace" followed by a stream failure destroys the existing file.** Explorer overwrites the destination in place, with no temporary file. When the read fails it deletes the file, so neither the original nor the new copy remains. | ERR-08: **3 of 3** | **Data loss.** The spec classifies this as O1 (failure reported, no file), but O1–O5 do not capture loss of a pre-existing file. |
| **F3** | **In T1, a blocked `Read` blocks every other application's clipboard data request.** A request from another process waited 119.8 s, the whole 120 s stall. **T2/T2A: at most 4–21 ms.** | STREAM-08: 3 of 3 (T1) versus 4 of 4 (T2/T2A) | Availability |
| **F4** | **In T1, a second paste issued while a `Read` is blocked is silently dropped.** No file is created and no error is shown. **T2 serves it concurrently.** | LIFE-02s: 3 of 3 (T1) versus LIFE-02st2: 3 of 3 correct (T2) | Silent failure |

**Safe behaviors** (each fault was repeated; **no O3, no O2, no O5**):

- **Read failures.** An injected failure at offset 0, 50%, the final data byte, or the end-of-stream read *after* all data was delivered gives an error box and a deleted destination (O1). This holds for `E_FAIL`, `STG_E_READFAULT`, network-unreachable and timeout HRESULTs.
  - A provider can therefore veto a completed copy at the last moment by failing the final end-of-stream read (ERR-04e).
- **Provider killed mid-read** (self-termination or external kill): an error box ("The remote procedure call failed.") appears within 0.2–0.4 s and the file is deleted (O1). Explorer stays stable.
- **Cancellation** while no read is blocked takes effect in 0.92–0.95 s and leaves no file.
- **Stalls of 30 s and 180 s** complete correctly. Explorer has **no timeout of its own**.
- **Clipboard replaced by another application mid-copy:** the copy completes correctly (T1 and T2).
- **Destination window closed mid-copy:** the copy continues and completes correctly.

**Counts:**

| Item | Result |
|---|---|
| B1 run records | 135: 127 completed, 8 inconclusive (all from harness defects; §11) |
| Cases | 47, each with the required repeats (P0 ×3, P1 ×2, P2 ×1) |
| Outcome classes | S 63, O1 55, O4 6, N 3, O2/O3/O5 0 |
| Safety suite | 28 of 28 at `dff1796` |
| Unit tests | 40 of 40 |
| Privacy audit | Clean (2,257 raw evidence files and 158 report files) |
| Clipboard | Clean after every run |
| Explorer | No crash |

**Recommendation:** proceed to B2 **on T2**, subject to owner decisions on F1 and F2 (§14).

## 2. O1–O5 outcome matrix (G-01, completed runs)

**Class labels:**

- **O1–O5:** as defined in spec §9.1.
- **S:** success reported and content correct.
- **N:** no destination effect and no report (paste silently did nothing).
- **C:** a complete, correct file remains after a reported failure (per file only).

Destination states come from the BLAKE3 verifier: `complete_correct`, `partial_correct_prefix`, `absent`, `original_preserved`, and so on.

| Case | Pri | Fault | Threading | Runs | Explorer reports | Destination | Class |
|---|---|---|---|---|---|---|---|
| BASE-01 | P0 | none, 1 MiB | T1 | 3/3 | success | complete, correct | S |
| BASE-01t2 / BASE-01a | P1/P2 | none, 1 MiB | T2 / T2A | 2/2, 1/1 | success | complete, correct | S |
| DESC-01s | P0 | none, `FD_FILESIZE` absent (control) | T1 | 3/3 | success | complete, correct | S |
| **DESC-03** | P0 | **early EOF at 50%**, `FD_FILESIZE` correct | T1 | 3/3 | **success** | **32 MiB of 64, final name** | **O4** |
| **DESC-04** | P0 | **early EOF at 50%**, `FD_FILESIZE` absent | T1 | 3/3 | **success** | **32 MiB, final name** | **O4** |
| DESC-05 | P1 | overrun +1 MiB (source grew) | T1 | 2/2 | success | 64 MiB (the advertised size); extra bytes read and discarded | S* |
| DESC-06 | P1 | `Stat.cbSize` ≠ `FD_FILESIZE` | T1 | 2/2 | success | complete, correct. `Stat` was never called | S |
| ERR-01 | P0 | `E_FAIL` at 50% | T1 | 4/4 | failure ("Unspecified error") | absent | O1 |
| ERR-01z | P0 | `E_FAIL` on the first read | T1 | 3/3 | failure | absent | O1 |
| ERR-02 | P0 | `STG_E_READFAULT` at 50% | T1 | 3/3 | failure ("A disk error occurred during a read operation.") | absent | O1 |
| ERR-03 | P1 | network unreachable at 50% | T1 | 2/2 | failure ("The network location cannot be reached…") | absent | O1 |
| ERR-04 | P0 | failure on the read that delivers the final byte | T1 | 3/3 | failure | absent | O1 |
| ERR-04e | P0 | all data delivered, then the end-of-stream read fails | T1 | 3/3 | failure | absent | O1 |
| ERR-05 | P0 | `E_FAIL` once, then "Try Again" | T1 | 3/3 | failure. **Try Again is not offered** (OK only) | absent | O1 |
| ERR-06 | P1 | `E_FAIL`, then "Skip" | T1 | 2/2 | failure. **Skip is not offered** | absent | O1 |
| ERR-07 | P0 | 5 files, error in file 3 | T1 | 3/3 | failure | files 1–2 complete and correct; files 3–5 absent | O1 (files 1–2: C) |
| **ERR-08** | P0 | existing file → **Copy and Replace** → `E_FAIL` at 50% | T1 | 3/3 | failure | **original deleted** | O1 by definition; **data loss (F2)** |
| STREAM-01 | P0 | 50 ms latency per read | T1 | 3/3 | success | complete, correct | S |
| STREAM-03 | P0 | 30 s stall at 50% | T1 | 3/3 | success | complete, correct | S |
| STREAM-04 | P0 | 180 s stall at 50% | T1 | 3/3 | success (no target timeout) | complete, correct | S |
| STREAM-04rt | P0 | 180 s stall, provider read timeout 30 s | T1 | 3/3 | failure ("…timeout period expired.") | absent | O1 |
| STREAM-05 / STREAM-06 | P0/P1 | 10 s `GetData` delay (descriptor / contents) | T1 | 3/3, 2/2 | success | complete, correct | S |
| STREAM-07 | P1 | short reads ≤ 4 KiB | T1 | 2/2 | success | complete, correct | S |
| STREAM-08 | P0 | 120 s stall + clipboard probe | **T1** | 3/3 | success | complete, correct | S (**F3**) |
| STREAM-08t2 / STREAM-08a | P0/P2 | same | **T2 / T2A** | 3/3, 1/1 | success | complete, correct | S |
| CANCEL-01a/b/c | P0 | cancel early / ~50% / ~99% (throttled) | T1 | 3/3, 4/4, 3/3 | cancelled | absent | O1 |
| CANCEL-02 | P0 | cancel during a 90 s stalled read | T1 | 3/3 | cancelled after ~89 s | absent | O1 |
| CANCEL-03 | P0 | CANCEL-02 with read timeout 30 s | T1 | 3/3 | cancelled; then a timeout error box | absent | O1 |
| CANCEL-04 | P1 | destination window closed at 25% | T1 | 2/2 | success (copy continued) | complete, correct | S |
| PROC-01 | P0 | provider terminates itself mid-read | T1 | 4/4 | failure ("The remote procedure call failed.") | absent | O1 |
| PROC-02 | P0 | provider killed 2 s into a stalled read | T1 | 3/3 | failure (RPC failed) | absent | O1 |
| PROC-03 | P0 | provider killed before the paste | T1 | 3/3 | **nothing happens** (no dialog) | absent | N |
| PROC-05 | P0 | guarded `OleFlushClipboard` at exit (8 MiB) | T1 | 3/3 | success | complete, correct | S |
| PROC-07 / PROC-07b | P1 | exit without releasing the clipboard (8 MiB / 64 MiB) | T1 | 2/2, 2/2 | success | complete, correct | S |
| LIFE-02 / LIFE-02t2 | P1 | 2 concurrent pastes | T1 / T2 | 2/2, 2/2 | success | both complete, correct | S |
| **LIFE-02s** | P0 | 2 concurrent pastes; the first stalls 20 s | **T1** | 3/3 | success for paste 1; **paste 2 silently dropped** | file 1 correct; **file 2 absent** | S + **N (F4)** |
| LIFE-02st2 | P0 | same | **T2** | 3/3 | success | both complete, correct | S |
| LIFE-03 / LIFE-03t2 | P0 | another app replaces the clipboard mid-copy | T1 / T2 | 3/3, 3/3 | success | complete, correct | S |
| SEC-01z | P1 | `ZoneIdentifier` = 3 offered (observation) | T1 | 2/2 | success | complete, correct; **Zone.Identifier written** (§6) | S |

\* **DESC-05 caveat.** It is S only because the synthetic "true size" equals the advertised size. With a live source that grew, the result would be a silently truncated copy reported as success (§3.1).

Per-run data: `reports/stage-b1/stage-b1-rollup.json` (with `stage-b1-rollup.md`) and the 135 records in `reports/stage-b1/*.json`.

## 3. Findings by area

### 3.1 Premature EOF and incorrect advertised sizes

**Premature EOF (DESC-03, DESC-04): Explorer trusts the stream's end, not the descriptor.**

- The provider returned `S_FALSE` with 0 bytes at 32 MiB.
- Explorer finished the copy, truncated the pre-allocated 64 MiB file to 32 MiB and called `SetData(Performed DropEffect = 1)`.
- No error was shown.
- The result is the same with `FD_FILESIZE` set (DESC-03) or absent (DESC-04).
- **O4 in 6 of 6 runs.**

**Overrun (DESC-05): Explorer trusts the descriptor.** Explorer read the stream to its end (65 MiB served) but wrote only the advertised 64 MiB, silently.

**Stat (DESC-06).** Explorer never calls `IStream::Stat` for a non-empty file, so a wrong `Stat` size has no effect.

**Consequence for the provider.** These two behaviors are inconsistent with each other (end of stream is trusted, but extra bytes beyond the descriptor are dropped). A provider must therefore:

- deliver **exactly** the advertised byte count;
- return a **failure HRESULT**, never an early end of stream, whenever the source cannot deliver it (truncated, shrunk, unreadable);
- **fail** rather than overrun when the source has grown.

ERR-04 and ERR-04e show that a failure at the last data read, or at the end-of-stream read, produces O1.

### 3.2 `IStream::Read` failures at different offsets

**Every injected failure gave the same O1 outcome.** This covers offsets 0, 50%, the final byte and the end-of-stream read, with `E_FAIL`, `STG_E_READFAULT`, `HRESULT_FROM_WIN32(ERROR_NETWORK_UNREACHABLE)` and `HRESULT_FROM_WIN32(ERROR_TIMEOUT)`:

- an error task dialog "Error copying file or folder" appears 0.2 s after the failure;
- it shows the system text for the HRESULT and **one button (OK)**;
- the destination is deleted.

**Partial and pre-allocated files.** Change notifications show what happens on disk:

- Explorer creates the destination under its **final name**;
- it immediately sets its size to the full advertised size;
- it then writes;
- on failure it deletes the file.

**No retry or skip.** No "Try Again" or "Skip" is offered for a virtual-file stream error (ERR-05, ERR-06), so any retry has to be OneCopy's own, outside the native paste.

**Multi-file objects (ERR-07).** Files completed before the failing file stay complete and correct. The failing file and every later file are absent, so the whole operation aborts.

### 3.3 Delayed, stalled and timed-out reads

| Situation | Explorer behavior |
|---|---|
| Latency (50 ms per read), short reads (≤ 4 KiB), 10 s `GetData` delays, 30 s and 180 s stalls | All complete correctly |
| A stalled read | Explorer waits indefinitely. Its progress dialog stays open; no target timeout was observed up to 180 s |
| A provider read timeout of 30 s (variant RT, STREAM-04rt) | Turns the stall into an error box ("This operation returned because the timeout period expired.") and a deleted file (O1) |

Explorer UI responsiveness during the 10 s `GetData` delays (STREAM-05, STREAM-06) was not measured. Only completion was.

### 3.4 Cancellation and provider termination

| Case | Cancel → dialog gone (runner clock, ±0.2 s) | Destination |
|---|---|---|
| CANCEL-01a/b (no blocked read) | 0.92–0.95 s | absent |
| CANCEL-01c (cancel during a throttled read that takes 5 s) | 3.8–4.1 s | absent |
| CANCEL-02 (cancel during a 90 s stalled read, no RT) | **88.7–89.2 s.** Cancel waits for the read to return | absent |
| CANCEL-03 (same, RT = 30 s) | 29.7–29.9 s, then Explorer shows a timeout error even though the user cancelled | absent |

**Spec AC-3 is met.** Cancel takes effect in ≤ 2 s when no read is blocked, and in ≤ 35 s with RT = 30 s. **Cancel cannot interrupt a blocked read**, so the provider read timeout (RT) is mandatory. That matches spec §11 fallback 4.

**Termination (AC-4 met):**

- **PROC-01** (provider terminates itself mid-read) and **PROC-02** (killed during a stalled read): an error box appears within 0.2–0.43 s and the file is deleted.
- **PROC-03** (provider killed after placing the clipboard, before the paste):
  - OLE's private formats `DataObject` and `Ole Private Data` stay on the clipboard, pointing at a dead process.
  - **Paste then does nothing and shows nothing** (N, 3 of 3).

### 3.5 Clipboard replaced by another application during an active copy (LIFE-03)

- **Copy:** the in-flight copy completes correctly in T1 and T2 (3 of 3 each). Explorer holds its own references to the data object and streams.
- **Effect on the replacing application:** its `EmptyClipboard` waited **6.9–15.6 ms in T1** (the owner STA was busy serving throttled reads) against **0.17–0.19 ms in T2**.
- **Harness:** it removed the tagged replacement text afterwards.

### 3.6 Destination partial-file and false-success behavior

| Condition | Can the result be mistaken for a complete file? |
|---|---|
| Stream error, provider death, cancel, read timeout | **No.** The file is deleted, in every run |
| Early end of stream (F1) | **Yes.** Explorer reports success and leaves a truncated file under its final name |
| Replace, then failure (F2) | The destination is gone and **so is the user's original** |
| Explorer interrupted mid-copy (crash, logoff, power) | **Not tested** (PROC-06; §12). Because Explorer pre-allocates the final-name file to full size, such an interruption could leave a full-size file with partial content (an O3 risk) |

`Performed DropEffect` is set on every reported success, **including the false success (F1)**. It is not a correctness signal.

### 3.7 Provider lifecycle and clipboard persistence after exit

| Exit mode | What happened | Clipboard afterwards |
|---|---|---|
| `clear` (every other run) | Released only while the provider still owned the clipboard (§7) | Empty |
| `leave` (PROC-07, PROC-07b) | `OleUninitialize` made **0 `GetData` calls and read 0 bytes**: no render-all | Only stale OLE formats |
| guarded `flush`, 8 MiB (PROC-05) | `OleFlushClipboard` asked for `FileContents` (`lindex -1`) and received a stream, but **read 0 bytes** from it | Descriptor, drop effect and marker persisted; **no file contents** |

- **Memory:** provider peak commit was 2.0–2.8 MiB in every run, so no size-proportional buffering occurred.
- **Limitation:** flush was tested only on a bounded 8 MiB object, by design. The byte budget would have capped any render.

## 4. T1 versus T2 (agile objects as the T2 candidate)

**Mechanism.** `windows-rs` agile objects (`IAgileObject` plus the free-threaded marshaler) are the T2 mechanism:

- **T2:** the data object is STA-bound and the streams are agile.
- **T2A:** both are agile.
- **T1:** both are STA-bound (`Agile = false`).

**Threads observed:**

- **T1:** every COM call ran on the owner STA, in all 108 T1 runs with a provider summary. The 10 killed-provider runs leave no summary.
- **T2/T2A:** stream reads ran on 2–4 RPC threads. The data object stayed on the STA in T2.

| Behavior | T1 | T2 / T2A |
|---|---|---|
| Another process's OLE `GetData` during a 120 s stalled read (STREAM-08) | **Blocked 119.8 s** (3 of 3); probe samples 22–24 | ≤ 4 ms (21 ms worst); probe samples 260–263 |
| Win32 format enumeration during the stall | ≤ 0.3 ms | ≤ 4.9 ms |
| Second paste while a read is stalled (LIFE-02s) | **Silently dropped** (3 of 3) | **Served concurrently.** The second copy finished in about 0.3 s while the first was stalled (3 of 3) |
| Another app's `EmptyClipboard` during throttled reads (LIFE-03) | 6.9–15.6 ms | 0.17–0.19 ms |
| Correctness of every completed copy | correct | correct |

**Not yet shown for T2:**

- Correct multithreaded behavior under **faults**: errors, kills and cancels were run on T1 only in B1. T2 ran only with stalls and concurrency.
- Behavior beyond two concurrent pastes, or under stress.
- Thread safety is by design (per-stream mutex, atomics, global log and statistics mutexes), not by stress test.

The preliminary agile evidence from Stage A is still treated as data only.

**Conclusion.** T1 is unsuitable for a provider whose reads can block (network-backed). T2 removes both T1 defects observed here. **B2 should run on T2** and repeat the key fault cases on T2.

## 5. HRESULT audit

The full audit is `reports/stage-b1/hresult-audit.md`. Besides the Stage A `OleIsCurrentClipboard` defect, two consequential issues were found and fixed:

1. **`GetMessageW` returning `-1`** kept the provider's message loop running. Fixed and unit-tested.
2. **An unchecked `SetTimer` failure** would have left the provider holding the clipboard with no stop polling. Fixed: on failure it releases and exits.

**Library behaviors pinned by tests:**

- `S_FALSE` maps to `Ok`;
- `GlobalUnlock` reports `Err` with code 0 on success;
- raw versus wrapped `OleIsCurrentClipboard` differ.

**Fault validation.** Injected faults must be failure HRESULTs (unit test, and safety test NB-02).

**Proposed additions to checklist W-01 (PROPOSED):**

- `GetMessage` returning `-1`;
- failure-shaped successes;
- `Err(Error::empty())` with code 0;
- success with a secondary status in the last error;
- `WAIT_FAILED`.

## 6. Provenance (observation only, no Mark-of-the-Web claim)

| Source offered | Destination |
|---|---|
| No `ZoneIdentifier` format (every other run) | No `Zone.Identifier` stream on any pasted file |
| `ZoneIdentifier` = 3 (SEC-01z, 2 of 2) | Explorer wrote `Zone.Identifier`: `[ZoneTransfer]`, `ZoneId=3` **followed by a NUL byte** (25 bytes) |

Explorer requests `ZoneIdentifier` on every paste. The format remains undocumented on Microsoft Learn (SDK header only).

**Not tested:**

- whether Windows components (SmartScreen, attachment services, Office Protected View) treat that stream as Internet-zone evidence;
- what the trailing NUL means.

**No provenance or Mark-of-the-Web claim follows** (D-033, checklist W-06). SEC-02/03/05 are B2 candidates.

## 7. Safety and clipboard-ownership results

**Safety suite: 28 of 28 at `dff1796`.**

| Group | Tests |
|---|---|
| Filesystem | FS-01–13 |
| Processes and records | PR-01–05 |
| B1 additions | NB-01–04 |
| Desktop | DT-01–06 |

The new tests cover:

| Test | What it checks |
|---|---|
| NB-01 | `write-synthetic` never overwrites |
| NB-02 | Unbounded B2 cases, flush above 64 MiB and `S_FALSE` "faults" are refused before the clipboard is touched |
| NB-03 | `dialog-act` refuses a window that is not this run's |
| NB-04 | B1 run ids are validated by cleanup |
| PR-05 | The B1 runner writes a durable inconclusive record after an injected failure |
| DT-05 | Clipboard release never empties another application's clipboard |
| DT-06 | Only content tagged with this run is removed |

**Clipboard ownership:**

- **After every run:** no OneCopy content was left on the clipboard (135 of 135).
- **Release paths used:** the agent's own atomic release; the replacement tag (LIFE-03); the marker data (PROC-05 flush); stale OLE-only formats within two sequence steps of the provider's own set (PROC-03, PROC-07; unit-tested rule).
- **Foreign content:** never emptied.

**Processes:** the shell and every run window's host process stayed alive in every run, and no provider was left running.

**Incident: one leftover dialog during the first unattended pass.**

- ERR-08 showed a "Copy File" conflict dialog whose buttons the harness did not yet know.
- The dialog stayed open on the operator's desktop for about 20 minutes, until the end of the batch.
- I then closed it with its own Cancel button (the dialog was attributed to the run by its synthetic file name).
- The harness now knows those buttons, and at the end of every run it closes any dialog attributed to that run.
- No data or settings were affected.

**Operator involvement:**

- D1 dialog calibration (operator clicks);
- every desktop batch ran only in an agreed hands-off window;
- no security settings changed, no elevation, no user files.

## 8. Driver calibration (spec §9.4)

**Comparison:** D1 (operator clicks the dialog button) against D2u (UI Automation invokes the button by control id). Paste is D2c in both. The calibration set is spec §9.4 minus BASE-02 (scale, B2).

**Result: 9 of 9 pairs equivalent.** No difference in outcome class, target verdict, dialog kinds, error text, destination states, change-notification names, stream behavior or clipboard state.

| Case | D1 runs | D2u runs | Equivalent |
|---|---|---|---|
| ERR-01 | 1 | 3 | 3/3 |
| CANCEL-01b | 1 | 3 | 3/3 |
| PROC-01 | 1 | 3 | 3/3 |

`reports/stage-b1/calibration/d1-vs-d2u.{json,md}`.

**What D2u is.** Experiment-only automation (D-041), not keyboard injection:

- it invokes one button, by control id, in one dialog;
- the dialog must be attributed to the run by owner chain, owner-chain text or run token, and is re-validated before each invocation.

The UI here is Hebrew, so labels are never used.

## 9. Environment

- **OS:** Windows 11 Home 25H2, build 26200.9711.
- **Explorer:** 10.0.26100.8875; folder windows in the shell process (separate process = 0).
- **UI language:** Hebrew.
- **Defender:** 4.18.26100.4, real-time protection on (unchanged).
- **Test volume:** NTFS, 1.3 TB free.
- **Machine:** AMD Ryzen 7 5700X3D, 32 GiB RAM.
- **Toolchain:** rustc 1.99.1.
- **Run records:** carry the commit and the dirty flag (0 dirty runs).

## 10. Exact commands

```powershell
cd experiments\e02-windows-virtual-files
cargo build --release --locked
cargo test --release --locked                      # 40 of 40 (8 agent, 21 synth, 11 verify)
cargo clippy --release --locked --all-targets      # 0 warnings
.\scripts\test-harness-safety.ps1 -IncludeDesktop  # 28 of 28 (hands-off window)
# Stage A regression on the B1 provider: pass
.\scripts\run-stage-a.ps1 -Case A-01 -Driver ExplorerCommand -ReportDir reports\stage-b1\regression-a
.\scripts\run-stage-a.ps1 -Case A-05 -Driver ExplorerCommand -ReportDir reports\stage-b1\regression-a
.\scripts\run-stage-a.ps1 -Case A-06 -Pastes 2 -Driver ExplorerCommand -ReportDir reports\stage-b1\regression-a
# D1 dialog calibration (operator clicks), then D2u
.\scripts\run-stage-b.ps1 -Case ERR-01 -DialogDriver Manual     # likewise CANCEL-01b, PROC-01
.\scripts\run-stage-b.ps1 -Case ERR-01 -DialogDriver Uia
# Matrix: pass 1 (47 cases), repeat rounds 2 and 3, ERR-08 rerun
.\scripts\run-b1-batch.ps1 -Cases BASE-01,...,STREAM-04     # evidence\batch1.log
.\scripts\run-b1-batch.ps1 -Cases LIFE-02,...,STREAM-04     # evidence\batch2-r1.log, -r2.log, -r3.log
.\scripts\run-b1-batch.ps1 -Cases ERR-08 -Repeats 3
# Analysis
node scripts\summarize-stage-b1.mjs reports\stage-b1
node scripts\b1-timings.mjs reports\stage-b1
node scripts\compare-b1-dialog-drivers.mjs reports\stage-b1 reports\stage-b1\calibration ERR-01,CANCEL-01b,PROC-01
.\target\release\e02-verify.exe audit --dir evidence --cases cases\<each>.toml        # clean, 2,257 files
.\target\release\e02-verify.exe audit --dir reports\stage-b1 --cases cases\<each>.toml # clean, 158 files
```

The full case lists are in the batch logs; `scripts/b1-cases.json` lists all 47 cases with priorities.

## 11. Harness defects found and corrected during B1

Every defect below was found from run evidence, fixed in a separate commit, and accounted for. **Raw records are never edited.** Corrections are listed in `reports/stage-b1/corrections.json` and applied by the roll-up.

| Defect | Effect | Fix | Runs affected |
|---|---|---|---|
| English button labels; the UI is Hebrew | First D1 runs misclassified as O4 | Control-id matching (`76fb295`) | 3, moved to `superseded-3835aab/` |
| Error box names no file; token-only attribution | Error dialog not attributed | Owner-chain and owner-text attribution (`dec08cc`) | Same 3 |
| 100 ms polling missed short-lived files | Missing destination events | `ReadDirectoryChangesW` observer (`76fb295`) | Same 3 |
| Operator clicked during the "automated" calibration part | Records labelled D2u were operator-clicked | Re-run hands-off | 3, moved to `superseded-76fb295/` |
| Short-lived `explorer.exe` launchers counted as crashes | False O5 | Watch the shell and run-window host processes (`edb289a`) | 3 corrected to S |
| Conflict-dialog ids unknown, then compared without the `#n` position | ERR-08 never chose Replace; the first run left a dialog open | `tileMain#1`, `btnCancel`, `Test-OffersAction` (`edb289a`, `a8c4da5`) | 4 marked inconclusive |
| `$( )` flattened the concurrent paste group | LIFE-02* pasted sequentially | `7dbb85f` | 4 marked inconclusive |
| Cancel versus later error precedence | CANCEL-03 pass 1 reported as failure | `edb289a` | 1 corrected to cancelled |
| Run-level verdict in multi-paste runs | T1 LIFE-02s shown as O4 instead of a dropped paste (N) | Per-destination verdict (`a8c4da5`) | Re-derived by the roll-up |
| End-of-run dialog cleanup on a null path | The record would have been lost after an early failure | Caught by safety test PR-05; fixed before any run | None |

## 12. Unsupported, inconclusive and not run

**Inconclusive (8):**

| Runs | Reason |
|---|---|
| ERR-08 × 4 | Harness could not choose Replace |
| LIFE-02, LIFE-02t2, LIFE-02s, LIFE-02st2 (first pass) | Pasted sequentially |

Every affected case has its full set of valid repeats.

**Superseded (6):** in `superseded-3835aab/` and `superseded-76fb295/`, with READMEs.

**Not run in B1** (outside the owner's B1 list, or needing a separate decision):

| Case | Reason |
|---|---|
| PROC-06 (Explorer killed mid-copy) | Would kill the operator's shell: folder windows share the shell process. Needs a VM or explicit approval. It is the only test of the pre-allocation O3 risk (§3.6) |
| LIFE-04/05 (Clipboard History) | Operator settings toggle |
| LIFE-06/07 | Lock, sleep |
| DEST-05/06 | |
| FOLD-* | |
| SEC-02/03/05 | |
| TGT-* (G-03, G-04) | |
| BASE-02–06, DESC-01/02, STREAM-02, ERR-09 | Scale; these are B2 |
| CANCEL-01 at 10 GiB, PROC-01 at 10 GiB | Ran at 64 MiB |
| T2 under errors, kills and cancels | |
| STREAM-08's Notepad Edit-menu part | Replaced by an OLE clipboard probe |
| Explorer UI responsiveness during `GetData` delays | |

**Blocked (unchanged):** SEC-04 (disposable VM only) and DEST-01 to DEST-04 (VHD deferred).

## 13. Deviations from the specification

1. **Sizes.** B1 fault cases use 64 MiB instead of 1 or 10 GiB (owner: B1 before scale). The case loader refuses cases above 256 MiB unless marked `large` (B2 only).
2. **Added cases:** ERR-01z, ERR-04e, BASE-01t2/a, DESC-01s, STREAM-04rt, STREAM-08t2/a, CANCEL-01a/b/c (one per cancel point), CANCEL-03 dual action, LIFE-02s/st2, PROC-07b, SEC-01z.
3. **PROC-01** uses self-termination (`TerminateProcess`) at a byte offset. **PROC-02** is a harness kill by PID and identity 2 s into a stall. The spec used `taskkill /F` for both.
4. **PROC-05** flush is at 8 MiB with a byte budget, instead of 512 MiB (owner: no uncontrolled render, bounded cases).
5. **Throttle** is implemented as a sleep inside `Read`, so a throttled read is a blocked read (this explains CANCEL-01c's 3.8–4.1 s).
6. **Dialog driver D2u** was added and calibrated (§8).
7. **Classification** gained S, N and C labels, per-destination verdicts, and a documented corrections file.
8. **No Process Monitor.** A `ReadDirectoryChangesW` observer was used instead.

## 14. Newly discovered issues and proposed mitigations (owner decisions)

| Issue | Proposed mitigation (PROPOSED) |
|---|---|
| **F1: early end of stream is a false success** | **Adapter rule:** the provider delivers exactly `FD_FILESIZE` bytes, always sets `FD_FILESIZE`, and turns any source shortfall, shrink or growth into a failure HRESULT, never an early `S_FALSE`. Before returning the final end-of-stream `S_FALSE`, it re-checks source consistency and fails that read on mismatch (ERR-04e shows this veto works). Add to the Windows checklist and spec §11 as demonstrated. |
| **F2: Replace followed by a failure destroys the original** | The provider cannot see that a Replace happened. Options: **(a)** accept and document the risk for native paste; **(b)** offer native paste only for sources likely to complete (local, small, or already verified), and use managed download (temporary file plus atomic rename) otherwise; **(c)** investigate in B2 whether `IDataObjectAsyncCapability` or descriptor flags change Explorer's overwrite strategy. **Owner decision needed (Gate B).** |
| **F3 and F4: T1 blocks other clipboard users and drops concurrent pastes** | Adopt **T2** (agile streams on RPC threads) together with the provider read timeout (RT). Run B2 on T2 and repeat the key fault cases on T2. |
| **No target timeout, and cancel waits for a blocked read** | **RT is mandatory.** Its value comes from cancel latency versus network stalls: 30 s was tested; a lower bound needs B2 data. |
| Explorer pre-allocates the final-name file at full size | Untested O3 risk if Explorer itself is interrupted. Test PROC-06 in a VM. |
| Provider death before paste: silent no-op; stale OLE formats on the clipboard | The product agent should register a restart or cleanup path, and tell the user when a pasted item became unavailable (UX spec). |
| `Performed DropEffect` is set on false success | Never treat it as proof of correct delivery. |
| No Try Again or Skip for stream errors | Retries must be OneCopy-side (re-copy), not native. |
| Provenance: `ZoneIdentifier` produces a Zone.Identifier stream (`ZoneId=3` + NUL) | Observation only. Evaluate in SEC-02/03 before any D-033 claim. |

## 15. Recommendation on B2

**Proceed to B2**, with the following conditions:

1. **Owner decisions:**
   - acknowledge F1 and approve the "exact length or fail" adapter rule;
   - decide how native paste handles F2.

   B2 measures scale and compatibility; it should not be read as clearing these two findings.
2. **Threading.** Run B2 primarily on **T2**, with T1 kept only for a throughput baseline.
3. **Scope additions:**
   - repeat ERR-01, DESC-03 (with the provider failing instead of ending early), PROC-01 and CANCEL-01 at 1 and 10 GiB on T2;
   - add G-03 (Common Item Dialog) and G-04 (`IFileOperation`).
4. **Hands-off windows.** The 10 and 100 GiB cases need separately agreed windows of several minutes each, as previously agreed.
5. **Not in B2 without approval:** PROC-06 (needs a VM) and SEC-04 (VM).

## 16. Commits (branch `experiments/e02-stage-b`)

| Commit | Content |
|---|---|
| `3835aab` | B1 fault injection, T2 variant, harness, 47 cases |
| `76fb295` | Harness fixes from the first calibration (control ids, owner attribution, change notifications) |
| `dec08cc` | Owner-chain text attribution; observed button ids |
| `ec620ce` | Records choices the target does not offer; closes leftover dialogs |
| `a9a2131` | Batch list parsing |
| `edb289a` | Conflict-dialog choices, process-based crash check, cancel precedence |
| `232c07d` | Roll-up and calibration tools; README |
| `89a1669` | Escaped `ZoneId` value |
| `7dbb85f` | Concurrent paste grouping |
| `cb3a681` | Documented corrections in the roll-up |
| `a8c4da5` | Per-destination verdicts; position-aware action matching |
| `dff1796` | Stale-OLE rule as a tested function; timing tool |

The evidence and this report are in the commits that follow `dff1796`. The Stage A closure documentation is in PR #3 (docs only).
