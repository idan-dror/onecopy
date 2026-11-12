# OneCopy — Product vision

**Status:** Pre-code baseline. Product principles marked CONFIRMED; positioning statements are working language.

## Vision

**Copy once. Access securely on your own trusted devices.**

OneCopy is an independent, cross-platform clipboard and on-demand file-transfer experience that unifies a user's trusted devices without becoming another cloud drive. It gives people a short, useful history of text, links, images, files and folders, accessible from their own Windows, macOS, Linux, Android and iOS/iPadOS devices within the limits of each operating system.

## User problem

A person working across a workstation, laptop and phone should not need to email themselves text, manually upload a large file, or repeatedly locate the same copied content. Existing vendor ecosystems handle some cases well but fragment at OS boundaries. File transfer tools often require a separate send/receive workflow and rarely integrate with familiar paste behavior.

## Experience principles

1. **Familiar first:** normal copy and paste should work wherever the OS permits; where not feasible, provide a clear OneCopy workflow.
2. **Local ownership:** each device runs the OneCopy core; server infrastructure coordinates rather than processing plaintext.
3. **Independent shared history:** a remote copy appears in OneCopy history without taking over the receiver's active OS clipboard.
4. **On-demand, not automatic upload:** files stay on their source until explicitly requested; ordinary file copies are live references.
5. **Security by default:** E2EE, user-approved device enrollment, revocation, minimal retention, sensitive-content exclusions.
6. **Fast and predictable:** direct paths when possible, resumable streaming for large files, visible failures, and honest availability.
7. **No artificial free file-size cap:** Free direct transfers are limited by physical/network/device conditions, not a product-imposed byte quota.
8. **Premium pays for infrastructure:** gateway relaying and temporary offline staging are opt-in value, not hidden requirements for Free operation.
9. **No OS restrictions bypass:** the feature vocabulary and UX reflect real platform capabilities.
10. **No per-app plugins:** native OS clipboard bridges, not a collection of third-party application extensions.

## Audiences (working personas)

- A developer switching between Windows/Linux/macOS machines and copying commands, snippets and artifacts.
- A creator transferring high-resolution images, video exports and folders without speculative cloud uploads.
- A multi-device user moving short text and shared content between desktop and mobile using supported sharing workflows.

These are proposed personas, not market validation. User interviews and usage tests should refine them.

## Defining experiences

### Cross-device text

Copy on desktop A → encrypted OneCopy history appears on desktop B → choose item → native OS clipboard on B is updated locally → paste into the target application.

### Large file

Copy file on desktop A → distribute only reference metadata → select file on B → B requests current contents from A → authenticated, resumable direct P2P stream → materialize at destination/paste target. If A is offline or file is gone, explain why content is unavailable.

### Offline Premium

User explicitly chooses `Keep Available Offline` → source uploads an E2EE temporary copy → verification completes → available from an enrolled destination device even if source is offline. The copy is identified by its staging timestamp, can be refreshed explicitly and expires according to the entitlement.

### Restricted mobile

Share to OneCopy or select from OneCopy history; do not promise invisible, continuous clipboard interception on Android/iOS.

## Success and failure

Success: reliable native desktop workflows, safe file references, fast direct transfer, honest availability, high trust in privacy, sensible Premium economics.

Failure: claiming universal Ctrl+C/Ctrl+V on stock mobile, silently staging source files, relaying Free payloads without consent/entitlement, or corrupting a file while reporting completion.

## Not OneCopy

Not a cloud backup system, version-control service, permanent shared drive, public file-hosting platform, password manager, clipboard command executor, universal remote filesystem mount, or application-plugin ecosystem.

## Long-term product trajectory

Start desktop-first with a shared headless core, prove hard native paste and networking constraints, then broaden OS integrations and introduce priced reliability infrastructure. Phase boundaries appear in `ROADMAP.md`; no schedule is approved by this document.
