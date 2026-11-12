# ADR 0006: Inline content cap, on-demand content references and Free control-plane quotas

Status: Proposed
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-030, D-031 (PROPOSED). Related: PR-005, PR-007, PR-013 (CONFIRMED); README invariant #5

## Context and evidence

The baseline promises Free users "small encrypted-history sync" without defining "small" (review DA-02). The Phase 0 corpus already includes 1–5 MiB images. Without a bound:

- the control plane stores and forwards arbitrarily large Free content;
- the history channel can be used as a covert free relay (review DA-23).

## Decision (proposed)

1. **Inline cap (D-030).**
   - Each history envelope carries at most **256 KiB of representation ciphertext per item**. This is an **experimental starting point** to be tuned in E-11 with measured size distributions, not a final value.
   - The cap covers all inline representations of the item combined.
2. **On-demand content references.**
   - A text or image representation whose ciphertext would exceed the cap is replaced by a `content_ref`: media type, size hint, optional bounded preview (omitted for sensitive items), and a source capability.
   - A receiver fetches the bytes from the origin over the same authenticated direct path used for files, when the user selects the item or promotes it.
   - The bytes come from the origin's own local encrypted copy of the captured clipboard data. That copy is the clipboard content itself, not a snapshot of a file, and its retention follows history eviction.
3. **Free control-plane quotas (D-031).**
   - Server-enforced limits: history items per minute, inline bytes per day, retained envelopes per account (bounded by the history window plus tombstones), signaling messages per minute, address-discovery requests per minute.
   - On a breach, sync is deferred with an explicit status. Content is never silently dropped.
   - Values are set from measurement and published.
   - **No product-imposed P2P file-size limit** (PR-013 is unchanged).

## Alternatives considered

- **No cap.** Unbounded cost and abuse.
- **A very small cap** (for example 16 KiB). Most screenshots would need the origin online.
- **Upload large items to object storage.** That is Premium staging, so it is not acceptable for Free.

## Security/privacy and interoperability effects

- Previews leak less than full content but are still content. They are disabled for sensitive items.
- `content_ref` fetches use the same capability and roster checks as files (NFR-004).

## User/product effects

- A large text or image copied on an offline device is listed but not retrievable until that device is online again. This must be shown explicitly.
- **Mobile origins (correction, 2026-11-12):** a large item copied on a phone is usually unretrievable while the OneCopy app there is suspended or not running (iOS suspension; Android process lifecycle). Receivers show the `ORIGIN_NOT_RUNNING` state ("Open OneCopy on [device]"). No automatic upload or prefetch works around it.

## Consequences and follow-up tests

- E-11 measures envelope sizes and verifies conversion at the cap, quota enforcement and deferral states.
- `DATA_MODEL_AND_PROTOCOL.md` defines `content_ref`.

## Acceptance record (who/when)

Pending. No acceptance recorded.
