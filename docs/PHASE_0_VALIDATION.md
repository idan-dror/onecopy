# Phase 0 — Research-risk validation plan

**Status:** PROPOSED experiment plan. **No production promises** should be inferred from target values.

## Purpose and exit statement

Phase 0 determines whether the OneCopy product contract can be implemented truthfully and securely, not whether an attractive UI can be built. Proof must be tied to source revision, environment, reproducible commands and artifacts. Technical spikes that fail are acceptable evidence and may result in scoped product fallbacks.

## Required deliverables

- A versioned desktop platform capability matrix with tested OS/compositor versions.
- Direct-only P2P feasibility results and a reasoned QUIC/iroh choice.
- E2EE device-trust + encrypted-history minimal protocol prototype (not audited for production use).
- Live-source, bounded-memory file stream with crash resume and mutation tests.
- Windows Explorer lazy virtual-file paste result; macOS Finder `Command-V` result and fallback proposal.
- Threat model, parser/path fuzz test findings, data inventory, open security risks.
- Measured performance and network benchmark report; explicitly accepted/rejected platform claims.
- ADRs showing owner decisions, consequences and evidence.

## Risk-first experiment register

| ID | Experiment | Evidence / pass condition |
|---|---|---|
| E-01 | Windows native capture and clipboard writing | Test normal text/image/file capture on interactive user session; avoid feedback loops |
| E-02 | Windows Explorer virtual file paste | COM `IDataObject` + `CFSTR_FILEDESCRIPTOR/FILECONTENTS` can lazily stream; document tested targets and failures |
| E-03 | macOS permissions and Finder paste | Observe permission states; prove `Command-V` works with a remote promise or record robust fallback |
| E-04 | Linux X11 clipboard | Text/image/URI round trip, owner exit/restart and paste UX |
| E-05 | Wayland compositor matrix | GNOME/KDE/wlroots independently tested; detect capability/permission rather than assume |
| E-06 | Direct-only QUIC/iroh | Authenticated LAN/WAN P2P with evidence that Free payload never goes through managed relay |
| E-07 | NAT survey | Connection attempts over consumer NAT/double NAT/CGNAT/IPv6/enterprise/mobile hotspots; measure success by class |
| E-08 | Live-file correctness | Copy only metadata; open current source upon request; detect deletions and incompatible mid-transfer edits |
| E-09 | Crash-safe large transfer | 1/10/100 GB, verified bytes, bounded memory; kill/restart sender/receiver, disk full, resume |
| E-10 | Folder/path safety | Unicode, case collisions, traversal, links, special files, very many small files; no root escape |
| E-11 | Encrypted sync/trust | Blind server can't decrypt; enrollment approval; signed epoch changes; replay/rollback rejection |
| E-12 | Mobile feasibility only | Android Share/content URI jobs and iOS Share/background HTTPS architecture/prototype as resources allow |
| E-13 | Gateway baseline research | Storage and relay pricing/throughput experiments without introducing paid infrastructure into Free path |

### Proposed register refinements (PROPOSED, 2026-11-12 — pending owner acceptance)

| ID | Refinement | Basis |
|---|---|---|
| E-02 | **First implementation-ready experiment.** The full specification is in [`docs/experiments/E-02_WINDOWS_VIRTUAL_FILE_PASTE.md`](experiments/E-02_WINDOWS_VIRTUAL_FILE_PASTE.md). It measures target-side partial-file, cancellation, termination and provenance behavior; it does not assume them. | D-024, D-033 |
| E-06a | Emulated NAT lab, local only. **Two independent tracks:** iroh configured for D-021, and standalone QUIC plus ICE-style traversal. Per NAT class: success, setup p50/p95, and packet-capture plus byte-counter evidence of **zero** Free inter-device packets through any relay. | D-021, D-023 |
| E-06b / E-07 | Field validation and network-class survey. Needs an owner-approved public host (D-028). Results are reported **per network class with confidence intervals**. No universal rate. | D-028, D-034 |
| E-12 | Mobile feasibility, including `BGContinuedProcessingTask` as an experiment and transient-source lifetimes. Blocked until devices are approved. | D-025, D-028 |
| E-13 | **Desk research only** during Phase 0. Measurements move to Premium development. | D-029 |

