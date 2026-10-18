# Camera Panel Reviewed Design

> **For agentic workers:** Use executing-plans or subagent-driven-development task by task, following owner routing and one-writer rules. Steps use checkbox syntax. No commits, publication, installation, or enabling without task-specific owner authority.

**Goal:** Expand Glint's existing popup with observed application/device information and evidence labels, without changing detection, camera-only visibility, or Edge Light behavior.

**Architecture:** Enrich the Rust observer's existing positive observations with bounded, best-effort metadata. Keep detection state authoritative and independent of attribution failures. Extend the existing popup rather than adding a second panel or capturing video.

**Tech Stack:** Existing Rust/Cargo, pipewire-rs, libc, Qt Quick/QML and Omarchy APIs; no new dependency proposed. Consult current official PipeWire and Linux procfs/sysfs documentation before implementation.

**Spec:** `HANDOFF.md` camera-panel roadmap and the design below. Owner reviewed this baseline and requested an execution-ready plan. See `2026-10-17-camera-panel-implementation.md`; no implementation is authorized by this documentation step.

## Global Constraints

- Hushlight and packaged Omarchy files remain untouched.
- No video preview, camera opening, frames/audio, virtual camera, effects pipeline, root, or runtime network.
- No browser-tab/site attribution, window titles, process command lines, environment reads, process control, or history.
- Preserve camera capture versus device-access wording; an absent indicator is never proof of inactivity.
- Preserve detection thresholds, confirming lookups, reconciliation limits, merge precedence, disconnect cleanup, and ordered stop/start transitions.
- Metadata failure never hides positive camera evidence, disables illumination, or invents an app/device pairing.
- Preserve camera-only icon visibility, lock gating, manual Edge Light controls, pointer recession, protected regions, and popup click forwarding.
- Existing 16 KiB JSON-line limit remains. No metadata or process identities in diagnostic logs.
- Write behavioral tests before nontrivial implementation; no tests searching source-file text.
- Work must stay off main/protected branches. Inspect current workspace first; never discard existing work or automatically commit/push.

## Review Focus

- Concurrent apps/cameras: show only established pairs, never a Cartesian product of holder and device lists.
- PID reuse/process exit: attribution must not transfer to a different process or survive a camera/session removal.
- Partial metadata: unavailable labels degrade to explicit unknowns without changing the camera gate.
- Untrusted/oversized labels: bound, sanitize and render plain text; metadata must not overflow framing or reset detection.
- Panel growth: list scrolling, focus and continuously updated popup cutouts must keep the toggle and controls reachable.

## Proposed User Experience

These are implementation proposals for review, not previously agreed detailed UX.

- Keep the current camera button. Its popup becomes a camera panel: heading, current observed activity, then existing Edge Light controls.
- Show one row per distinct observed app/device/evidence combination; handle multiple simultaneous observations rather than assuming a single caller.
- Each row has application/process label, device label, and either `Camera capture observed` or `Camera device accessed`.
- PipeWire application names are self-reported. Direct-access labels are process names, not authenticated application identity. Display `Reported app: …` or `Process: …` accordingly.
- Unknowns read `Unknown application` and `Unknown camera`. An unknown app does not prevent showing a known device, and vice versa.
- A browser process can be identified as a process; never name a tab, meeting, website, or inferred caller behind a portal/helper.
- No camera/device picker, app permissions, mute/kill buttons, icons downloaded at runtime, or automatic camera switching.
- Keep the three Edge Light sliders and their current meanings unchanged. Observational device names do not change which display is illuminated.
- Use a bounded scrollable activity section. Keep Edge Light's toggle visible outside that scroll section. Popup width may grow from 320 to 360 logical pixels, capped to available screen space; existing geometry reporting must follow its actual size.

## Metadata Contract

Keep protocol version 1 and existing required fields unchanged. Add optional `camera.details`:

```json
{
  "observations": [
    {
      "application": "brave",
      "applicationSource": "process",
      "device": "HD Pro Webcam C920",
      "devicePath": "/dev/video2",
      "evidence": "device-access"
    }
  ],
  "truncated": false
}
```

- `application`, `device`, and `devicePath` may be null. `applicationSource` is `process`, `pipewire-reported`, or `unknown`; `evidence` is `device-access` or `pipewire-capture`.
- `observations` contains active evidence only. No `details` or an empty list means identification is unavailable, not that no camera is used.
- Old helpers without details continue to work. Old widgets can ignore the additive field. Invalid details are discarded independently of an otherwise valid status.
- Do not serialize PIDs, process start times, executable paths, arbitrary FD targets, or internal correlation keys.
- Cap emitted rows at 32; bound each label to 80 UTF-8 bytes, remove control characters and bidi formatting controls, and retain valid UTF-8. Device paths must match `/dev/video` plus digits and be at most 64 bytes; otherwise emit null.
- Serialize within 16 KiB: drop excess detail rows and set `truncated`; if necessary omit details entirely. Never downgrade an otherwise valid active status solely because enrichment is too large.
- Deduplicate exact normalized display tuples and sort deterministically by device path, application label/source, and evidence. Do not merge capture and access rows merely because labels look similar; metadata is insufficient to prove they are the same session.

