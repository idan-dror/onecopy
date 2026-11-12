# ADR 0003: Source consistency boundary — transfer-time validation versus monitoring and snapshots

Status: Proposed
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-026 (PROPOSED), D-042 (OPEN). Related: README invariant #4, D-005, D-006, PR-011 (all CONFIRMED)

## Context and evidence

README invariant #4 forbids "automatic snapshots, backups, version history, file duplication or change monitoring". `TRANSFER_AND_ROUTING.md` requires detecting incompatible changes "while monitoring for detectable changes" during a transfer. The two texts conflict on the word *monitoring* (review DA-01).

No portable filesystem primitive gives an immutable view of a file that another process may be writing. Stat and identity checks have known blind spots [K]:

- coarse or lazily updated timestamps;
- writers that restore the modification time;
- memory-mapped writers that update metadata late;
- same-size in-place overwrites within timestamp granularity;
- client caching on network filesystems;
- cloud placeholders that hydrate on open.

## Decision (proposed)

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

**OPEN (D-042), requires a separate owner decision:**

- (a) opening Windows sources without write sharing, which prevents concurrent writers during a transfer but makes other applications report "file in use";
- (b) transfer-scoped copy-on-write clones (APFS `clonefile`, Linux reflink, ReFS block cloning). These are snapshot-like and create hidden temporary files on the source volume.

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
- README invariant #4 wording is clarified only after this ADR is accepted.

## Acceptance record (who/when)

Pending. No acceptance recorded.
