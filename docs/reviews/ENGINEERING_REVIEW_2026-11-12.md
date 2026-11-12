# OneCopy — Independent engineering review of the documentation baseline

**Status:** Review input, PROPOSED. This document approves nothing. It does not change any CONFIRMED decision, does not mark the baseline approved, and does not authorize implementation.
**Reviewer:** Claude (implementation engineer role, per `CLAUDE.md`).
**Baseline reviewed:** commit `23a0de3` (documentation package v0.1 plus `docs/research/onecopy-deep-research-2026-11-12.md`).
**Research date:** 2026-11-12. Every web source below was accessed on that date.

## How to read this review

Evidence labels used in every section:

| Label | Meaning |
|---|---|
| **[V]** | Verified against a primary or vendor source that was fetched during this review. |
| **[S]** | Secondary source (forum thread, issue tracker, blog, aggregator). Indicative only. |
| **[K]** | Engineering knowledge that was **not** re-verified during this review. Confirm before relying on it. |
| **[I]** | Reviewer inference or judgment. |

Limitations:

- No code was written and no experiment was run. "Verified" means "matches documentation fetched on 2026-11-12", not "observed working on a device".
- No macOS, iOS, Android or Linux hardware was used. The only local environment is a Windows 11 Home machine (build 26200).
- The research report contains 59 inline citation markers (`cite…turn…search…`) that reference 82 search-result IDs from the tool that produced it. These IDs cannot be resolved to URLs, so the report's specific claims cannot be traced to sources. Claims that matter to a decision were re-sourced here where possible (see §3 and §9).
- Vendor prices, OS behavior and library APIs change. Re-check anything here before procurement or a public claim.

### Errata (2026-11-12, after owner and teammate review)

These corrections fix the review's accuracy. They are not new findings.

1. **NAT measurement scope.** The 70% ± 7.1% figure (S8) is a *conditional* hole-punch success rate for libp2p DCUtR in the IPFS network: it counts only attempts that reached the hole-punching stage. It is not an end-to-end connection success rate, and it does not predict OneCopy's overall success. The earlier wording ("Free transfers will fail by design on a large share of Internet paths", "roughly 10–30% of pairs fail") over-extrapolated and has been corrected in §1, §3.7 and §4.
2. **Consistency trailer scope.** A source-consistency trailer stops OneCopy from signalling end-of-stream on inconsistent data. It does **not** control what Explorer, Finder or a provider framework does with a partially written destination file. The wording in §3.7, AR-03 and R-03 has been corrected. Partial-file behavior is an E-02 measurement.
3. **Roster freshness.** The example bound ("24 h") in AR-10 was arbitrary and has been removed. Any bound needs a documented security/usability trade-off.

Contents:

1. Executive assessment
2. Documentation audit
3. Technical research findings
4. Risk register
5. Proposed architecture refinements
6. Open decisions
7. Revised Phase 0 recommendation
8. Documentation changes
9. Sources

---

## 1. Executive assessment

### Verdict

The architecture is **feasible as a desktop-first product** and the core product decisions are sound. No CONFIRMED decision needs to be reversed on the evidence gathered. Three areas, however, are materially harder or more expensive than the baseline implies, and each needs an owner decision before or during Phase 0:

1. **Free direct-only Internet connectivity.** iroh's NAT traversal is coordinated through its relay, and application data flows over the relay before a direct path exists [V: S1, S5]. Disabling the relay also disables hole-punch coordination [V: S1]. So "iroh first" and "Free never uses a managed relay" pull in opposite directions. ICE-style designs (STUN plus signaling, no packet forwarding) can satisfy the rule, but a precise definition of what a Free connection may use is missing (§2 DA-03). The ≥90% direct-success target has no independent support. The largest public measurement found a *conditional* hole-punch success rate of about **70% ± 7.1%** for libp2p DCUtR in IPFS, among attempts that reached the hole-punching stage [V: S8]. That is not OneCopy's expected overall success rate, but it means some Free Internet transfers will fail by design and the rate must be measured per network class. LAN transfers are not affected. *(Corrected per Errata 1.)*
2. **Native lazy file paste outside Windows Explorer.** Forum evidence suggests Finder `Command-V` does not materialize file promises placed on the general pasteboard [S: S22]. The credible macOS route is a File Provider extension with on-demand ("dataless") placeholders [V/S: S23], which is a significant component and edges toward the "not a remote filesystem mount" boundary. Linux has no virtual-file clipboard mechanism (FUSE would be needed) [K]. Even on Windows, Explorer owns the destination write, so OneCopy cannot provide the documented journal, resume and verify-then-rename guarantees for a native paste (§2 DA-04).
3. **Automatic capture on GNOME Wayland,** the default desktop of Ubuntu and Fedora Workstation. Mutter 51 implements neither `ext-data-control-v1` nor `wlr-data-control` [V: S9]. Every workaround is policy-sensitive (§3.3).

### Strongest aspects

- **OneCopy history is separate from the OS clipboard,** and remote arrivals never take over the active clipboard (D-001, D-004). This matches platform reality, removes cross-device feedback loops, and makes the mobile limits honest instead of embarrassing.
- **Live references with a detect-and-fail consistency contract** (D-005, D-006). The contract is honest about what filesystems can guarantee and avoids speculative I/O and storage cost.
- **Capability reporting with reason codes** (`PLATFORM_CAPABILITIES.md` §Capability detection API) and per-compositor support wording.
- **Account login does not confer content trust,** source handles are capability-scoped, and there is no raw-path RPC. These are the right security primitives.
- **A risk-first Phase 0 with an explicit evidence format.**

### Weakest aspects

- The Free/Premium boundary is defined at the product level but not at the packet level. "Signaling", "coordination" and "relay" are undefined (DA-03).
- The resume and integrity model assumes OneCopy controls the destination write path. For Explorer, Finder and Android/iOS providers it does not (DA-04).
- The trust model is under-specified in six places: local IPC, stale rosters during LAN-only operation, revocation authority, key state shared between processes (app plus extension, agent plus UI), crash dumps, and OS-level clipboard history or cloud sync of promoted items (DA-07 to DA-10).
- Mobile "live references" mostly cannot exist. Share-sheet and share-extension access is transient, so the D-005 vocabulary needs a mobile interpretation (DA-05).
- Phase 0 is sized for a team: parallel spikes, 1,000 real-world NAT attempts, and a 100 GB × OS × network matrix. It also implies spending (public server, Apple hardware and developer account, test devices) that has not been approved (DA-29).

### Most important disagreement with the existing design

**"Evaluate iroh first" should become a two-track transport experiment.** For Free users, iroh's main differentiators are unusable: starting over a relay and falling back to it. Its value then reduces to NAT-traversal logic, QUIC multipath and key-based identity, and only if those work without its relay. If they do not, an ICE-style candidate exchange over the control plane plus STUN-like address discovery around a plain QUIC stack (for example `quinn`) is the cleaner way to satisfy D-008 by construction. iroh may still be the better choice for Premium and LAN. E-06 should therefore compare both tracks against the same evidence criteria (§5 AR-02, §7).

---

## 2. Documentation audit

Type codes: **C** contradiction · **A** ambiguity · **G** gap · **F** factual/stale. "Owner?" marks whether resolving the item needs a product-owner decision (Y) or is an engineering clarification (N).

