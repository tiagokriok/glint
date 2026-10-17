# Edge Light Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or executing-plans to implement this plan task-by-task. Follow the owner's routing policy: one writer, routine implementation on Go, no automatic publication. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Offer a camera-gated Glint bar control that opens manual Edge Light controls and automatically extinguishes the light when observed camera use ends.

**Architecture:** A Rust helper merges PipeWire camera-capture evidence with sustained `/dev/video*` device-holder evidence, matching Hushlight's detection contract, and emits bounded JSON status lines. A singleton QML service owns shared session state and the only display-edge overlay; per-monitor widgets own anchored control popups. No camera frames or processed video are needed for illumination. Device access permits the camera gate but is never described as proven streaming.

**Tech Stack:** Rust stable, Cargo, pipewire-rs, Qt Quick, Quickshell, Omarchy plugin APIs. Check current official documentation and installed versions before implementation; dependency selection is not an authorization to install system packages.

**Spec:** `HANDOFF.md` plus the confirmed behavior and defaults below. The owner approved execution and the defaults on 2026-10-16, then explicitly approved Hushlight-compatible device-access gating. Live installation/enabling and capture tests still require separate approval.

## Global Constraints

Execution status (2026-10-16): Tasks 1–4 implemented and locally verified; final
snapshot review findings addressed and scoped re-review approved. Task 5 local
checks pass; installed/live acceptance remains pending separate approval. Source
is on `feat/edge-light`. Evidence: `docs/edge-light-verification.md`.


- Hushlight and its installed plugin remain untouched; do not use its tests as Glint's suite.
- No Python, root, network runtime access, virtual camera, frame capture, or video effects.
- No changes to `/usr/share/omarchy/` or desktop configuration.
- No install, enable, commit, push, merge, or deployment without explicit approval.
- Glint's top-bar icon shows only while camera use is observed, not for microphone-only activity.
- Camera-use stop closes the popup, removes the icon, and turns Edge Light off.
- An unavailable observer is unknown, not proof of global camera inactivity; fail closed for illumination and controls.
- A failed individual detection path does not override positive evidence from the other path. "Observer failure" means helper/protocol failure or a merged state other than active.
- Sustained `/dev/video*` holder evidence may reveal Glint and gate Edge Light, as explicitly approved. Label it "Camera device accessed", not recording or streaming. A raw open event alone must not activate the gate.
- UI surfaces use Omarchy theme roles. The illumination itself deliberately uses adjustable white-to-warm light, not a privacy status color.
- Write behavioral tests before implementation; do not test QML by searching source text. No automatic commits at task boundaries.

## Confirmed Behavior

- Start with Edge Light, not the other roadmap features.
- Camera use gates the top-bar icon and access to the popup. For this feature, use means either observed PipeWire camera capture or sustained device access under Hushlight's existing contract.
- Clicking the icon exposes a manual toggle, width, brightness, and warmth controls.
- Turning the camera off also turns Edge Light off.

## Approved Defaults

- Each newly observed camera session starts with Edge Light off; no automatic re-enable after stop, reconnect, or reload.
- One display is illuminated: the display containing the widget clicked to enable it. Other displays must not acquire independent lights or conflicting state.
- Width: 2–15% of the shorter display dimension, default 6%; brightness: 10–100%, default 70%; warmth: 0–100%, default 30%.
- Brightness means rendered light intensity, not changing monitor hardware brightness. Warmth interpolates neutral white toward a warm white; it is not a calibrated Kelvin guarantee.
- Values survive popup close during the current plugin lifetime, but are not persisted to disk in v1.
- Closing the popup leaves an enabled light on. A camera stop, observer failure, display removal, plugin unload, session lock, or lock-state unknown turns it off and requires manual re-enable.
- The overlay does not accept input, reserve screen space, alter focus, or cover lock-screen surfaces. No fade animation in v1.
- Keep the existing camera-mark language initially; no microphone indicator is implemented in this feature.

## Review Focus

- Direct-camera paths: sustained holders may activate the gate, but brief probes must not; device access must never be presented as confirmed active video.
- Observer disconnect or malformed output: remove stale camera state and immediately extinguish illumination.
- Multiple displays and widget instances: exactly one enabled overlay, no duplicated observers or independent toggles.
- Lock, reload, and display removal: no stranded light and no surprise automatic re-enable.
- Scaling and screen-edge geometry: a continuous border with a transparent center and click-through interaction at all edges.

