# Windows engineering checklist

**Status:** Started 2026-11-13 from E-02 Stage A evidence. Each item carries its own status. **W-01 is required** by owner direction (2026-11-13, Stage A acceptance). The other items are PROPOSED until the owner accepts them.

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

## W-02 — Release the clipboard only if still the owner (PROPOSED)

**Source:** the same Stage A defect.

- Do not call `OleSetClipboard(NULL)` to release: it empties the clipboard regardless of owner.
- Release sequence: `OpenClipboard` → check `OleIsCurrentClipboard == S_OK` **and** that the clipboard owner window belongs to this process → `EmptyClipboard` → `CloseClipboard`.
- If ownership was lost, release nothing and record it.

## W-03 — Choose COM apartment and agility deliberately (PROPOSED)

**Source:** E-02 Stage A threading finding (report §10).

- `windows-rs` `#[implement]` objects are agile by default (`IAgileObject` plus the free-threaded marshaler), so cross-process calls may arrive on RPC worker threads, not on the creating STA.
- Every COM object declares its intended threading. STA-bound objects use `Agile = false`.
- Free-threaded objects must have thread-safe state, and tests that issue blocked and concurrent calls. Agile multithreaded behavior is not assumed correct until E-02 Stage B shows it.

## W-04 — No uncontrolled clipboard rendering of virtual files (PROPOSED, pending E-02 Stage B)

- Never call `OleFlushClipboard` on a data object that offers large virtual-file content.
- Bound every rendering path, and measure what happens when the provider process exits without releasing.

## W-05 — Experiment-only shell automation (CONFIRMED, D-041)

- Undocumented shell command IDs (for example the folder-view *Paste* `WM_COMMAND` used by E-02 driver D2c) and input automation are experiment-only. They are never a product dependency.

## W-06 — No provenance claims from undocumented formats (CONFIRMED rule, D-033)

- `CFSTR_ZONEIDENTIFIER` ("ZoneIdentifier") is defined in the Windows SDK header `ShlObj_core.h` but is not documented on Microsoft Learn.
- Supplying it does not by itself establish trustworthy Mark-of-the-Web semantics. Any provenance claim needs authoritative documentation and test evidence.

## Change log

| Date | Change |
|---|---|
| 2026-11-13 | Created with W-01 (required) and W-02 to W-06 from E-02 Stage A |
