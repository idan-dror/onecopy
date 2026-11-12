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

**Review note — RESEARCHED context (2026-11-12), target unchanged:** the largest public hole-punching measurement found (libp2p DCUtR in IPFS, 4.4M+ attempts, 85,000+ networks) reports 70% ± 7.1% hole-punch success, with TCP and QUIC statistically indistinguishable (arXiv:2510.27500). iroh's own documentation says about 9 in 10 connections go direct. Neither figure is specific to OneCopy's Free direct-only constraint. A revised, per-network-class gate is proposed in `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §7.7 for owner review.

## Go/no-go gates

**Gate A — Clipboard:** Windows reliable, macOS permission behavior acceptable, X11 tested, Wayland matrix accurately scoped. Mobile marketing wording must match limitations.

**Gate B — Native file paste:** Windows Explorer and macOS Finder behavior empirically verified. If Finder `Command-V` cannot be made reliable, owner may approve explicit OneCopy save/download workflow as supported fallback.

**Gate C — Direct path:** QUIC authentication, strict no-Free-managed-relay payload, representative NAT performance and clear failed-path UX. Transport dependency decision recorded.

**Gate D — Transfer:** 100 GB without file-size-proportional memory growth, safe resume, path safety, no falsely successful mixed/corrupt destination in fault suite.

**Gate E — Security:** threat model reviewed; high-risk protocol and filesystem invariants tested; no public beta without independent protocol/security assessment.

## Evidence format

For each run: `experiment_id`, repo commit, OS/version/compositor, hardware, selected network, file corpus hash/size, command invocation, environment/permissions, result, latency/throughput/RSS measurements, logs redacted of private content, defect issues and reviewer. Keep raw traces under ignored/sanitized test artifacts; summarize machine-readable and human-readable outcomes.

## Suggested slice order

1. Documentation approval and decision gates.
2. Parallel high-risk spikes E-02, E-03, E-05 and E-06.
3. Minimal Rust identity/encrypted-event prototype and X11/Windows bridge.
4. Transfer E-08 through E-10, with repeatable benchmarks.
5. Threat/fault review, mobile discovery and final decision report.

Scheduling is flexible; a fixed six-week calendar is **not** a release commitment.