## File Structure

Proposed files; use installed Omarchy patterns to confirm lifecycle ownership before scaffolding:

- `manifest.json`: `glint.privacy` plugin registration; declare a service entry only if needed for singleton ownership.
- `Widget.qml`: camera-gated bar control and popup integration.
- `Service.qml`: single shared observer/session owner, if supported by the installed plugin facade.
- `ui/EdgeLightPopup.qml`: toggle and three labeled sliders.
- `ui/EdgeLightOverlay.qml`: output-bound layer surface and border rendering.
- `ui/EdgeLightState.qml`: nonvisual QML state with testable transition methods.
- `helper/Cargo.toml`, `helper/Cargo.lock`: Rust helper and pinned dependency resolution.
- `helper/src/main.rs`: startup and JSON-lines output.
- `helper/src/camera.rs`: pure graph classification.
- `helper/src/observer.rs`: PipeWire subscriptions and coherent snapshots.
- `helper/src/device_access.rs`: device discovery, open/close notifications, bounded holder reconciliation, and sustained-access classification.
- `helper/src/status.rs`: status serialization and uncertainty semantics.
- `helper/tests/`: classification, stream protocol, and lifecycle regression tests.
- `tests/qml/tst_EdgeLightState.qml`: behavioral Qt Quick tests.
- `docs/edge-light-verification.md`: commands, coverage limits, and live acceptance results.

## Task 1: Resolve the Camera-Gating and Host-Lifecycle Prerequisites

**Files:** Update `docs/edge-light-verification.md` and this plan only.

**Interfaces:** Produces the verified definition of `cameraActive`, the singleton ownership mechanism, and the supported-path list used by subsequent tasks.

- [x] Read Hushlight's `AGENTS.md`, `Widget.qml`, `backend/status.py`, and `docs/capture-research.md` without modifying them.
- [x] Read installed Omarchy service, popup, and layer-surface examples and current Quickshell/PipeWire documentation. Confirm supported singleton sharing, lock visibility, output lifecycle, input-region control, and overlay stacking. Do not clone/enable a reference plugin. (Lock visibility: no reactive signal exists; the owner accepted best-effort polling — see the resolved block below.)
- [x] Inventory installed Rust/Cargo, PipeWire development libraries, Qt Quick test tools, and QML import paths. Record exact verification commands; request approval before installing missing dependencies.
- [ ] With separately approved live testing and user-controlled capture, verify camera start/stop metadata for the user's target app without reading frames, browser data, or command lines. Record observability gaps and concurrent-use behavior. (Deferred to Task 5; not authorized in Task 1.)
- [x] If live testing is not yet authorized, defer that check to Task 5 rather than block fixture-driven implementation of the approved Hushlight contract. Never claim live validation from fixtures.
- [x] Verify how to reproduce Hushlight's device discovery, event-triggered holder checks, debounce, hotplug fallback, and cleanup in Rust. Existing `fuser` is permitted as a bounded local helper; native notification handling is preferred to an additional watcher subprocess. No new system dependencies without approval. (Discovery/debounce/cleanup verified; native inotify is a candidate pending a chosen Rust crate, not yet pinned.)
- [x] Stop if neither approved observation path can be implemented, singleton ownership is unavailable, or lock safety cannot be established. Do not introduce raw-open or power-state heuristics or desktop changes. (Resolved: the owner accepted a best-effort lock reset; not a blocker.)
- [x] Update exact lifecycle APIs and commands in this plan before coding. Defaults and device-access gating are owner-approved.

**Acceptance:** A documented feasible Hushlight-compatible camera gate and host lifecycle, or an explicit blocker. Brave's PipeWire streaming gap remains a limitation, not a blocker to the approved device-access fallback; current live behavior still needs separately approved validation.

**Resolved host APIs and commands (Task 1, 2026-10-16).** Full evidence in `docs/edge-light-verification.md`. Details and commands:

