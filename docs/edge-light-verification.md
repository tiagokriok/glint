# Edge Light verification

## Camera panel installed and partially live-accepted (2026-10-19)

The release helper and runtime plugin files from merged `main` were staged,
validated, backed up, and installed at
`~/.config/omarchy/plugins/glint.privacy/`. Backup:
`~/.config/omarchy/glint-camera-panel-backup.80ftaO/`. The session was unlocked;
the normal guarded shell restart succeeded, shell ping returned `ok`, and exactly
one installed `glint-helper` remained. Glint and Hushlight stayed enabled.
`shell.json` and all Hushlight file hashes matched their pre-install snapshots.
No packaged Omarchy or Hushlight file was edited.

The owner started Brave with an HD Pro Webcam C920 and opened the real popup.
The installed helper and visible panel both reported `Process: brave`,
`HD Pro Webcam C920 · /dev/video2`, and `Camera device accessed`; the wording did
not claim streaming. Owner-provided screenshots confirmed the activity section,
toggle, and all three sliders were visible and usable. After camera use stopped,
the owner confirmed the icon and light disappeared; a bounded helper sample then
settled to `unobserved` / `No capture observed on supported paths`.

This is partial live acceptance, not complete hardware/compositor coverage. It
does not establish a PipeWire-capture attribution path, concurrent callers or
devices, hotplug/restart behavior during capture, fractional/rotated/multi-monitor
placement, tiny physical screens, or screen-reader behavior. No frames or audio
were collected. Historical evidence below retains its original date and scope.

### Observational identity and bounds

- **Process:** means bounded `/proc/<known-holder>/comm` metadata, accepted only
  with matching before/after process start time and an actual character-device
  FD matching the observed video device. No holder/device cross-product is used.
  This is not verified app identity or proof of streaming.
- **Reported app:** means the active PipeWire consumer's client-reported name
  (or consumer-reported owner when no client is referenced), never source-owner
  attribution. Portal/helper names may be all that is available. Labels are
  untrusted/self-reported, sanitized and rendered as plain text, not resources.
- PipeWire rows follow running consumer/active-link/running-V4L2-source pairs.
  **Camera capture observed** and **Camera device accessed** remain distinct;
  direct access is not confirmed streaming. Unknown app/camera metadata stays
  unknown, with a validated path shown only when available. No tab/site identity,
  permissions control, frames, audio or media preview is provided.
- Device names prefer PipeWire description/nick, with best-effort worker-side
  sysfs fallback verified against character-device major/minor. Direct access
  uses bounded sysfs/proc metadata only for already-known devices/holders; it
  never opens camera devices. Permissions, races, vanished FDs, missing metadata
  and budgets can leave unknowns. No indicator is proof of inactivity and no
  complete all-app/device coverage is promised.
- Each metadata job has a shared **100 ms cooperative budget**, **4 KiB** per
  metadata file (a read reaching the cap is unavailable), at most **32 device
  paths**, **16 direct holders** and **256 FD entries/checks per holder**.
  Individual filesystem calls have no hard timeout. Each worker has one running
  job, one replaceable pending job and one result slot; capture publication does
  not wait for enrichment. Public details cap at **32 rows**, **80 UTF-8 bytes**
  per label and **64 bytes** per validated video path, within the **16 KiB** line
  limit. Omitted rows can produce “More observations omitted”.
- Stop/unknown, helper/session reset and changed source generations clear or
  invalidate labels; late worker results are rejected. Direct metadata refreshes
  on fresh reconciliation; an equivalent PipeWire graph can retain completed
  fallback labels without background refresh. Optional missing/invalid metadata
  does not invalidate positive detection or reset manual light state. Existing
  camera/lock gates, sliders, target selection and protected cutouts remain.

### Attributed source-check evidence (not rerun by the documentation task)

