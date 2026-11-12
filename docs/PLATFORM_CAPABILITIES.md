# Platform capabilities and restrictions

**Status:** RESEARCHED constraints, to be verified against supported OS versions; no compatibility promise is made without device tests.

## Capability matrix

| Capability | Windows | macOS | Linux X11 | Linux Wayland | Android | iOS/iPadOS |
|---|---|---|---|---|---|---|
| Listen to ordinary copy | Yes, interactive user session | Permission-sensitive; polling (no change notification); reads may prompt (15.4+, default enforcement unverified) | Yes | Only on compositors exposing data-control; **not GNOME/Mutter** | Not from arbitrary unfocused apps | No continuous general background monitor |
| Write active clipboard | Yes | Yes | Yes | Compositor-dependent | While permitted/active | User-driven/in-app |
| Persistent per-user agent | Yes | Yes | Yes | Yes, clipboard restricted | Not unrestricted | No |
| On-demand native file paste | Shell virtual files (`IStream`); target-side guarantees unverified (E-02) | **Unverified pending E-03**; promises documented mainly for drag; File Provider is a candidate (ADR-0005) | No lazy virtual-file clipboard mechanism (real paths only); managed download fallback | Same as X11 | ContentProvider / Sharesheet model; provider streaming is an experiment | Share extension / Files model; File Provider is an experiment |
| Arbitrary large background P2P | Desktop feasible | Desktop feasible | Desktop feasible | Desktop feasible | Governed by jobs/services (UIDT, Android 14+) | Not reliable after suspension; `BGContinuedProcessingTask` (iOS 26+) is **an experiment, not a capability** |
| Explicit HTTPS background staging | Feasible | Feasible | Feasible | Feasible | Supported within job constraints | Background URLSession, file-backed uploads |

## Windows

**Research:** `AddClipboardFormatListener` provides clipboard change notifications. Shell clipboard formats `CFSTR_FILEDESCRIPTOR` and `CFSTR_FILECONTENTS` support virtual file representation and delayed file content via `IStream`. User-session process required for desktop interaction; session-0 services are unsuitable as a direct clipboard agent.

**Phase 0 test:** capture text and file list; native virtual-file paste into Explorer with a lazy content stream; include cancellation, multiple files, delayed read and large-file behavior. Test destinations other than Explorer separately; cannot infer universal app compatibility from one success.

## macOS

**Research:** `NSPasteboard` supports clipboard items and provides detection APIs, but modern AppKit distinguishes user-intent paste from other programmatic access and can require user consent. `NSFilePromiseProvider` supports promises, particularly drag-and-drop; documentation alone does **not** establish reliable Finder `Command-V` using a remote promise written to the General pasteboard.

**Phase 0:** verify actual supported macOS versions and permission states, denial behavior and recovery; test Finder `Command-V` separately from drag, file type, file size, cancel and source-unavailable cases. If not reliable, use OneCopy's explicit paste/download-to-folder workflow.

## Linux X11

**Research:** Standard selections support background clipboard managers while the graphical session exists, but clipboard ownership and format negotiation differ from Windows/macOS. File managers may expect URI lists or app-specific formats.

**Phase 0:** support text/image/URI-list capture, clipboard-owner exit/restart, paste among major desktops. Test that file reference metadata is not accidentally turned into remote data prefetch.

## Linux Wayland

**Research:** Ordinary data-device clipboard ownership is focus-constrained. Elevated clipboard-manager protocols such as `ext-data-control-v1` or compositor-specific variants can enable broader observation/injection, but versions/support differ and some remain experimental.

**Phase 0:** separately record GNOME/Mutter, KDE/KWin and wlroots-family environment/version/protocol availability; test capture, writing, clipboard availability after owner closes, permission prompts and native file-paste behavior. Support wording must be **per compositor and version**, not generic "Wayland works."

