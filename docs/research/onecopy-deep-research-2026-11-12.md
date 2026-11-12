# OneCopy: Decision-Ready Architecture and Feasibility Research

## Executive assessment and product feasibility

OneCopy is technically feasible as a **high-quality desktop-first, mobile-assisted universal clipboard and on-demand file-transfer system**, but the exact experience cannot be identical on every operating system. Windows, macOS, and X11 Linux can support a persistent user-session agent that observes clipboard changes and injects clipboard content. Wayland is feasible only where the compositor exposes an appropriate clipboard-management protocol or portal; the core Wayland data-device protocol normally exposes selection changes only to the keyboard-focused client, while the newer `ext-data-control-v1` protocol explicitly permits privileged clipboard-manager behavior but is still marked as a testing protocol. citeturn129445search9turn236632search8turn236632search9

Android is a substantial limitation: on current Android, `ClipboardManager.getPrimaryClip()` returns `null` unless the app is the default input method or has input focus. Android's own security guidance explicitly describes Android 10+ as preventing background applications from accessing foreground-app clipboard data. Therefore a normal OneCopy Android app cannot silently observe every copy operation in the background. The practical UX is a combination of explicit Sharesheet actions, OneCopy history, paste-from-OneCopy actions, content URIs, and potentially an **optional OneCopy keyboard/IME** for users who want deeper integration. citeturn236632search6turn236632search7turn650196search5

iOS/iPadOS is even more constrained. `UIPasteboard` exists and supports multiple items, but programmatic reads can trigger privacy notifications unless iOS recognizes explicit user paste intent. Applications are normally suspended shortly after entering the background, and background pushes are low priority, throttleable, and not guaranteed. Background `URLSession` can continue transfers after app suspension, but only for HTTP/HTTPS, and background uploads must be file-backed. Consequently, a permanently running OneCopy P2P clipboard daemon is **not implementable on stock iOS**. The native iOS product should instead use Share extensions, user-triggered paste/copy interactions, notifications, foreground P2P where available, and Premium HTTPS gateway transfers for reliable long-running background transfers. citeturn236632search0turn236632search2turn165319search0turn165319search3turn165319search4

A newly important macOS constraint must also be treated as a first-class design issue. Apple's current AppKit documentation says the General pasteboard defaults to asking before non-user-originated programmatic access, with per-application `alwaysAllow`, `alwaysDeny`, and `ask` states. Apple added detection APIs that reveal metadata or patterns without triggering a pasteboard read. Continuous silent clipboard capture therefore cannot be assumed on modern macOS unless the user grants the required pasteboard access. citeturn755371search0turn755371search1turn755371search2

The architecture should consequently promise **"copy once, access anywhere the platform permits"**, not claim identical invisible clipboard interception on every OS.

### Recommended product contract

The requirements already chosen are internally coherent with one refinement: distinguish **clipboard synchronization**, **file references**, **file transfer**, and **temporary staging** as four different protocol concepts.

| Concept | Meaning | Default storage |
|---|---|---|
| Clipboard item | A logical entry in OneCopy's recent history | Small encrypted metadata/content |
| File reference | Source device + source locator + metadata | Metadata only |
| Transfer | Requested stream from current source contents | Destination only |
| Gateway staging | Explicit temporary Premium copy | Encrypted managed object storage |

A normal copied file should not be read immediately merely because the user pressed Copy. OneCopy records enough metadata to display the entry and later ask the origin agent to open the source **when a paste/transfer request occurs**. This preserves the intended "not backup, not drive" semantics and eliminates expensive speculative I/O.

The selected behavior for changed files is also technically cleaner: **the source is opened at transfer time and OneCopy sends the bytes available at that time**. It should not compare them with the historical copy event and should not prompt for versioning. The only correctness rule is that a transfer must never silently produce an incoherent combination of old and new bytes if the source changes while it is being read. That is a transfer-consistency problem, not a version-control feature.

### Platform feasibility matrix

| Platform | Observe normal system Copy | Write system clipboard | Remote/native file paste | Persistent agent | Overall |
|---|---:|---:|---:|---:|---|
| Windows | **Yes** | **Yes** | **Strong feasibility** via Shell virtual-file mechanisms | **Yes**, user-session process | Excellent |
| macOS | **Conditional on privacy permission** | **Yes** | **Promising**, especially file promises/drag; Finder Copy/Paste requires prototype | **Yes**, LoginItem/LaunchAgent | Good, permission-sensitive |
| Linux X11 | **Yes** | **Yes** | Desktop/toolkit dependent | **Yes** | Good |
| Linux Wayland | **Compositor/protocol dependent** | **Compositor/protocol dependent** | Desktop/compositor dependent | Yes, but clipboard access is the blocker | Variable |
| Android | **No universal background observation** | Yes when app active | Content URI / Sharesheet / provider UX | Transfer jobs/services only within OS rules | Useful but not desktop-equivalent |
| iOS/iPadOS | **No reliable continuous background monitoring** | Yes during app interaction | Share extension, Files integration, user-driven paste | No persistent arbitrary network daemon | Useful but explicitly user-driven |

Windows has the strongest native path. Microsoft supports clipboard-change notifications through `AddClipboardFormatListener`, delayed rendering through `SetClipboardData(NULL)`, and Shell virtual-file transfer using `CFSTR_FILEDESCRIPTOR` plus `CFSTR_FILECONTENTS`; the latter can expose each file as an `IStream`, avoiding full in-memory materialization. That is exactly the class of mechanism needed to make a remote OneCopy object appear file-like to Explorer or another compatible target. citeturn129445search0turn129445search2turn129445search3

macOS offers `NSPasteboard`, a change counter, and `NSFilePromiseProvider`. Apple's file-promise model lets an application promise a future file and asynchronously write it when the receiving destination requests it. Apple explicitly uses file promises for drag-and-drop, but the documentation does not prove that arbitrary Finder **Command-V** from a OneCopy-written General pasteboard will accept every remote promised-file configuration. Treat native Finder remote paste as a Phase 0 experiment rather than an assumed feature. citeturn619806search4turn129445search5turn129445search6

On Windows, the background component should be a **per-user hidden/tray process**, not a privileged Windows Service. Services run in session 0 and Microsoft explicitly says services cannot directly interact with a user's desktop; a packaged desktop application can instead register a startup task for login startup. citeturn746149search6turn746149search7

On macOS, `SMAppService` supports LoginItems and LaunchAgents, with LaunchAgents executing in the logged-in user's context. That is the correct location for OneCopy's clipboard agent. citeturn746149search8turn746149search9

On Linux, an XDG autostart entry or user-level systemd service fits the same model. The freedesktop autostart specification is specifically for launching applications after graphical login, and systemd supports per-user managers and graphical-session targets. citeturn746149search10turn746149search11turn746149search13

## Native client architecture and interoperability