The 2026-10-17 Task 4 report records **124 Rust tests passed** and **98 Qt6 GPU
results passed**, including init/cleanup results, with zero failures/skips; both
native smoke scripts, Rust fmt/Clippy, QML lint/parse, shell syntax and manifest
validation passed. Host dynamic-type warnings and expected offscreen/socket
warnings remain; these are not warning-free host validation. Exact commands and
intermediate failures are in
`.superpowers/sdd/2026-10-17-camera-panel-implementation/task-4-report.md`.

Task 3's fresh helper line coverage is **87.46% (1,625/1,858)**, including native
adapters and `main.rs` at 0%, with no source exclusions. This is Rust-only prior
evidence, not QML coverage; Task 4 did not remeasure it. See the Task 3 section
below for reproducibility. No new whole-change independent review is claimed
here; that remains the coordinator's handoff responsibility.

Separately approved live acceptance is still required: owner-controlled capture
start/stop, physical device/FD and PipeWire links or honest unknowns, concurrent
users/devices when available, metadata loss, helper restart and hotplug. Physical
camera links, scaling/multi-monitor behavior, compositor interaction and screen
reader behavior have not been established by these source checks. Preserve all
Edge Light safety/access checks; collect no frames or unrelated process data.

## Camera panel Task 3 — local helper verification (2026-10-17)

Source-only PipeWire attribution now follows active camera source/consumer
relationships. App labels are consumer/client-reported, never source-owner
guesses. Device description/nick precede optional bounded worker-side sysfs
enrichment; classification stays pure and capture publishes before enrichment.
No installation, QML or Hushlight changes were made in this task.

All 124 Rust checks passed, including the extended private metadata-only
PipeWire fixture, graph masks/removals, concurrent pairs and stale-result tests.
`cargo fmt --check`, `cargo check --all-targets` and Clippy with `-D warnings`
passed. Fresh LLVM 22.1.8 instrumentation measured **87.46% helper line coverage
(1,625/1,858)** across all executable helper sources, including `observer.rs`
(81.34%) and unexecuted `main.rs` (0%). No source exclusions or tool installs.
Region coverage: 85.56%; no branch-coverage claim.

Exact reproducible invocation and full RED/GREEN/coverage evidence are in
`.superpowers/sdd/2026-10-17-camera-panel-implementation/task-3-report.md`;
manual runner: `python3 .superpowers/sdd/2026-10-17-camera-panel-implementation/task-3-coverage.py`.
Tests did not open cameras, connect to the user's normal daemon or load hardware
modules. Native fixture coverage proves property mapping, not live active-camera
links; sysfs fallback uses injected metadata fixtures. Live acceptance stays
separate and requires approval.

## Access update installed (2026-10-17)

Owner-approved pointer recession and permanent bar/popup cutouts are installed.
Backup: `~/.config/omarchy/glint-access-backup.HldJQj/plugin`. Full guarded shell
restart succeeded; ping returned ok, one helper was running, and Hushlight
Widget/manifest hashes matched. No camera or light was activated by the agent.

Local evidence: 44 Qt6 checks passed with `QT_QUICK_BACKEND=rhi` and
`QSG_RHI_BACKEND=opengl`, including real pixel checks for bar/popup/pointer holes,
feathered transitions, overlapping holes, distant illumination, and one brightness
multiplier. The software backend explicitly skips these GPU pixel checks. Native
local-socket and bar-click smoke tests passed; the former uses only a temporary
fixture socket, with expected server peer-close warnings during teardown. Rust's
unchanged 71 checks passed; no new Rust coverage percentage was measured.

Independent review found an activation/cadence edge case; a failing cadence test
and production elapsed-time gate fixed it. Retained response data is capped at
4 KiB; requests and stale samples expire at 250 ms; idle tracking is stopped.
Complete JSON response framing finishes before EOF to avoid Qt peer-close errors
discarding valid replies. Pointer data is neither logged nor persisted.

QML parsing, manifest validation and lint returned exit 0. Existing host dynamic
type/enum warnings remain; unrelated host read-only moduleName warnings appeared
after restart. These are not claimed warning-free results.

