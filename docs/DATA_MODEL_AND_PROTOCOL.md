# Data model, synchronization and protocol contracts

**Status:** PROPOSED technical model guided by CONFIRMED product semantics. Fields, encodings and error codes require protocol ADR approval. Not production cryptographic design.

## Principles

- Logical clipboard entries are **events/representations**, not the OS clipboard itself.
- File and folder entries contain *live, capability-scoped source references* and encrypted metadata. No automatic content snapshot.
- Large content never needs to exist inside the encrypted history envelope.
- All cross-device identities, epochs, sender counters, request authorization and envelope versions are validated.
- A restart or duplicate delivery must not create duplicate history items or corrupt state.
- Protocol messages must specify compatibility rules for older clients and the maximum supported object sizes.

## Conceptual model

```text
Account
  account_id (opaque)
  trusted_membership_epoch
  encrypted_roster_version

Device
  device_id
  signing_public_key
  key_agreement_public_key
  transport_public_identity
  trust_state

ClipboardItem
  item_id (random unique ID)
  origin_device_id
  origin_sequence
  copied_at (display only, untrusted for ordering)
  item_kind: text | rich_text | link | image | files | mixed
  representations[]
  sensitivity: normal | marked_sensitive | excluded
  source_refs[]
  logical_expiration
  offline_stage_ref? (premium)

Representation
  media_type / UTI / platform_format
  inline_ciphertext? (small data only)
  source_capability_id? (opaque)
  size_hint? (not authoritative)

SourceRef
  opaque_local_source_handle
  display_name
  optional_origin_display_path
  content_provider_kind
  stat_hint? (size, mtime informational)
  authorization_scope
  handle_expiration?
```

### Proposed model additions (PROPOSED, 2026-11-12 — pending owner acceptance)

```text
Representation (additions)
  content_ref?            // D-030: on-demand content reference when ciphertext > inline cap
    media_type
    size_hint             // informational
    preview_ciphertext?   // bounded; omitted for sensitive items
    source_capability_id  // same authorization path as files

SourceRef (additions)
  source_kind: NativePath | ContentUri | SecurityScopedUrl | ProviderHandle | TransientGrant   // D-025
  local_stage?            // mobile local staging state, only for TransientGrant sources
```

**Inline cap (D-030, ADR-0006).**

- Inline representation ciphertext is bounded per item: **256 KiB total, as an experimental starting point** to be tuned in E-11.
- Above the cap, the representation becomes a `content_ref`.
- The origin serves `content_ref` bytes from its local encrypted store of *captured clipboard data*. That store is the clipboard content itself, not a snapshot of a file. Its retention follows history eviction.
- Receivers fetch the bytes over the authenticated direct path when the user selects or promotes the item.

**Availability of `content_ref` items (correction, 2026-11-12).** A `content_ref` can be served only while the origin's OneCopy core is running and reachable.

- **iOS/iPadOS:** the app is normally suspended in the background, and remote wakeups are unreliable.
- **Android:** the app process may not be running.

So large items copied on a mobile device are frequently **unavailable** to other devices until the user opens OneCopy on the origin. The receiver must show an explicit state:

```text
content_ref availability:
  AVAILABLE           origin reachable and serving
  ORIGIN_OFFLINE      origin device not connected
  ORIGIN_NOT_RUNNING  mobile origin app suspended/terminated; "open OneCopy on <device>"
  EVICTED             origin no longer holds the captured data (history eviction)
  UNAVAILABLE         other or permission failure (reason code)
```

No automatic upload, cloud staging or background prefetch is used to work around this. Premium "Keep Available Offline" remains an explicit, separate user action (D-009).

**Explicit mobile local staging (D-025)** is an intentional, user-authorized exception for transient sources only. It never permits automatic source duplication.

**Free control-plane quotas (D-031).**

- Server-enforced: items per minute, inline bytes per day, retained envelopes per account (bounded by the history window plus tombstones), signaling messages per minute, address-discovery requests per minute.
- On a breach, sync is deferred with an explicit status. Content is never silently dropped.
- Values are set from measurement.
- No P2P file-size limit.

**Mobile local staging (D-025, ADR-0004).**

```text
NONE -> REQUESTED (explicit user action) -> COPYING -> KEPT (expires_at, size) -> EXPIRED | DELETED
COPYING -> FAILED
```

Bounded by a per-device quota, encrypted at rest, never automatic, and distinct from the Premium staging state machine below. Delete-after-send policy is OPEN (B-13).

An origin-local source handle may need to be reopened across process/device restarts. Native adapters are responsible for persisted permissions where legal and available. Source references **must not** be actionable arbitrary raw paths in remote RPCs.

## History event envelope

```text
EnvelopeHeader {
  protocol_version,
  account_id,
  membership_epoch,
  sender_device_id,
  sender_sequence,
  item_id,
  ciphertext_size,
  previous_sender_hash?,
}
Envelope { header, encrypted_payload, authenticated_sender_signature }
```

The body carries encrypted metadata and permitted inline content. Server can route opaque envelopes but cannot decrypt. The header should contain only unavoidable routing data; cryptographic design must define how header fields are authenticated, how replay protection persists and how membership changes are signed.

### Ordering and deduplication

- Each origin device increments a persisted monotonic sequence.
- Store a per-sender anti-replay high-water mark and/or a bounded gap map; reject invalid epochs or already seen event identifiers.
- Merge concurrent events deterministically using authenticated logical ordering rules, not blindly trusting extreme user wall-clock timestamps.
- A shared event log plus local rolling history retention is preferable to treating the server's newest 10 rows as canonical truth.
- Tombstones/removal/expiry need explicit definitions so offline clients do not resurrect deleted entries.
- Do not expose plaintext previews in presence notifications.

