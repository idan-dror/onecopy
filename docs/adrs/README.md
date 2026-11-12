# Architecture decision records (ADRs)

**Status of this directory:** every ADR here is **Proposed**. None is accepted. Acceptance requires an explicit owner record in the ADR's "Acceptance record" section and in `docs/APPROVAL_NEEDED.md`. Do not implement against a Proposed ADR except in a clearly bounded Phase 0 experiment.

Format: the template in `docs/DECISIONS.md`. ADRs are never backdated or silently rewritten. A changed decision gets a new ADR that supersedes the old one.

| ADR | Title | Status | Decision IDs |
|---|---|---|---|
| [0001](0001-free-connection-establishment-and-transport-tracks.md) | Free connection establishment and two-track transport evaluation | Proposed | D-021, D-022 (OPEN), D-023 |
| [0002](0002-delivery-modes-managed-download-and-native-paste.md) | Delivery modes: managed download and native paste | Proposed | D-024 |
| [0003](0003-source-consistency-boundary.md) | Source consistency boundary: transfer-time validation versus monitoring and snapshots | Proposed | D-026, D-042 (OPEN) |
| [0004](0004-mobile-explicit-local-staging.md) | Explicit, bounded mobile local staging for transient sources | Proposed | D-025 |
| [0005](0005-onecopy-owned-os-integrations.md) | OneCopy-owned OS integrations versus third-party application plugins | Proposed | D-027 |
| [0006](0006-inline-content-cap-and-free-control-plane-quotas.md) | Inline content cap, on-demand content references and Free control-plane quotas | Proposed | D-030, D-031 |
| [0007](0007-sensitivity-markers-and-received-file-provenance.md) | Clipboard sensitivity markers and received-file provenance | Proposed | D-032, D-033 |
| [0008](0008-trust-lifecycle-revocation-freshness-recovery.md) | Trust lifecycle: revocation authority, roster freshness, offline LAN and recovery | Proposed (options only; decision OPEN) | D-037 |
| [0009](0009-local-process-security.md) | Local process security: IPC authorization, single-writer cryptographic state, crash dumps | Proposed | D-038, D-039, D-040 |