Owner acceptance remains necessary for actual compositor pointer motion,
fractional/rotated/negative-origin output mapping, popup global bounds, and
performance on large displays. If safe geometry or pointer data is unavailable,
rendering is suppressed without changing the manual toggle. Sliders and camera
detection are unchanged. Hushlight and packaged host code were not edited.

## Owner-approved test installation (2026-10-16)

Click-path regression found during owner testing: Omarchy's `modulePointer`
forwards bar clicks to `activeItem.triggerPress(button)` (`Bar.qml:982–1019,
1902–1993`), so Glint's standalone `TapHandler` never received that forwarded
click. The widget now implements the host method; keyboard and accessibility
activation use the same camera/lock-gated function. Camera detection is unchanged.
`bash tests/bar-click-smoke.sh` stages the real widget with an offline bar facade:
RED "bar cannot forward clicks to Glint" before the fix, GREEN afterward for
left-click open/close, right-click ignored, and lock-unknown blocked. This creates
no observer or desktop window. Offscreen focus-grab/portal warnings are expected;
it does not prove physical input or Wayland popup rendering.

The click fix was copied to the installed widget (backup
`glint-backup.TIM9pz/Widget.before-click-fix.qml`). Full shell restart was refused
by Omarchy because the session was locked. Do not bypass that guard; a restart
after unlocking is needed to rule out stale QML before the owner's next test.

- Installed and enabled at `~/.config/omarchy/plugins/glint.privacy/` after a
  fresh release build and staged manifest validation. Runtime QML/UI files and
  the compiled helper were copied; repository metadata and build caches were not.
- Shell config backup: `~/.config/omarchy/glint-backup.TIM9pz/shell.json`.
- Plugin listing confirms Glint and Hushlight both enabled; layout retains both.
- Exactly one `glint-helper` process found after enabling. Recent filtered shell
  logs contained Glint reload messages and no Glint load errors.
- Installed Hushlight Widget.qml and manifest hashes matched the before snapshot.
- No camera was opened or Edge Light enabled by the agent. Camera-driven UI,
  visual/compositor behavior and start/stop acceptance are for the owner's test.
- To disable only Glint: `omarchy plugin disable glint.privacy`. Do not restore
  the whole backup blindly if unrelated shell settings change after installation.

## Local implementation (2026-10-16)

The Rust observer, singleton service, camera-only bar widget, controls, and one
input-transparent overlay were implemented and verified locally before the
owner-approved installation above. Hushlight remains unchanged; enabling Glint
adds only its entry to shell configuration.

Local tests use synthetic graph updates, ordinary temporary files for
inotify/fuser, harmless child processes, an isolated PipeWire daemon without
hardware modules, and offscreen Qt6. They do not connect the observer to the
user's normal PipeWire session or open any camera device. The isolated daemon
and its socket live under `/tmp/opencode` and are torn down after testing.

- Full Rust suite: 71 checks passed, including native adapter and child-lifetime
  fixtures. The two fixture entry tests are harmless no-ops in their parent run.
- Qt6 offscreen: 27 checks passed for session state, protocol framing, lock
  reset/unknown/stale replies, ranges, output selection, and border geometry.
- Release build, Clippy `--all-targets -- -D warnings`, QML parse checks, and
  `omarchy plugin validate .` passed locally.
- Qt6 `qmllint` exits 0 with remaining host dynamic-property, QProcess enum,
  and platform-registered PanelWindow warnings; this is not a warning-free
  runtime validation. No base `QQuickItem.state` property is overridden.
- Installed LLVM22 instrumentation measured 80.62% helper line coverage across
  all seven Rust source modules (including `main.rs` at 0%). Region coverage was
  78.55%; no branch coverage percentage is claimed. Re-run after further edits.