**GNOME Wayland limitation (RESEARCHED 2026-11-12; tier PROPOSED by D-036):** Mutter implements neither data-control protocol, so automatic background capture is not available through supported protocols. The proposed tier is B (manual capture). Every alternative needs a separate decision:

- XWayland bridge (unverified);
- portal Clipboard (only inside RemoteDesktop or InputCapture sessions, which would present misleading consent);
- a GNOME Shell extension (needs an ADR under D-027).

## Android

**Research:** Since Android 10, ordinary apps cannot read arbitrary foreground-app clipboard while unfocused; current APIs can return no clip unless app has focus or is the default input method. Content URIs with granted permissions are the right file exchange unit. User-initiated data transfer jobs and foreground-service policies differ by OS/API level.

**Product design:** native Kotlin app, `ACTION_SEND`/`ACTION_SEND_MULTIPLE`, OneCopy history, content provider and explicit user-triggered paste/copy. An optional OneCopy IME is an OPEN later experiment, not assumed and not required for baseline.

## iOS/iPadOS

**Research:** `UIPasteboard` reads can invoke privacy consent/user-intent restrictions, apps are suspended in background, and silent background pushes are unreliable. `URLSession` background transfers support HTTP/HTTPS rather than arbitrary QUIC and background uploads must use file-backed bodies.

**Product design:** native Swift app, user-triggered share/paste/history, Share extension, Files integration, foreground P2P as best effort. Premium HTTPS gateway can provide reliable background transfer where platform permits. Never describe the app as a continuously running iOS clipboard daemon.

**Experiment only (2026-11-12):** `BGContinuedProcessingTask` (iOS/iPadOS 26+) may let a *user-initiated* P2P transfer continue after the app is backgrounded. The system can terminate it, and lock-screen behavior has been reported as fragile. E-12 tests it. It is not a background-transfer capability claim.

Matrix cells updated 2026-11-12 with RESEARCHED facts from the review addendum below. No support claim was added.

## Clipboard support tiers (PROPOSED, D-036 — pending owner acceptance)

| Tier | Meaning |
|---|---|
| **A — automatic** | Background capture of ordinary copies (subject to permission), promotion, native paste where verified |
| **B — manual capture** | The user captures explicitly: focused OneCopy window, global shortcut where available, share action. Promotion works. |
| **C — share/promote only** | Content enters through OS share flows; promotion within app lifecycle limits |

| Environment | Proposed tier | Basis |
|---|---|---|
| Windows (interactive session) | A | Clipboard format listener |
| macOS, pasteboard access allowed | A | Polling plus allowed reads; behavior with "ask" or "deny" is to be measured in E-03 |
| macOS, access ask or deny | B | Must not work around the user's choice |
| Linux X11 | A | XFixes selection notifications [K] |
| Wayland with `ext-data-control-v1` or `wlr-data-control` (for example KWin, Sway/wlroots family, Hyprland, COSMIC, niri), per tested version | A | E-05 matrix |
| **GNOME Wayland (Mutter)** | **B** | Mutter implements neither data-control protocol. The portal Clipboard interface is available only inside RemoteDesktop or InputCapture sessions. An XWayland bridge is unverified. A GNOME Shell extension would need its own ADR (D-027). |
| Sandboxed (Flatpak) Linux builds | Per compositor | Compositors may withhold privileged protocols from sandboxed clients (`security-context-v1`) |
| Android | B/C | Background reads blocked since Android 10. An IME is deferred (D-018). |
| iOS/iPadOS | B/C | Paste privacy prompts; no background monitor |

Tiers are published per tested OS or compositor version (E-04, E-05, E-03, E-12). Never claim "Wayland works" generically.

## Sensitivity and exclusion markers (PROPOSED handling, D-032 / ADR-0007)

