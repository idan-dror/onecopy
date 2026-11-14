# Windows engineering checklist

**Status:** Started 2026-11-13 from E-02 Stage A evidence. Each item carries its own status.

- **W-01 is required** by owner direction (2026-11-13, Stage A acceptance).
- **W-02, W-03 and W-04 were accepted** by the owner on 2026-11-14 (E-02 Stage B1 review). Each keeps an explicit evidence limit.
- **W-08 and W-09** record owner decisions D-043 and D-044 (2026-11-14).
- **W-07** records audit findings (facts and their regression tests). Its proposed rule additions stay PROPOSED until the owner accepts them.

**Applies to:** all Windows code in OneCopy: the future Windows adapter (clipboard, shell, COM), its tests, and any Windows experiment code. It complements `PLATFORM_CAPABILITIES.md` §Windows and does not change any CONFIRMED decision.

**Use:** reviewers check every Windows PR against this list. A PR that deviates from an item states the reason and the test that covers it.

## W-01 — Distinguish `S_OK` from other success codes (required)

**Source:** E-02 Stage A defect, fixed in `9964dc9`. See `experiments/e02-windows-virtual-files/reports/E-02-stage-a-report.md` §9.3.

**What happened.** `OleIsCurrentClipboard` returns `S_OK` when the object is on the clipboard and `S_FALSE` when it is not. Both are *success* codes. `windows-rs` converts every success `HRESULT` to `Ok(())`, so `OleIsCurrentClipboard(..).is_ok()` was always true. The experiment's provider therefore believed it still owned the clipboard after another application had replaced it. On exit it called `OleSetClipboard(NULL)`, which empties the clipboard whoever owns it. A negative test caught it clearing another application's clipboard.

**Rules:**

1. **Predicate APIs.** Where an API answers a yes/no question with `S_OK` versus `S_FALSE` (or another success code), compare the raw `HRESULT` with `S_OK`. Never use `SUCCEEDED`, `.is_ok()`, `.ok()?` or `?` for it. Known cases include `OleIsCurrentClipboard` and `IEnum*::Next`. The E-02 Stage B audit extends this list. Treat any new API as a predicate until its documentation says otherwise.
2. **Success codes with meaning.** `CoInitializeEx`/`OleInitialize` returning `S_FALSE` means "already initialized here". It still needs a matching uninitialize call. `IStream::Read` returning `S_FALSE` means a short read or end of stream.
3. **Wrapper review.** Any binding that maps `HRESULT` to `Result` (for example `windows-rs`) is checked per call site for discarded success codes. Prefer a small raw binding, or an explicit match on the code, where the distinction matters.
4. **Destructive follow-ups.** Never base a destructive or clipboard-clearing action on a success-code check alone. Re-verify ownership atomically first (W-02).
5. **Tests.** Every predicate call site has a negative test where the answer is "no" (for example: another process owns the clipboard).

## W-02 — Release the clipboard only if still the owner (ACCEPTED 2026-11-14)

**Source:** the same Stage A defect.

- Do not call `OleSetClipboard(NULL)` to release: it empties the clipboard regardless of owner.
- Release sequence: `OpenClipboard` → check `OleIsCurrentClipboard == S_OK` **and** that the clipboard owner window belongs to this process → `EmptyClipboard` → `CloseClipboard`.
- If ownership was lost, release nothing and record it.

**Evidence limit.** Shown in the E-02 experiment provider on one Windows 11 build (26200): Stage A negative tests, and B1 safety tests DT-01 to DT-06 (28/28 at `dff1796`). It has not been shown for other Windows builds, other session types (Remote Desktop, multiple users), or a product adapter. The window between `OpenClipboard` and `EmptyClipboard` is narrowed, not proven race-free: another process cannot change the clipboard while this process has it open, but `OpenClipboard` itself can fail when another process holds the clipboard.

## W-03 — Choose COM apartment and agility deliberately (ACCEPTED 2026-11-14)

**Source:** E-02 Stage A threading finding (report §10); E-02 B1 findings F3 and F4.

- `windows-rs` `#[implement]` objects are agile by default (`IAgileObject` plus the free-threaded marshaler), so cross-process calls may arrive on RPC worker threads, not on the creating STA.
- Every COM object declares its intended threading. STA-bound objects use `Agile = false`.
- Free-threaded objects must have thread-safe state, and tests that issue blocked and concurrent calls.

**Evidence limit.** B1 showed that STA-bound streams (T1) can block other processes' clipboard reads for the whole of a stalled `Read` (120 s) and silently drop a second paste. Agile streams (T2) did not, in the tested cases. This supports *choosing deliberately*. It does **not** show that agile objects are correct in general. T2 is the experimental candidate for E-02 Stage B2 only, and is not production-approved until the B2 checks listed in E-02 §7.12 pass.

## W-04 — No uncontrolled clipboard rendering of virtual files (ACCEPTED 2026-11-14)