Independent snapshot review prompted regression fixes for candidate device
promotion from a stale single sample, state/property-only PipeWire info updates,
and skipped queued stop/start transitions. Device activation now needs a
confirming lookup after at least 750 ms; the one-second reconciliation bound
means normal initial activation is approximately one second or later. Relevant
notifications restart unconfirmed candidates. This is intentionally stricter
than promoting a stale Hushlight-style sample based on elapsed time alone.

The native fixture also exposed a zero initial PipeWire timer value disabling
the stop timer; the implementation now arms it for 250 ms. No live hardware
evidence is inferred from these tests.

Lock probes use the direct `qs ipc -n -p <OMARCHY_PATH>/shell call -- lock status`
transport from `/usr/share/omarchy/bin/omarchy-shell:59`, not the shell wrapper.
This is the same public read-only IPC call, but permits the QML Process timeout
to terminate the client directly without leaving bash/timeout grandchildren.
Malformed or IPC-error output fails closed through the lock-status parser.

### Outstanding live acceptance (separate approval required)

- Real target-app capture start/stop, concurrent holders, and observer failure.
- Built-in bar service injection, popup keyboard/outside-click behavior, theme
  rendering, and accessible controls in the actual shell.
- Two-monitor target selection/removal, rotation/fractional scaling, fullscreen
  stacking, click-through at the border, and actual session-lock occlusion.
- Plugin unload/reload and no orphan processes in the installed host.
- Whether screen sharing includes the border; no capture exclusion is promised.

## Historical prerequisite research

## Status: Task 1 prerequisites resolved; lock reset accepted as best-effort

All Task 1 prerequisites are resolved. There is no reactive lock signal (no
Hyprland event, no logind `LockedHint`, no public lock generation), so the owner
**accepted a best-effort lock reset** (fix round 2): poll the public non-auth lock
surface every 1 s while camera-active and reset on a detected lock, a changed
lock-event timestamp, or a lookup failure. A guaranteed reset for arbitrarily
quick cycles is explicitly not claimed. Live camera validation remains deferred
to Task 5.

The plan's defaults were explicitly owner-approved on 2026-10-16 (see
`docs/superpowers/plans/2026-10-16-edge-light.md`, "Approved Defaults"), and the
owner then approved Hushlight-compatible device-access gating. This prerequisite phase was
documentation/research only: no install, enable, camera capture, live shell
instance, window, or desktop configuration change was performed. Every read of
`/usr/share/omarchy` and `/home/tiagokriok/dev/personal/hushlight` was read-only.

Offline tool probes and read-only doc fetches ran in a scratch directory
(`/tmp/opencode/edge-light/…`). No product code or repository file was created
outside this document and the plan. Nothing here is live camera evidence; the
live start/stop check stays in Task 5.

### Verified locally — toolchain

- `rustc --version`: 1.98.1. `cargo --version`: 1.98.1.
- `pkg-config --modversion libpipewire-0.3`: 1.6.8 (dev headers present);
  `libspa-0.2`: 0.2. `pkg-config --modversion wayland-client`: 1.26.0.
- `fuser`, `inotifywait`, and `lsof` are on `PATH`.
- No `pipewire-rs` crate is present in the local Cargo cache. The first helper
  build therefore needs registry access at build time (a build-time dependency
  fetch, not runtime network). Confirm/approve before Task 2 resolves crates.

### Verified locally — Qt tooling (corrected)

The `PATH` tools `qmltestrunner`, `qmllint`, `qmlformat`, and `qml` are Qt5
(`pacman -Qo /usr/bin/qmllint` → `qt5-declarative 5.15.19`). Qt5 tools cannot
load Qt6/Quickshell QML and fail silently. The working Qt6 tools are
`/usr/lib/qt6/bin/qmltestrunner`, `/usr/lib/qt6/bin/qmllint` (6.11.2), and
`/usr/lib/qt6/bin/qmlformat`; `/usr/bin/qml6`, `qmlls6`, and `qmlscene6` are
also Qt6. All commands below use the Qt6 binaries explicitly.

