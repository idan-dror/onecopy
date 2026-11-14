# OneCopy — Roadmap and release gates

**Status:** Phase structure PROPOSED based on research; the product direction and major invariants are CONFIRMED. Dates and capacity are deliberately not fixed.

## Phase 0 — Feasibility and architecture validation

**Goal:** Eliminate expensive unknowns and prove security/correctness boundaries before polishing product code.

**Includes:** platform clipboard spikes, Windows virtual-file Explorer paste, Finder Command-V, Wayland compositor matrix, direct-only QUIC candidate, NAT matrix, encrypted history/trust prototype, live-source transfer mutation handling, 100 GB resume, threat model and fault suite.

**Exit:** `PHASE_0_VALIDATION.md` gates A–E documented and owner accepts resolved choices/fallbacks. No requirement that every platform achieves identical UX.

*Refinements accepted with the Phase 0 baseline (2026-11-12):*

- **Equipment (D-028).** Phase 0 follows a staged equipment plan: available hardware and free local environments first. Experiments needing unapproved equipment remain blocked.
- **E-13 (D-029).** E-13 is desk research only. Gateway measurements move to Phase 4.
- **Transport (D-023).** E-06 evaluates two independent transport tracks.
- **First experiment.** E-02 is the first implementation-ready experiment.

*Progress:*

- **2026-11-13: E-02 Stage A complete and accepted** (small-file Explorer paste, T1, no faults; spec §0.1). Stage B (faults, then large files and other targets) is in progress. E-02 and Gate B remain open.

## Phase 1 — Desktop technical MVP

**Goal:** Usable trusted-device clipboard/history with direct P2P transfer.

**Includes:** local Rust core on Windows and one Linux target first, then macOS; local trusted device roster, E2EE sync, ~10 item history, normal copy/paste bridge as allowed, text/image/file/folder representations, on-demand transfers, OneCopy-managed download with bounded resume and controlled finalization, native paste where E-02 evidence supports it, user-visible errors.

**Excludes:** managed Premium relay/staging, per-application add-ons, full mobile background integration and automatic snapshots.

**Exit:** core user journeys UJ-01–UJ-06 pass on supported desktop configurations; code is maintainable and testable; outstanding limitations are documented. For UJ-02, resume and controlled finalization are verified on **OneCopy-managed downloads**. Native paste is verified only against the target-specific claims that E-02 demonstrated, and each unsupported guarantee is documented (D-024, corrected 2026-11-12).

## Phase 2 — Desktop production hardening

**Goal:** A supportable consumer desktop app with defensible security guarantees.

**Includes:** compositor-specific Linux support, permission onboarding for macOS, signing/notarization/updaters, reliable crash behavior, protocol/FFI security reviews, abuse/rate-limit defenses, performance monitoring without content leakage, control-plane availability and compatibility testing.

**Exit:** approved platform matrix, security review, threat mitigations, release pipeline and operating procedures. No beta with unreviewed critical cryptography.

## Phase 3 — Mobile experiences

**Goal:** Honest OS-native Android/iOS access to OneCopy history and file workflows.

**Includes:** Android Kotlin Sharesheet/content provider/user-initiated transfer jobs; iOS Swift Share extension, Files integration and foreground P2P; app embedding the shared Rust core; lifecycle/permission behavior and explicit user-driven workflows.

**Excludes:** universal invisible Android background clipboard monitoring and permanent iOS QUIC daemon. Optional Android IME is separate evaluation.

**Exit:** supported mobile workflows pass real-device tests; app review/background constraints and native permissions respected.

## Phase 4 — Premium gateway

**Goal:** Reliable connectivity where direct is blocked and explicit temporary offline availability, with healthy unit economics.

**Includes:** encrypted managed live relay, quota/entitlement service, expiring client-encrypted object staging, readiness verification, manual refresh, pricing/abuse governance, regional benchmarks and operational SLIs. *(D-029: includes the gateway storage and relay measurements moved out of Phase 0.)*

**Exit:** paid gateway reliability/cost/security evidence, no unexpected Free payload routing through managed relays, staging retention controls operational.

## Cross-cutting milestones

- Versioned protocol and cross-client compatibility matrix.
- Threat-model refresh before every trust-boundary expansion.
- Benchmarks after networking or file-manifest change.
- ADRs for transport, group key hierarchy, native paste fallbacks, retained platform capability claims and gateway choice.
- Product copy/marketing reviews against actual OS limitations.

## Not committed by this roadmap

Release dates; a specific UI toolkit; exact cloud providers/prices; "unlimited Premium"; support for every Wayland compositor; Finder native paste success; network connectivity percentages without test data.
