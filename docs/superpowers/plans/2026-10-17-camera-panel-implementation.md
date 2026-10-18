# Camera Panel Implementation Plan

> **For agentic workers:** Use executing-plans or subagent-driven-development task by task. Follow owner model routing, maintain one writer, and use checkboxes to record evidence. Do not automatically commit, push, install, or enable.

**Goal:** Show evidence-backed camera application/device observations above Glint's unchanged Edge Light controls.

**Architecture:** Add optional attribution to existing version-1 camera status lines. Detection remains authoritative; enrichment is a separate, generation-checked result and cannot change camera state. Reuse the current popup, shared state, and protected-region machinery.

**Tech Stack:** Existing Rust/Cargo, pipewire-rs, libc, Qt6/QML and Quickshell; no new runtime or system dependency. Propose `serde_json = "1"` as a test-only dev-dependency so wire-format assertions use a real parser, not string matching or a second hand-written parser; approval of this plan includes that bounded tooling choice.

**Spec:** Reviewed design: `docs/superpowers/plans/2026-10-17-camera-panel.md`; project boundaries: `HANDOFF.md`. Decisions resolving implementation gaps are called out below. This plan is pending final review, not a coding instruction yet.

## Global Constraints

- Hushlight and packaged Omarchy remain untouched. No root, runtime network, frames/audio, camera opening, preview, or virtual camera.
- Never read browser profiles, window titles, process command lines/environments, or arbitrary FD target strings. No history, process control, or metadata logging.
- Preserve existing camera classification, confirmation/debounce/reconciliation timing, merge precedence and stop/start ordering. An unavailable label is not unavailable camera evidence.
- Preserve version 1 and existing required JSON fields. Keep the 16 KiB line cap. Enrichment overflow drops details, not positive detection state.
- Preserve all Edge Light slider values/meaning, pointer tracking/cutouts, lock checks, manual enable behavior and click forwarding.
- UTF-8 labels: at most 80 bytes, strip control/bidi formatting characters; emitted rows at most 32. Device paths match `/dev/video[0-9]+`, at most 64 bytes.
- Keep internal PIDs/start times/generations out of stdout and diagnostics. Treat PipeWire names as reported labels, not authenticated identities or browser-tab attribution.
- TDD for logic; no source-text assertions. Full relevant checks and at least 80% helper line coverage before delivery.
- Confirm working tree and base before implementation. Stay off default/protected branches; preserve existing changes. Prior commit/push/install authority does not carry to this feature.

## Review Focus

- False associations across multiple devices: only proven FD/device or active graph-link relationships become pairs.
- PID reuse and stale enrichment: old labels must not attach to a new caller/session.
- Metadata-only updates: must not reset the light, lock state, or camera epoch.
- Payload/label failures: must remain bounded and degrade independently of detection.
- Growing activity lists: must not obscure the toggle, overflow the panel, or invalidate the popup's protective cutout.

## Execution Decisions

1. **Direct-access identity requires a matching video FD.** A PID from aggregated `fuser` output plus matching start-time reads alone cannot exclude PID reuse before the first read. Without a current matching character-device FD, emit an unknown observation, not a process-name guess. This tightens the reviewed design's safety requirement without changing detection.
2. **No per-device `fuser` fan-out.** Keep the existing aggregate lookup. Associate only metadata from already-known holders with already-discovered video nodes.
3. **No PID-only cache.** Device-name caching is unnecessary in v1: read it during bounded enrichment. Process identity is checked before/after each scan.
4. **One attribution worker, one pending latest job.** Do not spawn per reconciliation. While busy, replace the queued job with the newest generation. Worker results are accepted only if the same source generation is still active. A slow kernel metadata read must not hold camera-state publication or shutdown indefinitely; worker lifetime belongs to the helper process, not the widget separately.
5. **UI bounds:** width `fittedContentWidth(360)`, activity height `min(160, contentHeight)`, maximum panel height `min(560, availableCardHeight)`. Keep the toggle outside the scrollable activity list; on unusually short screens allow the controls section to scroll too rather than clip or shrink controls to zero.

## Exact Data Interfaces

Use existing spelling/camel-case serialization conventions; internal Rust names stay snake_case.

