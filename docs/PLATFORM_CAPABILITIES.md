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

## Source references

See `RESEARCH_SOURCES.md` for Microsoft clipboard, Apple pasteboard/file promise/background transfer, Android clipboard/content URI and Wayland protocol references.
