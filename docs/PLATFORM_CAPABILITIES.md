# Platform capabilities and restrictions

**Status:** RESEARCHED constraints, to be verified against supported OS versions; no compatibility promise is made without device tests.

## Capability matrix

| Capability | Windows | macOS | Linux X11 | Linux Wayland | Android | iOS/iPadOS |
|---|---|---|---|---|---|---|
| Listen to ordinary copy | Yes, interactive user session | Permission-sensitive | Yes | Compositor/protocol-specific | Not from arbitrary unfocused apps | No continuous general background monitor |
| Write active clipboard | Yes | Yes | Yes | Compositor-dependent | While permitted/active | User-driven/in-app |
| Persistent per-user agent | Yes | Yes | Yes | Yes, clipboard restricted | Not unrestricted | No |
| On-demand native file paste | Strong Windows Shell APIs | File promises exist; Finder Cmd-V unknown | Desktop/app dependent | Desktop/app dependent | ContentProvider / Sharesheet model | Share extension / Files model |
| Arbitrary large background P2P | Desktop feasible | Desktop feasible | Desktop feasible | Desktop feasible | Governed by jobs/services | Not reliable after suspension |
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

## Android

**Research:** Since Android 10, ordinary apps cannot read arbitrary foreground-app clipboard while unfocused; current APIs can return no clip unless app has focus or is the default input method. Content URIs with granted permissions are the right file exchange unit. User-initiated data transfer jobs and foreground-service policies differ by OS/API level.

**Product design:** native Kotlin app, `ACTION_SEND`/`ACTION_SEND_MULTIPLE`, OneCopy history, content provider and explicit user-triggered paste/copy. An optional OneCopy IME is an OPEN later experiment, not assumed and not required for baseline.

## iOS/iPadOS

**Research:** `UIPasteboard` reads can invoke privacy consent/user-intent restrictions, apps are suspended in background, and silent background pushes are unreliable. `URLSession` background transfers support HTTP/HTTPS rather than arbitrary QUIC and background uploads must use file-backed bodies.

**Product design:** native Swift app, user-triggered share/paste/history, Share extension, Files integration, foreground P2P as best effort. Premium HTTPS gateway can provide reliable background transfer where platform permits. Never describe the app as a continuously running iOS clipboard daemon.

## Capability detection API (PROPOSED)

Each platform adapter should report machine-readable capabilities with reason codes:

```text
clipboard_observe: available | needs_permission | unsupported | temporarily_unavailable
clipboard_write: available | restricted | unsupported
virtual_file_paste: supported | experimental | unsupported
file_source_persistence: durable | session_bound | unavailable
background_transfer: full | user_initiated_only | https_only | foreground_only
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