- Offline Qt Quick test (`QT_QPA_PLATFORM=offscreen`, no window): a scratch
  `TestCase` ran green with exit 0 using
  `QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner -input <dir>`.
- `qmllint` against installed shell files resolved `Quickshell`,
  `Quickshell.Io`, `qs.Commons`, and `qs.Ui` once an import directory containing
  a `qs -> /usr/share/omarchy/shell` symlink was supplied; exit 0 with only
  non-fatal warnings (e.g. singleton property typing). `Region {}` resolved as
  `Quickshell/Region 0.0`.
- `omarchy plugin validate <dir>` (version 4.0.4): exit 0 for a valid manifest
  whose declared entry-point files exist; it errors on a missing `manifest.json`
  or a declared entry point that is not on disk.

### Verified locally — host imports

- `/usr/share/omarchy/shell/services/PluginShellApi.qml:30` exposes
  capability-scoped `serviceFor(id)`.
- `/usr/share/omarchy/shell/Ui/BarWidget.qml:12-44` documents the per-monitor
  widget base: `bar`, `moduleName`, `settings`, and `broadcast()`. Widgets are
  instantiated once per monitor, so shared ownership must be an explicit
  service, not widget-local state.
- `/usr/share/omarchy/shell/plugins/osd/Osd.qml:126-137` and `plugins/bar/Bar.qml`
  show the input-transparent, non-focusable overlay pattern (empty `Region`
  mask, `WlrLayer.Overlay`, `WlrKeyboardFocus.None`, `ExclusionMode.Ignore`).

## Camera-gating contract (owner-approved)

Hushlight's `docs/capture-research.md` records a user-confirmed Brave live camera
preview with no active PipeWire camera stream; device holders and `inotify`
showed access, not streaming. The owner approved matching that contract:
sustained `/dev/video*` holder evidence may reveal Glint and permit Edge Light,
with honest wording.

- `cameraActive` is defined as the helper's `camera.state === "active"`. It is
  true when either the PipeWire camera-capture path or the sustained
  device-holder path is active (merge precedence `active > unknown > unobserved`).
- Device-access evidence must read "Camera device accessed" and carry the
  limitation "Direct device access is not confirmed streaming"
  (`hushlight/backend/camera.py:22-40`). A raw open alone must not activate.
- Confirmed PipeWire capture wording ("Camera capture observed") wins when both
  paths are active (`hushlight/backend/runtime.py:214-224`,
  `hushlight/backend/status.py:60-82`).
- Hushlight timing constants to reproduce in Rust: 750 ms debounce, 1 s minimum
  between reconciliations, 5 s discovery/reconciliation fallback, 2 s
  holder-command timeout, 2 s child-reap timeout (`runtime.py:30-35`);
  `/dev/video*` discovery via glob, `inotifywait … -e open -e close` triggering
  bounded `fuser` checks where returncode 0 or 1 is a valid answer
  (`runtime.py:138-168`); holder-set change resets the debounce clock
  (`runtime.py:201-212`); lookup failure is `unknown` (`camera.py:31-40`).
- The Brave PipeWire gap remains a limitation, not a blocker. No frames,
  audio, browser data, or command lines are read.

## Host lifecycle APIs (resolved)

### Singleton ownership (bar widgets → one shared owner)

The supported mechanism is an Omarchy `service`-kind plugin: a headless
singleton instantiated once by the host, not one per monitor.

- Manifest: `kinds: ["bar-widget", "service"]` and
  `entryPoints: { "barWidget": "Widget.qml", "service": "Service.qml" }`
  (`README.md:74-84`).
- The host instantiates the service once in `ensureService` and publishes it in
  `_services` (`shell.qml:901-951`); ordinary third-party plugins may look up and
  control only their own service (`README.md:96`, `shell.qml:608-610`).
