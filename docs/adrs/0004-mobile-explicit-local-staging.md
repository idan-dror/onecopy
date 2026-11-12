# ADR 0004: Explicit, bounded mobile local staging for transient sources

Status: Proposed
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-025 (PROPOSED). Related: D-005, PR-008, PR-010 (CONFIRMED); D-009, PR-017 (Premium staging, CONFIRMED)

## Context and evidence

Some mobile sources are only temporarily accessible [K]:

- On Android, a URI received through `ACTION_SEND` carries a temporary permission grant.
- On iOS, a share extension receives items whose access is tied to the extension's lifetime.

For such sources a durable "live reference" usually cannot exist (review DA-05). D-005 forbids automatic duplication for a normal Copy.

## Decision (proposed)

1. When the source permission is transient, the user may **explicitly** choose "Keep on this device until sent" (working label). OneCopy then copies the content into its own app container.
2. This mobile local staging is:
   - **never automatic**;
   - **bounded** by a per-device storage quota;
   - **expiring** at a stated time;
   - deletable by the user at any time;
   - encrypted at rest with device-protected keys.
3. It is **distinct from Premium cloud staging** (D-009): nothing leaves the device until a normal transfer to a trusted device is requested. The two are never presented as the same feature.
4. Without local staging, a transient-source item shows "available while the share is open" and later "source no longer available".

## Alternatives considered

- **Always copy on share.** Violates the "never automatic" direction.
- **Never copy.** Mobile → desktop sharing becomes unreliable.
- **Upload to the cloud.** That is the Premium feature; it is not acceptable as a Free default.

## Security/privacy and interoperability effects

- Local copies extend the plaintext lifetime on the device. Expiry and deletion must be enforced, and the copy must use the app's protected storage (iOS Data Protection class; Android credential- or device-encrypted storage, chosen deliberately).
- Local copies must not appear in OS backups unless the owner decides otherwise (OPEN detail).

## User/product effects

- One extra explicit choice when sharing large content from a phone.
- The UI shows the expiry time and the quota.

## Consequences and follow-up tests

- E-12 measures grant and extension lifetimes and staging behavior.
- Still to be decided: quota value, expiry default, and whether a copy is deleted after the first successful send or only on expiry.

## Acceptance record (who/when)

Pending. No acceptance recorded.
