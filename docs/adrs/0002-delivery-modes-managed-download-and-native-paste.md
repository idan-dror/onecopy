# ADR 0002: Delivery modes — managed download and native paste

Status: Accepted (Phase 0 baseline, 2026-11-12)
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-024 (CONFIRMED). Related: PR-012 (CONFIRMED), PR-023, NFR-003, E-02, E-03

## Context and evidence

`TRANSFER_AND_ROUTING.md` specifies `.onecopy-partial` files, a checkpoint journal, verification and atomic rename. Those guarantees assume OneCopy writes the destination.

With Windows Shell virtual files (`CFSTR_FILEDESCRIPTOR` / `CFSTR_FILECONTENTS` over `IStream`), Explorer or another target writes the destination [V: Microsoft Shell data-transfer documentation]. OneCopy supplies bytes and can return errors from `IStream::Read`. It cannot:

- journal or resume the target's write;
- choose the target's temporary name;
- verify before the target finalizes;
- delete a partial file that the target leaves behind.

Similar limits apply to File Provider, Cloud Files and Android provider paths. What each target actually does on error, cancellation or source-process termination is unknown until measured.

## Decision

1. **Managed download.** OneCopy writes the destination file itself, in a location the user chose or approved. It provides:
   - partial-file isolation;
   - a journal of verified ranges and resume;
   - end-to-end verification;
   - controlled finalization (atomic rename where the filesystem supports it);
   - an explicit partial-file cleanup policy.

   This mode carries the PR-012 / NFR-003 guarantees.
2. **Native paste.** An OS component or third-party target writes the destination by pulling bytes from OneCopy.
   - OneCopy claims **none** of the managed-download guarantees for a target unless an experiment (E-02 for Windows targets, E-03 for macOS) demonstrated them for that target and condition.
   - OneCopy must never signal a clean end of stream for bytes that failed source-consistency validation.
   - A source-consistency trailer (or an equivalent final check) only governs OneCopy's own end-of-stream and error signal. It is **not** assumed to solve the target's handling of partial files.
3. **Mode selection.** When native paste is unavailable, unverified for the condition, or over evidence-based limits (size, entry count, path length), the user is offered managed download (PR-023). Thresholds come from E-02 data, not from assumptions.

## Alternatives considered

- **One guarantee set for all paths.** Not implementable, because OneCopy does not control third-party writers.
- **Native paste only on Windows.** Would drop resume and verification for very large transfers.
- **Managed download only.** Loses the native UX that is a key differentiator.

## Security/privacy and interoperability effects

- Managed downloads can apply provenance marking (ADR-0007).
- Native-paste provenance depends on the target and is observed in E-02.
- Path validation must happen before a descriptor is offered to any target. Target-side path handling is defense in depth only.

## User/product effects

- Users may see two actions: "Paste" (native) and "Save with OneCopy" (managed).
- UX must explain any limitation in one sentence (for example "large transfers can't resume after interruption when pasted into Explorer") if E-02 confirms that limitation.

## Consequences and follow-up tests

- E-02 measures partial-file outcomes per fault class.
- `TRANSFER_AND_ROUTING.md` and `UX_SPEC.md` describe both modes.
- PRD PR-028, PR-029 and PR-030 (CONFIRMED) map here.

## Acceptance record (who/when)

Accepted by the product owner, with an engineering teammate, on 2026-11-12 (PR #1), as part of the Phase 0 baseline. Scope: product direction and Phase 0 feasibility experiments. This acceptance does not approve unverified production architectures, future security-protocol choices or Premium infrastructure. Items this ADR leaves unspecified or OPEN remain open.