- `cameraActive` := the helper's `camera.state === "active"` (PipeWire capture OR sustained device-holder evidence, merge `active > unknown > unobserved`). Device path wording "Camera device accessed"; never "streaming".
- Singleton owner: declare `kinds: ["bar-widget", "service"]` with `entryPoints.barWidget` and `entryPoints.service`. The host instantiates `Service.qml` once; widgets reach it with `bar.shell.serviceFor("glint.privacy")` (`Ui/PluginBarApi.qml:12`, `plugins/bar/Bar.qml:229-279,1999-2006`). Do not use relative-import singletons. Omit `keepLoaded` so a plugin reload unloads the service and resets state (`shell.qml:1452-1462,1033-1050`). `Service.qml` is the sole creator/owner of the one overlay (a `service` entry may own window types: `lock/Service.qml` is a `service` `Item` containing `PanelWindow`; instantiated via `shell.qml:914-923`). A replacement `bar` plugin gives widgets a service-less facade; document that the feature is unavailable there.
- Lock/reset (owner-accepted best-effort, fix round 2): no reactive lock signal exists (no Hyprland event, no logind `LockedHint`, no public generation). Poll `omarchy-shell lock status` every 1 s **while `cameraActive`** (covers popup/enable gating), one outstanding probe, 2 s timeout → lookup failure. Reset/disable on any of: `locked == true`, `lastEventAt` changed since baseline, or failure. The first successful unlocked probe sets the baseline and permits enable; lock unknown never permits enable; no automatic re-enable; a `lastEventAt` change resets even while currently unlocked. Accepted limitations: a fast cycle whose timestamp advance is not observed between probes may be missed, and `lastEventAt` also advances on non-lock events. Supported locker scope is `omarchy.lock` only. Reset uses `resetSession()`.
- Lock vs camera: `cameraActive` is independent. A lock reset denies/closes the popup and disables the light but never fabricates a camera stop; `cameraActive` may stay true while lock is locked or unknown.
- Popup coordination: `bar.requestPopout(owner)` / `bar.releasePopout(owner)`; `bar.activePopout` (`Ui/PopupCard.qml:72-86`).
- Overlay: `PanelWindow` with `screen` bound to the target `QuickshellScreenInfo`; `mask: Region {}`, `WlrLayershell.layer: WlrLayer.Overlay`, `keyboardFocus: WlrKeyboardFocus.None`, `exclusionMode: ExclusionMode.Ignore` (`plugins/osd/Osd.qml:126-137`). Detect output removal via `Quickshell.screens` / `screensChanged`, then `outputRemoved(name)`.
- Commands: `QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner -input tests/qml`; `/usr/lib/qt6/bin/qmllint -I <importer> -I /usr/lib/qt6/qml <files>` with `<importer>/qs -> /usr/share/omarchy/shell`; `/usr/lib/qt6/bin/qmlformat <file> > /dev/null`; `omarchy plugin validate .`. The `PATH` `qmltestrunner`/`qmllint` are Qt5 and unusable; use the Qt6 binaries.
- Observation/dependency notes: PipeWire observation follows the official registry + listener event model (docs.pipewire.org "API Tutorial — Part 2"; Rust `pipewire` crate 0.10.1). Native inotify (`inotify(7)`) replaces `inotifywait` only once a Rust inotify crate is selected at build time; until then it is a candidate, not pinned. The 2 s helper restart is inherited from Hushlight `Widget.qml:114-119`, not a user decision.

## Task 2: Build the Camera Observer

**Files:** Create `helper/` files above and Rust tests.

**Interfaces:** stdout emits `{"version":1,"camera":{"state":"active|unobserved|unknown","evidence":"plain text","limitations":["plain text"]}}` as one complete JSON object per line (the state value is one alternative, not the literal pipe-separated string). No `recording` field is required. stderr is diagnostic-only. Lines are limited to 16 KiB excluding newline; a consumer drops an oversized line without buffering it indefinitely and resets to unknown. Later tasks treat only `active` as permission to expose controls.

**Pure Rust interfaces:** `status.rs` defines `State` and `Status { state: State, evidence: String, limitations: Vec<String> }`, and `merge(pipewire: &Status, device: &Status) -> Status`. `device_access.rs` defines immutable `AccessSnapshot { holders: BTreeSet<u32>, stable_since: Option<Duration>, available: bool }`, `advance(previous: &AccessSnapshot, holders: BTreeSet<u32>, available: bool, now: Duration) -> AccessSnapshot`, and `classify(snapshot: &AccessSnapshot, now: Duration) -> Status`. Timestamps are monotonic durations from one process-local epoch; holder IDs never leave the helper. `camera.rs` owns normalized `CameraGraph` fixtures and `classify(graph: &CameraGraph) -> Status`; define graph fields from the metadata audit in Task 1 before writing classifier fixtures.

