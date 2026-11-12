# OneCopy — Security and privacy design

**Status:** Security goals CONFIRMED; protocol/key hierarchy PROPOSED and requires qualified external review before production. This is a threat-model baseline, not a crypto audit.

## Security objectives

1. Control-plane and gateway operators cannot decrypt clipboard content, encrypted detailed metadata (filenames/origin paths), or staged file bytes.
2. Only currently trusted enrolled devices can request authorized history/content.
3. Network observers and active adversaries cannot substitute, replay or silently corrupt transfers.
4. A remote peer cannot exploit clipboard/file content or metadata to read arbitrary local paths, escape a destination root, execute commands or exhaust resources.
5. Users retain local controls for sensitive data and device revocation.

**Limit:** E2EE cannot protect data from malware executing as a legitimately enrolled endpoint with access to decrypted content. Revocation cannot retroactively erase plaintext previously downloaded by a compromised device.

## Threat actors and trust boundaries

| Actor | Capability | Important mitigation |
|---|---|---|
| Control-plane compromise | Read/alter/replay envelopes, roster/presence data | E2EE, signed roster transitions, counters, rollback detection |
| Gateway compromise | Observe/replace ciphertext and traffic metadata | Client E2EE, authenticated object manifests, expiring opaque IDs |
| Stolen account login | Authenticate to web account | Does not confer content trust; requires existing-device approval/recovery |
| Enrolled-device compromise | Access authorized content until revoked | Key rotation, revocation, local secure storage, clear limitation |
| Untrusted app/clipboard writer | Plant malicious URLs, HTML, paths and deceptive filenames | Render as untrusted data, never auto-execute/open, sanitize UI |
| Malicious network/NAT peer | Attempt spoofing, reflection, DoS | Mutual peer authentication, limits, anti-replay |
| Malicious transfer manifest | Request out-of-root writes, symlink traversal, pathological tree | Strict path checks, manifests limits, no symlink following |
| *Proposed rows (2026-11-12, pending acceptance):* | | |
| Local process / other OS user / sandboxed app | Query the agent's local endpoint for cross-device history or issue commands | Authenticated, authorized IPC (D-038, ADR-0009) |
| Crash or diagnostic capture | Heap contents (clipboard bodies, keys) in dumps or uploaded reports | No full-memory dumps, scrubbed minidumps, opt-in reporting (D-040) |
| OS and keyboard-app clipboard history or cloud sync | Retains or uploads promoted items outside OneCopy's E2EE | Set exclusion markers on promotion of sensitive items; document limitations (D-032, ADR-0007) |
| Revoked device on a LAN with stale peers | Requests transfers from peers that have not learned of the revocation | Roster freshness, authority and LAN policy (D-037, ADR-0008, OPEN) |
| Concurrent writers of cryptographic state (app plus extension, agent plus UI) | Nonce reuse or forked ratchets | Single-writer rule (D-039, ADR-0009) |
| Received file without provenance | Executables or documents bypass OS safety prompts | Provenance marking for managed downloads; native-paste behavior observed (D-033) |
| Third-party relay operator (if ever permitted) | Observes Free connection metadata | Prohibition recommended (D-022, OPEN) |

## Device trust and membership

- Each device generates its private identity material locally.
- Account authentication can discover an account but **does not** automatically authorize E2EE history access.
- Enrollment requires approval from an existing trusted device with authenticated QR/short-code comparison, or a dedicated recovery-key ceremony.
- Sign membership changes; use monotonic roster version/account epoch to prevent server-side rollback. Revoked devices are refused in all new sessions and excluded from future group encryption epochs.
- Recovery through ordinary email reset alone must never recreate content keys. Define disaster recovery if all devices are lost; be honest about irreversible data loss without a recovery key.

## Cryptography (PROPOSED)

- Use established audited primitives and libraries. Candidates: Ed25519 signatures, X25519 key agreement, HPKE envelopes, HKDF key derivation and standard AEAD.
- Transport QUIC/TLS identity is necessary but not sufficient: bind authenticated transport key → device ID → trusted roster/epoch → requested clipboard source capability.
- Encrypt shared history metadata and small payloads *before* sending to any server. Encrypt staged file content before object storage receives it.
- Two group-key strategies remain OPEN: per-account epoch secret wrapped individually to trusted members versus standard MLS. The first is simpler but **must not be advertised as MLS-equivalent forward secrecy/post-compromise security**. Choose only after security prototype and protocol review.
- Define nonces, signature domain separation, key usage, per-item derivation, account recovery, epoch transitions and replay databases in a dedicated approved ADR before product implementation.

## Local key storage

- Windows: Windows CNG/DPAPI or appropriate protected storage; never plaintext keys in ordinary settings.
- Apple: Keychain; Secure Enclave-supported P-256 where applicable, but do not assume arbitrary X25519 private keys can live in enclave.
- Android: Android Keystore/TEE; evaluate StrongBox selectively based on availability and algorithms.
- Linux: desktop secret service/keyring when possible; document weaker fallback encrypted-file modes.
- Native FFI must not leak secrets in logs, crash reports or unchecked callbacks; panics shall not unwind across ABI boundaries.

## Source capability authorization

The clipboard capture module stores an **opaque origin-local handle** scoped to the selected files and an authorized member/transfer context. Remote requests present a capability ID; they do not provide a filesystem path to open. Verify it against membership, current epoch, item lifetime, source handle validity and transfer identity. Constrain file descriptor and content provider operations to the originally authorized scope. Protect against TOCTOU and source symlink substitution.

