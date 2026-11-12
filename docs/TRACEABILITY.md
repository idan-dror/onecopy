# Requirements → Design → Validation traceability

**Status:** Initial map; maintain with every implementation PR. IDs from `PRD.md` and `PHASE_0_VALIDATION.md`.

| Requirement IDs | Primary design docs | Phase 0 evidence | Future acceptance |
|---|---|---|---|
| PR-001, PR-002, PR-019 | ARCHITECTURE, PLATFORM_CAPABILITIES | E-01, E-03, E-04, E-05, E-12 | Supported client matrix |
| PR-003, PR-004, PR-006, PR-022 | ARCHITECTURE, UX_SPEC, DATA_MODEL_AND_PROTOCOL | E-01, E-04, E-11 | Independent history, no OS clipboard takeover |
| PR-005, PR-025, PR-027 | PRD, DATA_MODEL_AND_PROTOCOL, UX_SPEC | E-01, E-11 | Representation handling, deterministic dedup/retention |
| PR-007, PR-021, PR-026, NFR-001 | SECURITY_AND_PRIVACY, DATA_MODEL_AND_PROTOCOL | E-11 | Device trust, key review, revocation |
| PR-008, PR-009, PR-010, PR-011 | TRANSFER_AND_ROUTING, DATA_MODEL_AND_PROTOCOL | E-08, E-09 | Live source, no silent mixed results |
| PR-012, PR-013, NFR-003 | TRANSFER_AND_ROUTING | E-09, E-10 (managed download); E-02 (native paste claims) | 100 GB bounded/resumable verified **managed download**; native paste only to demonstrated claims (D-024) |
| PR-014, PR-015, PR-016 | TRANSFER_AND_ROUTING, PREMIUM_GATEWAY | E-06, E-07, E-13 | No Free managed relay payload |
| PR-017, PR-018 | PREMIUM_GATEWAY, UX_SPEC, DATA_MODEL_AND_PROTOCOL | E-13 (early economics only) | Explicit, verified, expiring Premium copy |
| PR-020 | ARCHITECTURE, PRODUCT_VISION | Document review | No per-third-party-app add-ons |
| PR-023 | PLATFORM_CAPABILITIES, UX_SPEC | E-02, E-03, E-05 | Native paste or owner-approved fallback |
| PR-024, NFR-002 | SECURITY_AND_PRIVACY, UX_SPEC | E-11 + platform inspection | Sensitive data filtering and safe telemetry |
| NFR-004, NFR-005 | SECURITY_AND_PRIVACY, TRANSFER_AND_ROUTING | E-08, E-10, E-11 | No remote raw path read; safe manifests |
| NFR-006 | DATA_MODEL_AND_PROTOCOL | E-11 + schema tests | Version negotiation and safe incompatibility |
| NFR-007 | PLATFORM_CAPABILITIES | E-03, E-05, E-12 | Permissions honored |
| NFR-008 | SECURITY_AND_PRIVACY, ROADMAP | Security review and CI evidence | Signed release and audit before beta |
| NFR-009 | PHASE_0_VALIDATION | E-06, E-07, E-09 | Measured SLOs, not assumed |

## Proposed requirements from the PR #1 review (PROPOSED, 2026-11-12)

These rows map **proposed** requirements and are not accepted. Status must be updated when the owner accepts, amends or rejects them.

| Requirement IDs | Status | Decision / ADR | Primary design docs | Phase 0 evidence | Future acceptance |
|---|---|---|---|---|---|
| PR-028, PR-029, PR-030 | PROPOSED | D-024 / ADR-0002 | TRANSFER_AND_ROUTING, UX_SPEC | E-02, E-03, E-09 | Per-mode guarantees demonstrated; no unproven native-paste claim |
| PR-031 | PROPOSED | D-025 / ADR-0004 | DATA_MODEL_AND_PROTOCOL, UX_SPEC | E-12 | Explicit, bounded, expiring mobile local staging |
| PR-032 | PROPOSED | D-030 / ADR-0006 | DATA_MODEL_AND_PROTOCOL | E-11 | Inline cap enforced; `content_ref` fetch works |
| PR-033 | PROPOSED | D-031 / ADR-0006 | DATA_MODEL_AND_PROTOCOL, PREMIUM_GATEWAY | E-11 | Free quotas enforced; no P2P size cap |
| PR-034 | PROPOSED | D-032 / ADR-0007 | PLATFORM_CAPABILITIES, SECURITY_AND_PRIVACY | E-01, E-03, E-04 | Markers honored and set; limitations shown |
| PR-035 | PROPOSED | D-033 / ADR-0007 | SECURITY_AND_PRIVACY, PLATFORM_CAPABILITIES | E-02, E-09 | Provenance applied (managed) or observed (native) |
| PR-036 | PROPOSED | D-036 | PLATFORM_CAPABILITIES | E-04, E-05 | Published support tiers |
| NFR-010, NFR-011, NFR-012 | PROPOSED | D-038–D-040 / ADR-0009 | SECURITY_AND_PRIVACY, ARCHITECTURE | E-11 | IPC authorization, single writer, dump hygiene |
| NFR-013 | PROPOSED | D-021 / ADR-0001 | TRANSFER_AND_ROUTING | E-06a, E-06b | Zero Free inter-device packets through relays |
| NFR-014 | PROPOSED | D-034 | PHASE_0_VALIDATION | E-06a, E-07 | Per-class connectivity reporting |
| NFR-015 | PROPOSED | review DA-25 | PRD | E-01 (agent footprint) | Measured idle footprint budget |
| NFR-016 | OPEN | review DA-24 / B-12 | DATA_MODEL_AND_PROTOCOL | E-11 (version tests) | Compatibility window |
| (security review) | OPEN | D-037 / ADR-0008 | SECURITY_AND_PRIVACY | E-11 | Revocation, freshness, recovery policy |

## Maintenance expectations

Every new requirement must have stable ID, owner acceptance status, design location, relevant automated/real-machine evidence and explicit release gating. When a platform experiment fails, update capability claims, UX fallback and roadmap as well as the test record. Do not mark a requirement accepted merely because a unit test passes; distinguish integration/security validation and owner acceptance.
