# OneCopy — Decision register and ADR process

**Status:** Decision register. Phase 0 baseline accepted by the product owner on 2026-11-12 (scope: Phase 0 feasibility experiments; see "Phase 0 baseline decisions" below). Entries keep their individual status labels.

## Decision classes

- **CONFIRMED:** product owner agreed to behavior in design discussion.
- **RESEARCHED:** verified/derived from OS/vendor documentation but requires environment tests.
- **PROPOSED:** default recommended path, not an owner commitment.
- **OPEN:** not yet decided; block implementation only when the decision is necessary for the slice.

## Product decision register

| ID | Status | Decision | Implication |
|---|---|---|---|
| D-001 | CONFIRMED | OneCopy is its own independent logical clipboard/history | OS clipboard is a bridge, not source of shared truth |
| D-002 | CONFIRMED | Native clients on Windows/macOS/Linux/Android/iOS; local Rust core on every device | Native adapter per OS; no server-side core execution requirement for clients |
| D-003 | CONFIRMED | No browser/editor/third-party application add-ons | All integrations are OS-level or OneCopy-native |
| D-004 | CONFIRMED | Remote history updates do not overwrite every active OS clipboard | Local explicit promotion into OS clipboard |
| D-005 | CONFIRMED | Normal file clipboard entry is a live source reference | No automatic snapshot/hash/upload/version history/change watcher at Copy time |
| D-006 | CONFIRMED | Requested file opens at time of request; source modifications may require transfer failure/retry | Prevent incompatible mixed completions |
| D-007 | CONFIRMED | Large direct P2P files/folders with bounded streaming, resume and integrity, no arbitrary size cap | Disk/network/FS limitations acknowledged |
| D-008 | CONFIRMED | All managed gateway payload relaying and temporary staging are paid | Free strict direct-only payload routing |
| D-009 | CONFIRMED | Premium Keep Available Offline is explicitly requested, temporary, E2EE and frozen at staging | Quotas, expiry, manual refresh, verified readiness |
| D-010 | CONFIRMED | No OS permission bypass and no false identical mobile experience | Capability-aware marketing and UX |
| D-011 | PROPOSED | Default history of ten entries | Size needs explicit acceptance |
| D-012 | PROPOSED | QUIC transport family, iroh evaluated first | Direct-only and NAT success must be proven |
| D-013 | PROPOSED | UniFFI for Swift/Kotlin; stable C ABI candidate for Windows | FFI quality gate |
| D-014 | OPEN | E2EE group key: HPKE-wrapped epochs vs MLS | Security, future key rotation and recovery |
| D-015 | PROPOSED | Cloudflare R2 as storage benchmark candidate | No provider commitment before cost/throughput tests |
| D-016 | OPEN | Finder Command-V file promise support and fallback | Empirical Phase 0 gate |
| D-017 | OPEN | Wayland support by compositor/version | Platform tests, not universal claim |
| D-018 | PROPOSED | Optional Android IME deferred | Do not build without value/security proof |
| D-019 | OPEN | Exact sensitive-clipboard exclusion and retention policy | Privacy UX and attack surface |
| D-020 | PROPOSED | Focus desktop-first, implement mobile and Premium later | Phase sequencing per roadmap |

## Phase 0 baseline decisions (accepted 2026-11-12)

**Acceptance and scope.**

