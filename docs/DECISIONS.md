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
