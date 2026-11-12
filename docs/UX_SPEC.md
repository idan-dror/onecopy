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

Status must not conflate **source availability** with **offline staging availability**. When source is online and a staged copy exists, normal live transfer uses the current source unless the user explicitly requests the staged copy or an approved fallback policy says otherwise.

## Interaction scenarios

**Desktop → desktop text:** copy on A, history entry on B, choose, paste into any compatible local app. User retains the ability to work independently on B in between events.

**Desktop → desktop file:** copy 100 GB source on A (metadata only). B selects an item. Windows may present a native delayed virtual file, while another OS may present Save to Folder. The source must remain accessible until request reads it.

**Mobile → desktop:** from a mobile share action, OneCopy receives a user-authorized content URI/security-scoped item. If access is transient, warn that the source may become unavailable; do not silently materialize a giant permanent snapshot.

**Desktop → mobile:** user opens OneCopy, chooses an item, then copies locally or shares/saves via native mobile APIs. Respect mobile lifecycle limits.

**Blocked P2P Free:** describe direct connection failure and a Premium relay option. Do not silently turn the option on.

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