## Files and Responsibilities

- Create `helper/src/attribution.rs`: internal observation types, label normalization, deterministic projection/deduplication and bounds.
- Create `helper/src/process_identity.rs`: bounded process-name/start-time and character-device FD metadata lookup for known holder PIDs only.
- Create `helper/src/device_metadata.rs`: device-node identity and sysfs name lookup, without opening cameras.
- Modify `helper/src/device_access.rs`: retain device relationships separately from the existing aggregate detection state.
- Modify `helper/src/camera.rs` and `helper/src/observer.rs`: retain source device labels/path and resolve consumer application metadata, honoring partial-update masks.
- Modify `helper/src/status.rs`, `helper/src/runtime.rs`, `helper/src/main.rs`, `helper/src/lib.rs`: carry optional details through coherent snapshots and bounded JSON emission.
- Modify `ui/EdgeLightState.qml`: validate/project optional details independently of gating.
- Create `ui/CameraActivity.qml`: bounded scrollable plain-text activity rows.
- Modify `ui/EdgeLightPopup.qml`: compose activity section and unchanged light controls; retain popup protection reporting.
- Add Rust unit/integration and QML protocol/activity tests; extend existing bar/pointer/native fixtures where their interfaces change.
- Update `README.md`, `HANDOFF.md`, and `docs/edge-light-verification.md` with supported attribution paths and limits.

## Task 1: Define Pure Attribution and Protocol Projection

**Interfaces:** `attribution.rs` defines internal `Observation`, `ApplicationSource`, `EvidenceKind`; `project(observations: &[Observation]) -> CameraDetails` produces bounded public fields. `Status` gains optional details, with existing `Status::new` callers retaining their current behavior.

- [ ] Write failing tests for unknown labels, independent app/device availability, multiple observations, exact deduplication, deterministic ordering, Unicode boundaries, quotes, control/bidi characters, and row/path limits.
- [ ] Run targeted Cargo tests and observe the expected missing behavior before implementing projection.
- [ ] Implement the pure projection with no filesystem or PipeWire access. Do not replace the existing status classifier with attribution logic.
- [ ] Write failing serializer/merge tests: old status compatibility; positive evidence survives missing metadata; both positive paths retain their own rows; inactive/failed paths contribute no stale rows; maximum-sized/escaped metadata stays within 16 KiB while preserving state/evidence/limitations.
- [ ] Implement additive serialization and merge behavior. Details changes must emit an updated line, but existing meaningful stop/start events must not be coalesced away.
- [ ] Run all Cargo tests, fmt check and Clippy. Expected: green suite and no warnings.

## Task 2: Direct Device-Access Attribution

**Interfaces:** `process_identity.rs` exposes `observe_holder(proc_root: &Path, pid: u32, devices: &[DeviceIdentity], deadline: Instant) -> HolderAttribution`; `device_metadata.rs` exposes `read_device(sys_root: &Path, path: &Path) -> Option<DeviceIdentity>`. Filesystem roots are injected for fixture testing, not production CLI options. Internal `DeviceIdentity` carries device major/minor and optional bounded display name.

- [ ] Read Linux procfs `stat`, `comm`, FD metadata and sysfs video-class documentation. Preserve the existing aggregate `fuser` query; its combined PID output alone cannot identify a device pair.
- [ ] Write failing filesystem-fixture tests for one holder/one device, one holder/two devices, two unrelated holders/devices, process exit/reuse, denied metadata, truncated names, missing sysfs files, descriptor churn, and scan-budget exhaustion.
- [ ] Resolve names using bounded `/proc/<pid>/comm` and `/sys/class/video4linux/videoN/name` reads. Check `/proc/<pid>/stat` start time before/after attribution; discard the process label/pair if identity changes. Do not read cmdline, environ, arbitrary symlink target strings, or application files.
- [ ] Establish pairs by comparing character-device `rdev` metadata of known video nodes with metadata for the known holder's `/proc/<pid>/fd/*` entries. This is metadata-only: never open the descriptors or device nodes. A failed association yields unknown device, not all discovered devices.
- [ ] Bound enrichment to 16 holders, 256 FD entries per holder, 4 KiB per metadata file, and 100 ms elapsed budget per reconciliation; check the deadline between filesystem operations. These are best-effort local IO bounds, not a claim that the kernel guarantees every filesystem call returns within 100 ms.
- [ ] Keep metadata out of the critical detection result: publish coherent camera state first, then best-effort details. Do not hold stdout or PipeWire processing behind additional unbounded work. Cache device names only by verified device identity and invalidate on hotplug/removal; process labels must not be cached by PID alone.
- [ ] Add confirming-sample integration tests: brief probes produce no active observation rows; attribution failure leaves the active gate intact; stop/removal clears rows; another still-active caller remains visible. Run full Cargo suite and lint/format checks.

