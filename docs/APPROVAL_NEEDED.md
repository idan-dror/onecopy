# Owner approval checklist — before feature code

**Status:**

- **Phase 0 baseline ACCEPTED on 2026-11-12** (see the Approval record below).
- Scope: product direction and Phase 0 feasibility experiments only. Not unverified production architectures, future security-protocol choices or Premium infrastructure.
- Items that remain OPEN or PROPOSED are listed below and keep their status.

## Baseline acceptance

- [x] Confirm this documentation set captures the agreed OneCopy product behavior, particularly independent shared clipboard, local core per device, no third-party app add-ons, live source references and Premium-only managed gateway. *(2026-11-12)*
- [x] Confirm the decision-status convention, governance and Claude implementation instructions. *(2026-11-12, as part of the Phase 0 baseline)*
- [x] Accept Phase 0 as a *feasibility experiment*, not a promise of universal native paste support or a public beta. *(2026-11-12)*
- [x] Accept, amend or reject the owner-directed proposals D-021 to D-042 and ADR-0001 to ADR-0009. *(2026-11-12)*
  - **Accepted:** D-021 to D-036 and D-038 to D-040.
  - **Recorded as owner decisions:** D-022, D-041, D-042.
  - **Remains OPEN:** D-037.
  - **ADRs accepted:** 0001 to 0007 and 0009. ADR-0008 remains Proposed.
- [x] Approve the E-02 specification (`docs/experiments/E-02_WINDOWS_VIRTUAL_FILE_PASTE.md`) as the first implementation-ready experiment, with the §15 answers recorded in the Approval record. *(2026-11-12)*

## Recommended near-term decisions

These were recommendations before the baseline review. Unless noted, they were **not individually decided** by the Phase 0 baseline acceptance and keep their PROPOSED status in `DECISIONS.md`.

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

Notes (2026-11-12):

- **A-03** is superseded by D-023 (CONFIRMED): iroh and standalone QUIC plus ICE-style traversal are two independent tracks.
- **A-05** remains the fallback. Finder `Command-V` stays unverified pending E-03.
- **A-06** is implemented by D-036 (support tiers).
- **A-08** is limited to desk research during Phase 0 by D-029.

## Owner directions from the PR #1 review — accepted 2026-11-12

| Review item | Direction (summary) | Decision IDs | ADR | Status |
|---|---|---|---|---|
| OD-01 | Free connections may use signaling, presence, candidate exchange and STUN-style address discovery; no OneCopy or third-party relay forwards Free inter-device packets; direct P2P is the only Free data path; two independent transport tracks | D-021, D-022, D-023 | 0001 | CONFIRMED |
| OD-02 | Separate managed-download and native-paste guarantees; no native-paste guarantee claimed unless demonstrated; a trailer alone is not assumed to fix target partial-file behavior | D-024 | 0002 | CONFIRMED |
| OD-03 | Explicit, bounded, expiring mobile local staging for transient sources; never automatic; an intentional exception, not permission for automatic duplication; distinct from Premium staging | D-025 | 0004 | CONFIRMED (B-13 OPEN) |
| OD-04 | No idle or background source monitoring; consistency checks during active transfers; no default locking, no copy-on-write clones or hidden snapshots | D-026, D-042 | 0003 | CONFIRMED |
| OD-05 | OneCopy-owned OS integrations evaluated individually by ADR; third-party plugins out of scope; provider integrations limited to authorized history items | D-027 | 0005 | CONFIRMED |
| OD-06 | Staged equipment plan; start with available hardware and free local environments; no purchases, paid services or assumed budget without approval | D-028 | — | CONFIRMED |
| OD-07 | E-13 desk research only during Phase 0 | D-029 | — | CONFIRMED |
| Protocol and data | Inline cap (256 KiB experimental start); on-demand content references with explicit availability states; Free control-plane quotas without P2P size limits | D-030, D-031 | 0006 | CONFIRMED (values experimental) |
| Security | Sensitivity markers with limitations; received-file provenance; local IPC authorization; single-writer state; crash-dump hygiene | D-032, D-033, D-038, D-039, D-040 | 0007, 0009 | CONFIRMED |
| Security | Revocation authority, roster freshness, offline LAN access and recovery are investigated in E-11; no production policy without explicit security review; does not block E-02 | D-037 | 0008 | **OPEN** |
| Platform | Support tiers; GNOME Wayland limitation documented; Finder `Command-V` unverified pending E-03; iOS continued processing is an experiment only | D-036 | — | CONFIRMED |
| Networking | Per-network-class connectivity targets replace the universal 90% target; the ~70% figure is a conditional DCUtR hole-punch rate, not a OneCopy prediction; defer complex routing | D-034, D-035 | — | CONFIRMED |
| Product | No automatic or simulated keystroke injection as an initial product feature; isolated keystroke automation allowed inside approved experiments only | D-041 | — | CONFIRMED |

## Owner directions from the E-02 Stage B1 review — 2026-11-14

Source: the owner's review of E-02 Stage B1 (PR #4) and the Stage A closure (PR #3), with an engineering teammate. The B1 research findings were accepted as findings. That is not product acceptance of native paste, and PR #4 stays unmerged.

