# Glint

## Current local implementation

Approved access improvement (2026-10-17): permanently subtract the bar strip and
open popup bounds, plus a feathered cursor cutout. Sliders and camera detection
stay unchanged. Cursor-only local socket tracking runs only while lit; failures
hide rendering temporarily, preserving manual enabled state. Plan:
`docs/superpowers/plans/2026-10-17-edge-light-access.md`.

Edge Light source and local tests are present on `feat/edge-light`.
Owner authorized installation for testing on 2026-10-16: `glint.privacy` is
installed and enabled alongside unchanged Hushlight. Shell config backup:
`~/.config/omarchy/glint-backup.TIM9pz/shell.json`. See `README.md` for build/check
commands and `docs/edge-light-verification.md` for evidence and pending live
acceptance. The service owns one shared observer/state/overlay; per-monitor
widgets show only for observed camera use. Sustained-device fallback requires
a confirming lookup rather than promotion from a stale single sample.

User-approved icon change: Glint now uses a small filled video-camera mark,
drawn locally with theme-colored QML geometry (no font or runtime network
dependency). Hushlight keeps its original dots; the glyph table below is its
reference contract, not Glint's current camera icon.

Native Omarchy camera and microphone experience, inspired by macOS and built one feature at a time. Sibling of Hushlight, not a fork or replacement.

## Why a second project

Hushlight (`hushlight.privacy`) already ships: QML widget + Python stdlib backend (`pw-dump`, camera `fuser`). It is installed, tested, and stays untouched.

Glint's original scope was privacy dots without Python: a small Rust helper plus QML bar widget. The direction now includes a camera panel and media features, delivered incrementally. Detection remains event-driven PipeWire (`pipewire-rs`), not `pw-dump -m` polling, with Hushlight's conservative observation semantics.

Do not edit `/home/tiagokriok/dev/personal/hushlight` or `~/.config/omarchy/plugins/hushlight.privacy/` for this work.

## Privacy indicator UX (same as Hushlight)

- Theme-native marks. No hardcoded macOS orange/green/purple.
- Idle privacy marks vanish. How manual controls are accessed while idle remains to be designed.
- Microphone and camera only. Omarchy already shows recording; do not add a recording icon unless the user asks.
- Tooltip names what is on. Panels and effects are now part of the incremental roadmap below; their detailed UX is not yet approved.
- Absent indicator never means "nothing is recording". Report only observed supported paths.

Hushlight glyph language (keep unless the user asks to change it):

| Mark | Meaning |
| --- | --- |
| Hollow circle | Microphone |
| Filled circle with a hole | Camera |
| Filled circle | Microphone and camera |

## Original detection shape (extend as features are designed)

```text
glint/
  manifest.json          # id: glint.privacy, kind: bar-widget
  Widget.qml             # theme from Color.bar.*, launch helper, hide when idle
  helper/                # Rust binary, stdout JSON lines
  tests/
```

Install target (only with explicit approval): `~/.config/omarchy/plugins/glint.privacy/`

JSON status lines should stay close to Hushlight’s so QML can stay thin. Read Hushlight’s `Widget.qml` and `backend/status.py` as the contract, then copy the contract, not the Python.

## Detection

- Microphone: PipeWire capture streams that are actually linked/running. Ignore playback.
- Camera: PipeWire `device.api: v4l2` in use. Direct `/dev/video*` open is access, not proven streaming (same semantics as Hushlight).
- Approved Edge Light gate: merge PipeWire camera capture with Hushlight-compatible sustained `/dev/video*` holder detection (750 ms debounce, bounded reconciliation). Either positive signal can reveal Glint and permit Edge Light. Device-access evidence must say "Camera device accessed", never confirmed streaming; a raw open notification alone is insufficient.
- Do not claim browser-tab or non-portal capture.
- No root. No network. Helper dies with the widget (process group + `PR_SET_PDEATHSIG` or equivalent).

Read `hushlight/docs/capture-research.md` and `hushlight/AGENTS.md` landmines before coding. Re-verify on this machine; do not treat that research as complete.

## Incremental roadmap

Goal: offer a macOS-like camera and microphone experience on Omarchy, one feature at a time, rather than promise immediate feature parity.

Reference: https://support.apple.com/pt-br/guide/mac-help/mchlp2980/mac

Suggested implementation order, approved for documentation:

1. **Edge Light.** Manual toggle and controls for border width, brightness, and warmth. A QML display overlay illuminates the user's face without reading camera frames, processing video, or creating a virtual camera. Automatic low-light activation is deferred. First feature to design and implement; monitor targeting, input behavior, and idle access still need decisions.
2. **Camera panel.** Show observed app and device information where available. Reuse Omarchy's popup and theme conventions. Do not imply reliable attribution on unsupported capture paths.
3. **Live preview.** Investigate camera sharing, permissions, and whether preview can coexist with the calling app before choosing an implementation. Observing PipeWire metadata does not grant access to another application's frames.
4. **Background blur and replacement.** Requires a processed-video pipeline and a way for calling apps to consume its output. App integration versus a virtual camera remains undecided and needs explicit approval.
5. **Advanced processing.** Studio Light, automatic framing, and reactions. Research feasibility and dependencies separately; no implementation approach is committed yet.

This is a sequencing roadmap, not an approved detailed implementation design. Microphone modes remain part of the broader macOS reference but have no position in this sequence yet. Hushlight stays untouched throughout.

## Non-goals and deferred decisions

- Replacing or disabling Hushlight
- Python
- Go or Zig (no mature PipeWire stack worth the rewrite)
- OpenEffects / OBS / v4l2loopback are not approved dependencies. Revisit media-pipeline requirements before implementing video effects; do not install or integrate them implicitly.
- Canary-style history, notifications, Chromium companion

## Next

1. Document the roadmap first; no implementation in this documentation step. Next, settle the Edge Light design before coding.
2. When implementing detection: TDD on classify/merge/lifecycle, then QML, then `omarchy plugin validate`. Validate each additional feature with checks appropriate to its scope.
3. Live install/enable only with explicit approval.

Edge Light planning: `docs/superpowers/plans/2026-10-16-edge-light.md`. Confirmed: Glint's bar icon appears only while camera use is observed; clicking it opens Edge Light controls; camera-use stop turns the light off. This supersedes the original microphone-or-camera icon visibility for this first feature. Defaults are owner-approved (2026-10-16). The session lock has no reactive signal on Omarchy, so the owner accepted a best-effort lock reset: while camera-active, poll `omarchy-shell lock status` every 1 s and reset on a detected lock, a changed `lastEventAt`, or a lookup failure; lock unknown denies enable and never re-enables automatically. `cameraActive` stays independent of lock (a lock reset does not fabricate a camera stop). Task 1 prerequisites are resolved; see `docs/edge-light-verification.md`.

Hushlight verification (do not break it; do not run as Glint’s test suite):

```bash
cd /home/tiagokriok/dev/personal/hushlight
python3 -m unittest discover -s tests -v
```