## Task 3: PipeWire Attribution and Coherent Runtime Updates

**Interfaces:** extend normalized `GraphNode` with optional device description/path and consumer application metadata as needed. `GraphStore` produces camera state plus internal observations from the same coherent graph. Channel/runtime updates carry both together; merge remains `active > unknown > unobserved`.

- [ ] Write failing fixtures for consumer-owned app name versus source-owner name, missing clients, portal-owned sources/helpers, multiple consumers/cameras, partially updated node/client properties, removal and reconnect.
- [ ] Resolve application metadata from the running capture consumer/client, never automatically from the hardware source's owner. Prefer reported application name, then process-binary basename; generic or proxy labels remain explicitly reported/unknown, not a guessed end-user app.
- [ ] Device labels prefer source `node.description`, then `node.nick`, then verified video-device sysfs name; otherwise unknown. Accept a video path only when its property actually identifies `/dev/videoN`; do not invent a device mapping from node names or hardware presence.
- [ ] Apply properties/state change masks without wiping retained metadata on unrelated updates. A changed consumer, removed link/source/client, or disconnected graph must invalidate old rows as well as the old state.
- [ ] Test runtime ordering for active A → stopped → active B; no old labels survive into B. A positive direct-access path must remain intact during PipeWire disconnect, with only that path's rows retained.
- [ ] Extend the isolated metadata-only PipeWire fixture where feasible; no actual capture, normal-session socket, or hardware modules in automated tests. Run full suite and measure fresh helper line coverage with the existing LLVM tooling; require at least 80% overall, reporting remaining uncovered adapter paths honestly.

## Task 4: Camera Activity UI Without Edge Light Regressions

**Interfaces:** `EdgeLightState` exposes `cameraObservations` and `cameraDetailsTruncated`; required status parsing and `cameraActive` are unchanged. `CameraActivity.qml` consumes those values; the current popup remains its sole host.

- [ ] Write failing Qt6 tests for old helper snapshots, valid rows, null labels, malformed optional fields, oversized lists, stale clearing, and a metadata-only update while the light is enabled. Invalid details must not invoke `clearObserver()` for a valid camera status.
- [ ] Implement independent details parsing and clearing. Show fixed unknown labels when necessary; do not retain last-known app/device labels after stop/helper failure/new session.
- [ ] Build the activity section with plain-text labels, accessible names and bounded scrolling. Keep the existing toggle and sliders intact and reachable; show `More observations omitted` when truncated.
- [ ] Extend native click smoke and offscreen UI tests for multiple rows, long labels, maximum panel height, missing names, and activation/deactivation. Verify actual popup resizing updates the permanent clear region and pointer recession still works while controls are open.
- [ ] Run full Rust and QML suites, both native smoke tests, GPU cutout checks, QML lint/parse, manifest validation, fmt and Clippy. Record failures/skips/warnings rather than inferring live success from offscreen checks.

## Task 5: Review, Documentation and Optional Live Acceptance

- [ ] Update user-facing docs with label provenance, device-access semantics, metadata limits and unsupported browser-tab/portal attribution. Do not advertise authenticated app identity or all-app coverage.
- [ ] Capture a complete hashed change snapshot against the actual branch base and relevant dirty/untracked work. Use one fresh independent review under the owner's model-routing policy; no automatic provider switches on quota failure. Fix material findings with regression tests.
- [ ] Obtain separate explicit approval before installing/enabling this feature, committing, pushing, or changing the current installed plugin. Earlier feature publication is not blanket authorization for this change.
- [ ] If approved for live testing, the owner controls camera start/stop. Check an identified direct-access browser process, an observable PipeWire consumer when available, concurrent callers/devices where available, helper restart and unplug. Collect only the metadata needed for the feature; no frames, profiles or window titles.
- [ ] Verify the panel's reported pairs/unknowns against observed evidence and confirm Edge Light toggle, protected bar/popup, pointer recession, lock behavior and camera-stop shutdown remain usable. Document untested cases, not guarantees.

## Self-Review and Handoff

Checked scope, current aggregate-holder limitation, optional protocol compatibility, identity/device association, merge/clear semantics, bounded plain-text rendering, popup protection, coverage and installation/publication boundaries. All detailed UX and metadata-reading choices above remain proposals until owner approval. No product code, installed files, capture tests, commits or pushes belong to this planning step.