```rust
enum ApplicationSource { Process, PipewireReported, Unknown }
enum EvidenceKind { DeviceAccess, PipewireCapture }
struct Observation {
    application: Option<String>, application_source: ApplicationSource,
    device: Option<String>, device_path: Option<String>, evidence: EvidenceKind,
}
struct CameraDetails { observations: Vec<Observation>, truncated: bool }
struct DeviceIdentity { path: String, device_id: u64, name: Option<String> }
struct ProcessIdentity { pid: u32, start_ticks: u64 }
struct HolderAttribution {
    identity: Option<ProcessIdentity>, application: Option<String>,
    devices: Vec<DeviceIdentity>, incomplete: bool,
}
```

Define these in `attribution.rs`, except `ProcessIdentity`/`HolderAttribution` in `process_identity.rs`. Derive clone/equality as needed. Add `Status.details: Option<CameraDetails>`; `Status::new` initializes it to `None`. Keep `DeviceReconciler`'s detection snapshot independent of attribution.

The public JSON schema is exactly the reviewed design's optional `camera.details`; enums serialize as `process` / `pipewire-reported` / `unknown` and `device-access` / `pipewire-capture`. No protocol-version bump or new required field.

## Task 1: Pure Projection, Merge and Wire Contract

**Files:** create `helper/src/attribution.rs`, `helper/tests/attribution.rs`; modify `helper/src/{lib,status,runtime}.rs`, `helper/tests/{status_merge,runtime}.rs`, `helper/Cargo.toml` and `helper/Cargo.lock` for the test-only parser.

**Produces:** `normalize_label(raw: &str) -> Option<String>`, `project(rows: &[Observation]) -> CameraDetails`, additive status serialization/merge. `project` normalizes, exact-deduplicates, sorts by `(device_path, application, application_source, evidence)`, then caps to 32 rows. Null option values sort before non-null values. `truncated` describes rows actually omitted, not speculative missing activity.

- [ ] Write `normalizes_untrusted_labels`: `"  brave\n" → Some("brave")`; remove Unicode control characters plus U+061C, U+200E/F, U+202A–E, U+2066–9; 41 repetitions of `é` become 40 repetitions (80 UTF-8 bytes); blank input becomes None. Run it and observe RED before implementation.
- [ ] Write projection tests for independently null app/device fields; exact duplicates; same labels but distinct evidence retained; 33 distinct rows producing 32 and `truncated=true`; invalid or overlong paths becoming null; deterministic ordering from shuffled input. Implement the named functions, then rerun.
- [ ] Add the proposed `serde_json` dev-dependency after consulting its current docs; write serializer tests with `serde_json::Value`. Assert old field values unchanged, no internal identity fields, round-trip quotes/backslashes, and `line.len() <= 16_384` for worst-case escaping. Do not replace production serialization or add this dependency to the runtime dependency section.
- [ ] Write `metadata_budget_never_downgrades_active`: deliberately oversized details shrink/omit until the line fits while `state/evidence/limitations` remain intact. Change `bounded_json_line` to budget details before its existing base-status fallback.
- [ ] Write merge tests: only active source rows contribute; both active paths retain their respective rows; unknown source metadata is discarded; details-only changes emit a line; active → stopped → active emits all meaningful states. Implement merge/runtime integration without coalescing transitions.
- [ ] Run `cargo test --manifest-path helper/Cargo.toml`, `cargo fmt --manifest-path helper/Cargo.toml --check`, and `cargo clippy --manifest-path helper/Cargo.toml --all-targets -- -D warnings`. Expected: all green, no new warnings.

## Task 2: Bounded Direct-Access Attribution

**Files:** create `helper/src/{process_identity,device_metadata}.rs`, `helper/tests/process_identity.rs`; modify `helper/src/{device_access,main,lib}.rs`; create `helper/src/attribution_worker.rs` and `helper/tests/attribution_worker.rs` for worker generation/queue logic.

**Interfaces:**
- `read_device(sys_root: &Path, dev_root: &Path, video_name: &str) -> Option<DeviceIdentity>` validates the video name, reads node metadata and bounded sysfs `name`, returns logical `/dev/videoN` rather than exposing fixture-root paths.
- `observe_holder(proc_root: &Path, pid: u32, devices: &[DeviceIdentity], deadline: Instant) -> HolderAttribution` reads only stat/comm and FD metadata for that known PID. Root injection exists for tests, not a production CLI flag.
- `AttributionJob { generation: u64, holders: BTreeSet<u32>, devices: Vec<DeviceIdentity> }` and `AttributionResult { generation: u64, details: CameraDetails }`; source generation stays internal. Worker has at most one running job and one replaceable pending job; no unbounded queue.