| Platform | Honor on capture | Set on promotion of sensitive items | Evidence |
|---|---|---|---|
| Windows | `ExcludeClipboardContentFromMonitorProcessing`; `CanIncludeInClipboardHistory` = 0 | The same formats, plus `CanUploadToCloudClipboard` = 0 | [V] |
| macOS | `org.nspasteboard.ConcealedType`, `TransientType`; `AutoGeneratedType` = skip by default | Concealed plus Transient | [V, community convention] |
| Linux | No cross-desktop standard. Some password managers use the KDE `x-kde-passwordManagerHint` type | The same hint where supported | [K], to be verified in E-04/E-05 |
| Android | `ClipDescription.EXTRA_IS_SENSITIVE`, readable only when OneCopy may read the clip | `EXTRA_IS_SENSITIVE` | [V] |
| iOS/iPadOS | No general read-side marker known | `localOnly` and `expirationDate` options | [K] |

**Limitations** (to be stated in UX and docs):

- Markers are voluntary hints set by the source app.
- Their absence does not mean the content is not sensitive.
- OS and keyboard-app behavior with markers varies by version.
- Apps can misuse markers to hide content.
- Setting markers on promotion only helps where the OS or the recording app honors them.

## Received-file provenance (RESEARCH for D-033 / ADR-0007)

| Platform | Mechanism | Notes |
|---|---|---|
| Windows | Mark-of-the-Web (`Zone.Identifier` alternate data stream); `IAttachmentExecute::Save` "may attach evidence … in its NTFS alternate data stream" and "may run virus scanners" [V] | Alternate data streams are lost on FAT32 and exFAT. Whether Explorer marks virtual-file pastes is unknown; observed in E-02. `CFSTR_UNTRUSTEDDRAGDROP` is documented only for Internet Explorer drag-and-drop policy [V]. |
| macOS | `com.apple.quarantine` through `NSURLQuarantinePropertiesKey`, or the `LSFileQuarantineEnabled` Info.plist key [V/S] | Finder or File Provider native-paste behavior observed in E-03 |
| Linux | No OS-enforced mechanism. freedesktop `user.xdg.origin.*` extended attributes are informational [K] | Document as "not marked" unless a desktop standard is adopted |
| Android, iOS | No Mark-of-the-Web equivalent [K] | App sandbox and provider permissions apply |

## Capability detection API (PROPOSED)

Each platform adapter should report machine-readable capabilities with reason codes:

```text
clipboard_observe: available | needs_permission | unsupported | temporarily_unavailable
clipboard_write: available | restricted | unsupported
virtual_file_paste: supported | experimental | unsupported
file_source_persistence: durable | session_bound | unavailable
background_transfer: full | user_initiated_only | https_only | foreground_only
# proposed additions (2026-11-12):
support_tier: A | B | C
native_paste: verified | unverified | unsupported   # per target, from E-02/E-03 evidence
sensitivity_markers: honored | partial | unavailable
provenance_marking: applied | observed_by_target | unavailable
```

Expose reason codes to UI and diagnostics rather than hardcoding platform marketing claims.

## Platform verification record

For each supported OS: edition/version, desktop/compositor, app permission state, user action, observed outcome, reproduction commands, evidence artifact, fallback quality and limitations. Do not mark an experimental platform as fully supported based on documentation alone.

## Review addendum — RESEARCHED facts (2026-11-12)

**Status:** RESEARCHED only. This addendum records documentation facts found by the 2026-11-12 engineering review. It does **not** change the matrix above, any decision status or any product claim. Design responses are proposals in `docs/reviews/ENGINEERING_REVIEW_2026-11-12.md` (§5) and need owner decisions. Evidence grades: [V] vendor/primary source, [S] secondary source, [K] not re-verified.

### Windows

