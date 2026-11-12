# Owner approval checklist — before feature code

**Status:** OPEN. This document explicitly prevents the proposed design from being treated as already signed off.

## Baseline acceptance

- [ ] Confirm this documentation set captures the agreed OneCopy product behavior, particularly independent shared clipboard, local core per device, no third-party app add-ons, live source references and Premium-only managed gateway.
- [ ] Confirm the decision-status convention, governance and Claude implementation instructions.
- [ ] Accept Phase 0 as a *feasibility experiment*, not a promise of universal native paste support or a public beta.

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

## Approval record

```text
Baseline version: 0.1
Reviewed by: [pending]
Decision/date: [pending]
Exceptions / requested changes: [pending]
Approving commit or PR: [pending]
```

A single overall baseline approval is sufficient to let Claude do the **explicit Phase 0 experiments**, while technical design alternatives remain open until evidence and relevant ADR sign-off. Claude should not demand that Phase 4 billing details be fixed before testing Windows virtual file paste.