- A third-party bar widget reaches its owner through its bar facade:
  `bar.shell.serviceFor("glint.privacy")`. The facade's `shell` is
  `PluginShellApi` (`Ui/PluginBarApi.qml:12`) and is injected in
  `Bar.qml:229-279` and `Bar.qml:1999-2006`; the installed media widget uses the
  same shape (`plugins/services/media/BarWidget.qml:10`).
- Do not use a relative-import QML singleton: the host documents that
  relative-path imports do not share singleton state (`shell.qml:15-18`).
- Lifecycle: the host keeps an existing non-auth service instance across a
  rescan (`shell.qml:953-1006`) but destroys it on `reloadPlugins`
  (`shell.qml:1452-1462` → `unloadPluginServices`, `shell.qml:1033-1050`).
  Plain (non-`keepLoaded`) services are destroyed on hot-reload, which resets
  enable state — matching the approved "plugin unload/reload turns it off".
  `keepLoaded` (which would preserve the light across a reload) is therefore not
  used unless the owner directs otherwise.
- Sole overlay owner: the singleton `Service.qml` is the only creator of the one
  overlay window; widgets never create a window. A `service` entry point may own
  window types: the host instantiates it with
  `Qt.createComponent(url, Component.PreferSynchronous).createObject(...)`
  (`shell.qml:914-923`), and the installed `omarchy.lock` service is an `Item`
  root that already contains `WlSessionLock` and a `PanelWindow`
  (`plugins/lock/Service.qml:8-9,230-300`; manifest `kinds:["service"]`). Because
  exactly one service instance exists, exactly one overlay exists across all
  monitors; the widget only calls service methods with the target output.
- Limitation: if the user runs a replacement `kind: "bar"` plugin instead of the
  trusted built-in bar, widgets receive a service-less entry facade
  (`Bar.qml:238-242`, `README.md:98`) and cannot reach the service. Under that
  bar the feature would be unavailable; this must be stated, not silently broken.

### Session lock — accepted best-effort reset via supported polling

The owner accepted a **best-effort** lock reset instead of a guaranteed reset for
arbitrarily quick lock/unlock cycles (fix round 2). There is still no reactive
lock signal — no Hyprland event, no logind `LockedHint`, no public lock generation
— so the accepted mechanism is a bounded poll of the one public, non-auth lock
surface.

Protocol (owner-accepted):

- Source: `omarchy-shell lock status`, JSON `{locked, lastEventAt, …}`
  (`plugins/lock/Service.qml:510-537`). It is the IPC surface used by
  `omarchy-system-lock`, does not authenticate, and is never object-injected.
- When: only while `cameraActive` is true — the only time the icon, popup, or
  light can exist — so polling covers popup and enable gating and is idle when
  the camera is off.
- Cadence: one poll every 1 s; at most one probe outstanding at a time.
- Timeout: 2 s per probe; a timeout, nonzero exit, or unparsable output is a
  lookup failure.
- Reset on any of: `locked == true`; `lastEventAt` differs from the stored
  baseline (a transition happened even if currently unlocked); or a lookup
  failure. Reset means `resetSession()` / disable.
- Baseline: the first successful probe (which must be unlocked) stores
  `lastEventAt`; each successful probe refreshes the baseline after evaluation so
  one transition does not re-trigger on the next poll.
- Enable gating: enabling is denied, and the popup does not open, until a probe
  has succeeded with `locked == false`. Lock unknown never permits enable.
- No automatic re-enable: every reset (locked, timestamp change, or failure)
  requires a manual enable.

Accepted limitations (honest, not guarantees):

- Fast cycles / timestamp overwrite: `lastEventAt` records only the latest lock
  event and probes are discrete, so an arbitrarily quick cycle whose timestamp
  advance is not observed between probes may not reset the light.
- Oversensitivity: `lastEventAt` also advances on non-lock events logged by the
  lock service, so a change can reset the light without a real lock.