The recommended client design is **native shell + native clipboard/filesystem adapter + shared Rust core**. Do not place OS clipboard logic behind a generic cross-platform UI framework. Clipboard semantics are sufficiently different that native adapters should expose platform capabilities explicitly rather than pretending all platforms implement the same interface.

### Client split

```text
┌─────────────────────────────────────────────────────┐
│                  Native OneCopy UI                  │
│ account · devices · history · transfers · settings │
└───────────────────────┬─────────────────────────────┘
                        │ local IPC / in-process API
┌───────────────────────▼─────────────────────────────┐
│            Platform Agent / Integration             │
│ clipboard · startup · filesystem · share APIs      │
│ notifications · key store · background lifecycle   │
└───────────────────────┬─────────────────────────────┘
                        │ FFI
┌───────────────────────▼─────────────────────────────┐
│                 OneCopy Rust Core                   │
│ identities · sync · protocol · crypto · transfers  │
│ routing · manifests · resumability · state machine │
└───────────────────────┬─────────────────────────────┘
                        │
               QUIC / HTTPS / control API
```

Use Rust for protocol serialization, cryptographic state, manifests, integrity verification, routing logic, transfer state, local history database abstraction, and as much networking as the OS allows. Use native languages for the platform boundary:

| Target | Recommended outer layer | Shared-core binding |
|---|---|---|
| Windows | C# / WinUI 3 plus Win32/COM bridge where necessary | C ABI or generated C# wrapper |
| macOS | Swift + AppKit | UniFFI Swift |
| iOS/iPadOS | Swift + UIKit/SwiftUI where appropriate | UniFFI Swift |
| Android | Kotlin | UniFFI Kotlin |
| Linux | Rust native process; GTK/libadwaita or native toolkit if desired | Direct Rust |

UniFFI currently provides first-party Swift and Kotlin binding generation and is explicitly designed for sharing Rust business logic across foreign-language clients. Its documentation describes the Swift support as production-quality, while noting some current Swift 6 concurrency rough edges; therefore keep FFI interfaces coarse-grained and asynchronous rather than exposing large callback-heavy object graphs. citeturn161456search0turn161456search2

For Windows, using a stable C-compatible boundary remains the least risky long-term option because Rust itself guarantees interoperability through explicit foreign ABIs, but FFI panic/unwind behavior needs careful containment. No Rust panic should ever cross the native boundary; convert all failures to structured error values. citeturn161456search4

### Clipboard model

Do **not** mirror the platform clipboard blindly. OneCopy should maintain a canonical logical history:

```text
ClipboardItem
  item_id: UUIDv7
  account_epoch: u64
  origin_device_id: DeviceId
  created_at: Timestamp
  origin_sequence: u64
  kind: Text | RichText | Image | Files | Mixed
  representations[]
  source_refs[]
  sensitivity: Normal | Sensitive | Excluded
  expiry_policy
  gateway_state?
```

Each representation should describe a MIME/UTI/platform-specific format without automatically materializing every representation. For example:

```text
Representation
  media_type
  logical_size?
  inline_ciphertext?       // small text / small metadata only
  source_handle?           // origin-local opaque reference
  content_hash?            // optional when already materialized
```

For copied files:

```text
SourceRef
  source_id
  display_name
  source_kind: NativePath | ContentUri | SecurityScopedUrl | ProviderHandle
  display_origin_path?     // encrypted metadata, optional
  stat_snapshot?           // informational only
  source_capability_token  // opaque, scoped, revocable
```

Do not assume that "origin path" means a filesystem path. Android often exposes `content://` resources rather than paths, and iOS may provide security-scoped URLs from a document picker. Android specifically recommends content URIs with temporary permissions for file exchange; Apple requires security-scoped access for documents outside the app's sandbox. citeturn650196search5turn650196search7turn650196search2turn650196search3

The UI can therefore display:

`Windows PC · D:\Projects\demo.iso`

or:

`Pixel · Google Photos`

or:

`iPad · Files / On My iPad / Documents`

without fabricating a path where the operating system never provided one.

### Clipboard-loop prevention

Every locally written OneCopy clipboard payload should carry a private marker where the platform allows custom formats, for example:

```text
application/x-onecopy-origin:
  account_id_hash
  item_id
  writer_device_id
  write_nonce
```

The listener stores the last locally emitted marker and suppresses re-ingestion of the same item. Where custom metadata cannot survive the platform clipboard, use a short-lived fingerprint cache over type + normalized payload + local paste event. This must be tested against format conversion by applications, because an app may read HTML and subsequently place equivalent plain text back on the clipboard.

OneCopy should **not automatically replace every device's current system clipboard when a remote item arrives**. Remote arrivals belong to OneCopy history. A user action such as selecting an entry, invoking a global shortcut, or choosing Paste from OneCopy promotes that item to the local clipboard. This avoids destructive cross-device feedback when two devices are being used simultaneously.

### Sensitive-content policy

Automatic capture must be opt-out by application and format, with a quick global pause. Android allows applications to mark clipboard contents sensitive using `ClipDescription.EXTRA_IS_SENSITIVE`, and OWASP explicitly recommends respecting sensitive clipboard semantics, minimizing exposure, and treating pasted content as untrusted input. citeturn129445search8turn866979search5

A recommended policy is:

- Do not synchronize platform-marked sensitive items by default.
- Let users exclude password managers, terminals, RDP/VM apps, and selected process IDs/bundle IDs.
- Never execute pasted content.
- Never automatically open copied URLs or files.
- Keep clipboard previews local unless the metadata itself is encrypted end-to-end.
- Offer a "local clipboard only for 1 minute" shortcut.

On modern macOS, OneCopy must also respect pasteboard access denial rather than attempting workarounds; Apple now explicitly exposes an access state the user controls in System Settings. citeturn755371search1turn755371search5

## End-to-end system architecture and protocol

The recommended backend is a **minimal control plane**, separate from the Premium data plane.

```text
                               ┌──────────────────────┐
                               │ OneCopy Control Plane│
                               │                      │
                               │ auth / device roster │
                               │ encrypted item sync  │
                               │ signaling / presence │
                               │ STUN-like discovery  │
                               │ push notifications   │
                               └──────────┬───────────┘
                                          │
                   encrypted metadata     │
                  + candidate signaling   │
                                          │
        ┌─────────────────────────────────┴────────────────────────────┐
        │                                                              │
┌───────▼────────┐          direct authenticated QUIC           ┌──────▼───────┐
│ Device A       │◄────────────────────────────────────────────►│ Device B     │
│ native agent   │                                              │ native agent │
└───────┬────────┘                                              └──────┬───────┘
        │                                                              │
        │ Premium only                                                 │
        ▼                                                              ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                           OneCopy Gateway                                  │
│       relay nodes                    temporary encrypted object store      │
└────────────────────────────────────────────────────────────────────────────┘
```

