# OneCopy — product and engineering documentation

**Status:** Phase 0 baseline **accepted** 2026-11-12 (scope: product direction and Phase 0 feasibility experiments) · **Version:** 0.2 · **Prepared:** 2026-11-12  
**Code status:** Phase 0 experiments authorized only as specified and approved (E-02 first, staged). No product implementation authorized.  
**Audience:** Product owner, ChatGPT (design/review partner), Claude (implementation engineer).

OneCopy is a cross-platform, encrypted *device clipboard network*: it maintains an independent shared clipboard history on each trusted device, connects to the operating system's clipboard where permitted, and transfers copied files from their **current source only when requested**. Native integration should make copy/paste feel familiar, but not misrepresent mobile or Wayland limitations.

This documentation is the repository baseline. The Phase 0 baseline was accepted on 2026-11-12 (see `docs/APPROVAL_NEEDED.md`). Acceptance covers the product direction and Phase 0 experiments. It does not approve unverified production architectures, future security-protocol choices or Premium infrastructure, and PROPOSED and OPEN items keep their status.

## Documentation map

| File | Role |
|---|---|
| [Product vision](docs/PRODUCT_VISION.md) | Value proposition, principles, boundaries and success |
| [Product requirements](docs/PRD.md) | Functional requirements, non-goals, acceptance outcomes |
| [UX specification](docs/UX_SPEC.md) | Workflows, availability and failure states |
| [Architecture](docs/ARCHITECTURE.md) | Local core, native bridges, control and payload planes |
| [Platform capabilities](docs/PLATFORM_CAPABILITIES.md) | OS limitations and experiment targets |
| [Data model and protocol](docs/DATA_MODEL_AND_PROTOCOL.md) | Entities, message families, lifecycle and compatibility |
| [Security and privacy](docs/SECURITY_AND_PRIVACY.md) | Trust boundaries, E2EE, risks and minimum controls |
| [Transfers and routing](docs/TRANSFER_AND_ROUTING.md) | Live-source, resume, QUIC, direct-only Free |
| [Premium gateway](docs/PREMIUM_GATEWAY.md) | Relay, explicit temporary staging, pricing and quotas |
| [Phase 0 validation](docs/PHASE_0_VALIDATION.md) | Risk-first proof plan, test matrix and gates |
| [Experiment specifications](docs/experiments/README.md) | Phase 0 experiment specs; E-02 **approved**, Stage A **complete and accepted** (2026-11-13) |
| [Windows engineering checklist](docs/WINDOWS_ENGINEERING_CHECKLIST.md) | Windows adapter rules from experiment evidence (W-01 required) |
| [Roadmap](docs/ROADMAP.md) | Phases, sequencing and exit criteria |
| [Decisions](docs/DECISIONS.md) | Decision register and proposed ADR workflow |
| [ADRs](docs/adrs/README.md) | ADR-0001–0007 and 0009 **accepted** (Phase 0 baseline); ADR-0008 Proposed (OPEN) |
| [Approval needed](docs/APPROVAL_NEEDED.md) | Explicit owner choices before coding |
| [Traceability](docs/TRACEABILITY.md) | Requirement → design → evidence linkage |
| [Glossary](docs/GLOSSARY.md) | Consistent project vocabulary |
| [Research sources](docs/RESEARCH_SOURCES.md) | Research provenance and authoritative references |
| [Engineering review 2026-11-12](docs/reviews/ENGINEERING_REVIEW_2026-11-12.md) | Independent review findings and proposals — review input, **not approved** |
| [Claude guidance](CLAUDE.md) | Implementation protocol and PR/review discipline |

## Decision markers

Use these labels consistently in plans, issues and PRs:

- **CONFIRMED** — already agreed by the product owner in the design discussions.
- **RESEARCHED** — supported by the initial research or platform documentation, but subject to version-specific verification.
- **PROPOSED** — a recommended implementation approach, awaiting decision or experiment.
- **OPEN** — an uncertainty requiring a product decision or empirical proof.

If a technology is marked PROPOSED, do not silently promote it to CONFIRMED. A proposal can be tested without committing to long-term adoption when the experiment's scope is explicit.

## Intended workflow

1. Review **PRODUCT_VISION**, **PRD**, **UX_SPEC** and **APPROVAL_NEEDED** together.
2. Review architecture/security/transfer constraints and resolve only decisions needed for Phase 0.
3. Approve a baseline by dated decision entry or ADR; keep unresolved items visible.
4. Give Claude a *documentation-only* initial PR, if needed, for canonical repository placement. **Do not start feature code** until the baseline is accepted.
5. Implement Phase 0 as independently testable experiments, preserving evidence and the capability matrix.
6. Adopt or reject technologies based on evidence, not convenience.

## Core invariants (confirmed)

1. Native applications on Windows, macOS, Linux, Android and iOS/iPadOS; **the shared Rust core runs locally on every device** as a desktop agent or embedded mobile library.
2. OneCopy's logical clipboard/history is independent of the active OS clipboard. A remote entry does not overwrite all devices' active clipboards.
3. Native OS integration is the bridge to ordinary applications; **no application-specific browser/IDE/editor extensions** are part of the architecture.
4. Normal file copies create live source references. **No automatic snapshots, backups, version history, file duplication or change monitoring.** File bytes stream from the source when requested.
5. Direct encrypted P2P and small encrypted-history sync are available to Free users. **All OneCopy-managed payload relay and staging are Premium-only.**
6. `Keep Available Offline` is an explicit Premium action creating an encrypted, temporary, frozen-at-upload staged copy; it has expiry, quotas and a manual refresh action.
7. Never circumvent OS privacy restrictions or promise automatic mobile clipboard monitoring that the OS does not permit.
8. Reliability, performance and security are assessed with reproducible tests; never substitute assertions for evidence.

**Accepted clarifications (2026-11-12, Phase 0 baseline — read together with the invariants above):**

- **#3** — third-party application plugins stay out of scope; OneCopy-owned OS integrations are evaluated individually by ADR (D-027, ADR-0005).
- **#4** — "change monitoring" means idle or background monitoring; consistency checks on the opened source *during an active transfer* are required (D-026, ADR-0003). For **transient mobile sources only**, an explicit, user-authorized, bounded and expiring local copy is an **intentional exception** (D-025, ADR-0004). It does **not** permit automatic source duplication of any kind. Sources are not locked by default, and no copy-on-write clones or hidden snapshots are made (D-042).
- **#5** — Free connections may use signaling and STUN-style address discovery; no OneCopy-managed or third-party application relay, including library defaults, forwards Free inter-device packets; direct peer-to-peer is the only Free data path (D-021, D-022, ADR-0001).

## Evidence and provenance

This package is based on the supplied **OneCopy: Decision-Ready Architecture and Feasibility Research** (2026-11-12) and the product decisions discussed afterward. Research recommendations are clearly distinguished from confirmed product requirements; see `docs/RESEARCH_SOURCES.md` and `docs/DECISIONS.md`. The research report is retained in the repository at `docs/research/onecopy-deep-research-2026-11-12.md` (factual correction, 2026-11-12 review: the report is already committed).
