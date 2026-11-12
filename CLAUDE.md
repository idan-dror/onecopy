# CLAUDE.md — OneCopy implementation contract

**Status:** Proposed governance policy; operative once approved by the product owner.

## Roles and authority

- **Product owner:** decides scope, UX, business boundaries and accepts architecture-affecting decisions and merges.
- **ChatGPT:** architecture/specification partner and independent reviewer; recommendations are not approvals.
- **Claude:** implementation engineer. Design alternatives, test evidence and focused PRs are encouraged; **unilateral changes to confirmed architecture or product rules are prohibited**.

## Before any implementation

1. Read `README.md`, `docs/PRODUCT_VISION.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY_AND_PRIVACY.md`, `docs/PHASE_0_VALIDATION.md`, `docs/DECISIONS.md`, and `docs/APPROVAL_NEEDED.md`.
2. Report ambiguous/contradictory requirements with file/section references. Do not infer approval from research notes.
3. Produce a short slice plan containing scope, public interfaces, security risks, tests, OS matrix, acceptance criteria and expected artifacts.
4. Request explicit review for unresolved decisions marked **OPEN**, and for any change to a **CONFIRMED** rule.
5. Obtain owner approval of the documentation baseline before feature implementation.

## Engineering invariants

- Each device runs a local Rust core; native adapters are responsible for OS clipboard, permissions, filesystem access, lifecycle and UI.
- OneCopy history and active OS clipboard are distinct. Never automatically overwrite remote devices' OS clipboard on history arrival.
- No browser, IDE or per-third-party-application add-on architecture.
- Normal file copy syncs only a live reference/metadata. Do not stage bytes, hash a huge file, create a backup, watch changes or cache full content at copy time.
- Source contents are read on transfer request. If incompatible modification is detected mid-transfer, abort/retry safely; never silently return mixed-version bytes.
- Do not follow source folder symlinks by default; reject dangerous destination paths and name collisions; remote peers never send arbitrary read-path RPCs.
- Transfer streams must have bounded memory and disk-backed resume; never buffer entire large files.
- Do not introduce managed relay traffic for Free accounts, including through library defaults. Stage offline content only after an explicit authorized Premium user action.
- E2EE must protect data and metadata before cloud transit/storage. Do not invent cryptographic primitives or claim unreviewed forward-secrecy guarantees.
- Do not bypass platform privacy permission mechanisms or use unsupported background clipboard access on mobile.

## Code and PR standards

- Use small, single-purpose branches/PRs. Avoid mixing documentation, foundational plumbing and speculative features in one PR.
- Keep a documented workspace dependency policy; pin security-sensitive libraries and monitor advisories.
- Prefer deterministic unit/property tests and integration tests. Add fault injection for networking/storage/security-critical work.
- Include platform, command, commit SHA and result for every validation claim. Distinguish *not run*, *inconclusive* and *failed*.
- Never log clipboard bodies, filenames, source paths, secrets, credentials or raw hashes of private content. Use synthetic fixtures in tests.
- Document protocol and schema versioning; older clients must fail safely, not misinterpret fields.
- Public Rust/FFI APIs return structured errors; panics must not unwind across foreign boundaries.
- New feature-level behavior needs a corresponding requirement/test mapping in `docs/TRACEABILITY.md`.
- Any significant architecture, security, billing or OS behavior change requires an ADR and explicit approval. If asked to implement an unapproved proposal, perform only a clearly bounded feasibility experiment.

## PR template (minimum)

- **Objective / referenced requirement IDs**
- **Files/modules affected**
- **Chosen design and alternatives**
- **Security/privacy implications**
- **Validation matrix and commands**
- **Observed results, limitations and untested conditions**
- **Architecture decisions requested**
- **Rollback/compatibility notes**

## Workflow clarifications (PROPOSED, 2026-11-12 — operative only after owner acceptance)

These clarify how the rules above are applied. They do not relax any engineering invariant.

- **Status discipline.** Owner directions given in review are recorded as PROPOSED (or OPEN) until the owner explicitly accepts the baseline. Never mark them CONFIRMED on the owner's behalf.
- **Experiments before code.** Each Phase 0 experiment needs an approved specification in `docs/experiments/` before any experiment code is written. Experiment code lives in an isolated `experiments/<id>-<slug>/` workspace. It shares no crates with future product code, and is not promoted to product code without review.
- **Evidence handling.** Raw evidence (traces, dumps, recordings) stays under the experiment's git-ignored `evidence/` directory. Only sanitized summaries and reports are committed. Synthetic data only.
- **Commit separation.** Keep factual corrections, proposed decisions, experiment plans and experiment code in separate commits, and preferably separate PRs.
- **Equipment and spending.** Use available hardware and free local environments first. Do not purchase hardware, create accounts or paid services, or assume a budget without explicit owner approval (D-028). Report experiments blocked by missing equipment as *not run*.
- **Free networking.** For Free users, OneCopy infrastructure must not forward any inter-device packet: payload, handshake or traversal (D-021). Library default relays must be disabled. Third-party relays are OPEN (D-022).
- **Extensions.** Third-party application plugins are out of scope. A OneCopy-owned OS integration (share extension, File Provider–like provider, IME, desktop-shell integration) requires its own ADR and is limited to authorized history items (D-027).
- **Claims.** Do not claim native-paste guarantees, universal change detection or universal connectivity rates without experimental evidence (D-024, D-026, D-034).

## Initial work order

1. Documentation baseline acceptance (no code).
2. Phase 0 risk spikes: Windows Explorer virtual-file paste; Finder Command-V feasibility; Wayland capability matrix; strict direct-only iroh/QUIC experiment. *(Proposed refinement, D-023 and D-028: E-02 first; the transport experiment runs two independent tracks; hardware-dependent spikes wait for approved equipment.)*
3. Minimal headless Rust foundation and encrypted history only within explicitly approved Phase 0 scope.
4. On-demand 100 GB streaming/resume/source-change experiments and independent security review preparation.

Do **not** create cloud services, billing integrations, production auth, full UIs or mobile daemons merely to make an early demonstration look complete.