This separation is important because OneCopy's business rule says **all OneCopy-managed live relaying is Premium**, while Free users get direct P2P. Generic iroh's default connection model commonly establishes via a relay and then upgrades to a direct path. Iroh documents exactly that behavior: clients normally establish through a relay, attempt hole punching, and continue through the relay if direct connectivity fails. Adopting that behavior unchanged would consume gateway bandwidth even for a nominally Free transfer. citeturn627827search0turn627827search1

Therefore, **do not adopt iroh's default relay policy verbatim**. Either configure/build around iroh so Free accounts use only candidate exchange plus direct addresses, or implement a thin OneCopy rendezvous/NAT traversal layer over QUIC. Iroh remains a strong candidate because it already offers authenticated QUIC identities, NAT reports, hole punching, relay fallback, and path monitoring; it simply needs product-policy adaptation. citeturn627827search1turn627827search8

### Control-plane responsibilities

The server should know:

```text
Account
  account_id
  auth credentials / passkey records
  billing entitlement
  device roster
  encrypted sync envelopes
  routing/presence metadata
```

It should not need plaintext clipboard text, source file paths, filenames, gateway file contents, or direct-transfer contents.

A logical metadata-sync message can be:

```text
ClipboardEnvelope {
  protocol_version
  account_id
  account_epoch
  sender_device_id
  sender_sequence
  item_id
  encrypted_payload
  previous_hash
  signature
}
```

`encrypted_payload` contains the clipboard item metadata and, for sufficiently small content, the actual text/image data. `sender_sequence`, `account_epoch`, and `previous_hash` provide primitives for replay and rollback detection. The server may store envelopes for temporarily offline devices but cannot decrypt them.

Recommended sync behavior:

1. Origin increments a monotonic per-device sequence.
2. Origin encrypts the item to the current account/device group.
3. Origin signs the envelope.
4. Server fan-outs/store-and-forwards.
5. Receiving device verifies sender membership, signature, epoch, sequence, and anti-replay state.
6. Receivers deterministically merge the entry into the rolling OneCopy history.

A rolling history should not be modeled as "the server's last ten rows". Model it as an append-only logical event sequence plus local eviction. That makes simultaneous copies from multiple devices deterministic: order primarily by authenticated logical event time/sequence, with server receipt time only as a tiebreaker. Never allow a malicious or badly clocked peer to arbitrarily reorder history by asserting an extreme wall-clock timestamp.

### Enrollment state machine

```text
UNREGISTERED
   │ account authentication
   ▼
AUTHENTICATED_UNTRUSTED
   │ generate device identity locally
   │ submit public enrollment request
   ▼
PENDING_APPROVAL
   │ approval from existing trusted device
   │ OR explicit recovery flow
   ▼
TRUSTED
   │ receive current account/group epoch
   │ sync encrypted history
   ▼
ACTIVE

ACTIVE ──revoke──► REVOKED
                     │
                     └── cannot receive new epoch/content
```

A new device must generate its private identity key locally and only upload public material. Existing trusted devices should display a short authentication string or QR code derived from both enrollment keys before approval. Account-login credentials alone should not silently grant historical clipboard access, because compromise of a web account would then compromise every clipboard.

WebAuthn Level 3, finalized as a W3C Recommendation in August 2026, is suitable for account authentication and passkey-based recovery UX, but passkeys should authenticate the user to the OneCopy account, not substitute for device-level content keys. citeturn169970search3turn169970search4

### Clipboard item lifecycle

```text
CAPTURED
   │ sanitize / classify
   ▼
LOCAL
   │ encrypted metadata sync accepted
   ▼
SHARED
   │
   ├── source disappears ─► UNAVAILABLE
   │
   ├── history eviction ─► EXPIRED
   │
   ├── Premium stage ─────► STAGING ─► STAGED
   │
   └── transfer request ──► source opened at request time
```

`UNAVAILABLE` should be dynamic. If an origin laptop reconnects and the source becomes accessible again, the entry can return to available without creating a new item.

### Premium staging lifecycle

```text
NONE
 │ user presses "Keep Available Offline"
 ▼
QUEUED
 ▼
UPLOADING
 ▼
VERIFYING
 ├──fail──► FAILED
 ▼
READY
 │ expiry / quota / explicit delete
 ▼
EXPIRED/DELETED
```

The staged object must record:

```text
StagedObject {
  item_id
  staging_id
  staged_at
  source_device_id
  encrypted_size
  object_manifest_hash
  expires_at
}
```

The UI should say **"Offline copy created Nov 12, 14:32"**, not merely "available". If the source later changes, normal paste from an online source should use the current source. The staged copy remains exactly what was uploaded at staging time until the user presses **Refresh offline copy**. This is not version history; there is exactly one current staged copy.

## Transfer protocol, intelligent routing, and consistency

QUIC is the strongest default transport for desktop P2P. QUIC provides encrypted, flow-controlled streams, low-latency connection establishment, multiplexing, congestion control, and connection migration across path/address changes. These directly map to OneCopy's needs for parallel metadata/control streams, large payload streams, and resilience when a laptop switches networks. citeturn627827search2

### Transport decision

| Candidate | Advantages | Problems for OneCopy | Decision |
|---|---|---|---|
| Raw TCP + TLS | Extremely deployable | Head-of-line blocking, hand-built multiplex/resume/path handling | No |
| WebRTC DataChannel | Mature ICE/STUN/TURN traversal | SCTP/SDP stack is heavier than needed for native file streaming | Secondary option |
| libp2p | Broad protocol suite and NAT traversal | More abstraction/protocol surface than required | Evaluate, not first choice |
| iroh/QUIC | Rust-native, identity, hole punching, relay upgrade | Default relay-first behavior conflicts with Free/Premium boundary | **Primary experiment candidate** |
| Custom QUIC + ICE-like traversal | Precise product fit | Highest networking engineering burden | Fallback if iroh policy cannot fit |

WebRTC data channels are secure and use SCTP over DTLS over ICE, but their design is intended as a generic browser-compatible peer data API. The standards require SCTP/DTLS/ICE, while OneCopy is a native application with control over both endpoints; direct QUIC gives a simpler streaming model and native migration. citeturn454579search7turn454579search8turn454579search9

libp2p supports QUIC, relays, AutoNAT and hole punching, but its recommended private-node model also relies heavily on relay reservations to coordinate direct paths. That is an excellent reference architecture but again needs adaptation to OneCopy's no-free-relay business rule. citeturn454579search0turn454579search1

The direct-connectivity stack should implement:

```text
1. Local interface candidates
2. IPv6 global candidates
3. STUN-derived public UDP mappings
4. Optional PCP/NAT-PMP/UPnP mapping experiments
5. Candidate exchange over control channel
6. Simultaneous QUIC probing
7. LAN address preference
8. Direct-path validation
9. Premium relay only if entitled and needed/beneficial
```

TURN's own RFC explicitly says relay use is necessary when direct NAT traversal fails and recommends direct-path discovery first. Tailscale similarly documents direct connections as preferred and relay as fallback. citeturn627827search3turn627827search5turn627827search9

