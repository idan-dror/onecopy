# OneCopy — Project glossary

**Active OS clipboard:** Operating system-managed current clipboard selection that native paste reads. Separate from OneCopy's logical history.

**OneCopy clipboard / history:** The app-owned logical rolling history of encrypted entries, maintained by each device's local Rust core.

**Clipboard item:** A logical event with representations (text, image, rich text, files, etc.), origin and lifetime metadata. Not necessarily the bytes of an underlying file.

**Representation:** MIME/UTI/native format or compatible variant of a copied logical item.

**Live source reference:** A scoped locator to content on the origin device. Normal file copy stores this and metadata only; file bytes read when requested.

**Source capability:** An opaque authorization handle created by the origin for a selected item, used instead of allowing remote arbitrary path reads.

**Transfer:** An explicit operation that materializes current source file contents on another device, preferably direct encrypted P2P.

**Source consistency:** Detectable correctness condition that prevents a file changing incompatibly mid-transfer from being falsely reported as coherently complete; not historical file versioning.

**Manifest:** Bounded file/directory structure description and source/transfer revision information used for streaming/resume.

**Chunk:** Addressable bounded range of transferred bytes with integrity verification.

**Partial file/journal:** Local receiver state supporting restart-safe resume of verified chunks.

**Native OS bridge/adapter:** Per-platform component handling clipboard APIs, file access, permissions, lifecycle and native paste interfaces.

**Rust core:** Shared local logic for identity, protocol, encryption, history, transfers, routing and persistence, compiled into desktop agent or mobile app.

**Control plane:** Servers for authentication, device membership/signaling/presence, encrypted history envelope delivery and entitlement. Not a plaintext transfer intermediary.

**Payload plane:** Network path carrying file/image bytes directly device-to-device or over Premium relay/staging.

**P2P / direct:** Authenticated device-to-device transport, including LAN or Internet hole-punched paths, with no OneCopy-managed file-payload relay.

**Gateway relay:** Premium managed server forwarding ciphertext between live peers; not file storage.

**Offline staging:** Premium explicit client-encrypted, temporary, point-in-time cloud copy accessible when origin device is offline.

**Staged copy:** Frozen bytes from a successful offline staging operation; differs from current live source and does not auto-refresh.

**E2EE:** End-to-end encryption: content encrypted/decrypted on authorized devices; servers lack keys for user plaintext.

**Device trust / enrollment:** Approval process granting a specific device cryptographic membership beyond ordinary account login.

**Membership epoch:** Monotonic authorized-device group version used to gate future keys and delivery after roster change.

**Revocation:** Removing future authorization; cannot retract plaintext a previously trusted device already saved.

**NAT traversal / hole punching:** Process to establish direct connectivity across address-translating routers, with environment-dependent success.

**QUIC:** Authenticated encrypted multiplexed UDP transport candidate suitable for streaming and path migration.

**iroh:** Rust P2P networking toolkit, first experiment candidate only; default relay behavior must respect Free/Premium policy.

**OS capability matrix:** Verified per-version/per-compositor statement of which clipboard/native paste/background features are actually possible.

**ADR:** Architecture decision record with context, alternatives, owner decision and evidence.

**Go/no-go gate:** Measurable requirement that determines whether a feature or claim proceeds, is narrowed or receives a fallback.

## Proposed terms (PROPOSED, 2026-11-12 — pending owner acceptance)

These terms support the owner-directed proposals D-021 to D-042. Their definitions become binding only when the related decisions are accepted.

**Signaling:** Control-plane messages carrying authentication, roster, presence, connection candidates and connection-attempt timing between devices. Signaling is not inter-device packet forwarding. (D-021)

**Address discovery:** A STUN-style request and response through which a device learns its public address and port mapping from a server. The server never forwards packets between devices. (D-021)

**Inter-device packet forwarding (relay):** A server receiving packets from one device and sending them to another, whatever they contain: payload, handshake or NAT-traversal frames. Prohibited on OneCopy infrastructure for Free users (D-021). Third-party relays are OPEN (D-022). The Premium relay is the only planned OneCopy forwarding service.

**Delivery mode:** How a requested file is materialized at the destination: *managed download* or *native paste*. (D-024)

**Managed download:** OneCopy writes the destination file itself, with partial-file isolation, journaled resume, verification and controlled finalization. (PR-028)

**Native paste:** An OS component or application (for example Explorer) writes the destination by pulling bytes from OneCopy through a native data-transfer interface. OneCopy controls only the bytes it supplies and its own end-of-stream and error signals. (PR-029)

**Promotion:** User-initiated placement of a OneCopy history item onto the local active OS clipboard.

**Transient source:** A source whose access grant is limited in time or by process lifetime, such as an Android share-intent URI grant or an iOS share-extension item. (D-025)

**Mobile local staging:** An explicit, user-requested, bounded and expiring copy of a transient mobile source, kept in OneCopy's app container until sent. Not Premium cloud staging. (D-025)

**Inline content cap:** The maximum ciphertext size of representation content carried inside a history envelope. (D-030)

**On-demand content reference (`content_ref`):** A reference to a large text or image representation whose bytes are fetched from the origin on request through a source capability. (D-030)

**Transfer-time source validation:** Checks on a source opened for an active transfer or staging operation, ending with that operation. Distinct from prohibited idle or background monitoring. (D-026)

**Support tier:** The documented level of clipboard integration for an OS or desktop environment. (D-036)

**Sensitivity marker:** A platform- or application-provided hint that clipboard content should not be recorded or synced. Markers are voluntary and their absence does not imply the content is not sensitive. (D-032)

**Provenance marking:** OS metadata marking a file as originating from another machine, for example Windows Mark-of-the-Web or macOS quarantine. (D-033)

**Roster freshness:** The age of the most recent verified roster state a device holds. Its policy is OPEN (D-037).

**OneCopy-owned OS integration:** An OS-sanctioned extension point implemented by OneCopy itself (share extension, File Provider, IME, desktop-shell integration). Each is evaluated by its own ADR. Distinct from third-party application plugins, which remain out of scope. (D-027)

**Single-writer state:** Cryptographic state (sequence counters, ratchets, epoch state) mutated by exactly one process per device. (D-039)
