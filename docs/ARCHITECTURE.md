# OneCopy — System architecture

**Status:** Hybrid local-core architecture CONFIRMED. Module boundaries are PROPOSED until Phase 0 approval.

## Context

OneCopy is an independently maintained clipboard history with a native bridge to system clipboard facilities. It runs a **shared Rust core locally on every device**. The operating system — not OneCopy's logical history — ultimately decides whether content can be inserted into a third-party application via standard paste. There are **no third-party-app-specific extensions**.

```text
                  ┌─────────────────────┐
                  │ Native OneCopy UI   │
                  │ history/settings   │
                  └─────────┬───────────┘
                            │ local command/event API
                  ┌─────────▼───────────┐
                  │ OneCopy Rust Core   │
                  │ identity, history,  │
                  │ crypto, transfer,   │
                  │ routing, sync       │
                  └───────┬──┬──────────┘
                          │  │
       OS capabilities    │  └─────── authenticated network paths
                          │
                ┌─────────▼─────────┐
                │ Native OS Bridge  │
                │ clipboard, files, │
                │ key store, agents │
                └───────────────────┘
```

## Deployment topology

```text
[Device A: native app + local core + native bridge]
       |                \
       | encrypted       \ encrypted direct QUIC payload
       | sync envelopes   \
       v                   v
[Control plane]       [Device B: native app + local core + bridge]
 identity/roster         ^
 signaling/presence      |
 encrypted queue        |
       \                  /  Premium only, when enabled
        [Gateway live relay / encrypted temporary staging]
```

The control plane is not an E2EE endpoint. Clients authenticate one another using local identity keys and current device membership. Normal file transfer is direct P2P; Premium managed relay is a future authorized data path.

## Component responsibilities

| Component | Responsibility | Excluded |
|---|---|---|
| Native UI | History selection, device approvals, local paste/download controls, statuses | Cryptographic protocol logic |
| Native agent/bridge | Observe/writer OS clipboard when allowed; formats, filesystem handle permissions, login/background integration | Cross-device business policy |
| Local core/API | Event coordination, permission checks, lifecycle, error translation | Direct windowing API ownership |
| History | Ordered events, retention, dedupe, local database | Taking over active OS clipboard |
| Identity/trust | Local device keys, signed roster, enrollment, revocation, key epochs | Reliance on email login alone |
| Crypto | Standard key agreement, E2EE envelopes and chunk hashes; secret handling | Custom crypto primitives |
| Transport | Peer candidates, direct QUIC, reconnect/path measurement | Entitlement-bypassing relays |
| Transfer | Live-source handles, manifests, bounded streams, resume journal | Automatic archive/snapshot/versioning |
| Route selector | Predictive path choice, respect Free/Premium/cost/battery | Assuming relay always faster |
| Control API | Account auth, encrypted envelope delivery, roster/signaling, presence | Decrypting user content |
| Gateway (future) | Premium live ciphertext forwarding and encrypted expiring object staging | Permanent or public file drive |

## Client processes

**Windows:** Per-user tray/background process in interactive session, native Win32/COM clipboard adapter; avoid session-0 Windows Service. UI can communicate with the agent via authenticated local IPC.

**macOS:** User LoginItem/LaunchAgent; AppKit/Swift adapter and explicit pasteboard permission behavior; embedded Rust library with Swift bindings.

**Linux:** Per-user graphical-session process; X11/Wayland backends and capability detection. Linux host may call Rust core directly.

**Android:** Kotlin app plus permitted background transfer jobs/services and Sharesheet/content-provider entry points; Rust library embedded through FFI. No invisible system-wide background clipboard capture.

**iOS/iPadOS:** Swift app and Share/Files extensions as supported; Rust library embedded and invoked within app/extension lifecycle; no permanent P2P daemon.

## Interfaces (PROPOSED)

Provide coarse async boundaries; do not export raw filesystem paths to remote peers:

```text
Core.capture_local(PlatformClipboardInput) -> ItemId | CaptureRejected
Core.list_history() -> [ItemSummary]
Core.prepare_local_paste(ItemId, TargetCapabilities) -> PastePlan
Core.request_transfer(ItemId, DestinationPolicy) -> TransferId
Core.transfer_status(TransferId) -> TransferStatus
Core.authorize_new_device(EnrollmentRequest, Proof) -> MembershipChange
Core.revoke_device(DeviceId) -> MembershipChange
Core.subscribe_events() -> ordered events + snapshots
PlatformBridge.observe_changes() -> permitted clipboard events
PlatformBridge.write_clipboard(PasteRepresentations) -> result
PlatformBridge.open_source(OpaqueLocalSourceHandle) -> readable source or unavailable
```

The above is a contract sketch, not compiled Rust API. Distinguish ephemeral source handles from durable/reopenable source references, especially on mobile content URIs and security-scoped URLs.

## Data locality