- [ ] Read current procfs/sysfs docs, especially field 22 of `/proc/<pid>/stat`, command names containing spaces/parentheses, FD races, and `MetadataExt::rdev` / `FileTypeExt::is_char_device`.
- [ ] Write failing fixtures under `/tmp/opencode`: fake proc/stat/comm/sysfs files and temporary FD/video-node symlinks to `/dev/null` and `/dev/zero` for two distinct harmless character-device identities. Never use real camera nodes, `mknod`, or root. Inject metadata-reading functions where needed to deterministically simulate permission errors, process replacement, and time/budget exhaustion.
- [ ] Tests must establish one holder/one device, one/two devices, two unrelated holders with no cross-pair, failed comm with known device, missing stat, changed start time, vanished FD, truncated FD scan and deadline expiry. Do not claim a process identity when no current matching video FD exists.
- [ ] Implement bounded reads (4 KiB per metadata file), at most 16 holders and 256 FD entries per holder, and a 100 ms job deadline checked between operations. Use `metadata` on FD paths, not opening descriptors or reading symlink targets. A denied/missing path yields unknown attribution, not inactivity.
- [ ] Write worker tests before implementation: pending generation 2 replaced by 3; result 1 discarded after stop/new session; no enrichment job for unconfirmed short probes; results never alter detection state; worker failure/late result cannot block status publication. Attach details only to currently confirmed active direct-access state.
- [ ] Keep state publication on the current reconciliation path before queuing enrichment. Invalidate generation on holder/device changes, lookup failure, or observation stop. When a result returns, recheck the generation and active evidence before emitting details.
- [ ] Add integration tests preserving the current 750 ms confirmation threshold, 1 s minimum reconciliation spacing, 5 s fallback, child cleanup and existing positive-path merge semantics. Run the full Cargo suite/fmt/Clippy; expected: green.

## Task 3: PipeWire Metadata From Active Graph Relationships

**Files:** modify `helper/src/{camera,observer,status,runtime}.rs`, `helper/tests/{classification,transitions,runtime,pipewire_adapter}.rs`.

**Interfaces:** add `GraphNode.description: Option<String>`, `nick: Option<String>`, `video_path: Option<String>`; reuse existing `GraphOwner { binary, name }` for reported consumer/client identity. `camera::classify(&CameraGraph) -> Status` remains the public entry point and attaches details from the same active links that prove capture. No source ownership is used as consumer identity.

- [ ] Write failing fixtures for source owner `WirePlumber` plus consumer `Example Call`: only consumer appears as reported app. A generic reported label stays reported, never rewritten to a guessed Discord/browser app.
- [ ] Test app-name fallback to process-binary basename; no reported app for missing consumer/client; source device description before nick; verified sysfs fallback only for a validated `/dev/videoN`; unknown camera when no safe name exists. Native property mapping uses `node.description`, `node.nick`, `api.v4l2.path`, with validation before accepting a path.
- [ ] Extend existing state-only/property-only tests to all new metadata fields. Explicit property removal clears that value; unrelated update masks preserve prior values. Apply the corresponding client property-mask behavior too.
- [ ] Test removal/reconnect/concurrent consumers and cameras, portal sources excluded as before, and snapshot A → stop → B with A's rows absent from B. Positive direct-access rows survive a PipeWire-only disconnect.
- [ ] Implement node/client projection and normalized active-pair extraction. Never add observations from merely discovered, suspended, unlinked, or inactive-link devices.
- [ ] Extend the isolated native PipeWire metadata fixture; do not connect tests to the normal user daemon or load hardware modules. Run full Rust tests/fmt/Clippy and fresh LLVM line coverage using installed matching LLVM tooling. Gate: at least 80% helper lines overall, including adapters/main in the denominator; disclose untested live paths.

## Task 4: Optional Metadata Parser and Camera Activity Component

**Files:** create `ui/CameraActivity.qml`, `tests/qml/tst_CameraDetails.qml`, `tests/qml/tst_CameraActivity.qml`; modify `ui/{EdgeLightState,EdgeLightPopup}.qml`, `tests/bar-click-smoke.qml` and relevant existing protection/GPU tests.