## Benchmark corpus

```text
text: 1 KiB
image: 1–5 MiB
files: 1 GB, 10 GB, 100 GB (incompressible/generated reproducibly)
folders: 10,000 and 100,000 small files; 10 GB mixed tree
paths: Unicode, RTL, emoji, long paths, collisions, Windows reserved names
security: symlinks, traversal, special files, malicious manifests
mutations: append, overwrite, truncate, replace, delete during stream
```

Generate corpora safely; record checksums and generation seed. Do not use user clipboard data as test fixtures.

## Network and fault matrix

1 GbE LAN, modern Wi-Fi LAN, 100/20 and 20/5 Mbps WAN, 100 ms RTT + 1% loss, IPv6, symmetric/restrictive NAT, blocked UDP, cellular/hotspot and mid-transfer network change.

Fault injection: kill either process, restart control plane, sleep/wake endpoints, disconnect network, full destination disk, expire auth token, revoke peer, duplicate/reorder/replay metadata messages, source file removed/modified/replaced. Every failed outcome has a deterministic error classification.

## Proposed performance targets (from research)

| Metric | Target | Test constraints |
|---|---|---|
| Desktop capture latency | p95 < 100 ms | Supported APIs, idle system and permission granted |
| Remote history delivery | p95 < 500 ms | Both devices online, healthy same-region path |
| LAN direct establishment | p95 < 750 ms | Healthy local network, eligible direct peers |
| Internet direct establishment | p95 < 2 seconds | Compatible non-enterprise networks; measure success separately |
| Direct LAN throughput | ≥80% of measured baseline | Disk not bottleneck, same hardware/network |
| Transfer memory overhead | <256 MiB per active transfer | Constant with file size, configured concurrency |
| Resume overhead | Typically < one chunk + protocol overhead | Unmodified source, verified journal |
| Silent mixed-version completion | Zero in exercised fault suite | Does not imply mathematical impossibility on all filesystems |
| Corrupt completed transfers | Zero in exercised fault suite | End-to-end digest verification |

Targets are hypotheses, not contractual release SLOs. Candidate direct-success goal of ≥90% on non-enterprise consumer networks needs a statistically meaningful real-world trial (research suggests at least 1,000 connection attempts); do not infer from a few local tests.

**Connectivity targets per network class (PROPOSED, D-034 — pending owner acceptance):**

- The universal ≥90% direct-success goal is **withdrawn as unsupported**.
- Connectivity is measured and reported per NAT and network class, with confidence intervals: LAN; home NAT; double NAT; CGNAT; IPv6; enterprise; hotel or captive; hotspot; cellular.
- Per-class targets are proposed only after E-06a and E-07 data exist.
- The no-Free-relay evidence (NFR-013) is binary and mandatory regardless of rates.