| ID | Type | Location | Finding | Suggested resolution | Owner? |
|---|---|---|---|---|---|
| DA-01 | C | `README.md` §Core invariants #4 ("No … change monitoring"); `DECISIONS.md` D-005 implication; vs `TRANSFER_AND_ROUTING.md` §Live source changes ("Read chunks … while monitoring for detectable changes") | The invariant bans change monitoring outright. The transfer design requires detecting changes while reading. | Scope the invariant: no monitoring at copy time or while idle; change detection is allowed only on the opened source during an active transfer or staging operation. | Y (wording of a CONFIRMED invariant) |
| DA-02 | A | `README.md` #5, `PREMIUM_GATEWAY.md` §Product and financial boundary ("small encrypted-history sync"), `DATA_MODEL_AND_PROTOCOL.md` `inline_ciphertext? (small data only)`, `PRD.md` PR-005, `PHASE_0_VALIDATION.md` corpus (1–5 MiB images) | There is no byte threshold for "small". It is unclear whether a 5 MiB screenshot is stored and forwarded by the control plane for Free users, which has cost and policy implications and resembles staging. | Define an inline cap (for example ≤256 KiB ciphertext per item); larger representations become on-demand references fetched P2P. Add per-account Free quotas. | Y |
| DA-03 | G | `PRD.md` PR-015; `TRANSFER_AND_ROUTING.md` §Network paths; `GLOSSARY.md` "P2P / direct" | "Managed relay" and "signaling" are not distinguished at the packet level. Hole-punch rendezvous in iroh (and libp2p DCUtR) forwards packets [V: S1, S5]. ICE needs only signaling plus STUN [K]. | Define the Free connection-establishment budget (see OD-01). Add glossary terms: *signaling*, *address discovery*, *coordination relay*, *payload relay*. | Y (highest priority) |
| DA-04 | C (latent) | `PRD.md` PR-012 (resume); `TRANSFER_AND_ROUTING.md` §Resume and durability (`.onecopy-partial`, journal, atomic rename); vs `UX_SPEC.md` §Interaction scenarios ("Windows may present a native delayed virtual file") and E-02 | In an Explorer paste, Explorer creates and writes the destination file. OneCopy can only supply or fail `IStream::Read`. It cannot journal, resume after an agent restart, or rename on verification. Whether Explorer deletes a partial file on error is unknown. | Define per-path guarantees (§5 AR-03): native paste guarantees integrity and no false success; resume is best-effort. Large items route to managed download. | Y |
| DA-05 | G | `DECISIONS.md` D-005, `PRD.md` PR-008/PR-010, `UX_SPEC.md` §Mobile → desktop | iOS share extensions and Android `ACTION_SEND` grants give transient access [K]. A durable live reference to a mobile-shared file usually cannot exist without a copy, yet copies are what D-005 forbids for "normal Copy". | Define a "transient source" class and an explicit, user-initiated, bounded "keep on this device until sent" copy (§5 AR-16). | Y |
| DA-06 | G | `ARCHITECTURE.md` §Client processes; `DATA_MODEL_AND_PROTOCOL.md` `Device` | "Device" is not scoped. It is unclear whether two OS users on one PC, or Windows fast user switching, are one device or two. | Device = one OS user profile × one installation; keys are never shared across OS users. | N (confirm) |
| DA-07 | G | `SECURITY_AND_PRIVACY.md` §Device trust; `PRD.md` UJ-05 | Revocation authority is undefined. It is unclear who may revoke whom, and whether a stolen device can revoke every other device and lock the user out. There is also no bound on roster staleness when devices transfer over LAN while the control plane is unreachable, so a revoked device could still be served by a peer with a stale roster. | Specify revocation authority, recovery-key override, and a roster-freshness rule for authorizing transfers (§5 AR-10). | Y |
| DA-08 | G | `ARCHITECTURE.md` §Client processes (Windows "authenticated local IPC"); absent from `SECURITY_AND_PRIVACY.md` threat table | The local IPC trust boundary is not modeled. The agent holds history from *other* devices, so an unauthenticated local endpoint widens exposure. | Add an actor row and controls: per-user ACL, peer-credential checks, caller code-signature checks where available. | N |
| DA-09 | G | `PRD.md` PR-022, PR-024; `SECURITY_AND_PRIVACY.md` §Clipboard and content privacy | Promoting an item to the OS clipboard can push it into Windows Clipboard History or Cloud Clipboard, macOS Universal Clipboard or Spotlight clipboard history, and keyboard-app histories [V: S13; S: S39]. Platform "do not record" markers exist on Windows [V: S13], macOS [V: S21] and Android [V: S28], but the docs neither honor nor set them. | Add requirements to honor the markers on capture and set them when promoting sensitive items (§5 AR-05). | Y (privacy default) |
| DA-10 | G | `PRD.md` NFR-002; `SECURITY_AND_PRIVACY.md` §Privacy inventory | Crash dumps (Windows Error Reporting full dumps, custom minidumps, Android tombstones) can contain heap memory with clipboard bodies and keys [K]. The rules cover only telemetry and logs. | Forbid full-memory dumps, scrub minidumps, and keep crash reporting opt-in. | N |
| DA-11 | G | not covered | Files received from another device carry no provenance marking: no Windows Mark-of-the-Web or macOS `com.apple.quarantine` [K]. SmartScreen, Office Protected View and Gatekeeper therefore do not apply. | Decide on provenance marking (§5 AR-12). | Y |
| DA-12 | A | `PRD.md` PR-025; `DATA_MODEL_AND_PROTOCOL.md` §Ordering; research report §Control-plane responsibilities ("server receipt time only as a tiebreaker") | Server receipt time is server-controlled, so it conflicts with the NFR-001 threat model (a malicious server could reorder history). | Use signed hybrid logical clocks with an `item_id` tiebreak; bound accepted future skew (§5 AR-11). | N (protocol ADR) |
| DA-13 | G | `DATA_MODEL_AND_PROTOCOL.md` §Ordering; `APPROVAL_NEEDED.md` B-04 | No server retention bound for envelopes, no behavior for a device that is offline for months, no tombstone horizon. | Bound server retention to the history window plus tombstone horizon; returning devices receive a snapshot (§5 AR-18). | Y (B-04) |
| DA-14 | G | `PREMIUM_GATEWAY.md` §Temporary offline stage; `UX_SPEC.md` §History actions | Only the origin can upload a staged copy, so the source must be online. The docs do not say whether a non-origin device may *request* staging or what the UI shows when the origin is offline. | Specify who may request staging and the resulting states. | Y |
| DA-15 | F | `README.md` §Evidence and provenance; `RESEARCH_SOURCES.md` §Primary research input | Both say the report will be committed "when the owner supplies/commits it". It is already in the repository. | **Corrected in this PR.** | N |
| DA-16 | F | `docs/research/onecopy-deep-research-2026-11-12.md` | 59 unresolvable citation markers (82 distinct IDs). The claims cannot be traced to sources. | **Noted in `RESEARCH_SOURCES.md` in this PR.** Re-source any claim before using it in an ADR. | N |
| DA-17 | F | `TRANSFER_AND_ROUTING.md` §Transport experiment | The iroh description is accurate but incomplete. The relay is also the hole-punch coordinator, payload flows over it first, and iroh 1.0 has shipped [V: S1–S5]. | **Addendum added in this PR.** | N |
| DA-18 | F | `PHASE_0_VALIDATION.md` §Proposed performance targets | The ≥90% direct-success target has no independent support. The largest public measurement found a *conditional* DCUtR hole-punch rate of 70% ± 7.1% [V: S8] (see Errata 1). | **Context note added in this PR.** The target itself is unchanged and is listed as OD-01/OD-06 input. | N |
| DA-19 | F | `PLATFORM_CAPABILITIES.md` §macOS | Three facts are missing: there is no clipboard-change notification (apps poll `changeCount`) [V/K: S17]; `accessBehavior` exists from macOS 15.4, but whether the prompt is enforced by default on 26 or 27 is unverified [S: S19, S20]; and Local Network privacy applies from macOS 15 [S: S24]. | **Addendum added in this PR.** | N |
| DA-20 | F | `PLATFORM_CAPABILITIES.md` §Linux Wayland | No compositor snapshot is given. | **Addendum added in this PR** (from S9). | N |
| DA-21 | F | `PLATFORM_CAPABILITIES.md` §Windows | Four facts are missing: the clipboard-history and cloud-clipboard exclusion formats; `FILEDESCRIPTORW.cFileName` is `MAX_PATH`; the descriptor list is materialized up front; and `OleFlushClipboard` copies storage-medium formats into memory [V: S13–S15]. | **Addendum added in this PR.** | N |
| DA-22 | F | `PREMIUM_GATEWAY.md` §Storage economics; research report §Live relay economics | The research quotes Hetzner's 20 TB of included traffic, which applies to EU locations. US locations include about 1–5 TB, and Singapore about 0.5–8 TB with roughly 7× higher overage [S: S35]. R2 prices were re-verified and are unchanged [V: S34]. | **Addendum added in this PR.** | N |
| DA-23 | G | `PREMIUM_GATEWAY.md`; `PRD.md` | Nothing prevents abuse of the Free history channel as a free small-data relay (splitting data into many items). | Add per-account rate and size quotas to Free history sync. | Y |
| DA-24 | G | `PRD.md` NFR-006; `DATA_MODEL_AND_PROTOCOL.md` §Versioning | "Fail safely" is specified, but there is no compatibility window (for example, current and N-1 must interoperate). | Define a support window and a forced-upgrade policy. | Y |
| DA-25 | G | `PRD.md` nonfunctional requirements | No energy or footprint targets for an always-on agent: idle CPU, RAM, wakeups, and the cost of macOS `changeCount` polling. | Add NFRs with measurement methods. | N |
| DA-26 | A | `PRD.md` PR-001 ("native client"); `APPROVAL_NEEDED.md` B-06 | "Native" could mean a per-OS toolkit (WinUI/AppKit/GTK) or an installed app with native OS adapters and a shared desktop UI. For a solo developer that is roughly a 3× difference in desktop UI work. | Decide before Phase 1 (OD-12). | Y |
| DA-27 | A | `DECISIONS.md` D-003, D-018; `PRD.md` PR-020 | Several OneCopy-owned, OS-sanctioned extension points are unclassified: a GNOME Shell extension, an Android IME, a macOS/iOS File Provider extension, Windows Cloud Files. None is a third-party-app plugin, but D-003 is read broadly. | Clarify that D-003 targets per-*third-party-application* plugins. OneCopy's own OS-level extensions are allowed case by case, each requiring an ADR. | Y |
| DA-28 | A | `PRODUCT_VISION.md` §Not OneCopy ("universal remote filesystem mount") | A File Provider or Cloud Files placeholder domain (AR-04) is a filesystem surface. The boundary between "history items exposed as files" and "a mount" is undefined. | Allow only current history items' file references, with no browsing of the source filesystem and no write-back. | Y |
| DA-29 | G | `PHASE_0_VALIDATION.md`; `ROADMAP.md` | No equipment, procurement or cost plan. Internet NAT tests need at least one public host, which is a paid resource that `CLAUDE.md` does not allow Claude to provision. macOS/iOS work needs Apple hardware and, likely, an Apple Developer Program membership [K]. The development machine is Windows 11 Home, which has no Hyper-V or Windows Sandbox [K]. | Approve an equipment and budget list (§7.5). | Y |
| DA-30 | A | `DECISIONS.md` D-013; `APPROVAL_NEEDED.md` A-04; research report §Client split ("C# / WinUI 3 plus Win32/COM bridge") | A "stable C ABI" and a C# outer layer are both suggested. No document evaluates writing the Windows adapter in Rust (`windows-rs` can implement COM interfaces [K]), which removes one FFI hop. | Add it as an experiment option (§5 AR-08). | N |
| DA-31 | G | `CLAUDE.md` §Code and PR standards ("documented workspace dependency policy") | The dependency policy document does not exist yet. | Create it with the first code PR, not in this documentation PR. | N |
| DA-32 | G | `DATA_MODEL_AND_PROTOCOL.md` §Principles ("maximum supported object sizes") | The requirement exists but no numbers are given. | Set limits in the protocol ADR. | N |
| DA-33 | A | `UX_SPEC.md` §History actions ("invoke a native paste workflow only when supported and authorized") | It is unclear whether OneCopy may ever synthesize keystrokes (for example Ctrl+V via `SendInput`). On macOS that requires the Accessibility permission [K]; on Wayland it needs a portal. Either is a large permission expansion. | Decide (OD-15). The recommended default is no keystroke injection. | Y |
| DA-34 | G | `GLOSSARY.md` | Missing terms: signaling, address discovery, coordination relay, promotion (history → OS clipboard), transient source, source capability lifetime. | Add once DA-03 and DA-05 are decided. | N |
| DA-35 | A | `PRD.md` UJ-06 vs `ROADMAP.md` Phase 1/Phase 4 | UJ-06 says to "explain Premium relay as an option" in a Phase 1 that has no Premium. | Phase 1 explains the failure; Premium messaging arrives in Phase 4. | N |
| DA-36 | A | `PHASE_0_VALIDATION.md` E-13; `CLAUDE.md` ("Do not create cloud services") | E-13 "storage and relay pricing/throughput experiments" needs paid R2 and relay resources. The R2 free tier covers 10 GB-month [V: S34], which is not enough for 100 GB tests. | In Phase 0, limit E-13 to desk research; move measurements to Phase 4 preparation (§5 AR-19). | Y |

---

## 3. Technical research findings

