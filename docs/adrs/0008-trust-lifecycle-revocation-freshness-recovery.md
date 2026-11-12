# ADR 0008: Trust lifecycle — revocation authority, roster freshness, offline LAN and recovery

Status: Proposed — **options only; decision OPEN (D-037)**
Date: 2026-11-12
Owner: Product owner, with external security review
Decision IDs: D-037 (OPEN). Related: PR-021, PR-026, UJ-05, B-01, B-02, NFR-001

## Context and evidence

The baseline specifies signed membership changes and a monotonic epoch, but four policies are missing:

- **who may revoke whom.** A stolen device must not be able to lock the owner out by revoking every other device.
- **how fresh a device's roster must be** before it serves or accepts a transfer, in particular on a LAN while the control plane is unreachable.
- **how revocation reaches offline devices.**
- **how recovery works** when all devices are lost (B-02).

The owner's direction is to bring all four into the pre-implementation security review and **not** to choose a freshness timeout without a documented trade-off.

## Options (no decision)

**Revocation authority**

| Option | Strength | Cost |
|---|---|---|
| R1. Any trusted device may revoke any other | Fast response to theft | A stolen device can revoke all others (lockout) unless R4 exists |
| R2. Revocation needs a trusted device plus fresh account re-authentication (passkey) | Raises the bar for a thief | Fails if the account credential is also compromised; needs connectivity |
| R3. Quorum (k of n devices) | Strong against a single compromised device | Unusable for one- or two-device users |
| R4. Recovery key can override, re-establishing trust and revoking the rogue device | Ends lockout | Requires the user to keep the recovery key |

**Roster freshness for authorizing transfers**

| Option | Security effect | Usability effect |
|---|---|---|
| F1. Online roster confirmation required for every transfer | Revoked device refused immediately | No transfers while the control plane is unreachable, including LAN |
| F2. Bounded staleness window *T* | Revoked device may be served for up to *T* on LANs without Internet | Larger *T* means more offline usefulness and a longer exposure window |
| F3. Peers must agree on roster version, plus a bound | Detects a peer holding an older roster | Same trade-off as F2 when both are stale |
| F4. LAN gossip of signed revocations | Shrinks exposure when any updated device is present | More protocol surface |

Inputs needed before choosing *T*:

- realistic offline durations and LAN-only scenarios;
- the threat scenario "laptop stolen and revoked while the user's other devices have no Internet";
- whether exposure is limited to *new* transfers (it is: history already on the stolen device cannot be recalled, per UJ-05).

**Recovery (B-02):** a recovery key, approval from an existing device, or explicitly disclosed server-assisted escrow (which weakens E2EE). The current baseline already rejects email-only recovery (PR-026).

## Decision

None. Remains OPEN until the pre-implementation security review. E-11 prototypes the mechanics (signed roster log, version pinning, revocation propagation) behind interfaces so that any option can be evaluated.

## Security/privacy and interoperability effects

Decisive for UJ-05 and for the offline LAN threat model. The selected design must be reviewed externally before beta (Gate E).

## User/product effects

Determines lockout risk, offline usability and recovery UX.

## Consequences and follow-up tests

- E-11 property tests: monotonic roster, rollback rejection, revoked-device denial under each freshness option, and lockout scenarios.

## Acceptance record (who/when)

Pending. Decision OPEN.