- The product owner, with an engineering teammate, reviewed `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` and the proposals recorded from it, and **accepted the Phase 0 baseline on 2026-11-12** (PR #1). Entries marked CONFIRMED below were accepted then.
- **Scope limit:** this acceptance covers the product direction and the Phase 0 feasibility experiments. It does **not** approve unverified production architectures, future security-protocol choices, or Premium infrastructure. Mechanisms that the entries leave unspecified (quota values, per-OS IPC mechanisms, provenance attributes, mobile staging quota and expiry) still need their own decisions.
- Entries that remain OPEN stay open.

| ID | Status | Decision | Implication | Source / ADR |
|---|---|---|---|---|
| D-021 | CONFIRMED | Free connections may use control-plane signaling, presence, candidate exchange and STUN-style address discovery. OneCopy-managed infrastructure must not forward inter-device packets for Free users. | "Relay" covers *any* forwarded packet: payload, handshake and NAT-traversal frames. Verified by packet capture and server byte counters. Clarifies D-008 / PR-015. | OD-01; ADR-0001 |
| D-022 | CONFIRMED | Free connections must not forward inter-device packets through **OneCopy-managed or third-party application relays, including library defaults**. Control-plane signaling and STUN-style address discovery remain allowed. **Direct peer-to-peer connectivity is the only Free data path.** | Library default relays must be disabled in every Free configuration. | Owner decision 2026-11-12; ADR-0001 |
| D-023 | CONFIRMED | Evaluate two independent transport tracks: (1) iroh, configured to satisfy D-021/D-022; (2) a standalone QUIC implementation plus ICE-style NAT traversal | Supersedes the "iroh evaluated first" ordering in D-012. QUIC remains the transport family. The track choice is a later ADR based on E-06 evidence. | OD-01; ADR-0001 |
| D-024 | CONFIRMED | Two delivery modes with separate guarantees. **Managed download:** journaled resume, full verification, controlled finalization. **Native paste:** no claim of those guarantees unless demonstrated experimentally for the specific target. | A consistency trailer is not assumed to solve partial-file behavior in Explorer or other targets. Clarifies PR-012, PR-023 and UJ-02. | OD-02; ADR-0002 |
| D-025 | CONFIRMED | Mobile devices may keep an explicitly user-requested, bounded, expiring local copy when source permissions are transient. Never automatic. Distinct from Premium cloud staging. | An intentional, user-authorized exception for transient sources only; never permission for automatic source duplication. Quota, expiry and delete-after-send remain OPEN (B-13). | OD-03; ADR-0004 |
| D-026 | CONFIRMED | No idle or background source-file change monitoring. Consistency checks on the opened source are required during active transfers and staging. Stat/identity checks are not claimed to detect every concurrent modification. | Clarifies README invariant #4 and D-005; defines the snapshot/versioning boundary. | OD-04; ADR-0003 |
| D-027 | CONFIRMED | OneCopy-owned OS integrations and extensions (for example File Provider, share extensions, an IME, desktop-shell integrations) may be evaluated one at a time through ADRs. Third-party application plugins remain out of scope. Any File Provider–like integration exposes only authorized OneCopy history items, never general browsing of the source filesystem. | Clarifies D-003 / PR-020 and the "not a remote filesystem mount" boundary. No specific integration is approved by this entry. | OD-05; ADR-0005 |
| D-028 | CONFIRMED | Staged equipment and test plan. Start with available hardware and free local environments. No hardware purchases, paid services or assumed budget without explicit owner approval. | Experiments needing unavailable equipment are blocked, not substituted with guesses. | OD-06; `PHASE_0_VALIDATION.md` |
| D-029 | CONFIRMED | E-13 is desk research only during Phase 0. Gateway performance and infrastructure measurements belong to later Premium development. | No paid storage or relay resources in Phase 0. | OD-07 |
| D-030 | CONFIRMED | Bounded inline encrypted content in history envelopes, with **256 KiB of ciphertext per item as an experimental starting point only**. Larger text or images become on-demand content references fetched from the origin. | Bounded control-plane cost. Larger items are unavailable while the origin is offline, or while a mobile origin app is not running (explicit state). The cap value is not final. | Direction; ADR-0006 |
| D-031 | CONFIRMED | Free control-plane rate and storage quotas. No product-imposed P2P file-size limit. | PR-013 is unchanged. Quota values are set from measurement. | Direction; ADR-0006 |
| D-032 | CONFIRMED | Honor platform-provided clipboard sensitivity and exclusion markers on capture, set them on promotion of sensitive items, and document their limitations | Extends PR-024 / D-019. Markers are voluntary hints, not a guarantee. | Direction; ADR-0007 |
| D-033 | CONFIRMED | Preserve the security provenance of received files. The per-OS marking mechanism is researched and measured before it is fixed. | Managed downloads mark files; native-paste behavior is observed in E-02/E-03. | Direction; ADR-0007 |
| D-034 | CONFIRMED | Connectivity is measured, targeted and reported per NAT and network class. The universal ≥90% direct-connectivity target is withdrawn as unsupported. | Per-class targets are set after E-06a/E-07 data. | Direction |
| D-035 | CONFIRMED | Defer complex route-scoring algorithms until they show measurable benefit. Phase 1 Free routing uses measured direct-path selection. | PR-014 (CONFIRMED) is still met by measured selection. | Direction |
| D-036 | CONFIRMED | Publish per-desktop clipboard support tiers. Document that GNOME Wayland (Mutter) offers no data-control protocol for automatic background capture. | Clarifies D-017; actual tiers are set from E-04/E-05 evidence. | Direction; `PLATFORM_CAPABILITIES.md` |
| D-037 | OPEN | Revocation authority, roster freshness, offline LAN authorization and account recovery | **E-11 must investigate** all four. No production policy (including any freshness timeout) is chosen without an explicit security review. Does not block E-02. | Owner decision 2026-11-12; ADR-0008 |
| D-038 | CONFIRMED | Local IPC must authenticate and authorize its callers | No unauthenticated local endpoint may expose history or commands. The per-OS mechanism is fixed in the process-model ADR. | Direction; ADR-0009 |
| D-039 | CONFIRMED | Single-writer rule for cryptographic state (sequence counters, ratchets, epoch state) | Extensions and secondary processes enqueue work and never encrypt with shared state. | Direction; ADR-0009 |
| D-040 | CONFIRMED | Crash-dump and diagnostics hygiene: no full-memory dumps, scrubbed minidumps, opt-in reporting | Extends NFR-002. | Direction; ADR-0009 |
| D-041 | CONFIRMED | **No automatic or simulated keystroke injection as an initial product feature.** Isolated keystroke automation inside approved experiments is permitted for reproducible testing. | Experiment keystroke automation must not become a product dependency without a separate decision. | Owner decision 2026-11-12 |
| D-042 | CONFIRMED | **Do not lock source files against concurrent modification by default. Do not introduce transfer-scoped copy-on-write clones or hidden source snapshots.** Use scoped transfer-time source-consistency checks and report detectable incompatible modifications. | Residual races are documented, never claimed away. Any non-default locking option would need a separate decision. | Owner decision 2026-11-12; ADR-0003 |

### Cross-references to existing entries

| Existing | Related decision | Note |
|---|---|---|
| D-003 (CONFIRMED) | D-027 | Accepted clarification: the rule targets third-party application plugins. |
| D-005 (CONFIRMED) | D-025, D-026, D-030, D-042 | Accepted clarifications: transient mobile sources, monitoring scope, captured clipboard content, no locking or clones. |
| D-008 (CONFIRMED) | D-021, D-022 | Packet-level definition: direct P2P is the only Free data path. |
| D-012 (PROPOSED) | D-023 | **Superseded in its "iroh evaluated first" ordering by D-023.** The QUIC transport family remains PROPOSED until the transport ADR. |
| D-013 (PROPOSED) | — | Review AR-08 adds an option to write the Windows adapter in Rust; it is exercised inside E-02 only. |
| D-015 (PROPOSED) | D-029 | No R2 measurements during Phase 0. |
| D-016 (OPEN) | D-027 | Finder `Command-V` remains unverified pending E-03; any File Provider route requires its own ADR. |
| D-017 (OPEN) | D-036 | Tier structure accepted; per-compositor results remain empirical. |
| D-019 (OPEN) | D-032 | Marker handling is accepted; the overall sensitive-content default and retention policy remain OPEN. |
| D-011, D-013, D-018, D-020 (PROPOSED); D-014 (OPEN) | — | Unchanged by the Phase 0 baseline acceptance. Per `APPROVAL_NEEDED.md`, baseline approval authorizes Phase 0 experiments while technical alternatives stay open until evidence and ADR sign-off. |

## ADRs

ADRs live in [`docs/adrs/`](adrs/README.md). ADR-0001 to ADR-0007 and ADR-0009 were **accepted** on 2026-11-12 for the Phase 0 baseline. ADR-0008 remains *Proposed* (decision OPEN).

## ADR template

Create `docs/adrs/NNNN-title.md` only once the repository is established (a proposed decision is not yet an accepted ADR):

```markdown
# ADR NNNN: Decision title
Status: Proposed | Accepted | Rejected | Superseded
Date: YYYY-MM-DD
Owner: Product owner

## Context and evidence
## Decision
## Alternatives considered
## Security/privacy and interoperability effects
## User/product effects
## Consequences and follow-up tests
## Acceptance record (who/when)
```

Use stable ADR IDs and links from affected requirements. Never backdate or silently change accepted decisions. Supersede rather than rewriting history when architecture changes.

## Approval policy

Claude may investigate a PROPOSED library in an isolated experiment when authorized by the approved Phase 0 plan, but should not build production-facing architecture on it without an accepted decision. If findings contradict a CONFIRMED requirement, stop and present alternatives, with the minimum viable fallback and measurable impact.