- No universal locker support: only the Omarchy session lock (`omarchy.lock`,
  ext-session-lock) is covered. `hyprlock`, `swaylock`, logind blanking, and a
  replacement locker are out of scope, and the source requires `omarchy.lock` to
  be enabled.

### cameraActive is independent of lock state

Camera observation and lock state are separate inputs to the owner:

- `cameraActive` reflects only the helper's camera state and is never changed by a
  lock. A lock reset disables the light and denies/closes the popup; it does not
  fabricate a camera stop, and the camera indicator may remain active.
- While lock is locked or unknown, the popup and the light are denied (controls
  inaccessible) even though `cameraActive` may still be true. The light is reset
  and stays off until a manual enable after a successful unlocked probe.

### Popup coordination

- `bar.requestPopout(owner)` / `bar.releasePopout(owner)` implement
  one-popup-at-a-time; `bar.activePopout` reports the current owner
  (`README.md:166`; `Ui/PopupCard.qml:72-86`). `PopupCard` combines this with
  `HyprlandFocusGrab` for outside-click dismissal (`PopupCard.qml:82-86`).

### Overlay surface, input transparency, stacking, output lifecycle

- Overlay surface: `PanelWindow` with `screen` bound to the target
  `QuickshellScreenInfo` (`ProxyWindowBase.screen`, `_Window/quickshell-window.qmltypes:348`).
- Input transparency: `mask: Region {}` empties the layer-shell input region
  (`ProxyWindowBase.mask`, `…:367`; `Quickshell/Region 0.0`,
  `quickshell-core.qmltypes:646`; `Osd.qml:137`).
- Non-focusable, no reserved space, atop normal windows:
  `WlrLayershell.layer: WlrLayer.Overlay`, `WlrLayershell.keyboardFocus:
  WlrKeyboardFocus.None`, `exclusionMode: ExclusionMode.Ignore`
  (`_WlrLayerShell/quickshell-wayland-layershell.qmltypes:42-123`; `Osd.qml:126-135`).
- Output lifecycle: `Quickshell.screens` is the live list
  (`quickshell-core.qmltypes:943-947`, signal `screensChanged`); each entry is a
  `QuickshellScreenInfo` with `name` (`:1165`). The bar iterates `Quickshell.screens`
  (`Bar.qml:1197`) and `ScreenMoveRemap` watches `window.screen` position
  (`Ui/ScreenMoveRemap.qml:15,39`). Bind the overlay to the chosen screen and call
  `outputRemoved(name)` when that name leaves `Quickshell.screens`; the overlay
  window tears down with its screen.
- Lock occlusion: the Omarchy lock uses ext-session-lock (`WlSessionLock`), which
  by protocol hides ordinary layer surfaces while locked. This is protocol-level
  support only; compositor behavior is live-verified in Task 5.

### Accepted lock-reset decision (owner, fix round 2)

The owner chose **best-effort polling** (former option A, with the `lastEventAt`
transition check folded in) over a guaranteed reactive signal:

- Poll `omarchy-shell lock status` every 1 s while camera-active; reset on a
  detected lock, a changed lock-event timestamp, or a lookup failure.
- Startup waits for a successful unlocked probe before enabling; lock unknown
  never permits enable; no automatic re-enable.
- The true-guarantee option (a host-provided lock signal) is not pursued now.

Proposed behavioral tests for this decision are listed in
`.superpowers/sdd/2026-10-16-edge-light/task-1-report.md` (fix round 2).

### Provenance and documentation notes

- Approved defaults: the plan's "Approved Defaults" records owner approval on
  2026-10-16, and the best-effort lock reset is owner-accepted (fix round 2).
  `HANDOFF.md` was updated to match; the earlier "Other defaults remain
  proposals" line is superseded.
