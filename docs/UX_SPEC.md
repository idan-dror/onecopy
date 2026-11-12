# OneCopy — UX specification

**Status:** Confirmed core behavior with proposed interaction details. No UI mocks are approved by this document.

## Mental model

Two separate things exist on a device:

1. **Active system clipboard:** what standard paste uses right now in a local app.
2. **OneCopy history:** a small encrypted shared list of items captured locally or received from trusted devices.

A remote history update **does not** replace the active system clipboard. The user chooses whether to use it. Normal Copy remains normal Copy; OneCopy watches where OS permission allows.

## Primary surfaces (PROPOSED)

- Desktop tray/menu-bar access, compact history panel, global shortcut when safe and available, settings/devices/transfers view.
- Mobile native app history, system Sharesheet entry, user-triggered copy/share/save action.
- A device trust screen for pairing approvals, trusted roster and revocation.
- A visible transfer queue with progress, pause/cancel/resume where supported, destination and source availability.

No browser, VS Code or application-specific extensions.

## Clipboard history row

Display type, safe preview where appropriate, origin device name, copy time, availability, and relevant status. For file entries show display filename, best-effort origin path **only if safely available**, size as informational, and whether source is online; never imply a cached file is available unless staged or already materialized.

Do not render executable HTML from clipboard or automatically resolve/open a copied URL. Avoid exposing sensitive previews in notifications and unlocked surfaces.

### History actions

- **Copy locally / Paste using OS:** promote selected content to local active clipboard and indicate when user should press Ctrl+V/Cmd+V; or invoke a native paste workflow only when supported and authorized.
- **Paste/Download to Folder:** explicit fallback for virtual-file limitations; destination chosen by user.
- **Keep Available Offline:** Premium-only explicit staging, not automatic.
- **Refresh Offline Copy:** replace the one temporary staged copy with a newly uploaded snapshot, only upon user action.
- **Remove from history:** delete local/shared logical history entry according to the chosen retention semantics; do not delete source files.
- *(D-024)* **Paste** (native) versus **Save with OneCopy** (managed download). When native paste is unavailable, unverified or over its limits, offer Save with OneCopy and explain the difference in one sentence. Example, if E-02 confirms it: "Large transfers pasted into Explorer can't resume after an interruption."
- *(D-025)* **Keep on this device until sent** (mobile, transient sources only): an explicit choice that shows size and expiry. It is never automatic and is never labeled as an offline or cloud copy.
- **Pause synchronization/exclude app:** privacy control, behavior to be finalized.

## State vocabulary

| State | Meaning | User-facing example |
|---|---|---|
| Local | Captured on current device | "Copied just now" |
| Syncing | Encrypted history event not yet acknowledged | "Syncing history…" |
| Shared | Received/replicated item metadata | "Available in OneCopy" |
| Source online | Source agent reachable, source may be openable | "Ready to request" |
| Source offline | Origin device unreachable | "Source device offline" |
| Source unavailable | Deleted, permission revoked or handle expired | "Source no longer available" |
| Transferring | File data streaming to destination | progress, speed, source, route |
| Interrupted | Partial destination exists | "Resume transfer" when compatible |
| Failed: source changed | Incompatible modification detected | "Source changed during transfer; retry" |
| Staging | Premium encrypted upload in progress | "Creating offline copy…" |
| Verifying | Staging upload exists but not yet ready | "Verifying offline copy…" |
| Staged/Ready | Valid temporary encrypted copy | "Offline copy created [date/time]" |
| Expired | Staging object expired/deleted | "Offline copy expired" |
| *Kept on device* | Mobile local staging of a transient source | "Kept on this phone until [time]" |
| *Origin app not running* | A large item's bytes live on a mobile origin whose OneCopy app is suspended or not running (D-030 correction) | "Open OneCopy on [device] to get this item" |
| *Preview only* | Item above the inline cap; only metadata or a preview is synced, and full content needs the origin | "Full [image/text] is on [device]" with size |
| *Excluded* | Capture skipped because of a sensitivity marker | "Not saved: marked private by the app that copied it" |
| *Manual capture mode* | Support tier B (for example GNOME Wayland, or macOS with clipboard access denied) | "Press [shortcut] or open OneCopy to add your clipboard" |
| *Direct connection unavailable* | Free direct path failed on this network | "Couldn't connect directly to [device] on this network" (no paid option shown before Premium exists) |
| *Permission needed* | OS permission blocks capture or LAN access (for example macOS Local Network) | Names the permission and where to grant it, with no dark patterns |

Status must not conflate **source availability** with **offline staging availability**. When source is online and a staged copy exists, normal live transfer uses the current source unless the user explicitly requests the staged copy or an approved fallback policy says otherwise.

## Interaction scenarios

**Desktop → desktop text:** copy on A, history entry on B, choose, paste into any compatible local app. User retains the ability to work independently on B in between events.

**Desktop → desktop file:** copy 100 GB source on A (metadata only). B selects an item. Windows may present a native delayed virtual file, while another OS may present Save to Folder. The source must remain accessible until request reads it.

**Mobile → desktop:** from a mobile share action, OneCopy receives a user-authorized content URI/security-scoped item. If access is transient, warn that the source may become unavailable; do not silently materialize a giant permanent snapshot. *(D-025: offer an explicit, bounded, expiring "Keep on this device until sent".)*

**Desktop → mobile:** user opens OneCopy, chooses an item, then copies locally or shares/saves via native mobile APIs. Respect mobile lifecycle limits.

**Large mobile-origin items (correction, 2026-11-12):** an image or text above the inline cap, copied or shared on a phone, is listed on other devices with size and origin. When the phone's OneCopy app is suspended or not running, the item shows "Open OneCopy on [phone] to get this item". It becomes fetchable when the app runs again. The UI never implies the content is stored elsewhere, and never uploads it automatically.

**Remote file delivery modes (correction, 2026-11-12):** "Save with OneCopy" (managed download) guarantees resume and controlled finalization on supported destinations. Native paste shows only the guarantees demonstrated for that target. If a large native paste is interrupted, the UI offers to restart through managed download.

**Blocked P2P Free:** describe direct connection failure and a Premium relay option. Do not silently turn the option on. *(Clarification, 2026-11-12: until Premium exists, describe the failure and its likely cause only.)*

**GNOME Wayland (D-036):** automatic capture is unavailable. The user adds the current clipboard explicitly through a focused OneCopy window or a shortcut. Promotion works normally. The limitation is stated once during onboarding.

**Offline Premium:** create an encrypted temporary stage on request, show timestamp/expiry and storage/quota usage. Download as a separate explicit action. No source-change watcher and no automatic refresh.

## Accessibility/localization (PROPOSED)

Keyboard navigation, screen-reader descriptions for states and progress, high-contrast UI, system text scaling, accessible transfer errors, RTL/localization-ready strings, and explicit permission education without dark patterns.

## Error UX principles

- Explain *what happened* and *what can be done*: source offline, source deleted, permission denied, blocked network, insufficient disk, transfer interrupted, device revoked, staging quota exceeded.
- Never show a falsely successful paste/download before verification.
- Never prompt to maintain historical versions or silently revert file content.
- Do not promise that a user can paste with standard keyboard shortcuts on unsupported OS/targets.

## Questions for owner

History keyboard shortcut and default window behavior; whether 10-item default is acceptable; sensitive app exclusion UX; retention/removal semantics; whether staged-copy selection should be explicit even when the source is offline. Record resulting decisions in `DECISIONS.md`.
