# OneCopy — Product requirements document (PRD)

**Status:** Draft for owner review. Requirement IDs are stable; `CONFIRMED` means agreed product behavior, `PROPOSED` means a measurable suggested requirement.

## Scope and priority

- **Phase 0:** prove technical feasibility, security boundaries and cross-platform behavior; unpolished tools/prototypes are sufficient.
- **Desktop MVP:** a usable desktop history/paste/transfer product over direct P2P.
- **Production desktop:** reliable, signed, reviewed and supported platform integrations.
- **Mobile:** OS-conforming share/history/transfer experiences.
- **Premium:** managed relaying and temporary staging with quotas.

## Functional requirements

| ID | Requirement | Status | Target |
|---|---|---|---|
| PR-001 | A native client will exist for Windows, macOS, Linux, Android, iOS/iPadOS. | CONFIRMED | Product |
| PR-002 | The shared Rust core executes locally on every device; desktop agents or embedded mobile library as OS permits. | CONFIRMED | Architecture |
| PR-003 | OneCopy holds its own logical clipboard/history, independent from the OS clipboard. | CONFIRMED | MVP |
| PR-004 | A remote history arrival shall not automatically overwrite the receiving OS clipboard. | CONFIRMED | MVP |
| PR-005 | Support text, links, images, rich-text representations, files and folders, with format fallbacks. | CONFIRMED | Product, staged |
| PR-006 | Keep a small rolling synchronized history; initial 10 items proposed, configurable later. | CONFIRMED concept; PROPOSED count | MVP |
| PR-007 | Clipboard history metadata and small content sync E2EE between user-approved devices. | CONFIRMED | MVP |
| PR-008 | Normal file copy syncs a *live source reference* and metadata, not file bytes or snapshot. | CONFIRMED | MVP |
| PR-009 | Open/read the source on transfer request, sending its current accessible contents. | CONFIRMED | MVP |
| PR-010 | If source is offline/missing/inaccessible, report unavailable; do not synthesize a cached normal copy. | CONFIRMED | MVP |
| PR-011 | Detect incompatible mid-transfer modification when feasible; never silently complete a mixed-source transfer. | CONFIRMED | MVP |
| PR-012 | Support large direct transfers and folders with streaming, bounded memory, resume and integrity. | CONFIRMED | MVP |
| PR-013 | No product-imposed direct P2P maximum file size; practical device/filesystem limitations remain. | CONFIRMED | MVP |
| PR-014 | Select an efficient, reliable direct LAN/WAN route using measured signals. | CONFIRMED | MVP |
| PR-015 | Never transmit a Free user's file payload through OneCopy-managed relay. | CONFIRMED | Product |
| PR-016 | All OneCopy-managed live relay and temporary cloud staging require Premium entitlement. | CONFIRMED | Premium |
| PR-017 | `Keep Available Offline` is an explicit Premium action, E2EE, temporary, frozen at staging, subject to quota/expiry. | CONFIRMED | Premium |
| PR-018 | Offline copy readiness requires verified upload; refresh is explicit. | CONFIRMED | Premium |
| PR-019 | OS clipboard integration is native, best effort and capability/permission aware. | CONFIRMED | Platform |
| PR-020 | No app-specific add-ons/extensions for third-party apps. | CONFIRMED | Product |
| PR-021 | Pair devices with explicit trust; support device revocation and key changes. | CONFIRMED | MVP |
| PR-022 | Provide local controls to select remote item and promote it to active OS clipboard when supported. | CONFIRMED | MVP |
| PR-023 | Provide an explicit save/download alternative when transparent native file paste is unavailable. | PROPOSED | MVP |
| PR-024 | Sensitive clipboard content shall be excluded by default when platform marks it sensitive; support pause/exclusions. | PROPOSED | Product |
| PR-025 | Handle concurrent independent clipboard events deterministically without oscillating feedback loops. | PROPOSED | MVP |
| PR-026 | Enrollment and E2EE recovery cannot be achieved solely by an email password reset. | PROPOSED | MVP |
| PR-027 | History size/eviction and retention policy shall be visible, configurable later. | PROPOSED | MVP |

### Proposed requirements from the PR #1 review (PROPOSED, pending owner acceptance)