**Review note — RESEARCHED context (2026-11-12):** the largest public hole-punching measurement found (libp2p DCUtR in IPFS, 4.4M+ attempts, 85,000+ networks) reports a *conditional* hole-punch success rate of 70% ± 7.1% for attempts that reached the hole-punching stage, with TCP and QUIC statistically indistinguishable (arXiv:2510.27500). It is not an end-to-end connection success rate and not a prediction for OneCopy (corrected 2026-11-12). iroh's own documentation says about 9 in 10 connections go direct. Neither figure is specific to OneCopy's Free direct-only constraint. A revised, per-network-class gate is proposed in `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §7.7 for owner review.

## Go/no-go gates

**Gate A — Clipboard:** Windows reliable, macOS permission behavior acceptable, X11 tested, Wayland matrix accurately scoped. Mobile marketing wording must match limitations.

**Gate B — Native file paste:** Windows Explorer and macOS Finder behavior empirically verified. If Finder `Command-V` cannot be made reliable, owner may approve explicit OneCopy save/download workflow as supported fallback.

**Gate C — Direct path:** QUIC authentication, strict no-Free-managed-relay payload, representative NAT performance and clear failed-path UX. Transport dependency decision recorded.

**Gate D — Transfer:** 100 GB without file-size-proportional memory growth, safe resume, path safety, no falsely successful mixed/corrupt destination in fault suite.

**Gate E — Security:** threat model reviewed; high-risk protocol and filesystem invariants tested; no public beta without independent protocol/security assessment.

**Proposed gate refinements (PROPOSED, 2026-11-12):**

- **Gate B (Windows).** "Go" for native paste of a size or condition class requires E-02 evidence that no false-success outcome occurred for it (outcome classes O3/O4 in the E-02 spec). Partial-file outcomes (O2) are documented and need an owner decision on UX wording. Otherwise that class falls back to managed download.
- **Gate B (macOS).** Finder `Command-V` remains unverified until E-03 runs on approved hardware.
- **Gate C.** Per-class connectivity (D-034) plus binary zero-Free-relay evidence (NFR-013). The transport track decision is recorded in an ADR.
- **Gate E.** Includes the pre-implementation security review items in `SECURITY_AND_PRIVACY.md` (D-037 to D-040).

## Staged equipment and environment plan (PROPOSED, D-028 — pending owner acceptance)

| Stage | Environment | Approval | Experiments enabled |
|---|---|---|---|
| 0 — available now | Existing Windows 11 Home development machine (build 26200). Rust stable MSVC toolchain, Visual Studio 2022 with Windows SDK 10.0.26100, WSL2. A secondary NTFS volume with more than 400 GB free. | None (already available) | E-02, E-01, E-10 (Windows parts), E-06a (logic, in WSL2 network namespaces), E-11 (local blind server) |
| 0b — free local VMs | Linux VMs (GNOME, KDE, Sway) under a free hypervisor. Windows 11 Home has no Hyper-V or Windows Sandbox [K]. | Owner approves installing the hypervisor | E-04, E-05, E-06a (performance), E-08/E-09 (Linux side) |
| 1 — owner's existing devices, if any | Mac, Android 14+ phone, iPhone (iOS 26+) | Owner confirms availability and consent | E-03, E-12 (partial) |
| 2 — paid or new | Apple Developer Program, public VPS for E-06b/E-07, purchased devices, Sysinternals tools if not installed | **Explicit owner approval required; none assumed** | E-03 (extensions, notarization), E-06b, E-07, E-12 |

Experiments whose stage is not approved are reported as **not run (blocked)**, never inferred from documentation.

## Evidence format

For each run: `experiment_id`, repo commit, OS/version/compositor, hardware, selected network, file corpus hash/size, command invocation, environment/permissions, result, latency/throughput/RSS measurements, logs redacted of private content, defect issues and reviewer. Keep raw traces under ignored/sanitized test artifacts; summarize machine-readable and human-readable outcomes.

## Suggested slice order

1. Documentation approval and decision gates.
2. Parallel high-risk spikes E-02, E-03, E-05 and E-06.
3. Minimal Rust identity/encrypted-event prototype and X11/Windows bridge.
4. Transfer E-08 through E-10, with repeatable benchmarks.
5. Threat/fault review, mobile discovery and final decision report.

Scheduling is flexible; a fixed six-week calendar is **not** a release commitment.

**Proposed revised order (PROPOSED, 2026-11-12):**

1. Baseline acceptance.
2. **E-02** (Stage 0).
3. **E-06a** two-track NAT lab (Stage 0/0b).
4. E-04/E-05 Linux matrix (Stage 0b).
5. E-08 to E-10 transfer engine harnesses.
6. E-11 trust prototype and security review inputs.
7. E-03 and E-12 when hardware is approved.
8. E-06b/E-07 when a public host is approved.
9. Decision report.

E-13 is desk research and can run at any time.