- Never call `OleFlushClipboard` on a data object that offers large virtual-file content.
- Bound every rendering path, and measure what happens when the provider process exits without releasing.

**Evidence limit.** In B1 (64 MiB synthetic files, Windows 11 build 26200), a guarded flush read 0 content bytes and exiting without release rendered nothing. These observations do not prove that no other Windows build, clipboard-history setting or clipboard consumer would render the content. The rule therefore stays a prohibition plus a bound, not a reliance on observed behavior.

## W-05 — Experiment-only shell automation (CONFIRMED, D-041)

- Undocumented shell command IDs (for example the folder-view *Paste* `WM_COMMAND` used by E-02 driver D2c) and input automation are experiment-only. They are never a product dependency.

## W-06 — No provenance claims from undocumented formats (CONFIRMED rule, D-033)

- `CFSTR_ZONEIDENTIFIER` ("ZoneIdentifier") is defined in the Windows SDK header `ShlObj_core.h` but is not documented on Microsoft Learn.
- Supplying it does not by itself establish trustworthy Mark-of-the-Web semantics. Any provenance claim needs authoritative documentation and test evidence.

## W-07 — Findings from the E-02 B1 HRESULT audit (recorded 2026-11-14)

**Source:** `experiments/e02-windows-virtual-files/reports/stage-b1/hresult-audit.md` (PR #4).

Two further defects of the W-01 family were found and fixed in the experiment provider:

| Finding | Effect before the fix | Fix | Regression test |
|---|---|---|---|
| **`GetMessageW` returns `-1` on error.** The loop was `while GetMessageW(..).as_bool()`, so `-1` counted as "continue". | An error would have kept the provider's message loop spinning while it held the clipboard. | Loop only while the result is `> 0` (`control::keep_pumping`). | `message_loop_stops_on_quit_and_on_error` |
| **`SetTimer` returns `0` on failure; the result was unchecked.** | Nothing would poll the provider's stop conditions (stop file, parent exit), so it could hold the clipboard indefinitely. | On failure the provider releases the clipboard and exits at once. | `stop_timer_failure_is_reported` (added in E-02 B2 preparation; B1 had none because the failure could not be forced) |

**Proposed additions to W-01 (PROPOSED; owner decision):**

- `GetMessage` returns `-1` on error: loop on `> 0`, never on "truthy".
- **Failure-shaped success.** `GlobalUnlock` and an early-stopped `EnumChildWindows` return `FALSE` without an error, and `windows-rs` reports `Err` with code 0. Never propagate these with `?`.
- **Null out-parameter with `S_FALSE`.** `windows-rs` returns `Err(Error::empty())`, whose code is `S_OK`. Do not log or compare that code as a failure reason.
- **Success with a secondary status in `GetLastError`** (for example `CreateMutexW` and `ERROR_ALREADY_EXISTS`): read the last error immediately after the call.
- **`WAIT_FAILED`**: decide explicitly whether it means "stop" (fail-safe) or "retry".

## W-08 — Exact length or failure for virtual-file streams (CONFIRMED, D-043)

**Source:** E-02 B1 finding F1: Explorer accepted a stream that ended early as a completed copy (O4, 6 of 6), even with the size advertised.

- Every offered file advertises an explicit size (`FD_FILESIZE`).
- `IStream::Read` delivers **exactly** that many bytes in total, or returns a failure `HRESULT`.
- `S_FALSE` (end of stream) is returned only at the advertised size, and only after the final source-consistency check has passed. Never return `S_FALSE` early to signal a "successful but incomplete" transfer.
- Detect and reject, with a failure `HRESULT`: source shrinkage, unexpected growth, and detectable incompatible modification (file identity, size, change indicators; ADR-0003).
- The final consistency check runs **before** the bytes that complete the file are released, and again before `S_FALSE`.
- Do not claim universal detection of concurrent changes. Residual blind spots are documented per filesystem (ADR-0003).
- Tests cover T1 and T2, and failure at or immediately before the final read.

## W-09 — Native paste must not replace existing destination files (CONFIRMED, D-044)

**Source:** E-02 B1 finding F2: after *Copy and Replace*, Explorer overwrote the existing file in place and deleted it when the stream then failed. Neither the original nor the new content remained (3 of 3).

- Native-paste replacement of an existing destination file is **unsupported for product release** until a safe mechanism is demonstrated (D-044).
- "Small file", "local source" or "high expected success probability" are not mitigations.
- The mechanism that enforces this is **OPEN** (B-14). Until it is decided, Windows adapter work treats any native-paste workflow that can reach Explorer's replace path as not releasable.

## Change log

| Date | Change |
|---|---|
| 2026-11-13 | Created with W-01 (required) and W-02 to W-06 from E-02 Stage A |
| 2026-11-14 | Owner B1 review: W-02, W-03 and W-04 accepted with evidence limits; W-07 (B1 audit findings) recorded; W-08 (D-043) and W-09 (D-044) added |
