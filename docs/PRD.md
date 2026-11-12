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

## Example user journeys and acceptance outcomes

**UJ-01 Desktop text:** On supported desktop clients with capture permission, copying simple text produces an encrypted entry on another online trusted device. The receiving active OS clipboard remains unchanged until local user action. Selecting that entry allows paste to a normal app.

**UJ-02 Remote file:** Copying a multi-GB file does not transfer/upload its data at copy time. The destination sees the reference. When requested, its source opens and streams with progress, cancel/resume; the completed artifact passes integrity checks. An unavailable source is explained.

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

## Explicit non-goals

Automatic cross-device OS clipboard takeover; speculative file prefetch/snapshots; historical file versions; silent cloud upload; public share links; generic cloud backup; persistent arbitrary P2P on stock iOS; hidden Android clipboard surveillance; per-application plugins.

## Release criteria

Each release has a published supported-platform matrix, a minimum security assessment, documented fallback UX, a reproducing test suite and clearly labeled unresolved issues. MVP readiness requires completion of Phase 0 feasibility gates and separate explicit owner approval, not merely an implementation merged into main.

## Dependencies and links

See `ARCHITECTURE.md`, `PLATFORM_CAPABILITIES.md`, `TRANSFER_AND_ROUTING.md`, `UX_SPEC.md`, `PHASE_0_VALIDATION.md` and `TRACEABILITY.md`.