- PipeWire observation basis (official docs consulted): docs.pipewire.org "API
  Tutorial — Part 2: Enumerating Objects" documents the registry + listener model
  (`pw_registry_add_listener` / `registry_event_global`) that the Rust `pipewire`
  crate (docs.rs, 0.10.1) mirrors. Hushlight's `pw-dump -N -m` JSON monitoring is
  the CLI equivalent; Glint keeps the event-driven client model.
- Native device notifications: the Linux inotify API
  (`inotify_init`/`inotify_add_watch`, man7 `inotify(7)`) is the intended
  alternative to spawning `inotifywait`, but it needs a Rust inotify crate chosen
  at build time (not yet selected or verified locally). Until then, native
  handling is a candidate, not a pinned decision; `fuser` remains the bounded
  holder lookup.
- Helper restart interval: the 2 s restart (Task 3) is inherited from Hushlight's
  `Widget.qml:114-119` (`restartTimer interval: 2000`), not a user decision.

## Exact commands

Run from the repository root. Qt6 binaries are required.

```bash
# Build/test the Rust helper (Task 2)
cargo test --manifest-path helper/Cargo.toml
cargo fmt --manifest-path helper/Cargo.toml --check
cargo clippy --manifest-path helper/Cargo.toml --all-targets -- -D warnings

# Offline Qt Quick behavioral tests (Task 3, Task 4) — no window, offscreen
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl /usr/lib/qt6/bin/qmltestrunner -input tests/qml

# Lint QML that imports the installed Quickshell / qs.* modules
# (create the importer once, outside the repo, with: ln -s /usr/share/omarchy/shell <importer>/qs)
/usr/lib/qt6/bin/qmllint -I <importer> -I /usr/lib/qt6/qml Widget.qml Service.qml ui/*.qml

# Parse-only format check (Qt6 qmlformat has no --normalize; discard stdout)
/usr/lib/qt6/bin/qmlformat <file> > /dev/null

# Manifest validation
omarchy plugin validate .
```

Notes: the offscreen run creates no mapped window and no shell instance.
`-import`/`-I <importer>` is only needed when a file imports `qs.*`; pure
QtQuick + QtTest files need no importer. The host adapter used for tests is
`<importer>/qs -> /usr/share/omarchy/shell`.

## Supported-path list (for later tasks)

| Capability | Supported API | Evidence |
| --- | --- | --- |
| Camera gate | helper `camera.state == "active"` | owner-approved Hushlight contract |
| Singleton owner | `service` kind + `bar.shell.serviceFor(id)` | `shell.qml:886-951`, `Bar.qml:229-279` |
| Lock/reset (best-effort, accepted) | `omarchy-shell lock status` every 1 s while camera-active; reset on `locked` / changed `lastEventAt` / lookup failure; unknown denies enable | `lock/Service.qml:510-537` |
| Popup coordination | `bar.requestPopout` / `bar.releasePopout` / `bar.activePopout` | `PopupCard.qml:72-86` |
| Input-transparent overlay | `mask: Region {}`, `WlrLayer.Overlay`, `keyboardFocus: None`, `exclusionMode: Ignore` | `Osd.qml:126-137` |
| Output targeting/removal | `PanelWindow.screen`, `Quickshell.screens` / `screensChanged` | `quickshell-core.qmltypes:943-947` |

## Explicitly unsupported / not used

- Injecting `omarchy.lock` or any authentication service (outside the public map).
- `WlSessionLock` as a global lock observer.
- logind `LockedHint` and `hyprlock`/`swaylock` as lock signals (not emitted here).
- Raw `/dev/video*` opens, power-state, or busy-probing as capture signals.
- Relative-import QML singletons for shared state.
- Qt5 `qmltestrunner` / `qmllint` from `PATH`.

## Next verification (deferred; requires separate approval)

Live camera start/stop metadata on the owner-chosen app, lock/unlock with the
light enabled, output unplug, replacement-bar hosting, fullscreen stacking, and
compositor click-through. The owner controls capture; the observer must not open
camera devices or collect audio, frames, browser data, command lines, or window
titles. No success claim applies to the feature yet.