- [V] Windows defines registered clipboard formats `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` and `CanUploadToCloudClipboard` that control inclusion in Clipboard History and Cloud Clipboard sync.
- [V] `FILEDESCRIPTORW.cFileName` is `WCHAR[MAX_PATH]`; relative paths inside a virtual folder tree are therefore limited to `MAX_PATH`. `CFSTR_FILEDESCRIPTOR` lists every entry in one `FILEGROUPDESCRIPTOR` before content is read.
- [V] Microsoft recommends `IStream` for `CFSTR_FILECONTENTS`; for very large files an interface pointer "might be the only practical data transfer mechanism". `IDataObjectAsyncCapability` is documented for asynchronous drag-and-drop extraction.
- [V] While the source process runs, the clipboard holds only a pointer to its data object; `OleFlushClipboard` renders offered formats onto the clipboard (its documented example copies an `IStorage` into memory) so they survive process exit.

### macOS

- [V/K] `NSPasteboard` exposes `changeCount`; AppKit has no clipboard-change notification, so observation is by polling.
- [V/S] `NSPasteboard.accessBehavior` (macOS 15.4+) exposes ask / always-allow / always-deny states, and detection APIs let apps inspect types without reading content. Whether the read prompt is **enforced by default** on macOS 26.x/27.x is **unverified**; a secondary source reports the macOS 27.2 release notes remove the developer-preview switch.
- [V] nspasteboard.org conventions: `org.nspasteboard.ConcealedType`, `org.nspasteboard.TransientType`, `org.nspasteboard.AutoGeneratedType`, `org.nspasteboard.source`.
- [S] Developer reports indicate Finder `Command-V` does not materialize `NSFilePromiseProvider` promises placed on the general pasteboard; Apple documents promises primarily for drag-and-drop.
- [V/S] File Provider replicated extensions keep dataless items and the system calls `fetchContents` when content is accessed; reports indicate Finder copies of dataless items trigger fetches.
- [S] Local Network privacy (macOS 15+) requires user approval for LAN communication; behavior for launchd agents has had reported bugs.

### Linux Wayland

- [V] `ext-data-control-v1` is a staging protocol ("currently in the testing phase"). Compatibility table on 2026-11-12 — supported: KWin 6.7, Sway 1.11, Hyprland 0.52.1, COSMIC 1.0 beta, niri 26.04, labwc 0.20.2, river 0.5.0, Mir 2.26, phoc 0.55, Jay, Qtile, Mango, Treeland. Not supported: Mutter 51 (GNOME), Weston 15, Muffin, Wayfire 0.9, Cage, Gamescope, Louvre.
- [V] The xdg-desktop-portal `Clipboard` interface is available only to RemoteDesktop and InputCapture portal sessions.
- [S] `security-context-v1` lets compositors withhold privileged protocols (such as data-control) from sandboxed (e.g. Flatpak) clients; behavior is compositor-specific.

### Android

- [V] User-initiated data transfer jobs: Android 14 (API 34)+, `RUN_USER_INITIATED_JOBS` permission, scheduled while the app is visible (or under background-activity-start exemptions), notification required, not subject to App Standby Bucket quotas, stoppable by constraints/thermal/memory; apps must persist transfer state.
- [V] Android 16 applies the job runtime quota to jobs started while visible that continue in the background and to jobs running alongside a foreground service, and recommends UIDT for user-initiated transfers.
- [V] Google recommends marking sensitive clips with `ClipDescription.EXTRA_IS_SENSITIVE`.
- [S] Android 13 closed the log-access workaround for background clipboard reads ("working as intended").

### iOS/iPadOS

- [V] `BGContinuedProcessingTask` (iOS/iPadOS 26.0+) is a foreground-started task that can continue in the background with system-displayed progress; the system may terminate it under resource constraints and prioritizes terminating tasks with little progress. [S] One report describes an upload expiring when the device was locked (26.1/26.2 beta). Suitability for P2P transfers is an experiment, not a capability claim.
- [S] Since iOS 16.1, per-app "Paste from Other Apps" settings (Ask/Deny/Allow) exist.

## Source references

See `RESEARCH_SOURCES.md` for Microsoft clipboard, Apple pasteboard/file promise/background transfer, Android clipboard/content URI and Wayland protocol references.
