# Research provenance and references

**Status:** Evidence index for the 2026-11-12 baseline. Platform docs and prices are time-sensitive; refresh before implementation or public claims.

## Primary research input

**OneCopy: Decision-Ready Architecture and Feasibility Research** (2026-11-12; provided by product owner in the design discussion). It includes platform feasibility, native Rust-core architecture, clipboard data model, QUIC/iroh NAT traversal, live-source transfer consistency, E2EE threat modeling, gateway economics, competitor analysis, phased roadmap and Phase 0 benchmarks.

This package is a **derived specification**, not a verbatim copy of the research. It distinguishes agreed product policy from recommendations and unresolved assumptions. The full supplied report is retained at `docs/research/onecopy-deep-research-2026-11-12.md`.

**Provenance limitation (factual note, 2026-11-12 review):** the report contains 59 inline citation markers (`cite…turn…search…`) referring to 82 search-result IDs from the tool that generated it. These IDs do not resolve to URLs, so individual claims in the report cannot be traced to a specific source. Re-source any claim from the report before it is used as evidence in an ADR. See `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §9 for claims re-sourced so far.

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

## Independent review sources (accessed 2026-11-12)

Added by the 2026-11-12 engineering review. Full list with evidence grading in `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` §9. Primary/vendor sources:

- iroh FAQ, security/privacy and NAT traversal concepts: https://docs.iroh.computer/about/faq , https://docs.iroh.computer/concepts/security-privacy , https://docs.iroh.computer/concepts/nat-traversal
- iroh 1.0 announcement (2026-06-15): https://iroh.computer/blog/v1 ; iroh on QUIC multipath (2025-08-05): https://www.iroh.computer/blog/iroh-on-QUIC-multipath
- NAT traversal measurement (libp2p DCUtR, IPFS): https://arxiv.org/abs/2510.27500
- `ext-data-control-v1` compositor table: https://wayland.app/protocols/ext-data-control-v1
- xdg-desktop-portal Clipboard: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Clipboard.html
- Win32 clipboard formats (cloud clipboard/history formats): https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats
- `FILEDESCRIPTORW`: https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-filedescriptorw
- `OleFlushClipboard`: https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-oleflushclipboard
- Shell data transfer scenarios: https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios
- `NSPasteboard.changeCount`: https://developer.apple.com/documentation/appkit/nspasteboard/changecount
- nspasteboard.org marker types: http://nspasteboard.org/
- File Provider synchronization: https://developer.apple.com/documentation/fileprovider/synchronizing-the-file-provider-extension
- TN3179 local network privacy: https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy
- `BGContinuedProcessingTask`: https://developer.apple.com/documentation/backgroundtasks/bgcontinuedprocessingtask
- Android 16 behavior changes: https://developer.android.com/about/versions/16/behavior-changes-all
- UniFFI changelog: https://raw.githubusercontent.com/mozilla/uniffi-rs/main/CHANGELOG.md
- Rust RFC 2945 (C-unwind): https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html

Secondary sources (forums, issue trackers, blogs) used by the review are listed there with an [S] grade and must not be cited as authoritative.

## Evidence handling

For every new web/vendor claim, add access date, software/API version and experimental evidence when available. Do not rely on an old pricing figure or OS documentation summary to assert production support in a future release.