### 3.1 Windows

- **Capture** via `AddClipboardFormatListener` is event-driven and per user session (existing doc, correct).
- **Exclusion formats** [V: S13]. Windows defines three registered formats with documented meanings: `ExcludeClipboardContentFromMonitorProcessing` (exclude from history and cloud sync), `CanIncludeInClipboardHistory` (DWORD 0/1) and `CanUploadToCloudClipboard` (DWORD 0/1). Password managers commonly set the first [K]. OneCopy should treat it as "do not capture" and set these formats when promoting sensitive items, so that the item does not leak into Windows Clipboard History or Cloud Clipboard.
- **Virtual files** [V: S14, S16]. Microsoft recommends `IStream` for `CFSTR_FILECONTENTS` ("an interface pointer might be the only practical data transfer mechanism" for very large files). Two constraints are not in the docs:
  - `FILEDESCRIPTORW.cFileName` is `WCHAR[MAX_PATH]`, so relative paths inside a pasted virtual folder tree are limited to 259 characters.
  - `CFSTR_FILEDESCRIPTOR` is a single `FILEGROUPDESCRIPTOR` HGLOBAL that lists *all* entries before any content is read. A 100,000-file folder needs the full remote manifest during a synchronous `GetData` call, and the descriptor alone is tens of MB (about 600 bytes per entry [K]). This conflicts with "enumerate lazily" (`TRANSFER_AND_ROUTING.md` §File and folder streaming).
- **Clipboard lifetime** [V: S15]. While the source process runs, the clipboard holds only a pointer to the data object. `OleFlushClipboard` renders formats into memory (the documented example copies an `IStorage` "into memory") so they survive process exit. The agent must **never** flush a virtual-file data object; on exit it should clear the clipboard with `OleSetClipboard(NULL)`. Any UI framework helper that flushes automatically must be avoided [K]. A crashed agent means a failed paste.
- **Asynchronous extraction** (`IDataObjectAsyncCapability`) is documented for drag-and-drop [V: S16]. Explorer's behavior on clipboard paste must be measured.
- **Explorer owns the destination write** [I]. The following are unknown and are E-02 acceptance items: whether Explorer deletes the destination on an `IStream::Read` error, whether it applies Mark-of-the-Web, how it reports errors, and whether it retries.
- **Alternative: the Cloud Files API (`cfapi`)** [K]. These are Windows' sanctioned on-demand placeholders (the OneDrive model): hydration is resumable and controlled by the provider. Costs: sync-root registration, a visible Explorer location, and more code. Keep it as a fallback if E-02 shows false-success behavior.
- **Rust option** [K]. `windows-rs` can implement COM interfaces, so the clipboard and `IDataObject` adapter can live in the Rust agent, independent of the UI technology.
- **Firewall** [K]. An inbound UDP listener triggers a Windows Defender Firewall prompt unless the installer registers a rule. This affects LAN P2P and needs a test.
- **Source consistency** [K]. Opening the source without `FILE_SHARE_WRITE` *prevents* concurrent writers for the duration of the transfer, so detection becomes prevention. The cost is that the user's apps get "file in use" errors. A file ID plus USN, `LastWriteTime` and size gives a stronger change token than mtime alone.

### 3.2 macOS

- **No change notification** [V/K: S17]. `NSPasteboard.changeCount` increments on ownership change, and AppKit has no change-notification API, so clipboard managers poll. The p95 < 100 ms capture target implies polling at roughly 50–100 ms, which has an energy cost and should get an NFR (DA-25).
- **Pasteboard privacy** [V: S18; S: S19, S20]. `accessBehavior` (ask, always allow, always deny, default) exists from macOS 15.4, and detection methods let an app inspect types or patterns without reading content. In the 2025 developer preview the prompt had no "always allow" choice [S: S19]. One developer reports that the macOS 27.2 release notes remove the developer-preview switch but could not confirm that enforcement is on for every app [S: S20]. **Default enforcement on macOS 26.x and 27.x is UNVERIFIED** and must be tested on both (E-03). The design must handle `ask` and `deny` gracefully and must never attempt a workaround.
- **Sensitivity markers** [V: S21]. The nspasteboard.org conventions are `org.nspasteboard.ConcealedType` (confidential), `TransientType` (do not record), `AutoGeneratedType` (probably do not record) and `org.nspasteboard.source`. Honor them on capture and set them when promoting.
- **Finder `Command-V` with file promises** [S: S22]. Developers report that Finder never calls the promise's write step on paste, and Apple frames promises as a drag mechanism. Treat native Finder paste via promises as **likely unavailable**. E-03 should still test it, because it is cheap.
- **File Provider route** [V/S: S23]. For a dataless item, "the system calls `fetchContents(for:version:request:completionHandler:)` and saves the file's content" on access. Developer reports show Finder copies of dataless files triggering fetches, with batch quirks above about 2,000 files [S]. Candidate design: OneCopy writes a `public.file-url` that points to a placeholder in a OneCopy File Provider domain, and Finder's copy hydrates it on demand from the remote source. This needs an app extension, an app group and sandboxing; the domain is visible in the Finder sidebar (DA-28).
- **Local Network privacy** [S: S24]. Since macOS 15, apps need user approval to talk to LAN peers. Launch-agent edge cases and bugs are reported, and Terminal-spawned tools are exempt (which can mask the problem during development). E-03 and E-06 must test the shipping process type.
- **Durability** [K]. `fsync` does not flush the drive cache on macOS; `F_FULLFSYNC` does, at a cost. The journal-ordering spec must name the call for each OS.
- **APFS `clonefile`** [K] can create a transfer-scoped, copy-on-write, consistent source view almost instantly. That is a snapshot-like mechanism and needs approval (AR-13).
- **Competition.** macOS 26 added built-in clipboard history in Spotlight (off by default) [S: S39]. It is also another place where promoted items may be recorded.

### 3.3 Linux (X11 and Wayland)

- **X11** [K]. XFixes selection notifications allow background observation, and clipboard content is lost when the owner exits unless a manager takes it. File managers use `text/uri-list` and `x-special/gnome-copied-files`. There is **no lazy virtual-file mechanism**: a pasted file must be a real path. Lazy paste would require a FUSE-backed path, which is an architecture and packaging decision; otherwise use the explicit download-to-folder fallback.
- **`ext-data-control-v1`** [V: S9]. The protocol is still in "staging" ("currently in the testing phase").
  - **Supported:** KWin 6.7, Sway 1.11, Hyprland 0.52.1, COSMIC 1.0 beta, niri 26.04, labwc 0.20.2, river 0.5.0, Mir 2.26, phoc 0.55, Jay, Qtile, Mango, Treeland.
  - **Not supported:** Mutter 51 (GNOME), Weston 15, Muffin (Cinnamon), Wayfire 0.9, Cage, Gamescope, Louvre.
- **GNOME options, each with a cost:**
  1. **XWayland bridge.** Mutter synchronizes the X11 and Wayland selections [S]; how reliably a windowless X11 client can observe Wayland-native copies is unverified.
  2. **xdg-desktop-portal `Clipboard`.** It works only inside RemoteDesktop or InputCapture portal sessions [V: S10], so the consent dialog asks the user to allow remote control or input capture. That is misleading permission UX and, in the reviewer's judgment, against the spirit of D-010 [I].
  3. **A GNOME Shell extension** (policy question DA-27).
  4. **Manual capture.** A focused OneCopy window *can* read the selection, which allows a "send current clipboard" flow comparable to Android's manual send [K/I].
- **Sandboxing** [S: S12]. `security-context-v1` lets compositors withhold privileged protocols such as data-control from Flatpak clients. Packaging format therefore affects capability and must be included in the E-05 matrix.
- **Global shortcuts** on Wayland require the GlobalShortcuts portal or compositor support [K].

### 3.4 Android

- **Background clipboard reads have been blocked since Android 10** unless the app has focus or is the default IME (existing doc, correct). Android 13 closed the log-reading workaround as "working as intended" [S: S30]. KDE Connect falls back to a manual "Send clipboard" notification action or Quick Settings tile [S: S29].
- **`ClipDescription.EXTRA_IS_SENSITIVE`** is Google's recommended marker for sensitive clips [V: S28]. Honor it on capture where visible and set it on promotion.
- **User-initiated data transfer (UIDT) jobs** [V: S26]:
  - Available from Android 14 (API 34) and require `RUN_USER_INITIATED_JOBS`.
  - Must be scheduled while the app is visible (or under the background-activity-start exemptions) and must post a notification.
  - Are "unaffected by App Standby Buckets quotas", but are stopped when constraints fail, under thermal pressure or on low memory. Apps "must persist the data transfer state".
  - Android 16 meters ordinary jobs that start in the foreground and continue in the background, and recommends UIDT for user-initiated transfers [V: S27]. This fits explicit transfers and confirms the doc's direction.
- **Share intents** [K]. URI grants from `ACTION_SEND` are temporary. Persistable grants exist only through the Storage Access Framework (`ACTION_OPEN_DOCUMENT`).
- **Serving remote files lazily to other apps** [K]. `DocumentsProvider` plus `StorageManager.openProxyFileDescriptor` (API 26) supports seekable on-demand reads, which is the Android counterpart to virtual files.
- **IME precedent** [S: S38]. Microsoft SwiftKey syncs its clipboard with Windows Cloud Clipboard, so an IME-based design has market precedent (D-018 stays deferred).
- **Platform competitor.** An Android 17 "Universal Clipboard" for Android PCs is rumored and unshipped [S: S40].
- **Prohibited.** Capturing the clipboard through an accessibility service would breach D-010 and Play policy [K].

### 3.5 iOS and iPadOS

- **Paste privacy.** Programmatic reads prompt the user ("Allow Paste") since iOS 16, and there are per-app "Paste from Other Apps" settings since iOS 16.1 [S: S41]. `UIPasteControl` gives a user-intent paste without a prompt [K].
- **Writing** can use `localOnly` (no Universal Clipboard) and `expirationDate` [K]. Use both for sensitive promotions.
- **`BGContinuedProcessingTask`** [V: S25] (iOS and iPadOS 26.0).
  - It starts in the foreground, shows progress in a system Live Activity, and the user can cancel it. "The system can terminate a continuous background task abruptly depending on run-time conditions … [and] prioritizes the termination of tasks that reflect minimal or no progress".
  - One developer reports an upload expiring when the device locked on 26.1 and 26.2 beta [S].
  - **This changes the doc's "iOS P2P is foreground-only" assumption to "test it":** a user-initiated P2P transfer may be able to continue after the user leaves the app.