| Review item | Direction (summary) | Decision IDs | Status |
|---|---|---|---|
| OD-08 | F1: exact length or failure for native-paste streams; final source-consistency check before a successful end of stream; no claim of universal change detection; verify on T1 and T2 | D-043, PR-037, W-08 | CONFIRMED |
| OD-09 | F2: unrestricted native overwrite is not approved; no workflow where an interrupted OneCopy transfer can destroy an existing destination file; native-paste replacement unsupported for release until a safe solution is demonstrated; investigate alternatives A–D independently and prepare a decision record | D-044, PR-038, W-09 | CONFIRMED (mechanism OPEN: B-14) |
| OD-10 | Accept Windows checklist W-02, W-03 and W-04 with explicit evidence limits; record the two further B1 HRESULT findings and their regression tests | W-02–W-04, W-07 | ACCEPTED (W-07 rule additions stay PROPOSED) |

## Decisions to investigate before commitment

| ID | Unknown | Trigger for binding decision | Status |
|---|---|---|---|
| B-01 | E2EE group key hierarchy: epoch-wrapped secret vs MLS | Before implementing production key distribution/enrollment | OPEN |
| B-02 | Recovery: all-devices-lost behavior and recovery-key UX | Before releasing multi-device accounts | OPEN (investigated with D-037 in E-11) |
| B-03 | Sensitive-content defaults, exclusions and pause behavior | Before automatic capture in public beta | OPEN (marker handling decided by D-032) |
| B-04 | Exact local/remote history eviction and deletion semantics | Before offline sync and removal implementation | OPEN |
| B-05 | Data format and schema evolution (CBOR/protobuf/etc.) | Before cross-platform interop implementation | OPEN |
| B-06 | Native UI toolkit, desktop packaging and auto-update | Before product-grade client UX | OPEN |
| B-07 | Premium quotas, retention, relay regions, provider and billing | Before any paid service launch | OPEN |
| B-08 | Revocation authority, roster freshness, offline LAN authorization, recovery (D-037, ADR-0008) | Explicit security review after E-11; before Phase 1 trust code | OPEN |
| B-09 | Third-party relays for Free inter-device packets | — | **Resolved by D-022** (prohibited) |
| B-10 | Synthetic keystroke or automated paste into other apps | — | **Resolved by D-041** (no product injection; experiment-only automation) |
| B-11 | Transfer-time write prevention and copy-on-write clones | — | **Resolved by D-042** (no default locking; no clones or hidden snapshots) |
| B-12 | Cross-version compatibility window (review DA-24) | Before first external build | OPEN |
| B-13 | Mobile local staging quota, expiry default and delete-after-send policy (ADR-0004) | Before Phase 3 | OPEN |
| B-14 | How D-044 is enforced: (A) provider-side prevention of destructive Explorer replacement, (B) a native workflow that never overwrites an existing destination, including races, (C) OneCopy-managed download with temporary output, verification and controlled replacement, (D) another OS-supported approach within product boundaries | F2 decision record (E-02 B2 preparation); before any Windows native-paste release | OPEN |

## Approval record

```text
Baseline version: 0.2 — Phase 0 baseline (documentation package after PR #1)
Reviewed by: Product owner, with an engineering teammate; independent engineering review by Claude
             (docs/reviews/ENGINEERING_REVIEW_2026-11-12.md)
Decision/date: ACCEPTED for Phase 0 feasibility experiments — 2026-11-12
Scope limits: Not an approval of unverified production architectures, future security-protocol
              choices, or Premium infrastructure. OPEN/PROPOSED items keep their status.
Accepted decisions: D-021–D-036, D-038–D-040; owner decisions D-022, D-041, D-042.
                    ADR-0001–0007 and ADR-0009 accepted.
Still OPEN: D-014, D-016, D-017, D-019, D-037 (ADR-0008); B-01–B-08, B-12, B-13.
Exceptions / requested changes (applied before merge):
  1. Large mobile-origin items may be unavailable while the origin app is suspended;
     explicit availability states and UX text.
  2. UJ-02 and Phase 1 exit distinguish native paste from managed download; resume and
     controlled finalization are guaranteed only for supported OneCopy-managed downloads.
  3. Explicit mobile local staging is an intentional, user-authorized exception for transient
     sources; never permission for automatic source duplication.
E-02 authorization: Specification approved; implementation in Stage A then Stage B, with a
  review gate between them; the implementation PR is not merged without owner approval.
  - Process Monitor: optional; official Microsoft Sysinternals source only; no silent
    install; raw traces stay local.
  - Elevated VHD creation: deferred; safe instructions prepared; no disk-management commands
    without separate operator approval (DEST-01..04 blocked).
  - SEC-04 hostile-filename tests: disposable VM only; blocked/not run until available.
  - Keystroke automation: allowed inside the experiment only (D-041).
Approving PR: idan-dror/onecopy#1 (merged with a merge commit; the SHA is recorded in git history
  and in the E-02 Stage A PR description)
```

A single overall baseline approval is sufficient to let Claude do the **explicit Phase 0 experiments**, while technical design alternatives remain open until evidence and relevant ADR sign-off. Claude should not demand that Phase 4 billing details be fixed before testing Windows virtual file paste.
