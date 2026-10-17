#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
scratch=$(mktemp -d /tmp/opencode/glint-click-smoke.XXXXXX)
trap 'rm -rf -- "$scratch"' EXIT
cp "$root/tests/${1:-bar-click-smoke.qml}" "$scratch/shell.qml"
cp "$root/Widget.qml" "$scratch/Widget.qml"
cp -a "$root/ui" "$scratch/ui"
ln -s /usr/share/omarchy/shell/Commons "$scratch/Commons"
ln -s /usr/share/omarchy/shell/Ui "$scratch/Ui"
env -u WAYLAND_DISPLAY QT_QPA_PLATFORM=offscreen GLINT_TEST_SOCKET="$scratch/pointer.sock" timeout 10s qs -p "$scratch/shell.qml" --no-color