- **Background `URLSession`** remains HTTP/HTTPS only, with file-backed uploads (existing doc, correct).
- **Share extensions** get transient access and tight memory limits (numbers not re-verified [K]). Heavy work should be handed to the main app through an app-group queue.
- **A File Provider extension** is the natural "Files integration" surface [K].
- **Local Network privacy** prompts and Bonjour declarations apply to LAN P2P [K].
- **Multi-process key-state hazard** [I]. If the app and an extension can both encrypt (sender counters, MLS secret-tree ratchets), nonces can be reused. Make the main app the only encrypting writer.

### 3.6 Shared Rust core, FFI, process model and persistence

- **Language.** Rust targets all five OS families [K]. The async runtime is effectively chosen by the transport: `quinn`, iroh and its `noq` QUIC layer are tokio-based [K].
- **UniFFI** [V: S31]. The latest release is 0.32.0 (2026-06-30). Swift 6 support is partial: generated async code is not `Sendable` (issue #2448), and foreign-trait implementations must be `Sendable`. Async support is still incomplete in places. C# bindings are not first-party; `uniffi-bindgen-cs` is third-party [K, not verified]. This supports the docs' advice to keep the boundary coarse.
- **Panics** [V: S32; K]. Since Rust 1.81 a panic escaping an `extern "C"` function aborts the process instead of causing undefined behavior. That is still a crash, so every boundary needs `catch_unwind` plus structured errors (UniFFI-exported functions convert panics [K]).
- **Simplification: one message schema** [I]. Define the "local command/event API" as a single versioned message protocol (commands in, events and snapshots out) that can travel in-process over FFI or out-of-process over IPC. That means one schema, one compatibility story and less binding surface. The trade-off is less generated type safety, which UniFFI can still provide by carrying the messages.
- **Desktop process model** [I/K].
  - Prefer one per-user agent process that owns the core, the OS adapter and the tray. On Windows the `IDataObject` must live in a long-lived STA thread of that process anyway.
  - If the UI is a separate process, the IPC must authenticate the caller: a named pipe ACL'd to the user SID plus client image verification on Windows; XPC with a code-signing requirement on macOS; a Unix socket with `SO_PEERCRED` on Linux.
- **Persistence** [K].
  - SQLite (`rusqlite`) is reasonable.
  - For local database encryption (OPEN), compare SQLCipher against row-level AEAD with an OS-keystore-wrapped key.
  - On iOS the Data Protection class decides whether extensions or background tasks can open the database; choose "until first user authentication" deliberately.

### 3.7 Networking and transfers

**iroh facts** [V: S1–S5; S: S6, S7]:

- iroh 1.0 was released on 2026-06-15. It is wire-stable across v1, has its own QUIC multipath and QUIC NAT traversal, and ships official Swift and Kotlin bindings.
- The default is four public n0 relays, which are rate-limited and described as free for development and testing.
- Relays forward end-to-end encrypted packets but see IP addresses, which endpoints talk to each other, and how much.
- The relay is how peers "find each other and coordinate the simultaneous connection attempt". Data starts over the relay: "The entire point of the relay path is to be able to start communicating without needing holepunching."
- `RelayMode::Disabled` removes the fallback, peer introduction and hole-punch coordination, and "roughly one connection in ten stops working". The relay is self-hostable.
- Recent issues report path-selection problems in 1.4.0 and stalls with relays off [S].

**Implication** [I]: unmodified iroh conflicts with the Free rule in two ways. Payload can cross the relay, and the default relays are third-party servers, which is the "library defaults" hazard `CLAUDE.md` warns about.

**Ways to satisfy D-008:**

- **A. Self-hosted iroh relay, capped for Free.** Free sessions get a tiny byte and time budget, enough for handshake and NAT-traversal frames. The app opens no payload streams until the selected path is direct, which requires an iroh API that exposes the path type (unverified). The relay's byte counters provide evidence. This needs the owner to classify a capped coordination relay as signaling.
- **B. iroh with `RelayMode::Disabled`.** Candidates are exchanged over OneCopy signaling, plus STUN or QAD-style address discovery and a coordinated simultaneous dial. Whether iroh hole-punches across NATs without a relay path is **unknown** (E-06).
- **C. Plain QUIC (`quinn`) plus ICE-style connectivity checks.** STUN binding plus signaling carry candidates, and connectivity checks go peer to peer [K]. No server ever forwards a Free packet. Engineering cost is higher, but the policy boundary is unambiguous.

**Hole-punch success.** Trautwein et al. measured libp2p DCUtR across 4.4M+ attempts from 85,000+ networks [V: S8 abstract/summary]:

- The *conditional* hole-punch success rate was 70% ± 7.1%, counting only attempts that reached the hole-punching stage.
- TCP and QUIC were statistically indistinguishable.
- 97.6% of successful connections succeeded on the first attempt.
- Success was independent of relay characteristics.

This is neither an end-to-end connection rate nor a OneCopy prediction. iroh self-reports about 9 in 10 [V: S1]. Independent data therefore does not *support* a universal ≥90% Free Internet direct-success target; OneCopy's rate must be measured per network class. Because success is independent of the relay, a signaling-only design (A without payload, B or C) should not be inherently worse at hole punching than relay-coordinated designs [I].

**LAN is not guaranteed either** [K]: Wi-Fi client isolation (guest, hotel and enterprise networks), the macOS and iOS Local Network permission, and Windows Firewall can all block it.

**UDP-blocked networks.** Free fails, as documented. A Premium relay carried over HTTPS/443 (iroh-relay's design [K]) works there.

**Encryption layering** [I/K]:

- For live P2P or relayed transfers, QUIC's TLS between authenticated device keys already gives end-to-end confidentiality and integrity. A second application-layer encryption of the payload is unnecessary cost.
- Keep per-chunk digests for on-disk verification and resume. Use keyed BLAKE3 with a per-transfer key so digests are not global content identifiers (consistent with the "no raw hashes" rule).
- For staging, use a standard streaming AEAD (the STREAM construction as used by the age v1 format, or Tink Streaming AEAD) rather than an ad-hoc chunk scheme.

**Integration with native paste** [I]. The receiver should not release the final bytes to Explorer or File Provider until the sender has delivered a "source unchanged" trailer that follows the post-read revalidation. Otherwise OneCopy itself would signal a clean end of stream for inconsistent data. The trailer does **not** determine what the consuming application does with a partially written destination file after an error; that must be measured (E-02). *(Corrected per Errata 2.)*

**Source-consistency hardening** [K]:

- POSIX `ctime` cannot be set from user space (unlike mtime), so use (device, inode, size, mtime, ctime).
- On Linux, inotify or fanotify on the opened file *during* a transfer can raise early aborts.
- Writers using memory maps update timestamps lazily, which leaves a residual race to document.
- Sources on SMB/NFS, and cloud placeholders (OneDrive Files On-Demand, iCloud "Optimize Storage"), behave differently: opening a dataless file triggers a hidden download. These belong in the E-08 corpus.

**Destination safety** [K]:

- Add these to the path rules: FAT32's 4 GiB file limit (detect before transfer); free-space checks and preallocation; Windows trailing dots and spaces, 8.3 aliases and ADS `:`; macOS NFD normalization; volume-specific case folding.
- Never preserve setuid or setgid bits.

**Routing simplification** [I]. With QUIC multipath picking the lowest-latency direct path, Free routing in Phase 1 reduces to "best measured direct path". The scoring formula and 25% hysteresis only matter once a Premium relay exists. Defer the selector, keep the measurements.

### 3.8 Security and privacy

**Group keys (D-014)** [S: S33; K]:

- OpenMLS has completed a third-party audit (SRLabs). A critical authentication bypass (CVSS 9.8) was fixed in 0.7.2 in February 2026.
- mls-rs is conformance-tested but, per its README, has had no full third-party audit.
- MLS needs a delivery service that orders commits, and its per-sender ratchet state must have a single writer (§3.5).
- The epoch-secret design resembles Keybase's per-user-key model [K]. It is simpler, but it is still a *custom protocol* assembled from standard primitives and needs the same external review.
- **Recommendation:** keep D-014 OPEN. E-11 builds the epoch design behind a `GroupKeyProvider` interface and runs a time-boxed OpenMLS spike that measures state size, commit-ordering needs and multi-process rules. Decide before Phase 1.

**Enrollment** [K]. Use an established ceremony:

- a QR code carrying the new device's key commitment, scanned by an existing device; or
- for desktop to desktop, a commitment-then-reveal numeric comparison (as in Bluetooth LE Secure Connections numeric comparison or ZRTP's SAS) or an audited PAKE.

Do not invent the ceremony.

**Roster** [I]. Use a hash-chained, device-signed roster log with a monotonic version that every device pins. Add explicit revocation authority (for example any trusted device may revoke, recovery-key holders may override, a revoked device cannot sign) and a freshness bound before authorizing a transfer when the control plane is unreachable.

**Identity separation** [I]. Keep the long-term device signing key separate from the transport key (iroh EndpointId or TLS raw public key). The roster binds them, which lets transport keys rotate.

**OS-level leak paths** [V/S/K]:

- Windows Cloud Clipboard and Clipboard History, macOS Universal Clipboard and Spotlight history, iOS Universal Clipboard, keyboard-app clipboard histories.
- Crash dumps, swap and hibernation files.
- Secrets in memory should use `zeroize`-style handling.

**Received files** [K]. Apply MOTW or quarantine (AR-12), never preserve setuid or setgid, and set an executable-bit policy.

**Clipboard poisoning** [I]:

- Previews must reveal bidirectional controls and zero-width characters (Trojan-Source-style deception).
- Offer "promote as plain text" for HTML and rich text.
- Always show the origin device.
- A compromised enrolled device can inject plausible commands into every device's history; showing the origin helps, and manual promotion is the main control.

**Supply chain** [K]:

- Use `cargo-deny`, `cargo-audit` and `cargo-vet`.
- Pin the rustls crypto provider deliberately. `aws-lc-rs` and `ring` have different build and maintenance trade-offs for mobile cross-compilation; verify the current status before pinning.

**Compliance** [K]. App-store encryption export declarations and GDPR obligations need a legal review before beta.

### 3.9 Infrastructure and operations

**Control plane** [I]. Keep it minimal:

- account authentication;
- an opaque store for signed roster blobs;
- a per-account envelope queue bounded to the history window plus tombstones;
- a WebSocket for presence and signaling;
- push through APNs/FCM with no plaintext.

Free LAN transfers should keep working during a control-plane outage, within the roster-freshness bound.

**Phase 0 hosting.**

- A local or LAN blind test server is enough for E-11.
- Internet NAT tests (E-06 field part, E-07) need **one public host**: rendezvous plus STUN or QAD, plus an optional capped relay. That is a small paid VPS, and the owner must approve and provision it.

**Costs** [V: S34; S: S35]:

- R2 Standard is unchanged: $0.015/GB-month, Class A $4.50 per million, Class B $0.36 per million, free egress. The free tier is 10 GB-month, 1M Class A and 10M Class B operations.
- Hetzner's included traffic varies by region, so an Asia relay pool is materially more expensive per TB than EU or US.
- R2 multipart limits (part size and part count) must be re-verified before choosing staging part sizes [K].

**Abuse.**

- The Free history channel can be used as a covert relay (DA-23).
- The Premium relay is limited to a user's own enrolled devices, but account sharing could turn it into a distribution channel. Set an account-sharing policy and per-account concurrency limits.

### 3.10 Competition and product feasibility

| Product | What it covers | Gap OneCopy targets |
|---|---|---|
| Apple Universal Clipboard and macOS 26 Spotlight clipboard history [S: S39] | Seamless within Apple's ecosystem; local history | Not cross-ecosystem; small items |
| Windows Cloud Clipboard, SwiftKey on Android, Phone Link [S: S38] | Text sync through a Microsoft account. The SwiftKey cloud clip is reported to last 1 h; Phone Link sync depends on the phone maker | Vendor cloud, no E2EE claim, no Linux, no huge-file paste |
| KDE Connect / GSConnect [S: S29] | Open-source LAN clipboard and file sharing; Android → PC requires a manual send | LAN-centric, no history mesh over the Internet |
| LocalSend [K] | LAN file and text send on five OSes, no account | No history, no Internet mesh, no native paste |
| Tailscale Taildrop [S: S36] | Encrypted transfer between a user's own devices. Still **alpha** per docs updated 2026-01-05; resumes for up to 1 h, except when macOS or iOS receives | A send workflow, not a clipboard |
| ClipCascade [S: S37] | Open source; Windows, macOS, Linux, Android; server or P2P modes; claims E2EE; text, images, files; GPL-3.0 | No audit found; no native lazy paste |
| Android 17 "Universal Clipboard" [S: S40] | Rumored, for Android PCs | Platform-vendor commoditization risk for text |

**Differentiation holds** [I]: cross-ecosystem, E2EE history plus lazy huge-file paste plus live references with no upload. The most fragile differentiator is native huge-file paste, which is strong only on Windows until AR-04 is proven.

**Promises that are unrealistic or costly relative to their value** [I]:

1. Native lazy paste on macOS and Linux "like Windows". It needs File Provider and FUSE-class components.
2. Automatic capture on GNOME Wayland and on mobile.
3. ≥90% Free Internet direct success.
4. Unlimited-size transfers from iOS without the user staying in the app, until `BGContinuedProcessingTask` is validated.
5. Free image sync through the control plane without size caps.

---

## 4. Risk register

Scales: severity (S) and likelihood (L) are 1–5; priority = S × L. "Exp." lists the Phase 0 experiment that tests the risk (see §7).

| ID | Risk | S | L | P | Impact | Validation / mitigation | Exp. |
|---|---|---|---|---|---|---|---|
| R-02 | The chosen transport cannot *prove* that no Free payload crosses a managed relay, or iroh exposes no path control | 5 | 3 | 15 | Breach of a CONFIRMED business rule, or the transport choice is reworked later | Two-track E-06 with packet and byte-counter evidence; OD-01 | E-06 |
| R-01 | Free Internet direct success too low for some network classes (OneCopy-specific rate unknown; see Errata 1) | 4 | 4 | 16 | Free UX and perception; support load | Emulated NAT matrix, then a field trial; class-based honest UX | E-06, E-07 |
| R-09 | Promoted sensitive items leak into OS or keyboard clipboard history or cloud sync | 4 | 4 | 16 | Privacy breach outside OneCopy's E2EE | Set and honor markers (AR-05); tests per OS | E-01, E-03, E-04 |
| R-14 | Solo-developer scope: five native shells, a core and a server | 4 | 4 | 16 | Schedule and quality | Sequencing (§7); shared desktop UI decision (OD-12) | — |
| R-05 | GNOME Wayland auto-capture impossible without an extension | 3 | 5 | 15 | Weak Linux story on the default desktops | Support tiers (AR-15); OD-13 | E-05 |
| R-12 | Mobile live references are transient, so D-005 semantics collide with share-sheet reality | 3 | 5 | 15 | User confusion; policy drift | AR-16; OD-03 | E-12 |
| R-03 | Native paste leaves a truncated destination or reports success after a failure | 4 | 3 | 12 | Violates "never false success" | E-02 measurement of partial-file outcomes; managed-download routing; the consistency trailer covers only OneCopy's own end-of-stream signal | E-02 |
| R-04 | Finder `Command-V` impossible without File Provider | 3 | 4 | 12 | macOS UX below Windows | Test promises, then File Provider; OD-05 | E-03 |
| R-06 | macOS pasteboard privacy enforced, so auto-capture is prompted or denied | 4 | 3 | 12 | Degraded macOS capture | Onboarding; detection APIs; test on 26 and 27 | E-03 |
| R-08 | Stale roster or unclear revocation authority on LAN | 4 | 3 | 12 | A revoked device is still served | AR-10; property tests | E-11 |
| R-10 | Source-consistency false negatives (mmap writers, coarse mtime, SMB/NFS, cloud placeholders) | 4 | 3 | 12 | Silent mixed-version output | Extended mutation corpus; change tokens; optional deny-write | E-08 |
| R-16 | Local Network permission, firewall prompts or AP isolation block LAN | 3 | 4 | 12 | "Same network" failures | Tests per OS and process type; clear UX | E-06, E-03 |
| R-07 | Flaw in the group-key protocol | 5 | 2 | 10 | Confidentiality loss | Interface plus external review; OpenMLS spike | E-11 |
| R-11 | Durability or journal bugs corrupt resumed files | 5 | 2 | 10 | Silent corruption | Crash-injection harness; per-OS fsync semantics | E-09 |
| R-13 | Free-tier cost or abuse through the history channel | 3 | 3 | 9 | Server cost | Inline cap and quotas (AR-06) | — |
| R-15 | Young dependencies (iroh 1.x path-selection issues; UniFFI Swift 6 gaps) | 3 | 3 | 9 | Rework | Pin versions; narrow interfaces; track advisories | E-06 |
| R-18 | Concurrent key-state writers (app plus extension, agent plus UI) reuse nonces | 4 | 2 | 8 | Cryptographic break | Single-writer rule; tests | E-11 |
| R-17 | Windows virtual-folder limits (`MAX_PATH`, descriptor built up front) | 2 | 5 | 10 | Deep or large folders fail native paste | Fall back to managed download | E-02 |
| R-22 | Compromised enrolled device injects malicious clipboard items | 3 | 2 | 6 | Harmful pastes | Origin display; plain-text promotion; manual promotion | — |
| R-19 | Compliance (GDPR, export declarations) | 3 | 2 | 6 | Launch delay | Legal review before beta | — |
| R-20 | Premium relay economics outside the EU and US | 2 | 3 | 6 | Margin | Regional measurement in Phase 4 | (E-13 desk) |
| R-21 | Update or supply-chain compromise | 5 | 1 | 5 | Fleet compromise | Signed updates, SBOM, `cargo-vet` | — |

---

## 5. Proposed architecture refinements

Each item states the current design, the proposed change, advantages, disadvantages, and whether owner approval is required. **None is applied to the baseline by this PR.**

### AR-01 — Define the Free connection-establishment policy precisely

**Current.** PR-015 forbids managed payload relay; "signaling" is allowed but undefined.

**Proposed.** Free connections may use (a) control-plane messages (authentication, roster, candidate exchange, presence) and (b) address-discovery requests (STUN or QAD-like). The owner chooses whether (c), a byte- and time-capped coordination relay that carries no application streams, is also allowed. Write the rule as a testable invariant: "no OneCopy server forwards Free inter-device packets", or the capped variant.

**Advantages.** The rule becomes auditable and testable, and the transport choice follows from it.

**Disadvantages.** Option (a)+(b) only may rule out unmodified iroh for Free.

**Approval:** **Yes** (interpretation of a CONFIRMED rule).

### AR-02 — Run a two-track transport experiment

**Current.** iroh is evaluated first; ICE-style traversal is the fallback (D-012).

**Proposed.** E-06 compares track 1, iroh 1.x (self-hosted relay and the relay-disabled modes), with track 2, `quinn` plus ICE-style checks, against identical evidence: Free relay byte count, per-NAT-class success, setup time, code size and API stability.

**Advantages.** Avoids sunk cost in a library whose defaults conflict with the business rule.

**Disadvantages.** Roughly 1.5× the E-06 effort.

**Approval:** **Yes** (changes the PROPOSED experiment scope; ADR after E-06).

### AR-03 — Per-path delivery guarantees

**Current.** One set of guarantees (resume, journal, verify-then-rename) is implied for every path.

**Proposed.** Two delivery modes:

- **Managed download.** OneCopy writes the destination: full journal, resume, verify, then atomic rename.
- **Native paste** (Explorer `IStream`, File Provider, Android provider): OneCopy never signals a clean end of stream for inconsistent data. Partial-file handling and resume depend on the consuming application and are claimed only where E-02/E-03 demonstrate them. *(Corrected per Errata 2.)*

Items above a size threshold, or folders that exceed `MAX_PATH` or descriptor limits, default to managed download or show a choice.

**Advantages.** Every guarantee is honest and the native UX is kept where it is safe.

**Disadvantages.** Users see two behaviors and the threshold needs UX design.

**Approval:** **Yes** (PR-012 scope).

### AR-04 — Native lazy paste through OS file-provider frameworks, scoped to history items

**Current.** Finder `Command-V` relies on promises (OPEN D-016). Linux and Android paste are "desktop/app dependent".

**Proposed.** If E-03 confirms that promises fail, evaluate a macOS File Provider domain that exposes only current history file items as dataless placeholders; the pasteboard carries their file URLs. Later options are Windows `cfapi` (if E-02 fails) and Android `DocumentsProvider` with proxy file descriptors. Linux keeps download-to-folder unless FUSE is approved.

**Advantages.** These are the platform-sanctioned lazy-hydration paths, with system-managed progress and retry.

**Disadvantages.** A large component; sandbox and app-group work; a visible Finder location; it brushes against the "not a mount" boundary (DA-28).

**Approval:** **Yes.**

### AR-05 — Honor and set clipboard sensitivity markers on every OS

**Current.** PR-024 is PROPOSED and covers the platform "sensitive" flag only.

**Proposed.**

- **Capture:** treat as excluded any item carrying Windows `ExcludeClipboardContentFromMonitorProcessing` (or `CanIncludeInClipboardHistory`=0), macOS `org.nspasteboard.ConcealedType` or `TransientType`, or Android `EXTRA_IS_SENSITIVE`. Treat `AutoGeneratedType` as "skip by default".
- **Promotion:** for sensitive items, set the Windows exclusion formats, the macOS concealed and transient markers, Android `EXTRA_IS_SENSITIVE`, and iOS `localOnly` plus `expirationDate`.

**Advantages.** Prevents leaks into OS histories and clouds, and interoperates with password managers.

**Disadvantages.** Apps that misuse the markers will hide content from history.

**Approval:** **Yes** (D-019 default policy).

### AR-06 — Inline content cap and Free history quotas

**Current.** "Small" is undefined.

**Proposed.** Cap inline ciphertext at 256 KiB per item (to be tuned in E-11). Larger images and text become on-demand references fetched P2P. Apply per-account Free limits on item rate, bytes per day and retained envelopes.

**Advantages.** Bounded cost, no covert relay, and consistent with D-008.

**Disadvantages.** A large screenshot is unavailable when the origin is offline (for Free users).

**Approval:** **Yes.**

### AR-07 — Desktop process model and a unified command schema

**Current.** An agent plus UI with "authenticated local IPC", unspecified.

**Proposed.** One per-user agent process owns the core, the adapter and the tray. The UI runs in-process or as a separate, caller-authenticated process. The core API is a versioned command/event schema shared by FFI and IPC.

**Advantages.** Smaller attack surface; one binding contract; the Windows `IDataObject` lifetime is satisfied.

**Disadvantages.** Less generated type-safety unless the schema is wrapped.

**Approval:** No for experiments; **ADR before Phase 1.**

### AR-08 — Windows adapter in Rust

**Current.** A C ABI or a C# bridge (D-013).

**Proposed.** Implement clipboard listening and `IDataObject`/`IStream` in Rust (`windows-rs`); keep the UI toolkit a separate decision.

**Advantages.** No FFI hop in the most security-critical adapter; one language for the agent.

**Disadvantages.** COM-in-Rust is less documented than C++ or C#.

**Approval:** No (experiment choice inside E-02); it updates D-013 later.

### AR-09 — Encryption layering

**Current.** Implied application-layer encryption plus transport encryption plus chunk hashes.

**Proposed.**

- Live transfers rely on QUIC/TLS between roster-bound device keys and use keyed BLAKE3 chunk digests.
- Staging uses a standard streaming AEAD (age-v1 STREAM or Tink Streaming AEAD).
- History envelopes keep application-layer AEAD (the server stores them).

**Advantages.** Less CPU, less custom construction, clearer review scope.

**Disadvantages.** Payload confidentiality now depends on correct TLS key binding, which must be tested.

**Approval:** **Yes** (security ADR plus external review).

### AR-10 — Roster integrity, revocation authority and freshness

**Current.** "Sign membership changes; monotonic epoch."

**Proposed.** A hash-chained, signed roster log, with these rules:

- Any trusted device may revoke another device.
- A recovery key can re-establish trust.
- A revoked or self-revoked key cannot sign.
- A transfer is authorized only if the server and peer roster versions agree, or the local roster was refreshed within a bound chosen only after a documented security/usability trade-off. *(Corrected per Errata 3.)*

**Advantages.** Closes the lockout and stale-roster gaps.

**Disadvantages.** Offline LAN-only use is limited after the freshness bound.

**Approval:** **Yes.**

### AR-11 — History ordering

**Current.** "Authenticated logical ordering"; the research suggests a server-time tiebreak.

**Proposed.** A signed hybrid logical clock per item with `item_id` as tiebreak, rejecting timestamps more than N minutes in the future. Server time is never trusted for ordering.

**Advantages.** Deterministic, server-independent, simple.

**Disadvantages.** Rare cross-device "copy order" surprises under clock skew.

**Approval:** No (protocol ADR).

### AR-12 — Provenance marking of received files

**Current.** Not covered.

**Proposed.** Managed downloads write Windows Zone.Identifier (MOTW) and macOS `com.apple.quarantine`. Record what Explorer and Finder do for native paste. Strip setuid and setgid.

**Advantages.** OS safety prompts apply to cross-device executables and documents.

**Disadvantages.** Friction ("this file came from another computer").

**Approval:** **Yes.**

### AR-13 — Source-consistency hardening options

**Current.** A stat-based detect-and-fail check.

**Proposed.**

- (a) Stronger change tokens: Windows file ID plus USN; POSIX ctime plus inode.
- (b) Inotify or fanotify on the open file during a transfer (Linux).
- (c) Optionally, open Windows sources without write sharing, which blocks writers during the transfer.
- (d) Optionally, a transfer-scoped reflink or `clonefile` on copy-on-write filesystems, deleted afterwards.

**Advantages.** Fewer false negatives; (c) and (d) turn detection into a guarantee.

**Disadvantages.** (c) blocks the user's apps with "file in use". (d) is snapshot-like and creates hidden temporary files on the source volume.

**Approval:** (a) and (b) No; **(c) and (d) Yes.**

### AR-14 — Device identity scope

**Current.** Undefined.

**Proposed.** Device = OS user profile × installation, with separate keys per OS user.

**Advantages.** Clear rules for shared PCs.

**Disadvantages.** More roster entries.

**Approval:** Confirm.

### AR-15 — Linux support tiers

**Current.** Per-compositor wording.

**Proposed.**

- **Tier 1:** auto-capture on X11 and on data-control compositors (KWin, Sway/wlroots family, Hyprland, COSMIC, niri).
- **Tier 2 (GNOME Wayland):** manual "send current clipboard" through a focused OneCopy window and global shortcut, with promotion supported.
- A GNOME Shell extension is a separate decision.

**Advantages.** Honest, shippable, testable.

**Disadvantages.** The default Ubuntu experience is manual.

**Approval:** **Yes.**

### AR-16 — Mobile "transient source" semantics

**Current.** Live reference with a warning.

**Proposed.** Mobile-shared files are live only while the grant or extension session is valid. The user may explicitly choose "Keep on this device until sent", a bounded, expiring copy in the app container that is deleted after transfer. That copy is never automatic.

**Advantages.** Usable mobile sharing without breaking the spirit of D-005.

**Disadvantages.** It is a copy and needs storage quotas and UX.

**Approval:** **Yes** (D-005 interpretation).

### AR-17 — iOS continued processing

**Current.** iOS P2P is foreground-only.

**Proposed.** Evaluate `BGContinuedProcessingTask` (iOS 26+) for user-initiated P2P transfers, including lock-screen behavior.

**Advantages.** Possible background P2P without Premium.

**Disadvantages.** The system can terminate it at any time, and lock-screen behavior is reported as fragile.

**Approval:** No (experiment).

### AR-18 — Bounded server retention

**Current.** Unspecified.

**Proposed.** The server keeps only the envelopes needed for the current history window plus tombstones up to a horizon. A returning device receives that set.

**Advantages.** Bounded storage and privacy exposure.

**Disadvantages.** Long-offline devices see only recent items.

**Approval:** **Yes** (B-04).

### AR-19 — Phase 0 gateway work is desk-only

**Current.** E-13 includes measurements.

**Proposed.** Price and limit research only. Measured R2 and relay benchmarks move to Phase 4 preparation.

**Advantages.** Respects `CLAUDE.md` (no paid resources) and focuses Phase 0.

**Disadvantages.** Premium pricing stays unknown longer.

**Approval:** **Yes.**

### AR-20 — Defer the route selector

**Current.** Scoring formula and hysteresis in the MVP.

**Proposed.** Phase 1 Free uses "best measured direct path" (QUIC multipath or simple RTT preference), filtered by entitlement. Scoring arrives with the Premium relay. Keep collecting the metrics.

**Advantages.** Less speculative code, and PR-014 is still satisfied by measured selection.

**Disadvantages.** Battery and cost-aware routing comes later.

**Approval:** No (simplification of a PROPOSED mechanism).

---

## 6. Open decisions (prioritized)

### P0 — before Phase 0 experiments start

| ID | Decision | Options | Recommendation |
|---|---|---|---|
| OD-01 | What may a Free connection use? (DA-03, AR-01) | (a) signaling plus address discovery only; (b) also a capped coordination relay with no app streams; (c) LAN-only Free | **(a) as the target policy.** Measure (b) only as an E-06 comparison. Hole-punch success appears independent of relay presence [V: S8], so (a) need not cost success. |
| OD-02 | Guarantees for native paste vs PR-012 (DA-04, AR-03) | Same guarantees everywhere; per-path guarantees; native paste only below a size threshold | **Per-path guarantees** plus threshold routing, tuned after E-02 |
| OD-03 | Mobile file-source semantics (DA-05, AR-16) | Live-only (often unavailable); explicit bounded keep-until-sent copy; automatic copy | **Explicit bounded copy**, never automatic |
| OD-04 | Wording of README invariant #4 (DA-01) | Keep as is; scope to copy-time and idle | **Scope it** (transfer-time detection allowed) |
| OD-05 | Which OneCopy-owned OS extensions D-003 allows; boundary with "not a mount" (DA-27, DA-28) | Strict (none); case by case with ADR; broad | **Case by case with ADR,** limited to current history items |
| OD-06 | Phase 0 equipment and budget (DA-29, §7.5) | — | Approve the list in §7.5 |
| OD-07 | Phase 0 gateway scope (DA-36, AR-19) | Measure now; desk only | **Desk only** |

### P1 — before Phase 1

| ID | Decision | Recommendation |
|---|---|---|
| OD-08 | Group key D-014 | Decide on E-11 evidence; external review either way |
| OD-09 | Revocation authority and roster freshness (AR-10) | Adopt AR-10, freshness bound to be tuned |
| OD-10 | Provenance marking of received files (AR-12) | Mark managed downloads; record native-paste behavior |
| OD-11 | Sensitive-content defaults including OS-history markers (D-019, AR-05) | Adopt AR-05 |
| OD-12 | Desktop UI strategy (B-06, DA-26) | Decide after Phase 0 with an effort estimate; consider one shared desktop UI with native adapters |
| OD-13 | Linux tiers and GNOME approach (AR-15) | Tiers as proposed; decide on a GNOME Shell extension separately |
| OD-14 | Windows deny-write share mode and transfer-scoped clones (AR-13 c/d) | Default to deny-write with a "transfer anyway" override; defer clones |
| OD-15 | Keystroke-injection ("auto-paste") permission scope (DA-33) | No injection; the user presses Ctrl+V or Cmd+V |
| OD-16 | Wire encoding (B-05) | Deterministic CBOR, signatures over exact transmitted bytes |
| OD-17 | Inline cap and Free quotas (AR-06) | 256 KiB starting point, tuned in E-11 |

### P2 — later

OD-18 device identity scope (AR-14); OD-19 compatibility window (DA-24); OD-20 staging initiation from a non-origin device (DA-14); OD-21 Premium quotas, regions and provider (B-07, unchanged).

---

## 7. Revised Phase 0 recommendation

### 7.1 Principles

- Run the cheapest *decisive* experiment first. Every experiment ends in a written evidence record in the existing `PHASE_0_VALIDATION.md` format, and failure is a valid result.
- Experiment code lives in an isolated `experiments/` tree. It is throwaway code, not product foundation, which keeps it within `CLAUDE.md` work-order steps 2 and 3.
- Nothing needs cloud resources until the owner approves the public test host (OD-06).

### 7.2 Order of risks

Riskiest and cheapest first:

1. **E-02, Windows virtual-file paste.** Highest product leverage, it runs on the machine already available, and it needs no network: use a synthetic throttled, faulting stream.
2. **E-06a, transport policy probe in an emulated NAT lab.** Linux network namespaces with nftables NAT variants; WSL2 is acceptable for the logic, VMs for performance. Both tracks (AR-02).
3. **E-05 (with E-04), Linux capture matrix in VMs.** GNOME and KDE on Ubuntu or Fedora, plus Sway; host versus Flatpak.
4. **E-08 to E-10, transfer engine over loopback and LAN.** Plain QUIC is enough at first, because transfer framing is transport-agnostic.
5. **E-11, trust and crypto prototype** with a local blind server and a malicious-server harness, plus the OpenMLS spike.
6. **E-03, macOS** (needs hardware): pasteboard states on 26 and 27, Finder promise versus File Provider, Local Network permission per process type.
7. **E-07, Internet NAT field trial** (needs the public host), **E-06b** field validation, and **E-12** mobile feasibility (needs devices).

E-13 is desk research and can happen at any time.

### 7.3 Dependencies

```text
OD-01 ──► E-06a ──► E-06b ──► E-07
OD-02 ──► E-02 (acceptance thresholds)      E-02 ──► Gate B (Windows)
E-05 / E-04 independent ──► Gate A (Linux)
E-08..E-10 need only "some QUIC" ──► Gate D
E-11 independent ──► Gate E (with external review later)
Hardware (OD-06) ──► E-03 ──► Gate A/B (macOS)
Devices (OD-06)  ──► E-12
Public host (OD-06) ──► E-06b, E-07 ──► Gate C
```

Independent experiments: E-02, E-05, E-08–E-10 and E-11. E-06b and E-07 depend on E-06a and the public host. E-03 and E-12 depend only on hardware.

### 7.4 Acceptance criteria (measurable)

| Exp. | Pass criteria (all must hold unless noted) | Evidence |
|---|---|---|
| E-02 | (1) Explorer pastes 1, 10 and 100 GB virtual files with agent RSS < 256 MiB, stable over time. (2) After a failure injected mid-stream, Explorer shows an error **and** no destination file of the full advertised size remains; record whether a partial remains and its name. (3) Cancel takes effect within 2 s. (4) Killing the agent mid-paste produces an error within 30 s, with no hang. (5) Time-to-first-byte measured for 1, 1k and 10k-entry folders. (6) Behavior at the `MAX_PATH` boundary recorded. (7) Agent exit does not materialize content (RSS check, no flush). (8) Zone.Identifier present or absent recorded. (9) At least one non-Explorer target recorded (desktop, a file-open dialog, an Office app). | Screen recording, RSS trace, destination listing, commit SHA |
| E-06a | For each emulated NAT class (endpoint-independent, address-dependent, endpoint-dependent; single and double NAT; IPv6), 100 attempts per track. Free mode shows **0** relayed application bytes (strict policy), or bytes ≤ the cap with no app streams (capped variant). Success rate and setup-time p50/p95 recorded per class. Relay disabled for Free at configuration level, verified by packet capture. | pcap summaries, relay counters, per-class table |
| E-05 / E-04 | A matrix of at least 3 compositor families (GNOME, KDE, wlroots) plus X11, each tested as a host install and as Flatpak: observe, write, owner-exit persistence, global shortcut, `text/uri-list` paste into the default file manager. Every cell has a reproduction command. | Matrix plus logs |
| E-08 | 50 runs per mutation type (append, overwrite, truncate, replace, delete, rename) × filesystem (NTFS, ext4, exFAT, SMB share; APFS when hardware exists): **zero** silent mixed-version completions. Detection latency recorded, and residual races documented. | Run logs and corpus hashes |
| E-09 | 100 GB on loopback and on 1 GbE LAN. Steady-state RSS < 256 MiB. 20 random `kill -9` points per side give 100% verified resume or a clean restart. Final digest matches 100% of runs. Resume retransmits ≤ 1 chunk plus the in-flight window. Throughput ≥ 80% of the measured baseline is recorded (a target, not a gate). | Traces and digests |
| E-10 | 24 CPU-hours of fuzzing on manifest and path parsers with no crash. The whole path corpus (Unicode, RTL, reserved names, ADS, case collisions, traversal, symlinks, special files) is accepted or rejected exactly per spec. Zero writes outside the destination root. | Fuzz stats, corpus results |
| E-11 | A blind server never sees plaintext (traffic inspection). The malicious-server harness rejects duplicate, reorder, replay, rollback, corrupted-ciphertext and forged-roster cases. 10⁵ property-test cases for idempotent sync, monotonic roster and revoked-device denial. Single-writer rule enforced. OpenMLS spike report. | Test reports |
| E-03 | `accessBehavior` states and prompt behavior recorded on macOS 26.x and 27.x. Polling energy measured. Finder `Command-V` result for (a) promise, (b) existing file URL, (c) File Provider placeholder. Local Network prompt behavior for an app versus a LaunchAgent. | Recordings and logs |
| E-07 | At least 100 attempts per network class (home, double NAT, CGNAT, IPv6, enterprise, hotel, hotspot, cellular), using a diagnostic CLI run by volunteers who give informed consent and whose logs are redacted. Per-class success with 95% confidence intervals. | Dataset and analysis |
| E-12 | Android: share-URI grant lifetime, UIDT behavior through app switch, screen-off and network change. iOS: share-extension access lifetime, `BGContinuedProcessingTask` with lock, Local Network prompt. | Device logs |

### 7.5 Environments and equipment (needs owner approval where marked $)

- **Windows:** the existing Windows 11 Home machine. Home has no Hyper-V or Windows Sandbox [K], so plan VMware Workstation or VirtualBox, or a second Windows device, for clean-room tests.
- **Linux:** VMs for GNOME, KDE and Sway (disable guest clipboard-sharing tools during clipboard tests). One physical Linux machine is recommended for throughput tests.
- **Disk:** ≥ 400 GB free on each machine in a 100 GB test, for source, destination and partial files.
- **Network:** a 1 GbE switch; a travel router for NAT and double NAT; a phone hotspot.
- **$ Apple:** an Apple-silicon Mac (macOS 26.x, plus a way to test 27.x); an iPhone on iOS 26.x; an Apple Developer Program membership, likely required for extensions, app groups and notarization [K, verify].
- **$ Android:** a device on Android 14 or later (ideally 16).
- **$ Public test host:** one small VPS (rendezvous, STUN or QAD, optional capped relay). Claude will not provision it.

### 7.6 Practical sequence for a solo developer with AI assistance (rough effort: S ≤ 3 days, M ≤ 2 weeks, L > 2 weeks; estimates, not commitments)

| Milestone | Content | Effort | Gate input |
|---|---|---|---|
| M0 | Baseline approval; OD-01 to OD-07; procurement | S | — |
| M1 | E-02 plus E-01 Windows capture, write, virtual paste, with report | M | Gate A (Windows), Gate B (Windows) |
| M2 | E-06a two-track NAT-lab report; draft transport ADR | M–L | Gate C (partial) |
| M3 | E-05 and E-04 Linux matrix | M | Gate A (Linux) |
| M4 | E-08 to E-10 transfer engine with fault and fuzz harnesses | L | Gate D |
| M5 | E-11 trust prototype, threat-model update, OpenMLS spike | M–L | Gate E (pre-review) |
| M6 | E-03 macOS | M | Gates A and B (macOS) |
| M7 | E-06b and E-07 field trial; E-12 mobile feasibility | M–L | Gate C; mobile wording |
| M8 | Phase 0 decision report and ADRs | S | Owner go/no-go |

### 7.7 Go/no-go refinements

- **Gate B.** Windows "go" requires E-02 (2): no full-size false-success file. Otherwise native paste is limited to managed download, or AR-04's `cfapi` path is evaluated. macOS "go" for native paste requires E-03 (c). Otherwise the owner accepts download-to-folder (A-05).
- **Gate C.** Replace the single ≥90% target with per-class targets set *after* E-06a and E-07 data, together with honest per-class UX. The no-Free-payload-relay evidence is mandatory and binary.
- **Gate D.** Unchanged, plus explicit per-OS fsync semantics in the journal spec.
- **Gate E.** Unchanged. Add a single-writer key-state rule and the roster-freshness tests.

---

## 8. Documentation changes

### 8.1 Factual corrections applied in this PR

These are clearly labeled "RESEARCHED — review addendum" and change no decision status.

| File | Section | Change |
|---|---|---|
| `README.md` | Evidence and provenance; Documentation map | The research report is committed (path given). Row added for this review document, marked "review input, not approved". |
| `docs/RESEARCH_SOURCES.md` | Primary research input; new "Independent review sources" | Report present. Unresolvable citation tokens noted (DA-16). Review sources added with access dates. |
| `docs/PLATFORM_CAPABILITIES.md` | New "Review addendum" | Windows (exclusion formats, `MAX_PATH`, descriptor built up front, `OleFlushClipboard`); macOS (`changeCount` polling, `accessBehavior` status, nspasteboard markers, Finder promise evidence, Local Network privacy); Wayland (compositor snapshot, GNOME, portal limits, sandboxing); Android (UIDT details, Android 16 quotas, sensitivity extra); iOS (`BGContinuedProcessingTask`, paste settings). |
| `docs/TRANSFER_AND_ROUTING.md` | Transport experiment | iroh facts addendum (1.0 status, relay role in hole punching, relay-disabled consequences, default public relays). |
| `docs/PHASE_0_VALIDATION.md` | Proposed performance targets | Independent NAT-traversal measurement note; target unchanged. |
| `docs/PREMIUM_GATEWAY.md` | Storage economics | R2 re-verified; Hetzner regional traffic note. |

### 8.2 Proposed changes not applied (need owner decision)

| Target | Proposed change | Depends on |
|---|---|---|
| `README.md` §Core invariants #4 | "No … change monitoring **at copy time or while idle**; transfer-time detection on the opened source is required" | OD-04 |
| `PRD.md` | New PR-028 (sensitivity markers both ways), PR-029 (inline cap and Free quotas), PR-030 (provenance marking), PR-031 (per-path delivery guarantees), NFR-010 (energy and footprint), NFR-011 (crash-dump hygiene), NFR-012 (compatibility window) | OD-02, OD-11, OD-17, OD-10, OD-19 |
| `TRANSFER_AND_ROUTING.md` | Free establishment policy; two-track transport; consistency trailer; per-OS durability calls; path-rule additions; defer route scoring | OD-01, AR-02, AR-03, AR-20 |
| `SECURITY_AND_PRIVACY.md` | Threat rows for local IPC, crash dumps, OS clipboard history and cloud, stale roster, concurrent key writers; roster log and revocation authority; encryption layering | AR-05, AR-09, AR-10 |
| `DATA_MODEL_AND_PROTOCOL.md` | HLC ordering; bounded server retention; transient source class; size limits | AR-11, AR-16, AR-18, DA-32 |
| `PLATFORM_CAPABILITIES.md` matrix | Linux tiering; macOS File Provider option; iOS continued processing as "experimental" | AR-04, AR-15, AR-17 |
| `UX_SPEC.md` | States for "transient source", "managed download vs native paste", GNOME manual capture, Local Network permission onboarding | OD-02, OD-03, OD-13 |
| `PHASE_0_VALIDATION.md` | Replace the slice order and acceptance criteria with §7; E-13 desk-only; equipment list | OD-06, OD-07 |
| `DECISIONS.md` | New entries (OPEN or PROPOSED) mirroring OD-01 to OD-21; D-012 updated to "two-track experiment"; D-013 adds the Rust Windows adapter option | Owner review |
| `APPROVAL_NEEDED.md` | Add the P0 items OD-01 to OD-07 to "Recommended near-term decisions" | Owner review |
| `GLOSSARY.md` | Signaling, address discovery, coordination relay, payload relay, promotion, transient source, managed download, native paste | DA-34 |
| `TRACEABILITY.md` | Rows for the new PR and NFR IDs once accepted | Acceptance |
| New `docs/DEPENDENCY_POLICY.md` | Created with the first code PR (not documentation-only) | DA-31 |

---

## 9. Sources

All accessed **2026-11-12**. [V] marks primary or vendor sources; [S] marks secondary sources.

| ID | Source |
|---|---|
| S1 | [V] iroh FAQ — https://docs.iroh.computer/about/faq |
| S2 | [V] iroh, Security & Privacy — https://docs.iroh.computer/concepts/security-privacy |
| S3 | [V] iroh, NAT traversal — https://docs.iroh.computer/concepts/nat-traversal |
| S4 | [V] iroh 1.0 announcement (2026-06-15) — https://iroh.computer/blog/v1 |
| S5 | [V] "iroh on QUIC Multipath" (2025-08-05) — https://www.iroh.computer/blog/iroh-on-QUIC-multipath |
| S6 | [S] iroh issue #4657 (relays off, address change) — https://github.com/n0-computer/iroh/issues/4657 |
| S7 | [S] iroh issue #4644 (path selection, 1.4.0) — https://github.com/n0-computer/iroh/issues/4644 |
| S8 | [V] Trautwein, Ihle, Schubotz, Gipp, "Challenging Tribal Knowledge – Large Scale Measurement Campaign on Decentralized NAT Traversal", arXiv:2510.27500 (IMC '26 version: "Large-Scale Measurement of NAT Traversal for the Decentralized Web") — https://arxiv.org/abs/2510.27500 ; https://probelab.io/publications/large-scale-measurement-of-nat-traversal-for-the-decentralized-web-a-case-study-of-dcutr-in-ipfs/ (figures taken from abstract and summary; full paper not reviewed) |
| S9 | [V] ext-data-control-v1 protocol and compositor table — https://wayland.app/protocols/ext-data-control-v1 |
| S10 | [V] xdg-desktop-portal Clipboard interface — https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Clipboard.html |
| S11 | [S] Reports of Mutter lacking data-control (e.g.) — https://github.com/anomalyco/hex/issues/1111 |
| S12 | [S] wayland-protocols security-context discussion — https://gitlab.freedesktop.org/wayland/wayland-protocols/-/issues/207 |
| S13 | [V] Win32 Clipboard Formats (cloud clipboard and history formats) — https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats |
| S14 | [V] FILEDESCRIPTORW — https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-filedescriptorw |
| S15 | [V] OleFlushClipboard — https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-oleflushclipboard |
| S16 | [V] Handling Shell Data Transfer Scenarios — https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios |
| S17 | [V] NSPasteboard.changeCount — https://developer.apple.com/documentation/appkit/nspasteboard/changecount |
| S18 | [V] NSPasteboard.AccessBehavior.alwaysAllow — https://developer.apple.com/documentation/appkit/nspasteboard/accessbehavior-swift.enum/alwaysallow |
| S19 | [S] Lapcat Software, "Making my app worse because of macOS privacy protections" (2025-05-14) — https://lapcatsoftware.com/articles/2025/5/3.html |
| S20 | [S] clipsody PR #15 (macOS 26/27 notes; enforcement unverified by author) — https://github.com/tillackt/clipsody/pull/15 |
| S21 | [V] nspasteboard.org marker types — http://nspasteboard.org/ |
| S22 | [S] Apple Developer Forums on file promises and pasting — https://developer.apple.com/forums/thread/683081 ; https://developer.apple.com/forums/thread/108902 |
| S23 | [V] File Provider, "Synchronizing the File Provider extension" — https://developer.apple.com/documentation/fileprovider/synchronizing-the-file-provider-extension ; [S] forum reports, e.g. https://developer.apple.com/forums/thread/720243 |
| S24 | [V] TN3179 Understanding local network privacy — https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy ; [S] Eclectic Light (2026-10-22) — https://eclecticlight.co/2026/10/22/local-network-privacy-is-getting-better/ |
| S25 | [V] BGContinuedProcessingTask — https://developer.apple.com/documentation/backgroundtasks/bgcontinuedprocessingtask ; [S] lock-screen report — https://developer.apple.com/forums/thread/796138 |
| S26 | [V] Android user-initiated data transfer jobs — https://developer.android.com/develop/background-work/background-tasks/uidt |
| S27 | [V] Android 16 behavior changes (all apps) — https://developer.android.com/about/versions/16/behavior-changes-all |
| S28 | [V] Android secure clipboard handling — https://developer.android.com/privacy-and-security/risks/secure-clipboard-handling |
| S29 | [S] KDE bug 428167, Android 10+ clipboard restrictions — https://bugs.kde.org/show_bug.cgi?id=428167 |
| S30 | [S] Android 13 log-access workaround closed — https://hothardware.com/news/automation-apps-broken-android-13-privacy-push-working-as-intended |
| S31 | [V] UniFFI changelog and Swift docs — https://raw.githubusercontent.com/mozilla/uniffi-rs/main/CHANGELOG.md ; https://mozilla.github.io/uniffi-rs/latest/swift/overview.html |
| S32 | [V] RFC 2945 "C-unwind ABI" — https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html |
| S33 | [S] OpenMLS audit mention and GHSA write-up; [V] mls-rs README — https://docs.rs/crate/mls-rs/0.51.0/source/README.md |
| S34 | [V] Cloudflare R2 pricing — https://developers.cloudflare.com/r2/pricing/ |
| S35 | [S] Hetzner traffic summaries — https://docs.hetzner.com/cloud/billing/faq/ ; https://egresscost.com/hetzner |
| S36 | [S/V] Tailscale Taildrop docs — https://tailscale.com/kb/1106/taildrop |
| S37 | [S] ClipCascade — https://github.com/Sathvik-Rao/ClipCascade |
| S38 | [S] Windows/Android clipboard sync coverage — https://9to5google.com/2025/09/03/microsoft-preparing-clipboard-sync-between-windows-and-android-with-gboard-support/ |
| S39 | [S] macOS Tahoe Spotlight clipboard settings — https://www.macrumors.com/2025/11/04/more-spotlight-clipboard-settings-macos-26-1/ |
| S40 | [S] Android 17 "Universal Clipboard" rumor — https://www.androidauthority.com/android-17-universal-clipboard-rumor-3618614 |
| S41 | [S] iOS 16.1 "Paste from Other Apps" — https://macrumors.com/2022/10/17/ios-16-1-paste-from-other-apps-settings |

### Not independently verified in this review

These items are [K] above and should be confirmed before they drive a decision:

- `cfapi` details; `windows-rs` COM implementation ergonomics; Windows Firewall prompt specifics.
- macOS `F_FULLFSYNC` semantics and `clonefile` behavior.
- Android URI-grant lifetimes and `openProxyFileDescriptor` behavior.
- iOS share-extension memory limits, `UIPasteboard` `localOnly` and `expirationDate`, `UIPasteControl`.
- R2 multipart limits.
- The current maintenance status of `ring` and `aws-lc-rs`.
- `uniffi-bindgen-cs` status.
- The research report's statement that WebAuthn Level 3 became a W3C Recommendation in August 2026.
