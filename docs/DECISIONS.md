# OneCopy — Decision register and ADR process

**Status:** Initial reconstruction of decisions from product discussions and research. Owner must accept the baseline before implementation begins.

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

## Owner-directed proposals from the PR #1 review (recorded 2026-11-12)

**Status of this section:** the product owner and an engineering teammate reviewed `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` and gave the directions below. They are recorded as **PROPOSED** (or **OPEN** where the direction is to investigate). None is CONFIRMED until the owner explicitly accepts the documentation baseline (`APPROVAL_NEEDED.md` §Approval record). Where an entry clarifies a CONFIRMED decision, the CONFIRMED wording stays unchanged until acceptance.

| ID | Status | Decision (proposed) | Implication | Source / ADR |
|---|---|---|---|---|
| D-021 | PROPOSED | Free connections may use control-plane signaling, presence, candidate exchange and STUN-style address discovery. OneCopy-managed infrastructure must not forward inter-device packets for Free users. | "Relay" covers *any* forwarded packet: payload, handshake and NAT-traversal frames. Verified by packet capture and server byte counters. Clarifies D-008 / PR-015. | OD-01; ADR-0001 |
| D-022 | OPEN | Whether third-party relays (for example a library's default public relays) may carry any Free inter-device packet | Reviewer recommends prohibiting them, for privacy, reliability and the "library defaults" rule in `CLAUDE.md`. | New; ADR-0001 |
| D-023 | PROPOSED | Evaluate two independent transport tracks: (1) iroh, configured to satisfy D-021; (2) a standalone QUIC implementation plus ICE-style NAT traversal | Would supersede the "iroh evaluated first" ordering in D-012. QUIC remains the transport family. | OD-01; ADR-0001 |
| D-024 | PROPOSED | Two delivery modes with separate guarantees. **Managed download:** journaled resume, full verification, controlled finalization. **Native paste:** no claim of those guarantees unless demonstrated experimentally for the specific target. | A consistency trailer is not assumed to solve partial-file behavior in Explorer or other targets. Clarifies PR-012, PR-023. | OD-02; ADR-0002 |
| D-025 | PROPOSED | Mobile devices may keep an explicitly user-requested, bounded, expiring local copy when source permissions are transient. Never automatic. Distinct from Premium cloud staging. | Interprets D-005 for transient mobile sources. An intentional, user-authorized exception for transient sources only; never permission for automatic source duplication. | OD-03; ADR-0004 |
| D-026 | PROPOSED | No idle or background source-file change monitoring. Consistency checks on the opened source are required during active transfers and staging. Stat/identity checks are not claimed to detect every concurrent modification. | Clarifies README invariant #4 and D-005; defines the snapshot/versioning boundary. | OD-04; ADR-0003 |
| D-027 | PROPOSED | OneCopy-owned OS integrations and extensions (for example File Provider, share extensions, an IME, desktop-shell integrations) may be evaluated one at a time through ADRs. Third-party application plugins remain out of scope. Any File Provider–like integration exposes only authorized OneCopy history items, never general browsing of the source filesystem. | Clarifies D-003 / PR-020 and the "not a remote filesystem mount" boundary. | OD-05; ADR-0005 |
| D-028 | PROPOSED | Staged equipment and test plan. Start with available hardware and free local environments. No hardware purchases, paid services or assumed budget without explicit owner approval. | Experiments needing unavailable equipment are blocked, not substituted with guesses. | OD-06; `PHASE_0_VALIDATION.md` |
| D-029 | PROPOSED | E-13 is desk research only during Phase 0. Gateway performance and infrastructure measurements belong to later Premium development. | No paid storage or relay resources in Phase 0. | OD-07 |
| D-030 | PROPOSED | Bounded inline encrypted content in history envelopes, with **256 KiB of ciphertext per item as an experimental starting point only**. Larger text or images become on-demand content references fetched from the origin. | Bounded control-plane cost. Free users cannot get larger items while the origin is offline. | Direction; ADR-0006 |
| D-031 | PROPOSED | Free control-plane rate and storage quotas. No product-imposed P2P file-size limit. | PR-013 is unchanged. Quota values are set from measurement. | Direction; ADR-0006 |
| D-032 | PROPOSED | Honor platform-provided clipboard sensitivity and exclusion markers on capture, set them on promotion of sensitive items, and document their limitations | Extends PR-024 / D-019. Markers are voluntary hints, not a guarantee. | Direction; ADR-0007 |
| D-033 | PROPOSED | Preserve the security provenance of received files. The per-OS marking mechanism is researched and measured before it is fixed. | Managed downloads can mark files; native-paste behavior is observed in E-02/E-03. | Direction; ADR-0007 |
| D-034 | PROPOSED | Connectivity is measured, targeted and reported per NAT and network class. The universal ≥90% direct-connectivity target is withdrawn as unsupported. | Replaces the PROPOSED target in `PHASE_0_VALIDATION.md`. | Direction |
| D-035 | PROPOSED | Defer complex route-scoring algorithms until they show measurable benefit. Phase 1 Free routing uses measured direct-path selection. | PR-014 (CONFIRMED) is still met by measured selection. | Direction |
| D-036 | PROPOSED | Publish per-desktop clipboard support tiers. Document that GNOME Wayland (Mutter) offers no data-control protocol for automatic background capture. | Clarifies D-017. | Direction; `PLATFORM_CAPABILITIES.md` |
| D-037 | OPEN | Revocation authority, roster freshness, offline LAN authorization and account recovery | Part of the pre-implementation security review. No freshness timeout is chosen without a documented security/usability trade-off. | Direction; ADR-0008 |
| D-038 | PROPOSED | Local IPC must authenticate and authorize its callers | No unauthenticated local endpoint may expose history or commands. | Direction; ADR-0009 |
| D-039 | PROPOSED | Single-writer rule for cryptographic state (sequence counters, ratchets, epoch state) | Extensions and secondary processes enqueue work and never encrypt with shared state. | Direction; ADR-0009 |
| D-040 | PROPOSED | Crash-dump and diagnostics hygiene: no full-memory dumps, scrubbed minidumps, opt-in reporting | Extends NFR-002. | Direction; ADR-0009 |
| D-041 | OPEN | Whether OneCopy may ever synthesize keystrokes or automated paste into other applications | Reviewer recommends no injection by default. | Review OD-15 |
| D-042 | OPEN | Transfer-time write prevention (for example opening Windows sources without write sharing) and transfer-scoped copy-on-write clones | Both affect user-visible OS behavior or resemble snapshots, so they need explicit owner decisions. | Review AR-13; ADR-0003 |

### Cross-references to existing entries (no status change)

| Existing | Related proposal | Note |
|---|---|---|
| D-003 (CONFIRMED) | D-027 | Proposed clarification: the rule targets third-party application plugins. |
| D-005 (CONFIRMED) | D-025, D-026, D-030 | Proposed clarifications of the live-reference rule for transient mobile sources, monitoring scope and captured clipboard content. |
| D-008 (CONFIRMED) | D-021, D-022 | Packet-level definition of "managed relay" for Free. |
| D-012 (PROPOSED) | D-023 | Two-track evaluation would replace "iroh evaluated first". |
| D-013 (PROPOSED) | — | Review AR-08 adds an option to write the Windows adapter in Rust; it is exercised inside E-02. |
| D-015 (PROPOSED) | D-029 | No R2 measurements during Phase 0. |
| D-016 (OPEN) | D-027 | Finder `Command-V` remains unverified pending E-03; any File Provider route requires its own ADR. |
| D-017 (OPEN) | D-036 | Support tiers. |
| D-019 (OPEN) | D-032 | Marker handling is one component of the sensitive-content policy. |

## Proposed ADRs

Proposed ADRs live in [`docs/adrs/`](adrs/README.md). An ADR with status *Proposed* is not an accepted decision.

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
