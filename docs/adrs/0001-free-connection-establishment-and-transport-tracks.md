# ADR 0001: Free connection establishment and two-track transport evaluation

Status: Accepted (Phase 0 baseline, 2026-11-12)
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-021, D-022, D-023 (CONFIRMED)

## Context and evidence

D-008 and PR-015 (both CONFIRMED) say that Free users never send file payloads through OneCopy-managed relays. The baseline does not define what a Free connection *may* use to get established.

iroh's documentation states the following [V, accessed 2026-11-12]:

- The relay coordinates hole punching.
- Communication starts on the relay path.
- Disabling the relay removes relay fallback, peer introduction and hole-punch coordination.
- By default iroh uses public relays run by its vendor, which are rate-limited and intended for development and testing.

ICE-style traversal needs only signaling and STUN-style address discovery; peers send their connectivity checks to each other directly [K].

An independent measurement of libp2p DCUtR in IPFS reports a *conditional* hole-punch success rate of 70% ± 7.1% (attempts that reached the hole-punching stage), independent of relay characteristics. That is not an end-to-end rate and not a OneCopy prediction. See `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §3.7.

## Decision

1. **Free connection establishment (D-021).**
   - **Allowed:** control-plane signaling (authentication, roster, presence, candidate exchange, connection-attempt timing) and STUN-style address discovery.
   - **Prohibited:** OneCopy-managed infrastructure must not forward any inter-device packet for Free users. This covers payload, QUIC handshake and NAT-traversal frames alike.
2. **Third-party relays (D-022, owner decision 2026-11-12).** Free connections must not forward inter-device packets through OneCopy-managed **or third-party application relays, including library defaults**. Control-plane signaling and STUN-style address discovery remain allowed. **Direct peer-to-peer connectivity is the only Free data path.**
3. **Two independent transport tracks (D-023).**
   - **Track 1:** iroh, configured so that Free sessions have no relay transport (or only an address-discovery function that forwards no inter-device packets).
   - **Track 2:** a standalone QUIC implementation with ICE-style candidate exchange over OneCopy signaling.
   - Both tracks are judged against the same evidence. Neither is preferred in advance.
4. **Premium relay.** A Premium relay remains a separate, entitled capability. It is not part of this ADR.

## Alternatives considered

- **(a) A capped coordination relay** that carries only handshake and traversal frames for Free. Rejected by owner direction: a OneCopy server would still forward Free packets, and the boundary is harder to audit.
- **(b) iroh with default relays.** Violates D-008 and the "library defaults" rule.
- **(c) LAN-only Free.** Too restrictive and not requested.
- **(d) WebRTC data channels or libp2p.** Kept as secondary references; libp2p's DCUtR also uses relays for coordination.

## Security/privacy and interoperability effects

- Signaling and address discovery reveal device IP addresses and timing to the control plane. Direct connections reveal IP addresses to the peer. Both belong in the privacy inventory.
- No relay operator, OneCopy or third party, sees Free connection metadata, because D-022 prohibits third-party relays too.
- Transport choice must preserve the binding: authenticated transport key → device ID → roster epoch.

## User/product effects

- Some Free Internet connections will fail on restrictive networks: symmetric NAT on both sides, UDP blocked, Wi-Fi client isolation.
- Failure UX must be honest and classify the cause where possible. LAN behavior is unaffected except where the OS or network blocks it.

## Consequences and follow-up tests

- **E-06a** (emulated NAT lab, both tracks) must produce, for each network class:
  - success rate;
  - setup time, p50 and p95;
  - packet-capture and byte-counter evidence that **zero** Free inter-device packets traverse any OneCopy or third-party relay.
- **E-06b and E-07** (field trial) require an owner-approved public host (D-028).
- The transport ADR that selects a track is written after E-06a. With this ADR accepted, D-012's "iroh first" ordering is superseded.

## Acceptance record (who/when)

Accepted by the product owner, with an engineering teammate, on 2026-11-12 (PR #1), as part of the Phase 0 baseline. Scope: product direction and Phase 0 feasibility experiments. This acceptance does not approve unverified production architectures, future security-protocol choices or Premium infrastructure. Items this ADR leaves unspecified or OPEN remain open.
