# Architecture decision records (ADRs)

**Status of this directory:**

- ADR-0001 to ADR-0007 and ADR-0009 were **accepted on 2026-11-12** as part of the Phase 0 baseline (PR #1).
- ADR-0008 remains **Proposed**; its decision is OPEN (D-037).
- **Scope:** acceptance covers the product direction and Phase 0 feasibility experiments. It does not approve unverified production architectures, future security-protocol choices or Premium infrastructure. Mechanisms an ADR leaves unspecified still need their own decisions.

Format: the template in `docs/DECISIONS.md`. ADRs are never backdated or silently rewritten. A changed decision gets a new ADR that supersedes the old one.

| ADR | Title | Status | Decision IDs |
|---|---|---|---|
| [0001](0001-free-connection-establishment-and-transport-tracks.md) | Free connection establishment and two-track transport evaluation | Accepted | D-021, D-022, D-023 |
| [0002](0002-delivery-modes-managed-download-and-native-paste.md) | Delivery modes: managed download and native paste | Accepted | D-024 |
| [0003](0003-source-consistency-boundary.md) | Source consistency boundary: transfer-time validation versus monitoring and snapshots | Accepted | D-026, D-042 |
| [0004](0004-mobile-explicit-local-staging.md) | Explicit, bounded mobile local staging for transient sources | Accepted (quota, expiry and delete-after-send OPEN, B-13) | D-025 |
| [0005](0005-onecopy-owned-os-integrations.md) | OneCopy-owned OS integrations versus third-party application plugins | Accepted (no specific integration approved) | D-027 |
| [0006](0006-inline-content-cap-and-free-control-plane-quotas.md) | Inline content cap, on-demand content references and Free control-plane quotas | Accepted (values experimental) | D-030, D-031 |
| [0007](0007-sensitivity-markers-and-received-file-provenance.md) | Clipboard sensitivity markers and received-file provenance | Accepted (mechanism details after measurement) | D-032, D-033 |
| [0008](0008-trust-lifecycle-revocation-freshness-recovery.md) | Trust lifecycle: revocation authority, roster freshness, offline LAN and recovery | **Proposed — decision OPEN** | D-037 |
| [0009](0009-local-process-security.md) | Local process security: IPC authorization, single-writer cryptographic state, crash dumps | Accepted (per-OS mechanism in the process-model ADR) | D-038, D-039, D-040 |