These requirements reflect owner directions recorded on 2026-11-12. They are **not** CONFIRMED. They do not change the CONFIRMED rows above; where they refine one, the clarification is noted below the table.

| ID | Requirement | Status | Decision / ADR | Target |
|---|---|---|---|---|
| PR-028 | **Managed download:** when OneCopy writes the destination itself, it provides partial-file isolation, journaled resume of verified ranges, end-to-end verification and controlled finalization (atomic rename where supported). | PROPOSED | D-024 / ADR-0002 | MVP |
| PR-029 | **Native paste:** when an OS component or application writes the destination, OneCopy claims no managed-download guarantee for that target unless an experiment demonstrated it, and never signals a clean end of stream for data that failed source-consistency validation. | PROPOSED | D-024 / ADR-0002 | MVP |
| PR-030 | Offer managed download when native paste is unavailable, unverified for the condition, or beyond evidence-based limits (size, entry count, path length). | PROPOSED | D-024 / ADR-0002 | MVP |
| PR-031 | **Mobile local staging:** for transient mobile sources, the user may explicitly keep a bounded, expiring local copy until it is sent. Never automatic; distinct from Premium cloud staging. | PROPOSED | D-025 / ADR-0004 | Mobile |
| PR-032 | Inline encrypted content per history item is bounded (256 KiB of ciphertext as an experimental starting point). Larger text and images become on-demand content references fetched from the origin. | PROPOSED | D-030 / ADR-0006 | MVP |
| PR-033 | Free control-plane traffic is subject to published rate and storage quotas. There is no product-imposed P2P file-size limit. | PROPOSED | D-031 / ADR-0006 | MVP |
| PR-034 | Honor platform and application sensitivity or exclusion markers on capture. Set them when promoting sensitive items. Document their limitations to users. | PROPOSED | D-032 / ADR-0007 | MVP |
| PR-035 | Preserve the security provenance of received files using per-OS mechanisms where they exist. Native-paste provenance is observed, not assumed. | PROPOSED | D-033 / ADR-0007 | MVP |
| PR-036 | Publish per-OS and per-desktop clipboard support tiers, including the GNOME Wayland limitation. | PROPOSED | D-036 | Platform |

**Clarification notes (PROPOSED, no change to CONFIRMED text until accepted):**

- PR-008 and PR-010: for transient mobile sources, see PR-031.
- PR-012: the stated guarantees apply in full to managed download (PR-028). Native-paste claims are limited by PR-029.
- PR-015: defined at the packet level by NFR-013 and D-021. Third-party relays are OPEN (D-022).
- PR-014: met in Phase 1 by measured direct-path selection. Complex scoring is deferred (D-035).
- PR-023: refined by PR-030.
- PR-024: extended by PR-034.
- UJ-02: corrected in place (delivery-mode distinction).
- PR-031: explicit mobile local staging is an intentional, user-authorized exception for transient sources only. It never permits automatic source duplication. README invariant #4 and PR-008/PR-010 are otherwise unchanged.
- PR-032: large items whose origin is a mobile device may be **unavailable while the OneCopy app on that device is suspended or not running**. The receiver shows this explicitly (`UX_SPEC.md` state "Origin app not running"). Nothing is uploaded automatically to work around it.
- UJ-06: until Premium exists (Phase 4), the UX explains the blocked direct connection without offering a paid option.

## Example user journeys and acceptance outcomes

**UJ-01 Desktop text:** On supported desktop clients with capture permission, copying simple text produces an encrypted entry on another online trusted device. The receiving active OS clipboard remains unchanged until local user action. Selecting that entry allows paste to a normal app.

**UJ-02 Remote file:** Copying a multi-GB file does not transfer/upload its data at copy time. The destination sees the reference. When requested, its source opens and streams with progress and cancel. An unavailable source is explained. The outcome depends on the delivery mode (corrected 2026-11-12, D-024):

- **OneCopy-managed download** (supported destinations): resume after interruption, end-to-end integrity verification and controlled finalization are **guaranteed**.
- **Native paste** (for example Explorer): only the guarantees demonstrated for that specific target and condition in E-02/E-03 are claimed (PR-029). OneCopy never signals successful completion for data that failed validation, but resume and finalization belong to the target application.

