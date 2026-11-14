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

**Free connection-establishment policy (D-021, D-022 / ADR-0001, accepted 2026-11-12):**

- **Allowed for Free:** control-plane signaling, presence, candidate exchange, connection-attempt timing, and STUN-style address discovery.
- **Prohibited for Free:** OneCopy-managed infrastructure forwarding **any** inter-device packet. This covers payload, QUIC handshake and NAT-traversal frames.
- **Third-party application relays, including library defaults, are also prohibited (D-022).** Direct peer-to-peer connectivity is the only Free data path.
- **Evidence (NFR-013):** packet captures and server byte counters show zero forwarded Free inter-device packets.

## Transport experiment

**Primary hypothesis:** QUIC is suitable for authenticated multiplexed file transfers and connection migration. Evaluate Rust `iroh` first because it bundles identity, hole punching and relay capabilities. Its conventional relay-first connection strategy is **not** automatically compatible with strict Free direct-only routing.

Phase 0 must prove which relay network components (discovery, NAT information, temporary coordination) iroh uses, when payload bytes cross a managed relay, whether the policy can disable it completely, and what direct reachability is lost. If it cannot meet requirements cleanly, evaluate a narrow STUN/candidate-exchange mechanism around a standard QUIC implementation. WebRTC/libp2p are secondary comparisons, not committed dependencies.

**Two independent transport tracks (D-023 / ADR-0001, accepted 2026-11-12):**

- **Track 1:** iroh, configured to satisfy D-021 and D-022. Free sessions have no relay transport, and candidates arrive through OneCopy signaling.
- **Track 2:** a standalone QUIC implementation plus ICE-style candidate exchange and connectivity checks.
- Both tracks are evaluated in E-06a against identical evidence: per-class success, setup time, zero Free relayed packets, code and API stability.
- Neither is preferred in advance. This supersedes the "evaluate iroh first" ordering above.

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

## Delivery modes (D-024 / ADR-0002, accepted 2026-11-12)

| Aspect | Managed download (OneCopy writes destination) | Native paste (OS component or app writes destination) |
|---|---|---|
| Partial-file isolation | `.onecopy-partial` plus journal | Controlled by the target; measured in E-02/E-03 |
| Resume after interruption or restart | Journaled verified ranges | Not claimed unless demonstrated for the target |
| End-to-end verification before completion | Yes | OneCopy verifies what it supplies; the target's finalization is not under OneCopy's control |
| Controlled finalization | Atomic rename where supported | Not claimed |
| Cleanup on failure | Explicit policy | Target-dependent; measured |
| Provenance marking | Applied by OneCopy (D-033) | Observed per target |
| Existing destination file (D-044) | Never altered before the new content is complete and verified. The replacement procedure is decided with B-14. | Windows Explorer overwrote the existing file in place and deleted it after a stream failure (E-02 B1 F2). **Native-paste replacement is unsupported for release** until a safe mechanism is demonstrated. |

Rules:

- **No clean end of stream for inconsistent data.** For native paste, OneCopy never returns a clean end of stream for data that failed source-consistency validation. The sender's final consistency result (for example a trailer after post-read revalidation) must arrive before the last bytes are released.
- **Exact length or failure (D-043, owner decision 2026-11-14).** Each natively pasted file advertises an explicit size. The provider delivers exactly that many bytes, or returns a failure to the target. It never signals end of stream early for an incomplete transfer (on Windows, never `S_FALSE` before the advertised size). Shrinkage, unexpected growth and detectable incompatible modification of the source are failures. The final source-consistency check runs before the bytes that complete the file are released, and again before end of stream. Evidence: E-02 B1 finding F1 (Explorer accepted an early end of stream as a completed copy).
- **Existing destination files (D-044, owner decision 2026-11-14).** An interrupted, failed or cancelled OneCopy transfer must never destroy an existing destination file, in any delivery mode. Unrestricted native-paste overwrite is not approved. "Small file", "local source" and "high expected success probability" are not mitigations. The enforcing mechanism is OPEN (B-14).
- **The trailer does not fix the target.** It governs only OneCopy's own end-of-stream and error signal. It does **not** determine what the target does with a partially written file. That behavior is measured, not assumed.
- **Fallback.** Managed download is offered whenever native paste is unavailable, unverified for the condition, or beyond evidence-based limits (PR-030).

