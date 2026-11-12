# Requirements → Design → Validation traceability

**Status:** Initial map; maintain with every implementation PR. IDs from `PRD.md` and `PHASE_0_VALIDATION.md`.

| Requirement IDs | Primary design docs | Phase 0 evidence | Future acceptance |
|---|---|---|---|
| PR-001, PR-002, PR-019 | ARCHITECTURE, PLATFORM_CAPABILITIES | E-01, E-03, E-04, E-05, E-12 | Supported client matrix |
| PR-003, PR-004, PR-006, PR-022 | ARCHITECTURE, UX_SPEC, DATA_MODEL_AND_PROTOCOL | E-01, E-04, E-11 | Independent history, no OS clipboard takeover |
| PR-005, PR-025, PR-027 | PRD, DATA_MODEL_AND_PROTOCOL, UX_SPEC | E-01, E-11 | Representation handling, deterministic dedup/retention |
| PR-007, PR-021, PR-026, NFR-001 | SECURITY_AND_PRIVACY, DATA_MODEL_AND_PROTOCOL | E-11 | Device trust, key review, revocation |
| PR-008, PR-009, PR-010, PR-011 | TRANSFER_AND_ROUTING, DATA_MODEL_AND_PROTOCOL | E-08, E-09 | Live source, no silent mixed results |
| PR-012, PR-013, NFR-003 | TRANSFER_AND_ROUTING | E-09, E-10 | 100 GB bounded/resumable verified transfer |
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

## Maintenance expectations

Every new requirement must have stable ID, owner acceptance status, design location, relevant automated/real-machine evidence and explicit release gating. When a platform experiment fails, update capability claims, UX fallback and roadmap as well as the test record. Do not mark a requirement accepted merely because a unit test passes; distinguish integration/security validation and owner acceptance.
