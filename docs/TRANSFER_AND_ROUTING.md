# Transfer engine, live-file semantics and route selection

**Status:** CONFIRMED product semantics. QUIC recommended; `iroh`, chunk sizing and detailed mechanisms remain PROPOSED until proven.

## Required behavior

A standard Copy creates and syncs a live source reference. It **does not** trigger a large read, upload, snapshot, backup, version history or change watcher. At actual transfer request, source opens the then-current file and begins streaming. This naturally means copied metadata (e.g. displayed size) may differ from transfer-time reality; show actual transfer manifest information when available.

If source cannot be reopened, return unavailable. If it changes incompatibly during a transfer, fail and offer retry. No silent mixed-version completion.

## Network paths and entitlements

```text
Free: candidate exchange/signaling + IPv6/LAN/WAN direct P2P only
Premium: same direct paths + authorized managed live relay fallback
Premium staging: explicit temporary E2EE object copy, separate from relay
```

The control plane may pay small costs for login, encrypted event sync and connection signaling for Free users. That does **not** authorize sending Free file payload bytes through a OneCopy-managed relay. A peer may be unreachable under restrictive NAT/UDP environments; tell the user rather than silently crossing a paywall boundary.

## Transport experiment

**Primary hypothesis:** QUIC is suitable for authenticated multiplexed file transfers and connection migration. Evaluate Rust `iroh` first because it bundles identity, hole punching and relay capabilities. Its conventional relay-first connection strategy is **not** automatically compatible with strict Free direct-only routing.

Phase 0 must prove which relay network components (discovery, NAT information, temporary coordination) iroh uses, when payload bytes cross a managed relay, whether the policy can disable it completely, and what direct reachability is lost. If it cannot meet requirements cleanly, evaluate a narrow STUN/candidate-exchange mechanism around a standard QUIC implementation. WebRTC/libp2p are secondary comparisons, not committed dependencies.

**Review addendum — RESEARCHED facts (2026-11-12), no decision change.** From iroh's own documentation [V]:

- iroh 1.0 was released 2026-06-15 and promises wire compatibility across v1 releases. It includes iroh's own QUIC multipath and QUIC NAT traversal, and official Swift/Kotlin bindings.
- The relay is the coordination point for hole punching ("how two devices first find each other and coordinate the simultaneous connection attempt"). Communication starts on the relay path before hole punching completes.
- `RelayMode::Disabled` is supported but removes relay fallback, peer introduction and hole-punch coordination. iroh estimates roughly one connection in ten stops working without a relay.
- By default iroh uses four public relays operated by number 0. They are rate-limited and described as free for development and testing. Relays see endpoint IP addresses, which endpoints communicate, and relayed volume, but not content. The relay software is open source and self-hostable.

An independent measurement of libp2p DCUtR in the IPFS network (4.4M+ attempts) reports a *conditional* hole-punch success rate of 70% ± 7.1% [V, arXiv:2510.27500]. The rate counts only attempts that reached the hole-punching stage. TCP and QUIC were statistically indistinguishable, and success was independent of relay characteristics. This is not an end-to-end connection success rate and not a prediction of OneCopy's success rate (corrected 2026-11-12). Implications and options are proposals in `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §3.7 and §5 (AR-01, AR-02); they require owner decisions.

## Direct path candidate process (PROPOSED)

1. Discover local network interfaces and IPv4/IPv6 reachability.
2. Gather LAN addresses and relevant Internet candidates using STUN-like mapping and existing OS facilities.
3. Exchange candidate information through authenticated encrypted control messages.
4. Probe simultaneous direct connections; validate peer identity and current account trust/epoch.
5. Prefer actual fastest reliable path, not merely the route labeled LAN. Maintain path-quality data.
6. For Premium only, evaluate authorized relay if no viable direct route or if measured route is materially worse.
7. Switch paths conservatively; protect against thrashing and accidental duplicate chunk commits.

No automatic gateway staging is permitted as a transfer-optimization trick.

## File and folder streaming

Use folder manifests rather than implicit ZIP creation. Entries are relative paths with file/directory types and safe metadata. Default: **do not follow symlinks**; special files (sockets, FIFOs, devices) not accepted. Enumerate lazily where possible; bound file counts, depth and memory use and protect against symlink swaps/time-of-check-to-time-of-use.

Proposed chunk size range **4–16 MiB**, with 4 MiB as initial benchmark. Do not prehash a 100 GB file at Copy time. Compute per-chunk digests while reading, carry an authenticated manifest/final digest, use QUIC flow control and bounded in-flight chunks. Multiple QUIC streams may separate control from payload, subject to measured benefits.

## Resume and durability

Destination uses a `.onecopy-partial` data file and a local checkpoint journal tracking manifest identity and verified chunk ranges. On restart, validate integrity and resubmit missing ranges. Checkpoint ordering must avoid acknowledging data not durably written; a crash simulator should exercise writes interrupted before and after journal commits. Never mark final filename complete before final manifest checks and atomic commit/rename where filesystem supports it.

Disk full, source loss, destination name collisions, cancellation and peer revocation should preserve or clean up partials according to explicit policy. Memory consumption must be bounded independently of total transfer size.

## Live source changes

There is no portable guarantee of an immutable filesystem view during arbitrary concurrent writes. Implement the strongest portable *detect-and-fail* contract possible:

- On opening source, record available file identity, size, mtime/change token and handle continuity.
- Read chunks from the opened object while monitoring for detectable changes.
- Before marking transfer complete, revalidate identity/size/change indicators and manifest compatibility.
- If incompatible change is detected, reject the completion and offer restart from a newly opened current source.
- Chunk digests prove transmission fidelity, **not** that the source was immutable for the entire read. Document residual races and limitations.
- A resumed attempt must not mix ranges from different incompatible source states.

A stronger snapshot-based semantic is explicitly out of scope for normal Copy.

## Path safety and names

Reject absolute paths, `..`, embedded NULs, Windows device names/alternate data streams, separator trickery, Unicode normalization and case-insensitive collisions, path length overflows and special filesystem nodes. A destination root is chosen/authorized locally; never accept a remote-provided arbitrary destination. File handles should be bound to original source selection/capability; do not offer `read(path)` RPC.

## Routing metrics

Measure real RTT, throughput sample, loss/retransmission, network type, metered/battery state, failure rates and route cost. Proposed score combines predicted completion time, reliability, power and monetary cost; a suggested 25% hysteresis threshold is an experiment parameter, **not** a frozen algorithm.

## Benchmarks and pass conditions

- 1/10/100 GB incompressible files; many small files; mixed folders; mutable files.
- Direct LAN/WAN, double NAT/CGNAT, IPv6, UDP-blocked, variable latency/loss, network handoff.
- Kill/restart both endpoints, disconnect/reconnect, sleep/wake, disk full, source modified/truncated/deleted/replaced, permission revoked.
- Assert no unsupported managed payload relay in Free; zero silent corruption; bounded RSS; verified restore and resume; measurable throughput baseline.

See `PHASE_0_VALIDATION.md`. Proposed 80% baseline LAN throughput, <256 MiB per active transfer, and <one chunk typical resume retransmission remain targets subject to test.