- [ ] Write pure classifier tests first: running V4L2 camera linked to a capture consumer is active; hardware presence, suspended sources, playback, portal screen sources, and unlinked nodes are not active; incomplete metadata is unknown.
- [ ] Write transition tests for removal, multiple consumers, one consumer stopping while another remains, startup with existing activity, and disconnect clearing prior positive evidence.
- [ ] Write device-access tests before implementation: no holders is unobserved; lookup failure is unknown; stable nonempty holders are unobserved at 749 ms and active at 750 ms; short probes never activate; holder-set changes restart the debounce; disappearance clears access. Use Hushlight's verified constants: 1 s minimum between reconciliations, 5 s discovery/reconciliation fallback, 2 s holder-command timeout, and 2 s child-reap timeout. Use fake clocks and fake holder-command results, not real camera devices. Verify timeout/missing `fuser`, event overflow, watch loss, and hotplug trigger bounded recovery without claiming inactivity from failed checks.
- [ ] Test merge precedence `active > unknown > unobserved`, preserve limitations from both paths, and prefer confirmed PipeWire capture wording when both paths are active. PipeWire disconnect must not erase independently valid sustained device-access evidence.
- [ ] Run `cargo test --manifest-path helper/Cargo.toml`; confirm failures occur because the required behavior is absent.
- [ ] Implement classification, status serialization, event-driven PipeWire observation, and bounded device-holder reconciliation using Task 1 criteria. Publish coherent initial state, not a partial graph treated as inactivity. Emit "Camera device accessed" and "Direct device access is not confirmed streaming" for the fallback path; do not expose holder identities in stdout.
- [ ] Write and run helper lifecycle tests before their implementation: helper termination releases its connection, parent death leaves no helper or holder-command child, and reconnect cannot reuse stale graph state. Native notification handling avoids an `inotifywait` child; bounded `fuser` is allowed. Test process-group cleanup and parent-death protection, including the parent-exits-during-setup race, with harmless test processes, never real capture. Protocol-consumer tests belong to Task 3.
- [ ] Run Cargo tests, `cargo fmt --manifest-path helper/Cargo.toml --check`, and `cargo clippy --manifest-path helper/Cargo.toml --all-targets -- -D warnings`. Record coverage using an available approved tool; target at least 80% for testable helper logic.

**Acceptance:** Supported camera activity transitions are reproducible with fixtures and no capture is opened by the helper.

## Task 3: Implement Shared Session State

**Files:** Create `ui/EdgeLightState.qml`, singleton owner, and `tests/qml/tst_EdgeLightState.qml`.

**Interfaces:** State exposes `cameraActive: bool`, `enabled: bool`, `targetOutput: string`, `widthPercent: real`, `brightnessPercent: real`, `warmthPercent: real`. Methods: `applyCameraState(state: string)`, `enableForOutput(name: string)`, `disable()`, `outputRemoved(name: string)`, and `resetSession()`.

The owner also exposes `cameraEvidence: string`, `cameraLimitations: list<string>`, and `consumeStatusLine(line: string)` for the popup/protocol boundary. Unknown state strings fail closed. Empty output names cannot enable illumination. Clamp finite slider inputs to the approved ranges; non-finite inputs restore that slider's default. Helper exit resets camera status immediately; restart after 2 s without re-enabling the light. A healthy active device path remains active during a PipeWire-only reconnect.

- [ ] Write Qt Quick behavioral tests: enabling without active camera is refused; active camera permits manual enable; stop and unknown disable and clear target; reactivation stays off; popup closure does not disable; output removal and reset disable; non-target output removal has no effect.
- [ ] Add tests for two widget callers selecting different outputs: one shared target only. Check proposed defaults and range-clamping behavior, including non-finite values.
- [ ] Add protocol tests for valid access-only evidence, missing camera fields, wrong types, invalid states, unsupported version, malformed JSON, and lines above 16 KiB. Require unknown/reset for invalid input, no leaked holder identities, and preserved evidence/limitations for valid input. Test helper exit and delayed restart separately from a single-path reconnect.
- [ ] Run the Qt Quick test command established in Task 1 and observe failures before implementing state methods.
- [ ] Implement the minimal nonvisual state and one observer owner using the verified singleton pattern. Widget destruction must not strand an overlay or leave an unowned helper.
- [ ] Run QML behavior tests and Rust tests; confirm malformed, unsupported-version, and oversized status lines reset the shared camera state to unknown. Test that active device-access evidence reveals controls with honest wording, and disappearance turns the light off unless another approved path remains active.

