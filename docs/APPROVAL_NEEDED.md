# Owner approval checklist — before feature code

**Status:** OPEN. This document explicitly prevents the proposed design from being treated as already signed off.

## Baseline acceptance

- [ ] Confirm this documentation set captures the agreed OneCopy product behavior, particularly independent shared clipboard, local core per device, no third-party app add-ons, live source references and Premium-only managed gateway.
- [ ] Confirm the decision-status convention, governance and Claude implementation instructions.
- [ ] Accept Phase 0 as a *feasibility experiment*, not a promise of universal native paste support or a public beta.
- [ ] Accept, amend or reject the owner-directed proposals D-021 to D-042 and ADR-0001 to ADR-0009 (see the section below). Until accepted they remain PROPOSED or OPEN.
## Recommended near-term decisions

| ID | Decision requiring confirmation | Recommendation | Reason |
|---|---|---|---|
| A-01 | Default rolling history length | 10 items initially | Small, understandable default; easy to change later |
| A-02 | Transport family | QUIC | Secure, multiplexed streaming with migration support |
| A-03 | First transport spike | `iroh` as **experiment only** | Existing NAT/identity helpers; must prove direct-only Free |
| A-04 | Binding investigation | UniFFI Swift/Kotlin, stable Windows C ABI option | Keep Rust core reusable without coarse FFI bugs |
| A-05 | Finder fallback | Explicit OneCopy Paste/Download to Folder if Cmd-V fails | Preserve usefulness without false promises |
| A-06 | Wayland wording | Claim support only for tested compositors/versions | Respect OS restrictions |
| A-07 | Android optional IME | Exclude from initial scope | High UX/security complexity and no universal copy-monitor solution |
| A-08 | Storage experiment | R2 as a benchmark candidate only | Attractive published egress cost, not yet performance-proven |

Notes added 2026-11-12, with no change to the recommendations above until accepted:

- A-03 would be superseded by D-023 (two independent transport tracks) if that is accepted.
- A-05 remains the fallback. Finder `Command-V` stays unverified pending E-03.
- A-08 is limited to desk research during Phase 0 by D-029.

## Owner directions recorded from the PR #1 review (2026-11-12) — pending explicit acceptance

The owner and an engineering teammate directed the following after reviewing `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md`. They are documented as PROPOSED (or OPEN) and are **not approved** until the approval record below is completed.

| Review item | Direction (summary) | Decision IDs | ADR |
|---|---|---|---|
| OD-01 | Free connections may use signaling, presence, candidate exchange and STUN-style address discovery; OneCopy infrastructure never forwards Free inter-device packets; two independent transport tracks | D-021, D-023 (D-022 OPEN) | 0001 |
| OD-02 | Separate managed-download and native-paste guarantees; no native-paste guarantee claimed unless demonstrated; a trailer alone is not assumed to fix target partial-file behavior | D-024 | 0002 |
| OD-03 | Explicit, bounded, expiring mobile local staging for transient sources; never automatic; distinct from Premium staging | D-025 | 0004 |
| OD-04 | No idle or background source monitoring; consistency checks during active transfers | D-026 (D-042 OPEN) | 0003 |
| OD-05 | OneCopy-owned OS integrations evaluated individually by ADR; third-party plugins out of scope; provider integrations limited to authorized history items | D-027 | 0005 |
| OD-06 | Staged equipment plan; start with available hardware and free local environments; no purchases, paid services or assumed budget without approval | D-028 | — |
| OD-07 | E-13 desk research only during Phase 0 | D-029 | — |
| Protocol and data | Inline cap (256 KiB experimental start); on-demand content references; Free control-plane quotas without P2P size limits | D-030, D-031 | 0006 |
| Security | Sensitivity markers with limitations; received-file provenance; local IPC authorization; single-writer state; crash-dump hygiene | D-032, D-033, D-038, D-039, D-040 | 0007, 0009 |
| Security | Revocation authority, roster freshness, offline LAN behavior and recovery go to the pre-implementation security review; no arbitrary timeout | D-037 (OPEN) | 0008 |
| Platform | Linux support tiers; GNOME Wayland limitation documented; Finder `Command-V` unverified pending E-03; iOS continued processing is an experiment only | D-036 | — |
| Networking | Per-network-class connectivity targets replace the universal 90% target; the ~70% figure is a conditional DCUtR hole-punch rate, not a OneCopy prediction; defer complex routing | D-034, D-035 | — |

## Decisions to investigate before commitment

| ID | Unknown | Trigger for binding decision |
|---|---|---|
| B-01 | E2EE group key hierarchy: epoch-wrapped secret vs MLS | Before implementing production key distribution/enrollment |
| B-02 | Recovery: all-devices-lost behavior and recovery-key UX | Before releasing multi-device accounts |
| B-03 | Sensitive-content defaults, exclusions and pause behavior | Before automatic capture in public beta |
| B-04 | Exact local/remote history eviction and deletion semantics | Before offline sync and removal implementation |
| B-05 | Data format and schema evolution (CBOR/protobuf/etc.) | Before cross-platform interop implementation |
| B-06 | Native UI toolkit, desktop packaging and auto-update | Before product-grade client UX |
| B-07 | Premium quotas, retention, relay regions, provider and billing | Before any paid service launch |
| B-08 | Revocation authority, roster freshness, offline LAN authorization, recovery (D-037, ADR-0008) | Pre-implementation security review; before Phase 1 trust code |
| B-09 | Third-party relays for Free inter-device packets (D-022) | Before E-06a configuration is frozen |
| B-10 | Synthetic keystroke or automated paste into other apps (D-041) | Before any "paste for me" UX |
| B-11 | Transfer-time write prevention and copy-on-write clones (D-042) | Before Phase 1 transfer engine |
| B-12 | Cross-version compatibility window (review DA-24) | Before first external build |
| B-13 | Mobile local staging quota, expiry default and delete-after-send policy (ADR-0004) | Before Phase 3 |

## Approval record

```text
Baseline version: 0.1
Reviewed by: [pending]
Decision/date: [pending]
Exceptions / requested changes: [pending]
Approving commit or PR: [pending]
```

A single overall baseline approval is sufficient to let Claude do the **explicit Phase 0 experiments**, while technical design alternatives remain open until evidence and relevant ADR sign-off. Claude should not demand that Phase 4 billing details be fixed before testing Windows virtual file paste.
