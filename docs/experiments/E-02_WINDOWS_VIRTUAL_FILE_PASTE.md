# E-02 — Windows clipboard virtual-file paste: experiment specification

**Status:** PROPOSED. Implementation-ready once the owner approves the documentation baseline **and** this specification. **Not implemented.**
**Experiment class:** Phase 0 feasibility spike. This is throwaway code, not a product foundation.
**Related:** PR-019, PR-023 (CONFIRMED/PROPOSED); PR-028–PR-030, PR-035 (PROPOSED); D-024, D-033 (PROPOSED); ADR-0002, ADR-0007; Gate B (Windows).
**Stage:** 0. Runs on the existing Windows development machine; no purchases and no network services.

Evidence labels: **[V]** verified against Microsoft documentation on 2026-11-12; **[K]** known but not re-verified, so confirm during implementation; **[I]** inference.

---

## 1. Questions this experiment must answer

| Q | Question | Why it matters |
|---|---|---|
| Q1 | Can Explorer paste a OneCopy-provided virtual file (`IDataObject` with `CFSTR_FILEDESCRIPTORW` + `CFSTR_FILECONTENTS` over `IStream`) of 1 GiB, 10 GiB and 100 GiB with bounded provider memory? | Core native-paste feasibility |
| Q2 | What does the target do with the destination file when the stream fails, stalls, ends early, is cancelled, or the provider process dies? | D-024: native-paste guarantees can only be claimed if demonstrated. Partial-file and false-success risk. |
| Q3 | Does any fault produce a **false success**: a target-reported success, or a full-size file with the final name, whose content does not match? | Gate B: "never false success" |
| Q4 | Which `IDataObject`/`IStream` methods and formats does each target call, in what order, from which thread, and how long can a call block before the target UI or system clipboard users are affected? | Provider design (threading, timeouts) |
| Q5 | What limits apply to multi-file and folder descriptors (entry count, `MAX_PATH` relative paths, descriptor size, time-to-first-byte)? | Thresholds for falling back to managed download (PR-030) |
| Q6 | Does the provider process's lifetime and exit behavior (`OleSetClipboard(NULL)` versus `OleFlushClipboard`) risk materializing content into memory? | Bounded-memory invariant |
| Q7 | How do Windows Clipboard History and cloud clipboard interact with a virtual-file data object, with and without the exclusion formats? | Sensitive exposure (D-032), unintended reads |
| Q8 | Is provenance (Mark-of-the-Web, `Zone.Identifier`) applied to natively pasted files, and does any documented source-side signal change that? | D-033 |
| Q9 | How do targets handle hostile descriptor names (traversal, absolute, reserved, alternate data streams, case or normalization collisions)? | Defense in depth. OneCopy's own validation stays mandatory. |
| Q10 | Does at least one non-Explorer target behave the same, or differently? | Avoid inferring universal compatibility from Explorer alone |

The experiment **does not** answer networking, cryptography or cross-device questions. Bytes come from a local synthetic stream that can simulate network delays and faults.

## 2. Scope and non-goals

**In scope:**

- A local provider process that places a virtual-file data object on the clipboard.
- A deterministic synthetic byte source with injectable faults.
- Driving pastes into Explorer and other targets.
- Measuring memory, timing, call patterns, destination outcomes and provenance.

**Out of scope:**

- networking, QUIC or transport;
- encryption;
- the real OneCopy core;
- product UI;
- shell namespace extensions, Cloud Files (`cfapi`) placeholders, drag-and-drop (unless needed for comparison — not planned);
- any installer, service, startup registration or system-wide setting change made by the experiment code.

**Prohibited:**

- real user files or clipboard content;
- disabling or reconfiguring Microsoft Defender, SmartScreen or other security settings;
- writing outside the test root;
- leaving the provider registered anywhere after the run.

## 3. Experiment architecture

### 3.1 Components

```text
experiments/e02-windows-virtual-files/          (isolated Cargo workspace; not part of any product workspace)
  Cargo.toml / Cargo.lock                        (pinned; lockfile committed)
  crates/
    e02-synth/     deterministic, seekable byte generator + fault plan + expected-digest computation (library)
    e02-agent/     provider process: STA owner thread, clipboard data object, IStream objects, JSONL event log
    e02-verify/    destination observer (100 ms polling) + post-run verifier (size, digest, ADS, timestamps)
    e02-driver/    optional programmatic target: IFileOperation::CopyItems(IDataObject) (driver D3)
  cases/           one TOML file per test case (fault plan + variants)
  scripts/
    capture-env.ps1      environment record (no user/machine names)
    run-case.ps1         orchestrates agent, observer, memory sampler, driver; writes run record
    open-target.ps1      opens an Explorer window at the case destination (driver D2)
    sample-memory.ps1    1 s sampling of agent and target process counters
  evidence/        git-ignored raw output (JSONL logs, samples, recordings)
  reports/         committed sanitized summaries (run-record JSON + markdown report)
  README.md
```

