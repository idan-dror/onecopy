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