## Resume and durability

*Scope note (D-024): this section specifies **managed download**.*

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

**Boundary between transfer-time validation and prohibited behavior (D-026, D-042 / ADR-0003, accepted 2026-11-12):**

- **Prohibited:**
  - reading, hashing, copying or uploading source bytes at copy time;
  - idle or background watching of source files;
  - retained snapshots, versions or backups;
  - comparing the current source with the copy event.
- **Required during an active transfer or staging operation, on the opened source:**
  - identity and change indicators at open (file ID or inode, size, mtime, POSIX ctime, Windows USN where available);
  - re-validation before completion;
  - fail and retry on incompatible change;
  - no stitching of ranges from different source states.
- **Allowed, scoped to the opened file and ending with the transfer:** OS change notifications on that file. This is transfer-time validation, not monitoring.
- **Decided (D-042):** sources are **not** locked against concurrent modification by default (Windows sources are opened with write sharing allowed). No transfer-scoped copy-on-write clones or hidden source snapshots. Detectable incompatible modifications are reported, and residual races are documented.

**Detection is not guaranteed.** Stat and identity checks can miss concurrent modifications [K]:

- coarse or lazily updated timestamps;
- writers that restore the modification time;
- memory-mapped writers that update metadata late;
- same-size in-place overwrites within timestamp granularity;
- client caching on network filesystems;
- cloud placeholders that hydrate on open.

OneCopy claims detection **in tested scenarios** (E-08) and documents residual races per filesystem. It never claims universal detection.

## Path safety and names

Reject absolute paths, `..`, embedded NULs, Windows device names/alternate data streams, separator trickery, Unicode normalization and case-insensitive collisions, path length overflows and special filesystem nodes. A destination root is chosen/authorized locally; never accept a remote-provided arbitrary destination. File handles should be bound to original source selection/capability; do not offer `read(path)` RPC.

## Routing metrics

Measure real RTT, throughput sample, loss/retransmission, network type, metered/battery state, failure rates and route cost. Proposed score combines predicted completion time, reliability, power and monetary cost; a suggested 25% hysteresis threshold is an experiment parameter, **not** a frozen algorithm.

**Deferral (D-035, accepted):**

- Phase 1 Free routing is measured direct-path selection: prefer the best measured direct path (for example via QUIC path validation and RTT), filtered by entitlement.
- The scoring formula and hysteresis are deferred until a Premium relay exists and measurements show a benefit.
- Metrics collection continues so that the benefit can be measured.

## Additional destination safety items (PROPOSED, review §3.7)

Detect and handle each of the following before or during a managed download:

- the FAT32 4 GiB file-size limit;
- insufficient free space;
- Windows trailing dots and spaces;
- 8.3 short-name aliases;
- alternate-data-stream syntax (`:`);
- macOS NFD normalization;
- volume-specific case folding.

Never preserve setuid or setgid bits.

Durability calls are specified per OS, because `fsync` semantics differ (for example macOS `F_FULLFSYNC`) [K].

## Benchmarks and pass conditions

- 1/10/100 GB incompressible files; many small files; mixed folders; mutable files.
- Direct LAN/WAN, double NAT/CGNAT, IPv6, UDP-blocked, variable latency/loss, network handoff.
- Kill/restart both endpoints, disconnect/reconnect, sleep/wake, disk full, source modified/truncated/deleted/replaced, permission revoked.
- Assert no unsupported managed payload relay in Free; zero silent corruption; bounded RSS; verified restore and resume; measurable throughput baseline.
- *(D-021, D-022, D-034):* assert zero Free inter-device packets through any relay. Report connectivity per NAT and network class with confidence intervals, never as a single universal rate.

See `PHASE_0_VALIDATION.md`. Proposed 80% baseline LAN throughput, <256 MiB per active transfer, and <one chunk typical resume retransmission remain targets subject to test.