**Interfaces:** state adds `cameraObservations: var` (array, default `[]`) and `cameraDetailsTruncated: bool` (default false). `consumeCameraDetails(value)` validates without throwing into required-status parsing. `clearCameraDetails()` clears both. `CameraActivity` exposes `observations`, `truncated`, `maximumHeight` and read-only `contentHeight` for popup sizing.

- [ ] Write RED parser tests: old version-1 helper with no details accepted; null labels; supported enum values; malformed details/rows/string types; over-32 rows; oversized labels; invalid device path; malicious rich text/control characters; active→stop/unknown/new session clearing.
- [ ] Pin `metadata_only_update_preserves_light`: start an active/unlocked manually enabled session, apply new details with the same active state, assert unchanged `enabled`, `targetOutput`, `cameraEpoch`, lock readiness and slider values. A malformed details block yields empty metadata while the valid active status remains active.
- [ ] Implement `consumeCameraDetails` only after validating required fields; validate at most 32 rows and reject the entire invalid optional block. Retain the existing whole-line cap. Clear metadata on `clearObserver` and any non-active state; never retain last-known labels as current evidence.
- [ ] Write component tests for `Unknown application`, `Unknown camera`, `Reported app: …`, `Process: …`, exact evidence labels, and `More observations omitted`. Render every metadata-bearing label as plain text; never load metadata-supplied icons, URLs or QML.
- [ ] Implement a maximum-160-pixel activity scroll area, then unchanged Edge Light controls. Use `fittedContentWidth(360)` and the reviewed height caps. Verify short-screen scrolling without hiding the toggle and re-publish popup geometry on actual content-size changes.
- [ ] Extend native click smoke with multiple/long rows and assert popup open/close still goes through `triggerPress`. Verify protected-region sizing changes, pointer recession and camera-stop shutdown with real component behavior, not source matching.
- [ ] Run all Rust tests, Qt6 GPU tests, both native smoke fixtures, QML lint/parse, manifest validation, Cargo fmt and Clippy. Expected: green; retain known environment warnings explicitly rather than claiming warning-free native-host validation.

## Task 5: Review and Owner Handoff

- [ ] Update `README.md`, `HANDOFF.md`, `docs/edge-light-verification.md`: describe reported versus process identity, unknowns, scan limits and observational semantics. No claims of browser-tab identity, full all-app coverage, or preview.
- [ ] Capture base/HEAD, complete staged/unstaged diffs and relevant untracked files with hashes. Run one fresh independent review using owner routing; no redundant re-review layers or provider changes without approval. Fix material findings with failing regression tests first.
- [ ] Record exact check commands, counts, coverage, failures/skips and remaining native-host uncertainty against the reviewed snapshot. Do not carry old Edge Light coverage/test counts forward as evidence for new code.
- [ ] Stop for owner review before any install, enable, commit, push or publication. If later authorized to install, back up only Glint, retain Hushlight and use the normal guarded shell restart; never bypass lock refusal.
- [ ] Separately approved live acceptance: owner starts/stops capture; verify direct-access process/device pair or honest unknowns, available PipeWire consumer, concurrent users/devices when available, metadata loss, helper restart and hotplug. Collect no frames or unrelated process data. Confirm all Edge Light access/safety behavior remains usable.

## Check Commands

```sh
cargo test --manifest-path helper/Cargo.toml
cargo fmt --manifest-path helper/Cargo.toml --check
cargo clippy --manifest-path helper/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path helper/Cargo.toml
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl /usr/lib/qt6/bin/qmltestrunner -input tests/qml
bash tests/bar-click-smoke.sh
bash tests/bar-click-smoke.sh pointer-smoke.qml
omarchy plugin validate .
```

For QML lint, create an ephemeral importer with `qs` pointing to `/usr/share/omarchy/shell`, then use `/usr/lib/qt6/bin/qmllint -I <importer> -I /usr/lib/qt6/qml Widget.qml Service.qml ui/*.qml`. Run Qt6 `qmlformat` parse-only and discard output, preserving repository style. Run `bash -n tests/bar-click-smoke.sh`. Consult existing LLVM instrumentation instructions/evidence before measuring fresh coverage; do not install tools automatically.

## Self-Review

Checked optional-schema compatibility, independent failure paths, real device-pair proof, PID replacement before/after reads, generation rejection, queue bounds, partial-update preservation, rendering limits, cutout lifecycle and publication boundaries. No implementation or deployment occurs in this planning step. Review this execution plan before coding; the previously chosen inline execution method can be retained.