## Device enrollment state machine

```text
UNREGISTERED
 -> AUTHENTICATED_UNTRUSTED (account login does not grant content trust)
 -> PENDING_APPROVAL (local keys generated; public material published)
 -> TRUSTED (approved by existing trusted device / recovery procedure)
 -> ACTIVE (current membership epoch available)
 -> REVOKED (no future authorized sessions or envelopes)
```

A trusted device should approve via an authenticated QR/short-code comparison ceremony, protected from account-credential-only enrollment. Exact proof, signature and group-key mechanics are OPEN in `SECURITY_AND_PRIVACY.md`.

## File item lifecycle

```text
CAPTURED_LOCAL -> ENCRYPTED_SYNC_PENDING -> SHARED
SHARED -> [AVAILABLE | SOURCE_OFFLINE | SOURCE_UNAVAILABLE]
AVAILABLE -> REQUESTED -> TRANSFERRING -> VERIFIED -> COMPLETED
TRANSFERRING -> INTERRUPTED -> RESUME_PENDING | FAILED
```

Availability is dynamic: offline may become online again. Do not use an item copy timestamp as a cryptographic file revision. Files are read when requested.

## File transfer message families

| Family | Required purpose |
|---|---|
| `TransferOffer` / `TransferRequest` | Specify clipboard item, authorized peer, requested destination representation |
| `ManifestRequest` / `ManifestResponse` | Negotiate current file/folder listing and source revision token |
| `ChunkRequest` / `ChunkData` / `ChunkAck` | Address ranges, stream bytes, digest and verified progress |
| `TransferCheckpoint` / `ResumeRequest` | Resume from verified ranges only, bound to exact compatible manifest/source epoch |
| `TransferComplete` / `TransferReceipt` | End-to-end integrity and destination commit acknowledgment |
| `TransferError` | Structured reason: unavailable, denied, source_changed, network, expired, disk_full, invalid_path, revoked, incompatible_version |
| `SourceConsistencyResult` *(PROPOSED, D-024/D-026)* | Sender's post-read re-validation result for a file. The receiver must not finalize a managed download, or release the final bytes to a native-paste target, without a passing result. Not a guarantee about how the target handles partial files. |
| `ContentRequest` / `ContentData` *(PROPOSED, D-030)* | Fetch an on-demand `content_ref` representation through its source capability |

Manifest entries use normalized *relative* destination paths. Sender must not supply absolute destination targets. File type, size, source object identity and permission constraints need a well-defined cross-platform subset; special nodes should not be copied by default.

## Source consistency identity

A transfer attempt gets an opaque `source_revision_token` derived from a combination of source identity/stat information available when opened plus a transfer-unique nonce. It is not a version-history system and must not be confused with an immutable filesystem snapshot. Source-side checks before and after reading detect incompatible changes where possible. Chunk integrity alone does not prove immutability during read.

Resume is legal only when the source's current verifiable representation remains compatible with the existing transfer manifest. Otherwise discard/restart the affected transfer, never stitch chunks from incompatible attempts.

*Proposed clarifications (D-026, ADR-0003):*

- The token and checks are **transfer-time validation**, not monitoring. Nothing is recorded or watched before a transfer request.
- The checks are not claimed to detect every concurrent modification. Known blind spots are listed in `TRANSFER_AND_ROUTING.md`.

## Premium staging state machine

```text
NONE -> QUEUED -> UPLOADING -> VERIFYING -> READY
                       \           \-> FAILED
                        \-> FAILED
READY -> REFRESHING -> VERIFYING -> READY
READY -> EXPIRED | DELETED
```

Staged metadata includes `staging_id`, copy `created_at`, `expires_at`, encrypted size, object-manifest digest and encryption context. **READY requires completed, verified encrypted upload.** Staging creates one current frozen temporary copy; ordinary source updates do not alter it.

## Versioning and error safety

- Version envelope and manifest separately.
- Message parsers must limit lengths, depths and counts before allocating.
- Unknown security-critical enum values fail closed; optional compatible fields may be ignored only by explicit policy.
- Device membership/roster state is rollback-protected.
- Define test vectors for signatures, ciphertext, chunk digests and canonical encodings before production use.
- Protocol status `unsupported` is distinct from `temporarily_unavailable` and `permission_required`.

## Phase 0 contract tests

1. Two devices exchange a synthetic encrypted text item through a blind test control server.
2. Duplicate/reorder/late envelopes yield one correct logical entry.
3. Revoked peer denied on new connection and future epoch sync.
4. Invalid manifest paths, nested entries and oversize lengths rejected before file creation.
5. Interrupted transfer resumes verified bytes, but refuses incompatible source revision.
6. An arbitrary requested raw filesystem path cannot be opened through a remote endpoint.
7. *(Proposed)* Representations above the inline cap become `content_ref`s. Fetching one requires a valid capability and current roster membership.
8. *(Proposed)* Free quota breaches defer sync with an explicit status and never drop content silently.
9. *(Proposed)* A missing or failed `SourceConsistencyResult` blocks managed-download finalization and blocks a clean end of stream for native paste.

See `TRACEABILITY.md` for test mapping, `TRANSFER_AND_ROUTING.md` for transport, and `SECURITY_AND_PRIVACY.md` for trust assumptions.