### 3.2 Provider design

**Threading model.**

- The **owner thread** is an STA. It calls `OleInitialize`, creates a hidden message-only window, runs a message loop, calls `OleSetClipboard`, and registers `AddClipboardFormatListener` to observe `WM_CLIPBOARDUPDATE`.
- **Variant T1 ("STA streams").** `IStream` objects belong to the owner STA, so all incoming cross-process calls are serialized through its message loop.
- **Variant T2 ("MTA streams").** `IStream` objects live in the MTA, so a blocking `Read` does not occupy the STA that serves other clipboard consumers. The exact mechanism (for example creating them on an MTA worker and passing them through the Global Interface Table) is validated in the first implementation hour [K]. If it cannot be achieved, T2 is reported *not run* with the reason.

**Data object** (`IDataObject`):

- **Offered formats**, in this enumeration order:
  1. `CFSTR_FILEDESCRIPTORW` (`TYMED_HGLOBAL`);
  2. `CFSTR_FILECONTENTS` (`TYMED_ISTREAM`, one per `lindex`);
  3. `CFSTR_PREFERREDDROPEFFECT` (DWORD `DROPEFFECT_COPY`);
  4. a private marker format `OneCopyE02Marker` (opaque run ID only);
  5. optionally the exclusion formats (§5 variants).

  **No `CF_HDROP`**, because no real path exists.
- **Methods:**
  - `GetData`: descriptor, contents (a *new* stream object per call) and drop effect.
  - `QueryGetData`, and `EnumFormatEtc` (via `SHCreateStdEnumFmtEtc` [K], `DATADIR_GET` only).
  - `SetData`: accepts and logs any shell communication format, for example `Performed DropEffect`, `Paste Succeeded`, `TargetCLSID`.
  - `GetDataHere`: returns `DV_E_FORMATETC`.
  - `DAdvise`, `DUnadvise`, `EnumDAdvise`: return `OLE_E_ADVISENOTSUPPORTED`.
- **Optional async variant:** also implements `IDataObjectAsyncCapability` (variant AS) to observe whether a paste target uses it. It is documented for drag-and-drop [V].

**Stream** (`IStream`):

- `Read` serves synthetic bytes according to the fault plan.
- `Stat` returns `cbSize` (configurable: correct, absent or wrong per case) with `STATFLAG_NONAME` semantics.
- `Seek` supports positioning within the generated range (the generator is seekable) and logs every call.
- `Clone`, `Write`, `SetSize`, `CopyTo`, `Commit`, `Revert`, `LockRegion`, `UnlockRegion` return `E_NOTIMPL` or `STG_E_INVALIDFUNCTION`, and every call is logged. If a target requires one of them, that is itself a finding.
- **Read timeout** (variant RT): a blocking `Read` returns an error after `read_timeout_ms` instead of waiting indefinitely.

**Exit modes:**

- `clear` (default): `OleSetClipboard(NULL)` if `OleIsCurrentClipboard` is true.
- `leave`: exit without clearing.
- `flush`: `OleFlushClipboard`, used **only** in case P-04 with a guarded size.
- `crash`: `TerminateProcess` on itself at a planned point.

**Language:** Rust with `windows-rs` COM implementation (`#[implement]`), consistent with review AR-08. **Fallback:** if a `windows-rs` limitation blocks the provider for more than one working day, implement the provider in C++/WRL and record the reason. The measurements stay identical.

### 3.3 Synthetic source (`e02-synth`)

- **Content.** Byte *n* of file *i* in run seed *s* is the ChaCha20 keystream at word position *n* for stream *i*, with the key `BLAKE3.derive_key("onecopy e02 synth v1", s)` [I]. The stream is incompressible, seekable and reproducible. No content is stored, because the expected digest is recomputed by regenerating.
- **Expected digest:** BLAKE3 of the expected bytes (whole file), computed by `e02-verify` from (seed, file index, size).
- **Names:** synthetic, `e02-<case>-f<index>.bin`, with subfolders for folder cases. Never derived from real data.
- **Fault plan (per file).** Each fault has a trigger offset or time; all are deterministic.

