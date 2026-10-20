# Glint

Camera-gated Edge Light for Omarchy's built-in Quickshell bar. A Rust helper
observes supported camera activity; the theme-native camera mark opens observed
app/device details plus a manual light toggle and width, brightness, and warmth
controls. The camera panel is installed for local testing; live acceptance has
confirmed one direct-access Brave/C920 session and camera-stop cleanup.

Hushlight is a separate project and is not replaced or modified.

## Behavior

- The mark occupies no bar space unless camera capture or sustained camera-device
  access is observed. Microphone-only activity does not reveal it.
- Edge Light starts off. Enable it from the mark on the display to illuminate.
  One display is illuminated at a time; closing the popup leaves the light on.
- Camera observation becoming inactive/unknown, helper failure, output removal,
  or plugin reload turns it off. It does not automatically re-enable.
- Width is 2–15% of the shorter display dimension; brightness is rendered opacity,
  not hardware brightness. Warmth mixes neutral and warm white. Controls reset to
  6%, 70%, and 30% on plugin reload; no settings are written to disk.
- Lock handling is best-effort for `omarchy.lock`: poll public lock status every
  second while camera activity is observed, reset on a detected lock, changed
  event timestamp, or lookup failure. Quick lock/unlock cycles may evade a reset.
  Controls stay unavailable until a successful unlocked status lookup.
- The light clears the bar strip and the open Glint popup. A 96-logical-pixel
  pointer cutout with a 32-pixel feather reveals nearby content. Cursor-only
  local IPC runs at most every 50 ms while illumination is enabled; coordinates
  are never logged or persisted. Missing/stale cursor data or unavailable UI
  geometry temporarily hides illumination without changing the manual toggle.

## Detection limits

The source panel distinguishes **Process:** labels from bounded `/proc` metadata
and **Reported app:** labels supplied by PipeWire consumers/clients. Neither is
verified application identity: names are untrusted/self-reported and rendered as
plain text. PipeWire rows follow actual active consumer/source links; direct
access rows require an actual matching character-device FD, with process start
time checked before/after metadata reads, rather than pairing every holder with
every camera. Missing or inaccessible metadata stays **Unknown application** or
**Unknown camera**; a validated device path may still be shown.

Details are best-effort snapshots, not permissions or browser tab/site
attribution. There is no media preview, frame/audio access, permission control,
or guarantee of all-app/device coverage. Optional metadata failure does not
change positive camera evidence or the manual Edge Light controls. Details clear
on stop/unknown/helper or session reset; stale worker results are rejected.
See `docs/edge-light-verification.md` for scan budgets and evidence limits.

PipeWire capture requires a running video consumer, an active link, and a running
V4L2 producer. Direct `/dev/video*` holders are reconciled using native inotify
notifications and bounded `fuser` calls: a 750 ms stability threshold confirmed
by a second lookup, checks at most once per second, and a five-second fallback.
The earliest normal confirmation is therefore approximately one second.
Start/stop visibility therefore
has latency, and missed notifications can delay clearing until reconciliation.

Device access is labeled **Camera device accessed**, not confirmed streaming.
Only visible supported paths are observed; restricted/other-user holders and
some application paths can be invisible. No indicator is not proof of inactivity.
Glint never reads camera frames or audio. Runtime requires no root or network.

## Build and local checks

Requires Rust/Cargo, PipeWire development libraries, native build tooling, and
installed `fuser`, Qt6, Quickshell, and Omarchy. Cargo may fetch dependencies at
build time. No automatic build/install hooks are used.

```sh
cargo build --release --manifest-path helper/Cargo.toml
cargo test --manifest-path helper/Cargo.toml
cargo fmt --manifest-path helper/Cargo.toml --check
cargo clippy --manifest-path helper/Cargo.toml --all-targets -- -D warnings
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl /usr/lib/qt6/bin/qmltestrunner -input tests/qml
bash tests/bar-click-smoke.sh
bash tests/bar-click-smoke.sh pointer-smoke.qml
omarchy plugin validate .
```

The service launches `helper/target/release/glint-helper` relative to the plugin.
That binary must exist in the eventual plugin directory; cloning source alone
does not build it. The current machine's unqualified QML tools are Qt5: use the
Qt6 paths above. For lint imports, see `docs/edge-light-verification.md`.

The plugin declares both a service and bar widget under `glint.privacy`. Its
shared-service access requires Omarchy's built-in bar; replacement bars with
service-less facades are unsupported.

Installation/enabling requires owner approval; machine-specific deployment
status is recorded in `docs/edge-light-verification.md`. Live detection, multi-monitor compositor behavior,
click-through, lock occlusion, and visual quality require separately approved
testing. The local suite uses synthetic graphs, an isolated PipeWire metadata
daemon with no hardware modules, ordinary temporary files for watcher/holder
checks, harmless child processes, and offscreen QML tests; it does not activate a camera.

## Development

- `HANDOFF.md`: direction, boundaries, roadmap.
- `docs/superpowers/plans/2026-10-16-edge-light.md`: implementation plan.
- `docs/edge-light-verification.md`: evidence and outstanding live checks.
- `tests/preview.qml`: optional simulated visual harness, not a production bypass.