### Intelligent path selector

Do not route based simply on file size. The selector should score candidate paths using measured conditions:

```text
score(path) =
  predicted_completion_time
  + failure_penalty
  + monetary_cost_penalty
  + privacy_penalty
  + battery_penalty
```

Useful inputs:

```text
RTT
initial throughput probe
packet loss
direct vs relay
LAN vs WAN
source/destination network type
metered connection status
battery/power state
file/folder total size
current relay utilization
gateway entitlement/quota
source availability expectation
mobile OS lifecycle
```

Proposed behavior:

**Free account**
1. Same-LAN direct.
2. Internet direct.
3. If direct fails: stop with "This network blocks a direct connection. OneCopy Gateway is available with Premium."
4. Never silently send payload bytes through OneCopy relay infrastructure.

**Premium**
1. Probe direct.
2. Establish relay fallback where appropriate.
3. Prefer direct if predicted performance is materially equivalent or better.
4. Use relay if direct fails or measured direct performance is sufficiently worse.
5. Use staged gateway copy only when explicitly requested, not as an invisible transfer optimization unless the product later introduces an opt-in "allow automatic staging" setting.

A useful hysteresis rule is to avoid changing paths merely for a 5–10% predicted improvement. Path switching itself creates uncertainty. Start with a conservative threshold such as **25% predicted completion-time improvement for at least several seconds**, then tune using telemetry.

### Transfer framing

Large-file transfer should be chunk-addressable without requiring every file to be prehashed before sending.

A proposed manifest:

```text
TransferManifest {
  transfer_id
  item_id
  root_entries[]
  total_logical_bytes?
  chunk_size
  manifest_version
}

FileEntry {
  relative_path
  type: file | directory | symlink
  logical_size
  mtime?
  permissions_subset?
  source_identity
}

ChunkRecord {
  file_index
  offset
  length
  digest
}
```

Recommended default chunk size for experiments: **4–16 MiB**, dynamically tunable. Smaller chunks improve resume granularity but add metadata/hash overhead; larger chunks improve sequential throughput but waste more work after interruption. Do not choose the production value until 1/10/100 GB benchmarks are complete.

Use independent QUIC streams for control and data:

```text
stream 0: authenticated control messages
stream N: file range / chunk payloads
```

Bound outstanding unacknowledged application chunks and use QUIC flow control rather than buffering entire files. Memory use should remain approximately constant with file size.

### Crash-safe resume

The destination writes:

```text
filename.onecopy-partial
transfer.sqlite / journal
verified chunk bitmap
```

After a chunk is received:
1. Write to destination staging file.
2. Flush according to configured durability policy.
3. Verify digest.
4. Atomically mark the chunk complete in the transfer journal.

After all chunks verify, atomically rename from the temporary name to the final filename.

Resume handshake:

```text
ResumeRequest {
  transfer_id
  manifest_hash
  verified_ranges[]
}
```

If the source representation is no longer compatible with the recorded transfer manifest, the old transfer must be discarded and restarted rather than silently resuming against unrelated bytes.

### What if the source changes during transfer?

This is one of OneCopy's most important correctness problems.

The requirement says the transfer should send the file "as it exists when requested", but a file can continue changing after the request starts. There is no universal cross-platform filesystem primitive that gives an immutable read snapshot of an arbitrary file while another application is modifying it. Therefore OneCopy must define a coherent but practical contract.

**Recommended contract:** each file transfer is a best-effort read of the opened source object, with start/end identity checks and cryptographic verification of transferred chunks. If identity/size/change indicators show that the source changed incompatibly during the read, the transfer fails with **"Source changed during transfer; retry"**. It should never stitch verified chunks from two incompatible transfer attempts.

At file open, capture available identity metadata: file identifier/inode where available, size, modification/change metadata. After transfer, stat again. Chunk hashes protect transport integrity; they do **not** prove source immutability, because a chunk can be read correctly after the source changed. Consequently, source-state comparison remains necessary.

For very large append-only files, OneCopy can later support a specialized "read initial extent" policy, but that should not be part of MVP.

### Folders, links, and path security

Never serialize a folder into an implicit ZIP for normal transport. Walk the directory tree into a manifest and stream individual files. This avoids ZIP bombs, unnecessary recompression, huge temporary archives, and inability to resume individual files.

Default symlink handling should be **do not follow**. Treat a symlink as metadata only if the target platform can safely recreate it and the user has opted into link preservation. Otherwise copy it as a warning/unsupported entry. Following symlinks during recursive traversal can escape the selected directory and leak unrelated files.

Destination path validation must reject:

- absolute paths,
- `..` traversal,
- alternate path separators,
- Windows device names and alternate data-stream tricks,
- embedded NULs,
- path normalization collisions,
- case-insensitive name collisions,
- names exceeding platform limits,
- special-device/FIFO/socket nodes.

Never allow incoming metadata to choose an arbitrary destination outside the user-selected paste directory.

### Native paste strategy

On Windows, implement remote files as a COM `IDataObject` exposing `CFSTR_FILEDESCRIPTOR` and lazily serving `CFSTR_FILECONTENTS` through `IStream`. Microsoft's Shell documentation explicitly supports this "file contents as a stream" pattern. citeturn129445search2

On macOS, prototype both:
1. file promises via `NSFilePromiseProvider`, and
2. a OneCopy history/drag workflow.

Apple's file-promise API explicitly permits asynchronous creation of promised files in a destination directory, making it suitable for remote materialization when the destination supports promises. citeturn129445search5turn129445search6

On Android, expose downloaded/remote OneCopy content through a narrow `ContentProvider` or `FileProvider` and grant only temporary read permission. Android explicitly recommends content URIs rather than file paths for secure cross-app sharing. citeturn650196search5turn650196search7

On iOS, prioritize Share extension + OneCopy history + Files/document-picker integration. Share extensions can initiate background URL-session uploads and then be terminated by the system, which strongly favors the Premium staging path for large mobile-originating items. citeturn650196search0turn650196search1

### Mobile transfer reality

Android 14+ has User-Initiated Data Transfer jobs specifically for long-running user-requested upload/download operations; these jobs require a notification and must normally be scheduled while the application is visible. Android 15/16 further tighten foreground-service/job execution behavior. This is a suitable mechanism for explicit OneCopy transfers, but not for an invisible always-on clipboard daemon. citeturn746149search0turn746149search1turn746149search2

On iOS, arbitrary QUIC P2P should be viewed as a **foreground experience**. Background `URLSession` only supports HTTP/HTTPS and can continue in a separate system process, so Premium staging/downloading through an HTTPS gateway has a genuine reliability advantage on iOS that P2P alone cannot reproduce. citeturn165319search3turn165319search4

## Security, identity, privacy, and trust model

OneCopy's attack surface is unusually sensitive because clipboard contents frequently include credentials, private URLs, commands, documents, personal data and temporary secrets. The security design must assume three independent compromise cases:

1. **OneCopy server compromised**
2. **One enrolled device compromised**
3. **Local untrusted application writes malicious clipboard content**

The strongest practical target is: **a compromised OneCopy control/gateway server cannot decrypt clipboard content, file contents, filenames or origin paths; a compromised enrolled device can access content authorized to that device until revoked.**

That second limitation must be stated clearly. End-to-end encryption cannot protect a user's clipboard from malware running with the same privileges as the trusted endpoint that legitimately decrypts it.

### Cryptographic hierarchy

Recommended device identity:

```text
Device Identity
  long-term signing key
  long-term HPKE/key-agreement key
  transport identity key
```

Use audited standard primitives rather than a custom cipher construction. X25519 and Ed25519 are standardized by the IETF, and MLS 1.0 defines a standardized asynchronous group-key architecture using HPKE, AEAD, signatures and epoch changes. MLS is explicitly designed for groups whose members are not simultaneously online and provides forward secrecy and post-compromise security properties. citeturn169970search0turn169970search1turn169970search2

There are two viable group-key designs:

| Design | Advantages | Disadvantages |
|---|---|---|
| Custom account epoch key distributed to enrolled devices with HPKE | Simpler, easier MVP | Harder to achieve formal FS/PCS correctly |
| MLS group containing all account devices | Standardized membership/rekey model, FS/PCS | Considerably more implementation/state complexity |

**Recommendation:** prototype MLS but do not make it an MVP dependency until operational complexity is understood. For MVP, use a narrowly designed epoch-based group secret distributed independently to each device with HPKE, reviewed by an external cryptographer before production. Structure protocol messages so MLS can replace the key-distribution layer later.

Example:

```text
AccountEpoch {
  epoch: u64
  members[]
  epoch_secret
}
```

Every clipboard envelope derives an item key from `epoch_secret` + `item_id` using HKDF. Encrypt metadata using an AEAD. Device membership changes increment the epoch. A revoked device cannot decrypt future epochs.

Do **not** claim post-compromise security for this simpler model. A device that leaks its epoch secret compromises content encrypted under that epoch. MLS is the route if that property becomes a requirement. citeturn169970search0

### Transport authentication

QUIC already encrypts its transport using TLS. Iroh's endpoint model additionally authenticates peers by endpoint public key. OneCopy still needs application-layer authorization: possession of a transport identity proves "this is key X", while the account membership roster proves "key X is currently authorized for account Y." citeturn627827search0turn627827search2

Every P2P connection should bind:

```text
authenticated_transport_identity
        ↓
device_id
        ↓
current account membership epoch
        ↓
requested clipboard item capability
```

A peer must not be able to enumerate arbitrary filesystem paths simply because it is an enrolled device. File requests use opaque source-capability identifiers created at clipboard capture time.

### Device secret storage

On Apple platforms, put device secrets in Keychain, using Secure Enclave-backed P-256 keys where its capabilities fit the operation. Apple's documentation says Secure Enclave private keys remain non-exportable and only P-256 operations are supported, so do not design an X25519-only storage scheme that assumes the Secure Enclave can hold those keys directly. citeturn169970search6

Android Keystore similarly keeps supported key material non-exportable and may back keys with a TEE or StrongBox. StrongBox has narrower algorithm support and higher overhead, so use it selectively rather than treating it as mandatory. citeturn169970search7

On Windows, use DPAPI/CNG or an appropriate Windows credential/key facility rather than plaintext application storage. Microsoft's Credential Locker guidance explicitly warns against storing credentials in plaintext app data, though OneCopy's raw cryptographic key material may require lower-level CNG/DPAPI rather than `PasswordVault` itself. citeturn169970search5

Linux should use the desktop secret service/keyring where available and fall back to an encrypted local key file unlocked by OS/user credentials only with explicit documentation of the weaker guarantee.

### Threat-to-mitigation matrix

| Threat | Required mitigation |
|---|---|
| Control server reads clipboard | E2EE before upload |
| Gateway reads staged file | Client-side file encryption; object store receives ciphertext only |
| Server replays old metadata | Signed sequence + epoch + anti-replay database |
| Server rolls account roster backward | Monotonic roster epoch, device-signed roster transitions |
| Stolen auth cookie enrolls attacker | Existing-device approval / recovery ceremony |
| Revoked device continues receiving | New account epoch; stop routing presence/sync |
| Malicious enrolled peer requests arbitrary file | Capability-scoped source handles, never raw path RPC |
| Clipboard poisoning | Data-only handling, no auto-open/execute |
| Path traversal | Canonical destination-root validation |
| Symlink escape | Do not follow by default |
| Huge folder / billion tiny files | Manifest quotas and rate-limited enumeration |
| Decompression bomb | No automatic archive extraction |
| Memory DoS | Stream with bounded buffers |
| Storage DoS | Destination quota/free-space check and per-transfer limits |
| Filename spoofing | Unicode normalization checks, explicit display escaping |
| Transfer tampering | AEAD transport plus per-chunk digest / final manifest verification |
| Source changes mid-transfer | Source identity/stat recheck; abort/retry on incompatible change |
| Clipboard secret capture | sensitive-content policy, exclusions, pause |
| Relay abuse | authenticated Premium entitlement, bandwidth quotas, rate limits |
| Gateway link leakage | opaque unpredictable IDs plus cryptographic authorization |
| Supply-chain compromise | signed/notarized apps, reproducible CI where feasible, SBOM, dependency audit |
| Update takeover | code-signed updates with pinned update-signing policy |

### Recovery tradeoff

A genuinely E2EE system creates a difficult account-recovery question. If OneCopy itself can recover the content-encryption master secret from only an email reset, then the backend effectively has a path to decrypt user data and the E2EE claim is weakened.

Recommended modes:

**Default secure recovery:** a new device is approved by an existing trusted device.

**Recovery key:** user receives a printable recovery secret that can reconstruct account encryption state.

**Optional convenience recovery:** if offered later, explicitly explain that server-assisted escrow changes the security model.

Do not silently escrow a decryption key merely to simplify password resets.

### Metadata privacy

Encrypt filenames, origin paths, clipboard text, MIME descriptors where practical, and detailed device labels in the clipboard-sync envelope. The control service still necessarily sees some traffic metadata: account ID, enrolled device IDs, IP addresses during connection/signaling, timestamps, envelope sizes, push tokens, and gateway usage.

Therefore OneCopy can truthfully claim **end-to-end encrypted content**, but not "the server learns nothing."

GDPR principles are directly relevant if EU users are served: the regulation requires data minimization, storage limitation, integrity/confidentiality, and data protection by design/default. A product carrying arbitrary clipboard contents should prepare a formal data inventory, retention schedule, DPA/subprocessor inventory, deletion workflow, incident-response process and likely DPIA before broad launch. citeturn866979search0turn866979search1turn866979search2

### Security test strategy

Before production:

1. Independent cryptographic protocol review.
2. Native clipboard adapter security review per OS.
3. Fuzz all binary/protobuf/CBOR protocol parsers.
4. Property tests for path normalization and manifest traversal.
5. Fuzz malformed filenames, Unicode, symlinks and oversized manifests.
6. Adversarial peer simulator.
7. Compromised-control-plane test harness that reorders/replays/duplicates/drops envelopes.
8. Network chaos testing during live transfers.
9. Dependency/SBOM scanning in CI.
10. Mobile assessment against OWASP MASVS categories for storage, crypto, auth, network, platform and privacy. citeturn866979search6turn866979search7

## Premium gateway economics, deployment, and competitive position

### Temporary storage economics

Published prices checked on **November 12, 2026**:

| Provider | Storage | Internet egress | Important caveat | OneCopy fit |
|---|---:|---:|---|---|
| Cloudflare R2 Standard | $0.015/GB-month | $0 | Class A $4.50/M, Class B $0.36/M | **Best initial staging candidate** |
| Backblaze B2 | starts $6.95/TB/month | free up to 3× average monthly storage, then $0.01/GB | Temporary high-churn data can exceed 3× ratio quickly | Good secondary |
| AWS S3 Standard | roughly $0.023/GB-month first tier | commonly $0.09/GB first Internet tier | mature but egress expensive | Operationally strong, economically poor for consumer staging |
| Wasabi | $7.99/TB/month | free subject to fair-use rules | 1 TB minimum billing and 90-day minimum storage duration | Poor fit for ephemeral staging |
| Google Cloud Storage | region-dependent | charged by destination/region | complex network pricing | Technically viable, cost model less attractive |

Cloudflare's current R2 page lists Standard storage at $0.015/GB-month, Class A at $4.50/million, Class B at $0.36/million and no Internet egress charge. citeturn616936search0

Backblaze's current main pricing page lists B2 starting at $6.95/TB/month with free egress up to three times average monthly storage and $0.01/GB beyond that. Interestingly, a separate Backblaze transaction-pricing page still states $0.005/GB/month, so the exact contracted storage rate should be verified directly before budgeting; the main current product page should be treated as the safer planning source. citeturn616936search3turn616936search7

AWS currently documents standard Internet transfer-out examples at $0.09/GB, and its pricing material still presents the first S3 Standard storage tier around $0.023/GB-month. citeturn771731search2turn771731search10

Wasabi's July 2026 pricing is $7.99/TB/month and still advertises free API/egress under fair-use terms, but its product page specifies a 1-TB minimum monthly charge and a 90-day minimum storage duration for pay-as-you-go. That is almost exactly the wrong billing profile for OneCopy's short-lived temporary files. citeturn616936search4turn616936search5turn616936search6

### Representative staging economics

For an illustrative Premium customer who stages **500 GB total during a month**, each staged object lives **one day** on average and is downloaded once:

- R2 average billed storage is only about 16.7 GB-month-equivalent, or roughly **$0.25 storage**, with negligible request cost and no direct R2 Internet egress charge at current published rates. citeturn616936search0
- On B2, the storage itself remains inexpensive, but average storage is ~16.7 GB, making only ~50 GB of general egress fall inside the advertised 3×-storage allowance; approximately 450 GB could then fall into $0.01/GB egress, around **$4.50**, subject to exact billing interpretation. citeturn616936search7
- AWS's Internet egress component alone would be roughly **$45 at $0.09/GB**, before discounts/free aggregate allowances and before request/storage cost. citeturn771731search2

That makes **R2 Standard the strongest Phase 0 object-storage candidate**, but benchmark real upload/download performance from the target regions before committing. Free egress is irrelevant if users experience poor throughput.

### Live relay economics

Live relay is different from object storage because a relay must handle network traffic continuously and often transfers each user byte twice internally: sender → relay and relay → receiver. Provider billing normally counts the relay's outbound leg, but CPU, socket count, packet rate and regional footprint also matter.

A bandwidth-oriented VPS/bare-metal deployment can be far cheaper than hyperscaler egress. Hetzner currently includes 20 TB/month with many EU cloud instances and documents €1/$1.20 per TB overage for applicable traffic classes; current EU cloud pricing starts with low-cost instances, though relay production sizing must be based on NIC throughput and connection concurrency rather than CPU price alone. citeturn524089search1turn524089search2

A sensible launch topology is:

```text
EU relay pool
US-East relay pool
US-West relay pool
Asia relay pool
```

with GeoDNS/latency selection and strict Premium authorization. Do not promise a relay in every geography before usage justifies it.

The service should track:

```text
relay_bytes_out
relay_minutes
concurrent_sessions
staged_GB_hours
staged_download_GB
Class A/B object requests
failed_transfer_bytes
abuse_bytes
```

### Subscription model

Do not offer "unlimited gateway".

A safer structure is:

**Free**
- unlimited-size direct P2P transfer,
- shared clipboard,
- account and trusted devices,
- signaling/STUN/control plane,
- no OneCopy relay payload,
- no managed staging.

**Premium**
- a monthly relay transfer allowance,
- a temporary-staging GB allowance,
- maximum simultaneously staged bytes,
- maximum retention, e.g. 7 or 30 days depending tier,
- explicit offline-staging action,
- refresh staged copy,
- higher limits available under higher tiers.

Usage should hard-stop or ask the user before overage; no surprise metered bill.

Abuse controls should include per-account concurrent-transfer limits, per-device rate limits, maximum staged object count, API rate limits, malware-report/abuse handling, and prevention of public anonymous download links. OneCopy staging should be available **only to enrolled devices**, not evolve accidentally into a generic file-hosting/CDN service.

### Market position

Apple Universal Clipboard demonstrates that seamless cross-device copy/paste can be compelling, but it is restricted to Apple's ecosystem, requires Apple-account continuity features, proximity, Wi-Fi/Bluetooth/Handoff, and Apple explicitly describes it as best for small selections; copying whole files is supported Mac-to-Mac rather than as a universal cross-platform transfer mechanism. citeturn522049search0turn522049search1

Microsoft Phone Link offers cross-device copy/paste and file transfer, but clipboard support is limited to specific Android manufacturers/devices, and the product's cross-device scope is fundamentally Windows ↔ phone rather than a general Windows/macOS/Linux/iOS/Android device mesh. citeturn522049search2

Tailscale Taildrop already provides encrypted, fastest-path file transfer among a user's devices and resumes many interrupted transfers. It is therefore the closest benchmark for OneCopy's P2P file path, but Taildrop is a file-send feature, not a synchronized rolling clipboard with native paste semantics. Taildrop is also still documented as alpha. citeturn522049search3

LocalSend is a strong zero-account local-network competitor covering all five major OS families, but it is intentionally LAN-only, with no account, Internet device mesh, clipboard history, managed relay or offline staging. citeturn522049search4

OneCopy's defensible differentiation is therefore the combination of:

**native clipboard semantics + cross-platform history + native lazy file paste + direct P2P + trusted account device mesh + optional paid reliability infrastructure.**

The difficult part to copy is not "file transfer." It is making the transfer **feel like the operating system's own clipboard** while maintaining security and working around six different OS clipboard/background models.

## Implementation roadmap, risks, feasibility gates, and experiments

### Repository layout

A practical monorepo:

```text
onecopy/
  core/
    protocol/
    crypto/
    identity/
    history/
    transfer/
    routing/
    manifests/
    gateway-client/
    persistence/
    telemetry/
  platform/
    windows/
      agent/
      shell/
      virtual-files/
    macos/
      app/
      agent/
      pasteboard/
      file-promises/
    linux/
      agent/
      x11/
      wayland/
    android/
      app/
      share/
      provider/
      jobs/
    ios/
      app/
      share-extension/
      file-integration/
  server/
    api/
    auth/
    device-registry/
    sync/
    signaling/
    presence/
    entitlement/
    push/
  gateway/
    relay/
    staging-api/
    object-lifecycle/
  schemas/
  fuzz/
  benchmarks/
  infra/
  docs/
```

Keep schemas versioned separately from implementation crates so older clients can remain compatible.

### Build-versus-buy decisions

| Area | Recommendation |
|---|---|
| Authentication UI | Buy/use mature OIDC + passkey infrastructure |
| Device E2EE identity | Build OneCopy-specific layer |
| Transport | Build atop QUIC/iroh rather than raw UDP |
| STUN | Use standard protocol/library |
| Relay | Self-host/adapt proven relay approach |
| Object storage | Buy; R2 first experiment |
| Database | Managed PostgreSQL for control plane |
| Push | APNs/FCM |
| Crypto primitives | Established audited libraries only |
| Auto-update | Platform-native/signed updater, not hand-rolled crypto |
| Clipboard adapters | Build natively |
| Metrics/logging | Buy commodity observability, redact payload metadata |

### Roadmap

**Phase 0 — Feasibility, not product polish**

Prove the promises that could otherwise invalidate the product:

- Windows clipboard capture/write.
- Windows Explorer remote virtual-file paste.
- Linux X11 capture/write.
- Wayland capability detection across GNOME/KDE/wlroots.
- macOS clipboard permission behavior and Finder/file-promise compatibility.
- Windows ↔ Linux and Windows ↔ macOS QUIC direct transfer.
- NAT traversal success rates.
- Android Sharesheet/content-provider transfer.
- iOS Share extension + background HTTPS staging.
- source-changing-during-transfer behavior.
- 100 GB crash/resume.

**Phase 1 — Desktop technical MVP**

Windows + one Linux target first, then macOS. Implement account, device enrollment, 10-item history, text, images, files/folders, direct QUIC, encrypted sync, transfer resume, tray/menu agent and native history UI.

**Phase 2 — Multi-desktop production**

Wayland matrix, macOS permissions, native file-paste quality, code signing, updaters, crash recovery, security audit, telemetry, rate limiting and control-plane HA.

**Phase 3 — Mobile**

Android and iOS as honest platform-native experiences, without pretending their OSes permit the desktop daemon model.

**Phase 4 — Premium Gateway**

Relay and temporary staging after direct-P2P reliability has been measured. Building gateway later prevents infrastructure from masking weaknesses in the free direct path.

### Top engineering risks

| Rank | Risk | Mitigation |
|---|---|---|
| 1 | iOS cannot deliver desktop-style clipboard daemon | Define mobile UX differently from day one |
| 2 | Android cannot observe arbitrary clipboard in background | Sharesheet/history/optional IME |
| 3 | Wayland clipboard-manager fragmentation | Capability matrix + compositor-specific adapters |
| 4 | macOS privacy blocks automatic reads | Onboarding permission UX + detection APIs |
| 5 | Native virtual-file paste incompatible with common target apps | Phase 0 compatibility matrix |
| 6 | Direct P2P success inadequate on CGNAT/corporate networks | Measure; Premium relay remains fallback |
| 7 | Relay bandwidth destroys subscription margin | Quotas + region/provider optimization |
| 8 | E2EE enrollment/recovery becomes too complex | Explicit device approval and recovery-key design |
| 9 | Huge folder/edge-case files create correctness/security bugs | Manifest limits, fuzzing, normalized path layer |
| 10 | Source changes create corrupted resume state | manifest identity + restart semantics |

The Android and iOS limitations are not theoretical risks; they follow directly from current platform rules. citeturn236632search7turn236632search2turn165319search3

### Go/no-go gates

**Gate A — native clipboard viability**

Go if:
- Windows capture/write latency is reliable.
- macOS works acceptably after explicit user permission.
- Linux has a documented supported-compositor matrix.

No-go for "universal invisible clipboard" marketing if mobile restrictions remain as documented; product wording must instead describe platform-specific interaction.

**Gate B — virtual file paste**

Go for flagship "paste huge remote file directly" positioning only if Explorer and Finder experiments show reliable behavior in the dominant file-manager workflows.

If Finder Command-V cannot support the desired lazy remote-file model reliably, ship explicit OneCopy paste/download on macOS while keeping Windows virtual paste.

**Gate C — direct networking**

Run at least 1,000 real connection attempts across:
- home NAT,
- double NAT,
- CGNAT,
- IPv6,
- enterprise Wi-Fi,
- hotel Wi-Fi,
- mobile hotspot,
- cellular.

A useful initial Go target is **≥90% direct success across non-enterprise consumer cases**, with Premium relay filling the remainder. This is a proposed product SLO, not a currently proven capability. libp2p itself notes that public-network hole punching is inherently environment-dependent, reinforcing the need to measure rather than assume success. citeturn454579search2

**Gate D — large transfer**

Require:
- 100 GB transfer without memory growth proportional to size,
- kill/restart resume,
- network-switch recovery,
- disk-full behavior,
- source-change abort rather than corruption,
- checksum match after repeated fault injection.

**Gate E — security**

No public beta with real clipboard data before an independent protocol review and native security assessment.

### Phase 0 benchmark suite

Use the same generated incompressible test corpus on every platform:

```text
1 KiB text
1 MiB image
1 GB file
10 GB file
100 GB file
100,000 small files
mixed 10 GB directory
Unicode/path-edge corpus
symlink corpus
actively mutating file
```

Run matrix:

```text
Windows ↔ Linux X11
Windows ↔ Linux Wayland
Windows ↔ macOS
macOS ↔ Linux
Android ↔ Windows
iOS ↔ Windows
```

Network conditions:

```text
1 Gbit Ethernet LAN
Wi-Fi 6/7 LAN
100/20 Mbps WAN
20/5 Mbps WAN
100 ms RTT + 1% loss
mobile hotspot
cellular
NAT + NAT
UDP blocked
mid-transfer Wi-Fi → cellular/network change
```

Faults:

