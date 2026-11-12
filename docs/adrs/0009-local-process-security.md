# ADR 0009: Local process security — IPC authorization, single-writer cryptographic state, crash dumps

Status: Proposed
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-038, D-039, D-040 (PROPOSED). Related: NFR-002; `ARCHITECTURE.md` §Client processes

## Context and evidence

- **Local IPC.** The desktop agent holds history from *other* devices. An unauthenticated local endpoint would let any local process, sandboxed app or other OS user query it (review DA-08).
- **Shared cryptographic state.** Some state must never be advanced by two writers at once: per-sender sequence counters, nonce derivation inputs, MLS secret-tree ratchets (if MLS is chosen), epoch state. A desktop agent plus UI process, or an iOS/Android app plus its extensions, could otherwise reuse nonces or fork state (review §3.5).
- **Crash dumps** (Windows Error Reporting full dumps, custom minidumps, Android tombstones) can contain heap memory holding clipboard bodies and keys [K]. NFR-002 covers only logs and telemetry.

## Decision (proposed)

1. **IPC authorization (D-038).** Every local endpoint authenticates and authorizes its caller:
   - Windows: a named pipe whose ACL allows only the current user SID, plus verification of the client process image where feasible;
   - macOS: XPC with a code-signing requirement;
   - Linux: a Unix-domain socket in a user-private directory with `SO_PEERCRED` checks.

   The exact mechanism per OS is set in the Phase 1 process-model ADR. The UI may also run inside the agent process, which removes the IPC endpoint entirely (review AR-07).
2. **Single writer (D-039).** Exactly one process per device mutates cryptographic state: the desktop agent, or the mobile main app. Extensions and secondary processes enqueue requests through an app-group or IPC queue and never encrypt with shared counters or ratchets.
3. **Crash and diagnostics hygiene (D-040).**
   - No full-memory dumps.
   - Minidumps exclude heap, or are scrubbed.
   - Crash reporting is opt-in.
   - Diagnostic bundles contain no clipboard bodies, filenames, paths, keys or raw content hashes.
   - Secrets are zeroized after use where the language permits.

## Alternatives considered

- **Trust same-user processes.** Same-user malware is already out of scope, but sandboxed apps and other OS users are not.
- **Multi-writer state with locks.** Fragile across process crashes and OS suspension.

## Security/privacy and interoperability effects

- Reduces local exposure and prevents a class of catastrophic nonce reuse.
- Crash diagnosability is reduced, which is an accepted trade-off.

## User/product effects

- Share extensions may complete their work only after the main app runs (mobile).

## Consequences and follow-up tests

- E-11 adds tests for concurrent writers and an unauthorized-IPC probe.
- The process-model ADR precedes Phase 1.

## Acceptance record (who/when)

Pending. No acceptance recorded.
