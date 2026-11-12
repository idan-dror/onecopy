# ADR 0005: OneCopy-owned OS integrations versus third-party application plugins

Status: Accepted (Phase 0 baseline, 2026-11-12)
Date: 2026-11-12
Owner: Product owner
Decision IDs: D-027 (CONFIRMED). Related: D-003, PR-020 (CONFIRMED); PRODUCT_VISION "not a universal remote filesystem mount"; D-016, D-017, D-018

## Context and evidence

Several native capabilities need an OS-sanctioned extension point that OneCopy itself implements:

- macOS and iOS File Provider extensions (dataless placeholders that the system hydrates on access) [V/S];
- Windows Cloud Files placeholders [K];
- Android `DocumentsProvider` [K];
- iOS share extensions;
- an Android IME (D-018);
- a GNOME Shell extension, for GNOME Wayland capture.

None of these is a plugin for a third-party application. D-003 is nonetheless sometimes read as forbidding them.

## Decision

1. Third-party application plugins remain out of scope. This covers browser extensions, IDE or editor extensions, and per-application add-ons (D-003 is unchanged).
2. OneCopy-owned OS integrations are a separate category. Each one may be evaluated through its **own ADR**, which states:
   - the user-facing benefit;
   - the permissions it needs;
   - its attack surface;
   - its maintenance and review cost (including app-store review);
   - its fallback if rejected.
3. Any File Provider, Cloud Files, `DocumentsProvider` or similar integration:
   - exposes **only authorized OneCopy history items** (current file references the user can request);
   - offers no browsing of the source filesystem;
   - offers no write-back to the source;
   - offers no persistent mirror.

   This keeps OneCopy outside the "universal remote filesystem mount" category.

## Alternatives considered

- **Strict reading of D-003** (no OS extensions). Removes the most credible paths to native macOS paste and Files-app integration on iOS.
- **Broad allowance without ADRs.** Uncontrolled attack surface.

## Security/privacy and interoperability effects

- Every extension is a new process and trust boundary. It is subject to the single-writer rule (ADR-0009), to sandbox entitlements, and to its own threat-model entry.

## User/product effects

- A File Provider domain appears as a location in Finder or Files.
- A GNOME Shell extension requires a user-visible installation and is tied to specific GNOME versions.

## Consequences and follow-up tests

- E-03 decides whether the macOS File Provider ADR is needed. Finder `Command-V` remains **unverified** until then.
- No extension is implemented without its own accepted ADR.

## Acceptance record (who/when)

Accepted by the product owner, with an engineering teammate, on 2026-11-12 (PR #1), as part of the Phase 0 baseline. Scope: product direction and Phase 0 feasibility experiments. This acceptance does not approve unverified production architectures, future security-protocol choices or Premium infrastructure. Items this ADR leaves unspecified or OPEN remain open.
