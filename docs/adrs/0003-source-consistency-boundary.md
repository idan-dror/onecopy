# ADR 0003: Source consistency boundary — transfer-time validation versus monitoring and snapshots

Status: Accepted (Phase 0 baseline, 2026-11-12)
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-026, D-042 (CONFIRMED). Related: README invariant #4, D-005, D-006, PR-011 (all CONFIRMED)

## Context and evidence

README invariant #4 forbids "automatic snapshots, backups, version history, file duplication or change monitoring". `TRANSFER_AND_ROUTING.md` requires detecting incompatible changes "while monitoring for detectable changes" during a transfer. The two texts conflict on the word *monitoring* (review DA-01).

No portable filesystem primitive gives an immutable view of a file that another process may be writing. Stat and identity checks have known blind spots [K]:

- coarse or lazily updated timestamps;
- writers that restore the modification time;
- memory-mapped writers that update metadata late;
- same-size in-place overwrites within timestamp granularity;
- client caching on network filesystems;
- cloud placeholders that hydrate on open.

## Decision

**Prohibited** (unchanged intent of the CONFIRMED rules):

- reading, hashing, copying or uploading source bytes at copy time;
- background or idle watching of source files, before or between transfers;
- retained snapshots, versions or backups of sources;
- "your file changed since you copied it" comparisons with the copy event.

**Required** during an active transfer or staging operation, on the source opened for that operation:

- record identity and change indicators at open (file ID or inode, size, mtime, POSIX ctime, Windows USN where available);
- re-validate before reporting completion, and between chunks where cheap;
- fail and offer a retry on incompatible change;
- never stitch ranges from different source states.

**Allowed** during an active transfer, scoped to the opened file and ended with the transfer: OS change notifications on that file (for example Linux inotify or fanotify). This is *transfer-time validation*, not monitoring.

**Claim limit:**

- OneCopy claims detection of incompatible changes **in tested scenarios**. It does not claim universal detection.
- Residual races are documented per filesystem.
- Chunk digests prove transmission fidelity, not source immutability.

**D-042 (owner decision, 2026-11-12):**

- **Do not lock source files against concurrent modification by default.** For example, Windows sources are opened with write sharing allowed. Any non-default locking option would need a separate decision.
- **Do not introduce transfer-scoped copy-on-write clones or hidden source snapshots** (APFS `clonefile`, Linux reflink, ReFS block cloning or similar).
- Use scoped transfer-time source-consistency checks and report detectable incompatible modifications. Document residual races rather than claiming universal detection.

## Alternatives considered

- **Strict reading of invariant #4** (no detection at all during transfer). Contradicts PR-011.
- **Hash the whole source before streaming.** Contradicts the no-prehash rule and doubles I/O for large files.

## Security/privacy and interoperability effects

- Transfer-time notifications are local, scoped to one file and short-lived. No source content or names leave the device.

## User/product effects

- Users may see "Source changed during transfer; retry".
- Edits made between copy and transfer are expected: the transfer sends the current file.

## Consequences and follow-up tests

- E-08 mutation corpus across NTFS, ext4, exFAT and SMB (APFS when hardware is available) records detection latency and residual races.
- README invariant #4 wording is clarified to match this ADR (accepted 2026-11-12).

## Acceptance record (who/when)

Accepted by the product owner, with an engineering teammate, on 2026-11-12 (PR #1), as part of the Phase 0 baseline. Scope: product direction and Phase 0 feasibility experiments. This acceptance does not approve unverified production architectures, future security-protocol choices or Premium infrastructure. Items this ADR leaves unspecified or OPEN remain open.
