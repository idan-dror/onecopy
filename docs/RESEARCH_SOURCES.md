# Research provenance and references

**Status:** Evidence index for the 2026-11-12 baseline. Platform docs and prices are time-sensitive; refresh before implementation or public claims.

## Primary research input

**OneCopy: Decision-Ready Architecture and Feasibility Research** (2026-11-12; provided by product owner in the design discussion). It includes platform feasibility, native Rust-core architecture, clipboard data model, QUIC/iroh NAT traversal, live-source transfer consistency, E2EE threat modeling, gateway economics, competitor analysis, phased roadmap and Phase 0 benchmarks.

This package is a **derived specification**, not a verbatim copy of the research. It distinguishes agreed product policy from recommendations and unresolved assumptions. Retain the full supplied report separately in `docs/research/onecopy-deep-research-2026-11-12.md` when available.

## Official/primary references (starting points)

### Windows
- Clipboard change listener: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-addclipboardformatlistener
- Windows Shell clipboard formats, file descriptors and content streams: https://learn.microsoft.com/en-us/windows/win32/shell/clipboard
- User-session vs Service interaction: https://learn.microsoft.com/en-us/windows/win32/services/interactive-services

### Apple platforms
- `NSPasteboard`: https://developer.apple.com/documentation/appkit/nspasteboard
- Pasteboard access behavior (verify OS-version availability): https://developer.apple.com/documentation/appkit/nspasteboard/accessbehavior-swift.enum
- `NSFilePromiseProvider`: https://developer.apple.com/documentation/appkit/nsfilepromiseprovider
- Background transfers: https://developer.apple.com/documentation/foundation/downloading-files-in-the-background
- UIKit `UIPasteboard`: https://developer.apple.com/documentation/uikit/uipasteboard

### Android
- Android 10 clipboard privacy restrictions: https://developer.android.com/about/versions/10/privacy/changes
- Android copy/paste and content URIs: https://developer.android.com/develop/ui/views/touch-and-input/copy-paste
- Secure clipboard handling: https://developer.android.com/privacy-and-security/risks/secure-clipboard-handling
- User-initiated data transfer jobs: https://developer.android.com/develop/background-work/background-tasks/uidt

### Linux / Wayland
- Standard Wayland data-device protocol: https://wayland.app/protocols/wayland
- Data-control (staging/testing) protocol: https://wayland.app/protocols/ext-data-control-v1
- wlroots data-control variant: https://wayland.app/protocols/wlr-data-control-unstable-v1
- XDG autostart: https://specifications.freedesktop.org/autostart/latest/

### Networking and cryptography
- QUIC RFC 9000: https://www.rfc-editor.org/rfc/rfc9000
- iroh documentation: https://www.iroh.computer/docs
- MLS RFC 9420: https://www.rfc-editor.org/rfc/rfc9420
- HPKE RFC 9180: https://www.rfc-editor.org/rfc/rfc9180
- WebAuthn: https://www.w3.org/TR/webauthn-3/
- OWASP MASVS: https://mas.owasp.org/MASVS/

### Infrastructure costs
- Cloudflare R2 pricing: https://developers.cloudflare.com/r2/pricing/
- Backblaze B2 pricing: https://www.backblaze.com/cloud-storage/pricing
- Amazon S3 pricing: https://aws.amazon.com/s3/pricing/
- Wasabi pricing: https://wasabi.com/pricing/

## Research findings requiring experiments

- **Mac Finder:** documented file promises do not prove ordinary Command-V remote-file compatibility.
- **Wayland:** protocol existence does not prove universal compositor support.
- **Android/iOS:** application permissions and background behavior depend on OS version, user actions and scheduling.
- **iroh:** default relay-first pattern may violate Free no-managed-relay payload rule.
- **Live-source consistency:** filesystem metadata checks catch common modifications but do not create a universal immutable snapshot.
- **Cloud pricing:** no-egress object storage alone does not imply lowest total latency/cost.

## Evidence handling

For every new web/vendor claim, add access date, software/API version and experimental evidence when available. Do not rely on an old pricing figure or OS documentation summary to assert production support in a future release.
