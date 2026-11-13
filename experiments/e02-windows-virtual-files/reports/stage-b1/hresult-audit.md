# E-02 Stage B1: HRESULT and return-code audit

**Scope:** every Windows API call in the E-02 experiment workspace (`crates/e02-agent`, `crates/e02-verify`) at commit `ec620ce` and later, plus the COM methods the provider implements.

**Question:** besides `OleIsCurrentClipboard` (the Stage A defect, fixed in `9964dc9`), where can `S_FALSE`, another non-`S_OK` success code, or a failure-shaped success change the meaning of a result?

**Method:**

1. Grep for every Win32 and COM call, and every `.is_ok()`, `.ok()`, `?` and `as_bool()` applied to one.
2. Read the documented return values of each API.
3. Read the `windows-rs` 0.62.2 / `windows-core` 0.62.2 wrapper code for how each return value is mapped.

## Verdicts

| API (site) | Return values that matter | Handling | Verdict |
|---|---|---|---|
| `OleIsCurrentClipboard` (`ownership.rs`) | `S_OK` = on clipboard; `S_FALSE` = not | Raw binding; compared with `S_OK` | **Fixed in Stage A.** Tests `raw_check_distinguishes_s_false_where_the_wrapper_cannot` and `windows_rs_maps_s_false_to_ok` pin both the trap and the fix |
| `OleSetClipboard(NULL)` | Empties the clipboard whoever owns it | Not called any more; release is open → check → `EmptyClipboard` | **Fixed in Stage A** (checklist W-02) |
| `GetMessageW` (`main.rs`) | `>0` message, `0` `WM_QUIT`, **`-1` error** | Was `while GetMessageW(..).as_bool()`: `-1` counted as true, so an error kept the loop spinning | **Fixed in B1** (`control::keep_pumping`). Test `message_loop_stops_on_quit_and_on_error` |
| `SetTimer` (`main.rs`) | `0` on failure | Was unchecked. Without the timer nothing polls the stop conditions | **Fixed in B1.** On failure the provider releases the clipboard and exits. No unit test: the failure cannot be forced |
| `GlobalUnlock` (agent, verify) | `FALSE` with `NO_ERROR` when the lock count reaches zero (that is success) | `windows-rs` maps it to `Err` with code 0; every call site ignores the result | **Correct.** Test `global_unlock_reports_err_when_the_unlock_succeeds` pins the trap |
| `CreateMutexW` + `GetLastError` (`main.rs`) | Valid handle **and** `ERROR_ALREADY_EXISTS` when another agent runs | Last error read immediately, with no call in between | **Correct.** Desktop test DT-03 covers it. The pattern is fragile: any call inserted in between would reset the last error |
| `WaitForSingleObject` (parent watchdogs) | `WAIT_OBJECT_0`, `WAIT_TIMEOUT`, `WAIT_FAILED` | `!= WAIT_TIMEOUT`, so `WAIT_FAILED` counts as "parent exited" | **Correct by intent:** fail-safe, the process stops |
| `OleInitialize` (agent, probe) | `S_OK`, `S_FALSE` (already initialized on this thread) | `?` accepts both; `OleUninitialize` is always called | **Correct**: balanced for both codes |
| `CoInitializeEx` (`dialogs.rs`) | `S_OK`, `S_FALSE`, `RPC_E_CHANGED_MODE` | Explicit: uninitialize only on success codes; tolerate changed mode | **Correct** |
| `CoInitializeEx` (`explorer.rs`) | Same | `.ok()?`; never uninitialized | **Harmless:** a one-shot CLI process. Recorded, not changed |
| `IShellWindows::Item` (`explorer.rs`) | `S_FALSE` with a NULL out pointer when there is no window | `windows-rs` turns the NULL into `Err(Error::empty())`, whose code is `S_OK`; the loop skips it | **Correct by construction.** An error log would show the code `0x00000000` |
| `IDataObject::QueryGetData` (clipboard probe) | `S_OK`, `S_FALSE`, `DV_E_*` | Raw `HRESULT` compared with `S_OK`; recorded verbatim | **Correct** |
| `ISequentialStream::Read` (provider implementation) | `S_OK`; `S_FALSE` = end of stream; failures | The implementation returns a raw `HRESULT` (the `windows-rs` signature keeps it). Injected faults are validated as failure codes | **Correct.** Tests `validation_rejects_success_hresults_and_conflicts` and NB-02: an `S_FALSE` "fault" is rejected |
| `IDataObject::GetCanonicalFormatEtc` (implementation) | `DATA_S_SAMEFORMATETC` (a success code with meaning) | Returned deliberately | **Correct** |
| `ReadDirectoryChangesW` (`fsnotify.rs`) | Success with 0 bytes = buffer overflow | Logged as `fs_overflow` | **Correct** |
| `EnumWindows` / `EnumChildWindows` | `FALSE` when the callback stops early (not an error) | Results ignored | **Correct** |
| `IsClipboardFormatAvailable`, `OpenClipboard`, `EmptyClipboard`, `GetClipboardData`, `PostMessageW`, `SHGetPathFromIDListW`, `QueryFullProcessImageNameW`, UI Automation methods | Plain success or failure | `Result` or `BOOL` | **Correct** |

## Consequential findings and tests added in B1

1. **`GetMessageW` returning `-1`** would have kept the provider's message loop spinning on an error. Fixed and unit-tested.
2. **An unchecked `SetTimer` failure** would have left the provider holding the clipboard with nothing polling its stop conditions. Fixed. On failure it now releases the clipboard and exits at once.
3. **Pinned library behaviors**, so a future `windows-rs` change shows up as a test failure, not as a silent behavior change:
   - `S_FALSE` maps to `Ok`;
   - `GlobalUnlock` reports `Err` with code 0 when the unlock succeeds;
   - raw versus wrapped `OleIsCurrentClipboard` differ.
4. **Injected faults are validated as failure codes**, so a test can never "inject" `S_FALSE`.

## Proposed additions to checklist item W-01 (PROPOSED; owner decision)

- `GetMessage` returns `-1` on error: loop on `> 0`, never on "truthy".
- **Failure-shaped success.** `GlobalUnlock` and an early-stopped `EnumChildWindows` return `FALSE` without an error, and `windows-rs` reports `Err` with code 0. Never propagate these with `?`.
- **Null out-parameter with `S_FALSE`.** `windows-rs` returns `Err(Error::empty())`, whose code is `S_OK`. Do not log or compare that code as a failure reason.
- **Success with a secondary status in `GetLastError`** (for example `CreateMutexW` and `ERROR_ALREADY_EXISTS`): read the last error immediately after the call.
- **`WAIT_FAILED`**: decide explicitly whether it means "stop" (fail-safe) or "retry".
