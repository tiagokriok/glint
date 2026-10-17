# Edge Light Access and Pointer Recession Implementation Plan

Status: locally implemented/tested, reviewed and applied to the owner-authorized
test installation on 2026-10-17. Full shell restart completed. Live owner
acceptance (especially cross-output popup coordinates and motion performance)
remains pending; see `docs/edge-light-verification.md`.

> **For agentic workers:** Use executing-plans for the owner-approved inline OpenAI fallback. One writer, no commits or publication. Steps use checkbox syntax.

**Goal:** Keep the bar and Glint controls accessible and make illumination recede around the pointer, without changing the three existing sliders.

**Architecture:** Keep the one click-through overlay, but subtract protected UI regions and a feathered pointer region from its rendered light. Query only cursor coordinates through Hyprland's local command socket using Quickshell.Io.Socket, while illumination is enabled. Camera observation and the Rust helper remain unchanged.

**Tech Stack:** Installed Qt Quick, QtQuick.Shapes, QtQuick.Effects.MultiEffect, Quickshell.Io.Socket. No new packages, shader compiler, subprocess-per-frame, input grab, or desktop configuration changes.

**Spec:** Owner approved bar/popup protection and macOS-style pointer recession on 2026-10-17, leaving current sliders unchanged. Apple reference: https://support.apple.com/en-us/125934.

## Global Constraints

- Hushlight and packaged Omarchy sources remain untouched.
- Preserve camera/lock gating, one output/overlay, click-through, and manual enable semantics.
- Preserve width, brightness, warmth, and their current ranges/defaults.
- Pointer coordinates are transient only: no logs, history, window titles, frames, or network.
- No commit, push, merge, or new system dependency. Updating the existing test installation remains within the ongoing owner-authorized test workflow; never bypass the locked-session restart guard.
- Work in the already authorized unborn feat/edge-light checkout; preserve existing changes.

## Review Focus

- Bar access must not depend on receiving pointer samples; protect its strip unconditionally.
- A control popup must remain unobscured when the pointer leaves it.
- Negative output coordinates, fractional scale, and rotation must not offset the clear area.
- Socket failure or stale pointer data must never strand an opaque border over controls.
- Tracking must stop when disabled, camera state clears, session becomes locked/unknown, or service unloads.

## Design Defaults (implementation choices)

- Clear a 96 logical-pixel-radius area around the cursor, with a further 32-pixel feather. Leave the rest of the ring lit. No animation delay before controls become clear.
- Sample at most every 50 ms, one local IPC request outstanding, 250 ms deadline, at most 4 KiB response. Invalid, missing, or stale pointer information temporarily suppresses illumination, not the camera icon; recovery never changes the user's manual enabled preference.
- Request only `j/cursorpos` from `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`. Connect per request and consume its bounded response through EOF. Never use the event socket as an assumed mouse-motion feed; the documented event list has no motion event.
- Fully clear the bar strip on its actual edge and output. Clear the actual popup rectangle plus 8 logical pixels of padding, while open. Do not approximate with only the small pointer hole.
- If popup coordinates cannot be established safely, temporarily suppress the ring while controls are open rather than cover them. Do not silently disable the user's light toggle.
- Use the installed GPU mask effect and a Qt Quick Shapes radial-gradient mask; do not add a hand-built shader compiler dependency or a full-screen CPU canvas repaint loop.

## Task 1: Protected Geometry and Regression Tests

**Files:** create `ui/LightGeometry.js`, `tests/qml/tst_LightGeometry.qml`; modify `ui/EdgeLightState.qml`, `Widget.qml`, `ui/EdgeLightPopup.qml`, `Service.qml`.

**Interfaces:** geometry functions `barRect(width, height, edge, size)`, `localPoint(globalX, globalY, outputX, outputY)`, `paddedRect(rect, padding)`. Rectangles and points use logical output-local coordinates. State exposes protected bar metadata and popup bounds separately from enabled/camera state.

- [ ] Write failing behavior tests for all four bar edges, zero/oversized dimensions, negative output origins, portrait outputs, and padded popup bounds. No source-text tests.
- [ ] Implement finite-number validation and clamped geometry; invalid geometry is unavailable, not a guessed zero rectangle.
- [ ] Wire widget output/edge/barSize updates and popup open/close/resize to the shared owner. Use actual popup content geometry mapped to global coordinates, then subtract output origin exactly once. Validate the host mapping API before relying on it; implement the documented conservative popup fallback if unavailable.
- [ ] Extend the existing offline bar-click smoke test to verify protected-region lifecycle, second-monitor ownership, close, and widget destruction.
- [ ] Run Qt6 offscreen tests and `bash tests/bar-click-smoke.sh`; require all green before rendering changes.

