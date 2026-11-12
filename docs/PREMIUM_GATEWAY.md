# Premium gateway — Relay and temporary offline staging

**Status:** Free/Premium separation CONFIRMED. Provider, quotas, pricing and implementation are PROPOSED.

## Product and financial boundary

The OneCopy-managed gateway is a paid reliability feature, **not a compulsory transfer path**. Free users receive small encrypted-history sync, account/device signaling and unlimited-size *direct* P2P. They do not consume OneCopy-managed file-payload relay or staging. Do not silently relay a free transfer because a library uses relay-first connectivity by default.

Premium consists of **two independent capabilities**:

1. **Live relay:** passes encrypted bytes between online peers when direct connection fails or proves materially inferior. Does not persist files.
2. **Keep Available Offline:** explicit, temporary, client-side encrypted storage of a source's point-in-time bytes to allow enrolled-device retrieval while the original source is offline.

Neither capability changes the default meaning of a normal Copy.

## Live relay (future)

- Authenticate account entitlement/device membership and approve each relay session.
- Relay only opaque ciphertext; no server-side file parsing or content encryption keys.
- Meter outbound bytes, sessions, concurrency, errors, regions and latency.
- Select based on measured route performance; do not claim relay inherently faster than P2P.
- Enforce account quotas, abuse controls, hard-stop/consent on exhausted allowance; avoid surprise metered charges.
- Favor cost-conscious bandwidth hosting for relays; capacity and regional deployment must follow actual demand and benchmark evidence.

## Temporary offline stage

```text
NONE → QUEUED → UPLOADING → VERIFYING → READY
                                   ↘ FAILED
READY → EXPIRED / DELETED
READY → user-initiated REFRESH → UPLOADING → VERIFYING → READY
```

- Only an authorized Premium user can request staging, and the user must explicitly choose it.
- Origin opens the source at staging time, encrypts and uploads chunks; source changes during the upload must be handled by the same consistency rules as normal transfer.
- A verified frozen copy is identified by its staging date/time, manifest, expiry, encrypted size and key context.
- Do not show `READY` until all encrypted objects and the manifest are durable and verified.
- This is **not** version history: at most one current staged copy is presented; refresh replaces it under an atomic safe procedure.
- Source edits never trigger automatic staging refresh. Normal online transfer continues to use the current live source.
- Staging is private to trusted enrolled devices; no public anonymous download links or generic cloud drive UX.
- Stage expiration/deletion is mandatory; handle quota exhaustion and mid-upload cancellation safely.

## Storage economics and provider comparison

The initial OneCopy research (as of 2026-11-12) identifies Cloudflare R2 Standard as a good **benchmark candidate**, not an approved provider, with published storage around **$0.015/GB-month**, class A/B request charges and no direct Internet egress fee. Backblaze B2, AWS S3, Wasabi and GCP object storage were compared; short-lived/high-churn downloads make provider egress and minimum-retention conditions central to the decision. Prices are point-in-time and must be revalidated prior to procurement.

Example from research: 500 GB uploaded over a month with average one-day retention is about 16.7 GB-month average stored capacity; at $0.015/GB-month, R2 storage cost is approximately $0.25 **before request/other gateway costs**. This is an illustration, not an all-in per-user cost forecast.

A relay's outbound byte charges, infrastructure throughput and connection overhead are distinct from object-storage billing. Benchmark real per-region throughput and effective billed cost before choosing topology.

**Review addendum — RESEARCHED facts (2026-11-12), no decision change:**

- [V] Cloudflare R2 Standard pricing was re-checked on 2026-11-12 and is unchanged: $0.015/GB-month storage, Class A $4.50/million, Class B $0.36/million, no egress charge. The free tier is 10 GB-month storage, 1M Class A and 10M Class B operations per month.
- [S] The research report's Hetzner figure (20 TB included traffic) applies to EU cloud locations. Secondary sources report about 1–5 TB included in US locations (reduced December 2024), and about 0.5–8 TB in Singapore with roughly €7.40/TB overage, about seven times the EU/US overage rate. Verify on Hetzner's own pages before any relay-region costing.

## Subscription policy (PROPOSED)

- Premium monthly relay allowance with sensible hard limits.
- Separate staged GB-hours/GB-days, concurrent staged-byte cap, object count, maximum retention (e.g. 7 or 30 days — undecided), and transfer concurrency.
- User-facing quota/progress/expiry warnings; explicit consent before potential overage if any metering model is introduced.
- No generic "unlimited gateway" claim until costs, throttling and abuse are proven.
- Entitlement requests and gateway actions must be authorization-checked even if the UI hides Premium controls.

## Security and abuse considerations

- Encrypt content client-side; relay sees session metadata only, staging provider sees ciphertext/object sizes.
- Opaque unpredictable IDs, signed scoped authorization, account/device-bound download tokens and expiry.
- Rate-limit account/device operations and monitor abnormal file churn, malformed requests and unauthorized access attempts.
- Confirm deletion semantics, lifecycle policy and regional data processing; avoid overpromising secure erasure on storage-provider internals.

## Relationship to Free connectivity and mobile local staging (PROPOSED, 2026-11-12)

- **Free (D-021).** Free connections use signaling, presence, candidate exchange and STUN-style address discovery. The Premium relay is the only planned OneCopy service that forwards inter-device packets. It must be unreachable by Free sessions by construction, not merely by policy check.
- **Free quotas (D-031).** Free history and control traffic have published rate and storage quotas. Free P2P file size is not limited.
- **Mobile local staging (D-025).** The explicit, on-device copy of a transient mobile source is a different feature from Premium "Keep Available Offline". It never leaves the device except through a normal transfer to a trusted device, and the two must never share UI labels.

## Go/no-go experiment

*Phase 0 scope (PROPOSED, D-029):* during Phase 0, E-13 is desk research only (pricing, limits, terms). The measurements below are performed in Premium development (Phase 4 preparation), after explicit owner approval of any paid resources.

Measure R2 and at least one alternative across target regions using 1/10/100 GB encrypted payloads, concurrent clients, failure/retry and rapid expiry, actual item request counts and effective costs. Separately benchmark live relay hosts for throughput, cost/TB, packet loss, 95th percentile latency, concurrency and abuse resilience. Premium cannot be priced responsibly until these results exist.

## Deferred design decisions

Storage region(s), data retention window, quota tiers, gateway relay topology, provider, billing vendor and free-to-paid UX. None is approved by this document.