| Fault | Effect |
|---|---|
| `latency_ms` per `Read` | Adds a delay to every read |
| `throttle_mib_s` | Caps throughput |
| `stall {at_offset, duration_ms}` | Pauses delivery |
| `error {at_offset, hresult}` | Fails the read |
| `premature_eof {at_offset}` | Ends the stream early |
| `overrun {extra_bytes}` | Delivers more than advertised |
| `short_reads {max_bytes}` | Limits bytes per read |
| `descriptor_delay_ms` | Delays the descriptor `GetData` |
| `contents_getdata_delay_ms` | Delays the contents `GetData` |
| `final_read_error {hresult}` | Simulates a failed source-consistency result on the last read |
| `self_kill {at_offset}` | Provider kills itself |

### 3.4 Event log (no content)

- **Format:** JSONL with these fields: `ts_ns` (monotonic), `run_id`, `case_id`, `thread_id`, `apartment`, `event`, and per-event fields.
- **Events:** `set_clipboard`, `clipboard_update`, `query_get_data`, `get_data`, `set_data`, `stream_create`, `stream_read`, `stream_stat`, `stream_seek`, `stream_other_call`, `stream_release`, `fault_injected`, `exit`.
  - Format names appear as registered names.
  - `SetData` logs the DWORD value only for shell communication formats.
- **Read logging:** every non-`S_OK` read, the first and last 200 reads per stream, and every 1,000th read. Plus a per-stream histogram of request sizes, return sizes and inter-call gaps, and total bytes served.
- **Memory:** the provider samples its own `PrivateUsage` and `WorkingSetSize` (`GetProcessMemoryInfo`) every 1 s.
- **No content.** The log never contains generated content, real paths outside the test root, user names or machine names.

## 4. Dependencies and tools

Environment checked on 2026-11-12 (read-only inspection):

| Item | Requirement | Status on dev machine |
|---|---|---|
| OS | Windows 11 (record edition, version, build and UBR) | Windows 11 Home, build 26200; Explorer 10.0.26100.x |
| Rust | Stable, `x86_64-pc-windows-msvc` | rustc 1.99.1 stable MSVC: present |
| MSVC linker + Windows SDK | Visual Studio 2022 Build Tools or Community with the C++ workload, Windows 11 SDK | VS 2022 Community, SDK 10.0.26100: present |
| Crates (pinned in `Cargo.lock` at implementation) | `windows` (features: `Win32_Foundation`, `Win32_System_Com`, `Win32_System_Com_StructuredStorage`, `Win32_System_Ole`, `Win32_System_Memory`, `Win32_System_DataExchange`, `Win32_UI_Shell`, `Win32_UI_Shell_Common`, `Win32_UI_WindowsAndMessaging`, `Win32_Storage_FileSystem`, `Win32_System_ProcessStatus`, `Win32_System_Threading`), `windows-core`, `rand_chacha`, `blake3`, `serde`, `serde_json`, `toml`, `clap` | To be added. No network crates. |
| Disk | NTFS destination with ≥ 250 GiB free (a 100 GiB destination plus headroom; the source is synthetic, so no source file) | Secondary NTFS volume with > 1 TB free: OK |
| PowerShell | 5.1 (built in) for scripts; `Get-Counter`, `Get-Item -Stream`, `Get-MpComputerStatus` (read-only) | Present |
| Screen recording | Snipping Tool screen recording (built in) for D1 runs | Present (verify) |
| Process Monitor (Sysinternals) | **Optional.** File-system trace of the target's create, write, rename and delete on the destination. Needs elevation. | **Not installed.** Installation needs owner approval. Fallback: `e02-verify` 100 ms directory polling. |
| Elevated `diskpart` (VHD creation) | **Optional.** Small NTFS, FAT32 and exFAT VHDs for disk-full and filesystem cases | Available on Home with elevation. **The operator runs these steps; elevation is not automated.** |
| Hypervisor | Not needed, except to run Q9 hostile-name cases in a disposable VM (preferred) | Optional (Stage 0b) |

## 5. Variants (combinable flags per case)

| Flag | Values | Default |
|---|---|---|
| `T` threading | T1 STA streams / T2 MTA streams | T1 |
| `FS` advertised size | `FD_FILESIZE` set (correct) / absent / set but wrong | set |
| `PUI` | `FD_PROGRESSUI` on / off | on |
| `META` | `FD_WRITESTIME` and `FD_ATTRIBUTES` set / absent | absent |
| `AS` | `IDataObjectAsyncCapability` offered / not offered | not offered |
| `RT` | Provider read timeout off / 30 s | off |
| `EX` | Exclusion formats (`ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`=0, `CanUploadToCloudClipboard`=0) offered / not | not offered |
| `UD` | `CFSTR_UNTRUSTEDDRAGDROP` offered / not. Documented only for IE drag-and-drop policy [V]; informational. | not offered |