**UJ-03 Simultaneous copying:** Device A and B copy independently within seconds. Both appear once in history in a deterministic order; no recursive copy feedback or automatic takeover of OS clipboards.

**UJ-04 Source changes:** Modify/truncate the source during a multi-chunk transfer. The system detects incompatible change in tested scenarios and never reports an incoherent destination as complete.

**UJ-05 Revocation:** A device is removed from the trusted roster. It cannot start a newly authorized transfer or receive future encrypted history epochs; historical bytes already decrypted on it cannot be taken back.

**UJ-06 Blocked direct network:** Free mode cannot connect on a restricted network. Display that direct P2P is blocked and explain Premium relay as an option; do not silently relay payloads.

**UJ-07 Explicit staging:** User requests Premium staging and sees queued/uploading/verifying/ready/error/expired states. Offline file timestamp is displayed. Remote download can occur after source goes offline only when a ready copy exists.

**UJ-08 Mobile:** User can share explicitly to/from OneCopy through supported OS UI; product never claims an always-running silent copy detector.

## Nonfunctional requirements

| ID | Requirement | Status |
|---|---|---|
| NFR-001 | Compromised OneCopy server cannot decrypt ciphertext content, staged files or detailed encrypted item metadata. | CONFIRMED target |
| NFR-002 | No keys/secrets or clipboard bodies in telemetry and diagnostic logs. | PROPOSED |
| NFR-003 | Crash-safe transfers with bounded memory and verified completion. | CONFIRMED |
| NFR-004 | Source file access constrained to the originally authorized clipboard object/capability. | PROPOSED security invariant |
| NFR-005 | Deny unsafe paths, traversal, symlink escape and device/special-file behavior. | PROPOSED security invariant |
| NFR-006 | Protocols/version gates reject incompatible clients safely. | PROPOSED |
| NFR-007 | Per-platform consent/permissions must be respected, not bypassed. | CONFIRMED |
| NFR-008 | Code signing, reproducible CI where feasible, dependency/security checks before public beta. | PROPOSED |
| NFR-009 | SLOs from research are experiment targets, not contractual performance promises. | RESEARCHED |
| NFR-010 | Local IPC endpoints authenticate and authorize callers. No unauthenticated local endpoint exposes history or commands. | PROPOSED (D-038 / ADR-0009) |
| NFR-011 | Exactly one process per device mutates cryptographic state (single-writer rule). | PROPOSED (D-039 / ADR-0009) |
| NFR-012 | Crash dumps and diagnostics never capture clipboard bodies, keys, filenames or source paths: no full-memory dumps, scrubbed minidumps, opt-in reporting. | PROPOSED (D-040 / ADR-0009) |
| NFR-013 | For Free users, no OneCopy-managed server forwards any inter-device packet (payload, handshake or traversal), verified by packet capture and server counters. | PROPOSED (D-021 / ADR-0001) |
| NFR-014 | Direct-connectivity results are measured, targeted and reported per NAT and network class. No universal success-rate claim. | PROPOSED (D-034) |
| NFR-015 | The always-on desktop agent has a measured idle CPU, memory and wakeup budget (values set from E-01 measurements). | PROPOSED (review DA-25) |
| NFR-016 | A defined cross-version compatibility window. | OPEN (B-12) |

## Explicit non-goals

Automatic cross-device OS clipboard takeover; speculative file prefetch/snapshots; historical file versions; silent cloud upload; public share links; generic cloud backup; persistent arbitrary P2P on stock iOS; hidden Android clipboard surveillance; per-application plugins.

## Release criteria

Each release has a published supported-platform matrix, a minimum security assessment, documented fallback UX, a reproducing test suite and clearly labeled unresolved issues. MVP readiness requires completion of Phase 0 feasibility gates and separate explicit owner approval, not merely an implementation merged into main.

## Dependencies and links

See `ARCHITECTURE.md`, `PLATFORM_CAPABILITIES.md`, `TRANSFER_AND_ROUTING.md`, `UX_SPEC.md`, `PHASE_0_VALIDATION.md` and `TRACEABILITY.md`.