## Clipboard and content privacy

- Honor platform-marked sensitive clips where exposed; default to not syncing them (PROPOSED policy).
- Provide pause and application/process exclusions (PROPOSED); test password managers, terminals, remote desktop/VM apps.
- Never run copied code or scripts, auto-open links, render untrusted HTML as executable HTML, or automatically download from URLs.
- Preview private content cautiously, especially notifications and lock screens. Do not rely solely on filename extensions to assess safety.
- Avoid storing private plaintext longer than needed in memory and on disk. Secure deletion guarantees are platform/storage dependent and should not be overstated.

## Transfer and filesystem safeguards

Bound each allocation, frame, manifest depth, file count, path length, concurrent transfer and storage reservation. Validate canonical destination-relative paths, Windows reserved names, Unicode/case collisions and path separators. Never follow symlinks by default. Verify per-chunk integrity and final manifest. Abort safely on incompatible source mutation. Do not auto-extract archive contents.

## Privacy inventory

**Server can still observe:** account and device identifiers, control-plane access/IP addresses, general timing, envelope sizes, presence/routing metadata, push tokens, entitlements and gateway usage. E2EE does not equal metadata invisibility. Document collection, purpose, retention, access, deletion, incident response and subprocessors before beta. Prepare GDPR-aligned minimization and a formal DPIA assessment where applicable.

**Telemetry forbidden:** clipboard bodies, secret keys, plaintext filenames, source paths, sensitive rich content, raw content hashes and unrestricted high-cardinality identifiers. Synthetic fixture data only in diagnostic logs/tests.

## Pre-implementation security review scope (PROPOSED, 2026-11-12 — pending owner acceptance)

The owner directed that the following be resolved in a pre-implementation security review, before trust or key code is written for Phase 1.

1. **Revocation authority (D-037, ADR-0008, OPEN).** Who may revoke whom; how a stolen device is prevented from revoking every other device and locking the owner out; how a recovery key can override.
2. **Roster freshness and offline LAN authorization (D-037, OPEN).** How old a device's roster may be before it serves or accepts a transfer, especially on a LAN while the control plane is unreachable.
   - **No timeout is chosen without a documented trade-off.** A larger bound improves offline usability and lengthens the window in which a revoked device can still be served. A smaller bound shortens that window and causes more offline failures.
   - Options are listed in ADR-0008: online confirmation, a bounded window, peer roster-version agreement, LAN gossip of revocations.
3. **Account recovery (B-02, D-037).** A recovery key, approval by an existing device, or disclosed server-assisted escrow. Email-only recovery remains rejected (PR-026).
4. **Local IPC authorization (D-038).** Per-OS caller authentication: a named pipe with a user-SID ACL plus client verification on Windows; XPC with a code-signing requirement on macOS; `SO_PEERCRED` on Linux. Alternatively the UI runs inside the agent process.
5. **Single-writer cryptographic state (D-039).** Exactly one process per device mutates counters, ratchets or epoch state.
6. **Crash dumps and diagnostics (D-040).** No full-memory dumps, scrubbed minidumps, opt-in reporting, zeroization where feasible.
7. **Sensitive clipboard exposure (D-032).**
   - Honor platform and application markers on capture and set them on promotion (see `PLATFORM_CAPABILITIES.md`).
   - Limitations: markers are voluntary; their absence does not imply the content is not sensitive; OS behavior varies by version; apps can misuse markers.
8. **Received-file provenance (D-033).**
   - Managed downloads apply Windows Mark-of-the-Web (for example through `IAttachmentExecute`) and macOS quarantine.
   - Native-paste provenance is observed in E-02/E-03.
   - Linux and mobile have no enforced equivalent.
   - Never preserve setuid or setgid bits.
9. **Free connection metadata (D-021).** Signaling and address discovery reveal IP addresses and timing to the control plane. Direct connections reveal IP addresses to peers. Record both in the privacy inventory.
10. **Encryption layering (review AR-09, not adopted).** Reliance on QUIC/TLS for live transfers, keyed BLAKE3 digests, and a standard streaming AEAD for staging are review proposals for the cryptography ADR only.

## Security verification gates

- Review formal threat model and key protocol externally before public beta.
- Fuzz parsers, malformed envelopes/manifests, Unicode and path normalization.
- Property-test anti-replay, idempotent sync, monotonic roster, revoked-device denial.
- Malicious-control-server harness: duplicate, reorder, replay, drop, rollback, corrupt ciphertext and roster updates.
- Network chaos: mutated source, partial completion, disk full, cancellation, denial, reconnect, stale resume and unauthorized path RPC.
- Signed binary and updater verification, SBOM, dependency audit, secrets scanning and mobile MASVS checks.

## Security-specific open decisions

MLS vs epoch-secret group key; secure recovery and lost-all-devices behavior; clipboard sensitivity default; encrypted metadata granularity; local database encryption; update signing model. See `APPROVAL_NEEDED.md`.

Added 2026-11-12:

- revocation authority, roster freshness and offline LAN authorization (D-037);
- third-party relays for Free (D-022);
- synthetic keystroke injection (D-041);
- transfer-time write prevention and copy-on-write clones (D-042);
- provenance mechanism details (D-033).