## 6. Targets and drivers

| ID | Target | Driver | Notes |
|---|---|---|---|
| G-01 | Explorer folder window | **D1** manual Ctrl+V (ground truth); **D2** scripted: `open-target.ps1` opens the folder, focuses the window, sends Ctrl+V | D2 is valid only after the calibration subset matches D1 (§9.4) |
| G-02 | Desktop | D1 | Destination is the Desktop. Synthetic names only; files are removed after the run. |
| G-03 | Common Item Dialog hosted by another process (Notepad *Save As* or *Open* dialog file list, Ctrl+V into the list) | D1 | **Required second target.** Shell copy hosted in a non-Explorer process. |
| G-04 | `IFileOperation::CopyItems(IDataObject, destination)` + `PerformOperations` | D3 (`e02-driver`) | `CopyItems` accepts an `IDataObject` [V]. Whether it consumes virtual-only formats is itself a result. |
| G-05 | Microsoft Edge (paste into a page's file input or drop zone, if one is available locally) | D1 | Optional, informational |
| G-06 | Office or Outlook | D1 | Optional; *not run* if not installed |

## 7. Test cases

Priority: **P0** mandatory for the Gate B decision · **P1** should run · **P2** optional.
Repeats: P0 cases run **3×** (each 100 GiB case runs **2×**); P1 cases run 2×; P2 cases run 1×.
Default destination: `<test-root>\dest\<run-id>\` on the NTFS volume. Default target and driver: G-01/D1 for calibration, then D2.

### 7.1 Baseline (BASE)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| BASE-01 | P0 | 1 file, 1 MiB | Call sequence; result; digest |
| BASE-02 | P0 | 1 GiB | Throughput; provider and target memory; digest |
| BASE-03 | P0 | 10 GiB | Same as BASE-02 |
| BASE-04 | P0 | 100 GiB | Same as BASE-02; memory slope versus bytes |
| BASE-05 | P1 | 0-byte file (`FD_FILESIZE` = 0) | Created correctly |
| BASE-06 | P0 | Reference writer: `e02-verify --write-reference` writes 10 GiB of synthetic data directly to the same volume | Baseline throughput for the ratio |

### 7.2 Descriptor and size semantics (DESC)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| DESC-01 | P0 | 1 GiB, FS absent | Progress UI; result; any `Stat` or `Seek` usage |
| DESC-02 | P0 | 5 GiB (> 4 GiB, high DWORD used), FS set | Correct size handling |
| DESC-03 | P0 | **Premature EOF** at 50%, FS set (correct size) | Target result; destination size and name; outcome class |
| DESC-04 | P0 | **Premature EOF** at 50%, FS absent | Same as DESC-03 |
| DESC-05 | P1 | Overrun of +1 MiB, FS set | Extra bytes written, truncated, or error |
| DESC-06 | P1 | `Stat.cbSize` ≠ FS | Which value the target trusts |
| DESC-07 | P2 | META set | Timestamps and attributes applied to the destination |

### 7.3 Delayed and synthetic stream behavior (STREAM)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| STREAM-01 | P0 | 1 GiB, `latency_ms` = 50 per Read | Throughput; read-size pattern |
| STREAM-02 | P0 | 10 GiB, `throttle_mib_s` = 20 | Progress-UI behavior; memory |
| STREAM-03 | P0 | `stall` 30 s at 50% | UI responsiveness; resume after stall; result |
| STREAM-04 | P0 | `stall` 180 s at 50% | Does the target time out? After how long? Outcome |
| STREAM-05 | P0 | `descriptor_delay_ms` = 10,000 | Explorer UI blocked? Paste-menu latency |
| STREAM-06 | P1 | `contents_getdata_delay_ms` = 10,000 | Same as STREAM-05 |
| STREAM-07 | P1 | `short_reads` ≤ 4 KiB | Throughput; correctness |
| STREAM-08 | P0 | Stall 120 s with **T1** versus **T2**; meanwhile another process reads clipboard text (timed `Get-Clipboard` loop) and opens Notepad's Edit menu | Effect of a blocked STA on system-wide clipboard users |

### 7.4 Errors and partial files (ERR)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| ERR-01 | P0 | `error` `E_FAIL` at 50% (1 GiB) | Dialog text; destination state after dialog *Cancel*; outcome class |
| ERR-02 | P0 | `error` `STG_E_READFAULT` at 50% | Same as ERR-01 |
| ERR-03 | P1 | `error` `HRESULT_FROM_WIN32(ERROR_NETWORK_UNREACHABLE)` at 50% | Message shown to the user |
| ERR-04 | P0 | `final_read_error` on the last Read (simulated source-consistency failure) | Whether a full-size or near-full partial remains; outcome class |
| ERR-05 | P0 | ERR-01, then the user chooses *Try again* | Does the target call `GetData` again (new stream) or `Seek`? Final result |
| ERR-06 | P1 | ERR-01, then *Skip* | Destination state |
| ERR-07 | P0 | 5 files, `error` in file 3 | States of files 1–5 |
| ERR-08 | P0 | Destination already contains the same name (pre-created synthetic file); the user chooses *Replace*; `error` at 50% | **Is the original file lost?** Outcome |
| ERR-09 | P1 | ERR-04 at 10 GiB | Same as ERR-04 at scale |

### 7.5 Cancellation (CANCEL)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| CANCEL-01 | P0 | Cancel in the progress dialog at about 10%, 50% and 99% (10 GiB, throttled) | Time from click to stream `Release`; destination state |
| CANCEL-02 | P0 | Cancel while `Read` is stalled (RT off) | Does cancel wait for `Read`? Time to dismiss |
| CANCEL-03 | P0 | CANCEL-02 with RT = 30 s | Cancel latency bound by the provider timeout |
| CANCEL-04 | P1 | Close the Explorer window during the copy | Does the copy continue? Outcome |

### 7.6 Process termination and lifetime (PROC)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| PROC-01 | P0 | `taskkill /F` the provider during an active `Read` (10 GiB) | Time to target error; dialog text; destination state; Explorer stability |
| PROC-02 | P0 | `taskkill /F` the provider between Reads (stall plus kill) | Same as PROC-01 |
| PROC-03 | P0 | Provider killed **before** paste (after `OleSetClipboard`) | Paste availability and the error shown |
| PROC-04 | P0 | Provider exits gracefully with `clear` | Clipboard emptied; no memory spike |
| PROC-05 | P0 | **Guarded `flush`:** a 512 MiB virtual file; the provider calls `OleFlushClipboard` | Does flushing read the stream into memory (provider `Read` calls, memory)? Is the data available after exit? **Hard cap 512 MiB.** |
| PROC-06 | P1 | Target process (`explorer.exe` hosting the window) killed mid-copy, then restarted | Provider stream released? Leaks? Provider stays healthy |
| PROC-07 | P1 | Provider `leave` exit | Clipboard state afterwards |

### 7.7 Clipboard lifetime and concurrency (LIFE)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| LIFE-01 | P0 | Paste the same data object twice in sequence | A new stream per `GetData`; both digests correct |
| LIFE-02 | P1 | Two concurrent pastes to two folders | Concurrent streams on the same `lindex`; correctness |
| LIFE-03 | P0 | Another app copies text during the paste (clipboard replaced) | Does the in-flight copy complete? Outcome |
| LIFE-04 | P0 | Clipboard History **on** (operator toggles Settings → System → Clipboard), EX off; no paste for 60 s | `GetData` or `QueryGetData` calls made without a paste (who reads?) |
| LIFE-05 | P0 | LIFE-04 with EX on | Difference from LIFE-04 |
| LIFE-06 | P2 | Lock the workstation during a throttled copy | Copy continues? |
| LIFE-07 | P2 | Sleep and resume during a throttled copy | Outcome |

### 7.8 Destination conditions (DEST) — optional elevated VHD steps run by the operator

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| DEST-01 | P1 | 1 GiB NTFS VHD, 2 GiB file, FS set | Early refusal versus mid-copy failure; partial state |
| DEST-02 | P1 | DEST-01 with FS absent | Same as DEST-01 |
| DEST-03 | P1 | FAT32 VHD, 5 GiB file | Early "file too large" check? Partial? |
| DEST-04 | P1 | exFAT VHD, 1 GiB file | `Zone.Identifier` (alternate data streams) absent as expected |
| DEST-05 | P0 | Name collision → *Keep both* / *Skip* | Naming behavior |
| DEST-06 | P2 | Read-only destination folder (synthetic test folder) | Error path |

### 7.9 Folders and descriptor limits (FOLD)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| FOLD-01 | P0 | Folders with 1, 1,000 and 10,000 small files (4 KiB) as `FILE_ATTRIBUTE_DIRECTORY` entries plus relative paths | Descriptor build time and HGLOBAL size; time to first byte; total time; correctness |
| FOLD-02 | P0 | Relative path lengths of 200, 259 and 260 or more characters (truncation is unavoidable at 260 because of `cFileName[MAX_PATH]` [V]) | Target behavior at the boundary |
| FOLD-03 | P1 | Nesting depth 50 | Behavior |
| FOLD-04 | P1 | Unicode names: NFC versus NFD pair, RTL, emoji; case-collision pair (`a.bin`/`A.bin`); trailing dot or space | Created names; collisions handled? |
| FOLD-05 | P2 | 100,000 entries | Descriptor ~tens of MB [K]; feasibility only |

### 7.10 Security and provenance (SEC) — hostile-name cases preferably in a disposable VM

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| SEC-01 | P0 | After BASE-02 and DEST-05 on each target: read `Get-Item -Stream *` and the `Zone.Identifier` content | Mark-of-the-Web applied: yes/no, zone, per target |
| SEC-02 | P1 | UD on (`CFSTR_UNTRUSTEDDRAGDROP` DWORD) | Any change in prompts or Mark-of-the-Web. Informational, because the format is documented for IE drag-and-drop. |
| SEC-03 | P1 | Paste a benign synthetic `.cmd` that only echoes text, then open it | Any SmartScreen or attachment prompt? |
| SEC-04 | P0 | Hostile `cFileName` values: `..\x.bin`, `sub\..\..\x.bin`, an absolute path *inside the test root's sentinel directory*, `x.bin:ads`, `CON`, `NUL.bin`, `COM1` | **No write outside the destination folder.** Record each outcome. Sentinel directories are checked before and after. Run in a disposable VM if Stage 0b is approved; otherwise only inside the nested test root. |
| SEC-05 | P1 | Microsoft Defender protection history after the runs (read-only) | Were pasted files scanned or flagged? |
| SEC-06 | P0 | Log audit: search the logs for known synthetic plaintext markers and for user-profile paths | Logs contain none |

### 7.11 Additional targets (TGT)

| ID | Pri | Setup | Measure / record |
|---|---|---|---|
| TGT-01 | P0 | BASE-02, DESC-03, ERR-01, CANCEL-01 and PROC-01 against **G-03** (Common Item Dialog in Notepad) | Same columns as for Explorer |
| TGT-02 | P0 | BASE-02 and ERR-01 through **G-04** (`IFileOperation`) | Supported? Same outcome as G-01? |
| TGT-03 | P2 | BASE-01 against G-05 and G-06 | Informational |

## 8. Measurements

| Metric | Method |
|---|---|
| Provider private bytes and working set | Self-sampling every 1 s (`GetProcessMemoryInfo`) |
| Target process memory (`explorer.exe` window host, or `notepad.exe` for G-03) | `sample-memory.ps1` (`Get-Process` every 1 s). Record the "launch folder windows in a separate process" setting. |
| Memory slope | Linear fit of provider private bytes against bytes served. Must be ≈ 0. |
| Throughput | Bytes served over time (provider log) and destination size over time (observer). Ratio to BASE-06. |
| Call pattern | Event-log counts, order, request-size histogram, thread and apartment |
| Latency marks | Paste → descriptor `GetData`; → first contents `GetData`; → first `Read`; → last `Read`; → `Release`; target dialog shown (D1 recording timestamp) |
| Destination lifecycle | Observer: name, size, attributes, and the appearance and disappearance of any temporary names, at 100 ms resolution. Process Monitor trace if approved. |
| Outcome | Final listing; BLAKE3 digest versus expected; `Zone.Identifier`; timestamps |
| System clipboard impact (STREAM-08) | `Get-Clipboard` call latency distribution while a `Read` is blocked |

## 9. Outcome classification and acceptance criteria

### 9.1 Destination outcome classes (every fault case)

| Class | Definition | Meaning for claims |
|---|---|---|
| **O1** | The target reports failure and no destination file remains (or only a target-owned temporary file that is removed) | Native paste may be claimed as "no false success" for this condition |
| **O2** | The target reports failure; a partial file remains, smaller than the advertised size and/or under a temporary name | Native paste must state the limitation. Owner decision on UX wording (Gate B). |
| **O3** | A file with the **final name and full advertised size** remains with **wrong content**, whatever the target reported | **False success.** Native paste is not offered for this condition without a demonstrated mitigation. |
| **O4** | The target reports **success** but the content is wrong or truncated | **False success.** Same consequence as O3. |
| **O5** | The target hangs for more than 120 s or crashes | Native paste is not offered without a mitigation (for example a read timeout) demonstrated to remove it |

### 9.2 Pass criteria (each must be met for Gate B "go" on the stated condition)

| AC | Criterion |
|---|---|
| AC-1 | BASE-02/03/04 complete with a digest match on G-01 in every repeat. Provider private bytes ≤ 256 MiB and memory slope < 1 MiB per GiB served. |
| AC-2 | No **O3 or O4** in any P0 fault case: DESC-03/04, ERR-01/02/04/05/07/08, CANCEL-01/02/03, PROC-01/02, LIFE-03, for G-01 and G-03. Each **O2** is listed with its exact condition. |
| AC-3 | Cancel takes effect within 2 s when no `Read` is blocked. With RT = 30 s, cancel takes effect within 35 s while a `Read` is blocked. No O5. |
| AC-4 | After the provider is killed (PROC-01/02), the target shows an error within 30 s, does not crash, and the outcome class is recorded |
| AC-5 | PROC-04 shows no content materialization at exit. PROC-05 quantifies the `flush` risk. |
| AC-6 | LIFE-04/05 document whether Clipboard History reads the data object. With EX on, there are no contents reads without a paste. |
| AC-7 | At least one non-Explorer target (G-03) has results for the TGT-01 set |
| AC-8 | SEC-04: zero writes outside the destination folder on every target tested. Any write outside is a **critical finding** and is reported immediately. |
| AC-9 | Folder limits measured (FOLD-01/02) and expressed as proposed fallback thresholds (PR-030) |
| AC-10 | SEC-06 log audit is clean |
| AC-11 | Every P0 case has the required repeats, or is marked *not run* or *inconclusive* with a reason |

Provenance (SEC-01/02) has **no** pass or fail. It is a finding that feeds D-033.

### 9.3 Experiment completion criteria

The experiment is **complete** when:

- every P0 case is classified (pass, fail, inconclusive, or not run with a reason);
- the outcome-class table per target is filled in;
- §11 fallbacks are evaluated against the results;
- the report states which native-paste claims, if any, are supported, for which sizes, targets and conditions.

### 9.4 Driver calibration

Before D2 or D3 results are used:

1. Run BASE-02, ERR-01, CANCEL-01 and PROC-01 with D1 and with D2/D3.
2. Compare call sequences, outcome classes and dialogs.
3. Treat a driver as equivalent only if the outcome classes and observable destination states match.

## 10. Environment preparation and safety

1. **Test root.** Create `<NTFS-volume>:\OneCopyE02\` with `dest\`, `sentinel\` and `nested\a\b\c\` (for SEC-04). Do not use system folders or user-profile folders. G-02 uses the Desktop with synthetic names only and is cleaned up afterwards.
2. **Record the environment** (`capture-env.ps1`, read-only):
   - OS edition, version, build and UBR; Explorer version; the folder-window separate-process setting;
   - Clipboard History and cloud clipboard state; Defender product version and real-time protection state;
   - volume filesystem and free space; CPU model and RAM size;
   - git commit SHA; rustc version; hash of `Cargo.lock`.

   **Never record** user name, machine name or user-profile paths.
3. **Close** other clipboard managers. Record anything that cannot be closed.
4. **Settings.** Clipboard History is toggled **manually by the operator** for LIFE-04/05 and restored afterwards. The experiment code changes no settings.
5. **Disk space.** Delete destination files after each verified run. The 100 GiB cases write about 100 GiB per run to the SSD; record the total bytes written.
6. **Optional elevation.** Elevated steps (VHD creation, Process Monitor) are run by the operator and listed in the run record.
7. **Abort.** A kill switch (`run-case.ps1 -Abort`) stops the provider, clears the clipboard and stops the samplers.

## 11. Fallback options (decision tree)

| Finding | Fallback |
|---|---|
| O3/O4 for some condition (for example premature EOF with FS absent) | Make that condition impossible on the provider side (always set FS, never return early EOF, error instead). Rerun. If false success persists, do not offer native paste for that condition: use managed download, or evaluate Cloud Files placeholders under an ADR (ADR-0005). |
| O2 is common for large files | Native paste only below a measured size threshold. Above it, managed download is the default (PR-030), subject to the owner's UX decision. |
| A blocked `Read` freezes other clipboard users (STREAM-08, T1) | Adopt T2 (MTA streams) plus a provider read timeout. If T2 is infeasible, enforce short timeouts and return errors. |
| Cancel cannot interrupt a blocked `Read` | Provider read timeout (RT) is mandatory; its value comes from CANCEL-03 |
| Descriptor limits (`MAX_PATH`, 10k+ entries) cause failures or long blocking | Folder native paste only within measured limits; beyond them, managed download |
| `IFileOperation` cannot consume virtual formats | Use D2 automation only; record G-04 as unsupported |
| `windows-rs` blocks the provider | C++/WRL provider (§3.2) |
| Explorer applies no Mark-of-the-Web to pasted files | Record. Native paste then carries no provenance; owner decision on whether that is acceptable for executables and documents (D-033). |

## 12. Evidence collection and reporting

**Run record** (one JSON per case repeat, committed after sanitization):

```json
{
  "experiment_id": "E-02",
  "case_id": "ERR-01",
  "repeat": 1,
  "run_id": "<uuid>",
  "commit_sha": "<git sha of experiment code>",
  "env": { "os": "...", "build": "...", "ubr": 0, "explorer_version": "...", "separate_process": false,
           "clipboard_history": "on|off", "defender_rtp": true, "dest_fs": "NTFS", "dest_free_gib": 0,
           "cpu": "...", "ram_gib": 0, "rustc": "...", "cargo_lock_blake3": "..." },
  "target": "G-01", "driver": "D1",
  "variants": { "T": "T1", "FS": "set", "PUI": "on", "META": "absent", "AS": false, "RT": "off", "EX": false, "UD": false },
  "files": [ { "name": "e02-ERR-01-f0.bin", "size": 1073741824, "seed": 42 } ],
  "faults": [ { "kind": "error", "at_offset": 536870912, "hresult": "0x80004005" } ],
  "timings_ms": { "paste_to_descriptor": 0, "to_first_read": 0, "to_last_read": 0, "to_release": 0, "to_target_dialog": 0 },
  "bytes_served": 0, "read_calls": 0,
  "memory": { "provider_private_peak_mib": 0, "provider_slope_mib_per_gib": 0.0, "target_private_peak_mib": 0 },
  "destination": { "exists": true, "final_name": true, "size": 0, "digest_match": false, "temp_names_seen": [], "zone_identifier": null },
  "target_report": "error dialog text (verbatim, no paths)",
  "outcome_class": "O2",
  "result": "pass|fail|inconclusive|not_run",
  "notes": "",
  "artifacts": ["evidence/<run_id>/events.jsonl", "evidence/<run_id>/recording.mp4"],
  "reviewer": "pending"
}
```

- **Raw evidence** (event logs, memory samples, Process Monitor traces, screen recordings) stays in the git-ignored `evidence/<run_id>/` directory. Its BLAKE3 hashes go into the run record, so the raw evidence can be produced on request.
- **Report** (`reports/E-02-report.md`):
  - an environment summary;
  - an outcome-class matrix (cases × targets);
  - memory and throughput plots or tables;
  - the call-pattern summary;
  - provenance findings;
  - proposed thresholds;
  - the claims that are and are not supported;
  - deviations from this specification.
- **Sanitization check before commit:** no user names, machine names or user-profile paths; no content.

## 13. Effort and sequence (estimates, not commitments)

1. Implementation: provider, synthetic source, observer and verifier, scripts. About 4–6 working days, including the threading-variant spike.
2. Calibration (§9.4). About 0.5 days.
3. P0 cases on G-01 and G-03, then G-04. About 2–3 days (each 100 GiB run is minutes of copying plus verification).
4. P1 and P2 cases as time allows. About 1–2 days.
5. Report and proposed thresholds. About 1 day.

## 14. Items to confirm at implementation start (first hour)

- `windows-rs` `#[implement]` support for `IDataObject`, `IStream` and `IDataObjectAsyncCapability`, and how to place stream objects in the MTA (variant T2) [K].
- `SHCreateStdEnumFmtEtc` availability and behavior for the format enumerator [K].
- Exact registered format names (`FileGroupDescriptorW`, `FileContents`, `Preferred DropEffect`, `Performed DropEffect`, `Paste Succeeded`, `UntrustedDragDrop`) against the SDK headers [K].
- Whether Snipping Tool screen recording is available on this build; otherwise use timestamps from D2 logs plus screenshots.

## 15. Readiness

This specification is **implementation-ready** once:

1. the owner approves the documentation baseline and this specification;
2. the owner states whether optional Process Monitor installation and operator-run elevated VHD steps are permitted (the experiment is still valid without them, using the polling observer and skipping DEST-01 to DEST-04);
3. the owner states whether SEC-04 must wait for a disposable VM (Stage 0b) or may run inside the nested test root.

No other dependency is missing on the current development machine.