- Small captured content, history and trusted device state: encrypted local storage and E2EE sync envelopes as necessary.
- Normal large file: remains where it was copied; copy-event metadata references it, without content upload or full prehash.
- Partial destination file: local disk only, journal with verified chunk/range state.
- Premium staged file: client-encrypted object held temporarily in managed storage; content key only available to authorized devices.
- Server: opaque encrypted sync envelopes, account/roster metadata, signaling and limited routing information.

## Sync versus OS clipboard

1. Native adapter observes permitted local Copy and normalizes representations.
2. Local core classifies and adds a logical history event.
3. Crypto wraps small content and metadata in E2EE envelope.
4. Control plane store-and-forwards to other trusted devices.
5. Receiving core verifies epoch, signature, sequence, deduplicates and adds history item.
6. **Do not touch receiving OS clipboard on arrival.** A local selection/user action may promote an entry to the OS clipboard, subject to adapter capabilities.

Suppress re-ingestion loops using an OS-specific origin marker where supported and a conservative recent fingerprint cache where not. Suppression must be observable in tests and must avoid discarding genuine new copies of similar content.

## Proposed refinements (PROPOSED, 2026-11-12 — pending owner acceptance)

These refinements reflect owner-directed proposals. They do not change the CONFIRMED hybrid local-core architecture.

1. **Delivery modes (D-024, ADR-0002).** The transfer engine owns *managed download*: destination writer, journal, verification, finalization. The native bridge exposes *native-paste providers* that pull bytes from the transfer engine's bounded stream API. On Windows that is `IDataObject`/`IStream`; other providers need their own ADRs (ADR-0005). The two modes carry different, documented guarantees.
2. **Free connection path (D-021, D-023, ADR-0001).**
   - A transport abstraction with two candidate implementations, evaluated independently.
   - Free configuration has no relay transport.
   - Address discovery is a control-plane service that never forwards inter-device packets.
   - The Premium relay is a separate, entitlement-gated component.
3. **Content references (D-030, ADR-0006).** Representations above the inline cap become on-demand `content_ref`s. The origin core serves them through source capabilities, from its local encrypted store of captured clipboard data. Mobile origins can serve only while their app is running, so availability states are explicit (`DATA_MODEL_AND_PROTOCOL.md`).
4. **Process model and local IPC (D-038, D-039, ADR-0009).** Recommended: one per-user agent process per desktop owns the core, the OS adapter and the tray. A separate UI process is allowed only behind authenticated IPC. Exactly one process per device mutates cryptographic state; mobile extensions enqueue work for the main app. A process-model ADR precedes Phase 1.
5. **Mobile local staging store (D-025, ADR-0004).** A bounded, expiring, encrypted area in the app container for explicitly kept transient sources. It is separate from the Premium staging client.
6. **OneCopy-owned OS integrations (D-027, ADR-0005).** Share extensions, File Provider–like providers, an IME or desktop-shell integrations are separate components, each with its own ADR and threat-model entry. Provider-style integrations expose only authorized history items.
7. **Routing (D-035).** Phase 1 uses measured direct-path selection. The scoring and hysteresis selector is deferred until a Premium relay exists and measurements show benefit.

## Technology candidates (not yet commitments)

- Core: Rust workspace, asynchronous runtime to be selected, SQLite local persistence candidate.
- Swift/Kotlin bindings: UniFFI experiment; Windows stable C ABI/C# bridge candidate. *Option added 2026-11-12: write the Windows clipboard/COM adapter in Rust (`windows-rs`), exercised inside E-02 only.*
- Transport: standard QUIC, with `iroh` first Phase 0 candidate; strict direct-only behavior is a gate. *Proposed (D-023): evaluate iroh and a standalone QUIC plus ICE-style traversal as two independent tracks.*
- Control: minimal test service during Phase 0; managed PostgreSQL, OIDC/passkeys later as evaluated.
- Gateway object staging: Cloudflare R2 first regional cost/performance experiment, not selected provider.

## Quality attributes

Correctness and authorization outrank raw throughput. Failure modes must be explicit. Every remote-file operation should be cancellable, observable and recoverable where source is unchanged. Versioned messages, backwards-compatible semantics and independent native adapter tests are required before product release.

## Open issues

Enrollment/recovery key architecture; whether MLS is warranted; concrete UI framework per desktop; protocol/schema encoding; iroh compatibility; macOS Finder Command-V; Wayland compositor matrix; precise source consistency guarantee on each filesystem. See `APPROVAL_NEEDED.md` and `PHASE_0_VALIDATION.md`.

Added 2026-11-12:

- revocation authority, roster freshness, offline LAN authorization and recovery (D-037, ADR-0008);
- third-party relays for Free (D-022);
- transfer-time write prevention and copy-on-write clones (D-042);
- the process-model ADR (AR-07 in the review);
- native-paste target guarantees (E-02, E-03).