**Acceptance:** All safety transitions pass without a real camera or live desktop modification.

## Task 4: Add the Bar Control, Popup, and Light Overlay

**Files:** Create `manifest.json`, `Widget.qml`, `ui/EdgeLightPopup.qml`, and `ui/EdgeLightOverlay.qml`.

**Interfaces:** Widgets consume Task 3's shared state. Toggle calls `enableForOutput(widgetOutputName)` or `disable()`; controls update the bounded state values. Overlay exists only when `cameraActive && enabled` on the selected output.

- [ ] Register the plugin with default width zero while camera state is not active; microphone-only events must never reveal it.
- [ ] Implement the anchored theme-native popup with accessible toggle and slider labels, current values, keyboard operation, and Omarchy's one-popup-at-a-time coordination.
- [ ] Implement a continuous white-to-warm border with a transparent center using an input-transparent, non-focusable layer surface. Width is computed from logical output dimensions; no hardware brightness changes.
- [ ] Add behavioral QML geometry tests: on 1920×1080 and 1080×1920 logical outputs, 6% width is 64.8 logical pixels; 2% and 15% stay within the approved range. Render four non-overlapping strips so corners are not brighter than edges; brightness controls strip opacity, and warmth interpolates `#ffffff` to `#ffd6aa`. Include a visual test harness with simulated camera status only; do not expose a production camera-gate bypass. Offscreen geometry tests cannot prove compositor click-through or lock safety.
- [ ] Wire camera stop/failure to popup dismissal; wire lock, output removal, and teardown to `resetSession()` or `outputRemoved()`. Lock must reset enable state, not merely hide a surface that reappears lit on unlock.
- [ ] Run QML behavior tests, `omarchy plugin validate .`, and `qmllint` with the installed imports from Task 1. Use `qmlformat` parse-only if required by the local two-space style; do not rewrite style globally.

**Acceptance:** Static validation and state tests pass. Actual compositor interaction remains unverified until the approved live acceptance step.

## Task 5: Verify the Complete Feature and Document Limits

**Files:** Update `docs/edge-light-verification.md` and `HANDOFF.md`.

- [ ] Run the full Rust/QML tests, format checks, lint, and manifest validation; record exact commands and results, including any unavailable checks rather than claiming success.
- [ ] Request explicit approval before any live install/enable or real capture test. Without it, stop with local validation results and outstanding acceptance cases.
- [ ] On the approved test setup, verify camera off means no icon/space; supported camera start reveals the icon; manual enable creates the border; each slider changes only its intended property; popup close leaves it on; camera stop turns it off and closes the popup.
- [ ] Verify observer death, restart, malformed output, plugin unload/reload, lock/unlock, output unplug, and two-monitor use leave no light or stale icon. Verify a second active camera consumer keeps the camera gate active.
- [ ] Check edge and center click-through, popup interaction, top-bar access, fractional scaling, monitor rotation, and fullscreen applications. Confirm overlays are absent from lock surfaces; record whether ordinary screen sharing includes the border rather than promise capture exclusion.
- [ ] Document supported apps/paths, detection limitations, per-session behavior, and manual installation/build steps. Do not describe device access as proven streaming or claim macOS parity.

**Acceptance:** Evidence supports all confirmed behavior, proposed defaults have owner approval, and remaining coverage gaps are explicit. No publication is part of this plan.

## Execution Handoff

Implement Tasks 1–4 in order with one Go writer, preserving unrelated work and using an isolated feature workspace before product edits. Resolve the remaining host API questions in Task 1; stop for unsupported lifecycle requirements rather than reach into private authentication objects. Write tests first for each logic unit, observe failure, implement, and rerun the relevant suite before advancing. Keep an execution ledger recording evidence and deviations; do not commit automatically.

After local checks, capture the complete change snapshot for one fresh routine reviewer under the owner's routing policy. Address material findings with regression tests. Task 5's live checks require separate owner permission; local completion must explicitly list them as pending until run.

Planning self-review: checked camera fallback semantics, merge precedence, timing constants, protocol boundaries, session reset behavior, multi-output ownership, geometry, and publication limits. Host service/lock APIs and installed QML imports are intentionally a bounded Task 1 research gate, not assumed working interfaces.