## Task 2: Bounded Pointer Sampler

**Files:** create `ui/PointerTracker.qml`, `ui/PointerProtocol.js`, `tests/qml/tst_PointerProtocol.qml`; modify `Service.qml`.

**Interfaces:** tracker exposes `active: bool`, `valid: bool`, `globalX/globalY: real`; owns its socket and timing. Parser accepts a bounded UTF-8 JSON response with finite numeric x/y and no other retained data. The owner converts coordinates using Task 1's output-local transform.

- [ ] Write failing parser/state tests for fragmented and oversized responses, malformed JSON, wrong types, negative coordinates, EOF, timeout, stale data, and disabling while a request is outstanding.
- [ ] Verify local socket request framing against official Hyprland IPC documentation and the installed implementation, without collecting unrelated metadata.
- [ ] Implement 50 ms maximum cadence, one outstanding request, 250 ms deadline and 4 KiB cap with empty-delimiter chunk parsing. Close/discard on failure or deactivate; do not retry in a busy loop.
- [ ] Test transport against a temporary local fixture socket. Assert that disabled tracking creates no requests and that a late reply cannot reactivate an old session. Do not rely on a real compositor for automated tests.
- [ ] Wire active only to the enabled, unlocked, camera-active light session. No cursor polling while idle.

## Task 3: Subtractive Light Mask

**Files:** create `ui/LightMask.qml`; modify `ui/EdgeLightBorder.qml`, `ui/EdgeLightOverlay.qml`; extend `tests/qml/tst_EdgeLightBorder.qml` and the simulated preview.

**Interfaces:** border receives protected rectangles, output-local pointer position/validity, and recession radius/feather. The mask is the union of protected regions and the cursor hole; overlaps must never reintroduce light. Keep brightness applied exactly once.

- [ ] Read current official MultiEffect/mask and Shapes radial-gradient docs before implementation. Confirm installed module behavior in an offscreen harness; installed QtQuick.Effects is present, qsb is not.
- [ ] Write failing render/geometry tests for transparent bar/popup regions, overlapping holes, feather boundaries, untouched distant edges, pointer outside the selected output, color/brightness preservation, and unchanged corner intensity.
- [ ] Apply an inverted alpha mask using existing Qt GPU effects; keep `mask: Region {}` and non-focusable/non-exclusive window properties unchanged. Visual cutouts must not become input regions.
- [ ] Suppress rendering when safe geometry or fresh pointer data is unavailable; leave the toggle state intact. Reset tracking on session/output teardown.
- [ ] Run render tests with a supported offscreen GPU backend. If unavailable, explicitly record that limitation instead of treating geometry-only assertions as pixel evidence.

## Task 4: Verify and Apply to the Existing Test Installation

**Files:** update `README.md`, `HANDOFF.md`, `docs/edge-light-verification.md`.

- [ ] Run full Rust and Qt6 suites, click smoke, QML lint/parse and manifest validation. Rust helper is unchanged; retain its prior coverage evidence and make no new coverage claim without measuring.
- [ ] Perform one independent snapshot review if an authorized reviewer is available; no silent provider switch on quota failure. Fix material regressions with RED→GREEN tests.
- [ ] Back up only the installed Glint files being changed, then update the validated QML/UI files. Leave Hushlight and unrelated shell settings untouched.
- [ ] Restart only through the normal guarded Omarchy command; if locked, stop and report the pending restart.
- [ ] Owner live acceptance: enable the light, move across all edges, open controls and move away, adjust each slider, disable again; repeat on a second output and fractional scaling. Verify click-through, no capture interruption, and no UI stalls during pointer motion. Do not start camera capture or enable the light on the owner's behalf.

## Planning Checks

Scope excludes automatic low-light activation, slider redesign, camera preview, camera-detection changes, privileged input monitoring, and host patches. Main implementation risks are popup global-coordinate mapping and native mask rendering; each has an explicit verification step and safe fallback. No implementation or installed-file changes were made while writing this plan.