```text
kill sender
kill receiver
sleep sender
sleep receiver
disconnect network 30s
change source bytes
truncate source
rename source
delete source
destination disk full
revoke destination device
expire auth token
restart control server
duplicate/reorder metadata messages
```

### Proposed SLOs

These are **engineering targets**, not statements of current capability.

| Metric | Phase 1 target |
|---|---|
| Local clipboard detection on supported desktop | p95 < 100 ms |
| Remote history propagation, both devices online | p95 < 500 ms within same region |
| LAN direct connection establishment | p95 < 750 ms |
| Internet direct establishment | p95 < 2 s |
| Text paste readiness | p95 < 500 ms after selection |
| Large-file memory overhead | < 256 MB per active transfer under default concurrency |
| LAN throughput | ≥80% of achievable same-host-network baseline where disk is not bottleneck |
| Resume data retransmission | < one application chunk plus transport overhead |
| Corrupt completed transfers | 0 in fault suite |
| Silent mixed-version source result | 0 |
| Successful restart recovery | ≥99.9% in automated chaos suite |
| Gateway staging integrity | 100% manifest verification before "Offline ready" |

Telemetry must not include clipboard bodies, filenames, paths or content hashes that can act as identifiers. Collect coarse performance signals such as path type, bytes, duration, NAT category, error class, app version and platform version. Treat even IP addresses and device identifiers as personal/security-sensitive metadata.

## Final architecture decisions and open questions

The research supports the following **implementation blueprint**.

**Native clients:** proceed. Use native platform integrations with a shared Rust protocol/crypto/transfer core. UniFFI is reasonable for Swift/Kotlin but should remain behind a deliberately narrow interface. citeturn161456search0turn161456search2

**Windows agent:** per-user startup process, not Windows Service. Windows gives OneCopy the strongest clipboard APIs and streamed virtual-file capabilities. citeturn129445search2turn746149search6

**macOS agent:** LoginItem/LaunchAgent. Automatic clipboard capture now requires explicit attention to Apple's General-pasteboard privacy controls; make this visible during onboarding instead of hiding it. citeturn746149search8turn755371search1

**Linux:** support X11 first and implement Wayland as capability-specific backends. Do not list "Wayland" as one uniformly supported clipboard environment until GNOME, KDE Plasma and wlroots experiments pass. The core Wayland protocol alone does not grant an unfocused clipboard manager the same observation model as X11, while `ext-data-control-v1` remains a testing protocol. citeturn236632search8turn129445search9

**Android:** Kotlin native app, explicit Sharesheet/content-provider integration, user-initiated transfer jobs, and optionally investigate a OneCopy IME. Do not promise invisible system-wide copy monitoring. citeturn236632search7turn746149search1turn650196search5

**iOS/iPadOS:** Swift native app + Share extension + history + system Files integration. Treat P2P as foreground/best-effort; use Premium HTTPS staging/background transfer for high-reliability long-running transfers. Apple background `URLSession` is specifically engineered to survive app suspension, whereas arbitrary custom P2P protocols are not supported by that subsystem. citeturn165319search3turn165319search4

**Networking:** choose QUIC as the data transport. Evaluate iroh first because its Rust implementation already provides authenticated endpoint identities, hole punching, relays and network-condition reporting, but modify the routing policy so Free transfers never consume OneCopy relay payload bandwidth. If iroh cannot cleanly support that policy, build a small ICE-like candidate-exchange layer around a standard QUIC implementation rather than adopting WebRTC merely for NAT traversal. citeturn627827search1turn627827search8turn627827search2

**Free routing:** control-plane signaling + STUN-like discovery + direct P2P only. Signaling and STUN are provider costs but extremely small relative to relayed multi-GB traffic; treat them as cost of the free service rather than "gateway usage."

**Premium routing:** direct remains preferred, with managed relay available when direct is blocked or materially worse. Temporary object staging remains an explicit action. Do not stage merely because a file is large.

**Premium staging:** start the prototype with R2 Standard because current published pricing has no egress fee and no long minimum storage duration. Encrypt before upload; object keys are opaque; automatic lifecycle expiry is mandatory. citeturn616936search0

**Source semantics:** no snapshots, no version control, no changed-file prompt. At a requested transfer, open the current source and stream it. If it vanishes, report unavailable. If it changes incompatibly while being transferred, fail coherently and offer retry.

**Offline copy semantics:** the Premium staged copy is a point-in-time copy because staging necessarily materializes bytes. Show the staging time and a Refresh action. Normal online paste still uses the current source.

**Encryption:** use application-level encrypted metadata in addition to QUIC transport encryption. Device authorization must be end-to-end and independent of server account authentication. Treat MLS as a worthwhile post-MVP or prototype path for stronger group rekey/forward-security properties, but do not invent those guarantees for a simpler epoch-key scheme. citeturn169970search0

**Recovery:** trusted-device approval + recovery key. Do not allow an email-only reset to transparently recover end-to-end content keys.

**History:** maintain approximately ten logical entries, sync encrypted events rather than blindly syncing system clipboard state, and only alter a device's current OS clipboard on explicit local user action.

**Native remote paste:** Windows is sufficiently supported by documented Shell APIs to proceed. macOS Finder-paste compatibility is still an explicit unknown and one of the first experiments. Android/iOS should use their sanctioned sharing/content-provider models instead of emulating desktop clipboard behavior. citeturn129445search2turn129445search5turn650196search5

The highest-priority unanswered questions are therefore narrow enough to test rather than debate:

| Priority | Unknown | Recommendation |
|---|---|---|
| P0 | Can Finder consume OneCopy-backed promised remote files through normal paste, not only drag? | Build prototype immediately |
| P0 | Which current Wayland compositors permit reliable clipboard-manager capture and injection? | Test GNOME, KDE, wlroots independently |
| P0 | Can iroh be configured so Free mode never requires payload relay while retaining strong direct traversal? | Prototype before transport commitment |
| P0 | Real-world direct-QUIC success rate across CGNAT/corporate networks | Large connection benchmark |
| P0 | Correct file consistency behavior under live modification on each filesystem | Fault-injection test |
| P1 | Whether an optional Android IME materially improves clipboard UX enough to justify complexity | Small UX/security prototype |
| P1 | Best iOS split between foreground P2P and Premium HTTPS background staging | Device benchmark |
| P1 | R2 upload/download performance by target geography | 1/10/100 GB regional benchmark |
| P1 | Whether MLS complexity is justified for an account containing typically 2–10 devices | Security prototype/review |
| P2 | User-owned always-on relay/staging node | Add after managed core is stable |

The technical conclusion is **Go**, with one qualification: OneCopy is feasible as a universal **device clipboard network**, but not as a literally identical always-on system clipboard interceptor on all platforms. The strongest architecture embraces that reality rather than fighting OS security models: seamless and automatic on desktop where permitted, explicit and native on mobile where required, direct P2P whenever possible, and paid managed infrastructure only when it supplies a real benefit—connectivity, background reliability, or explicit offline availability.