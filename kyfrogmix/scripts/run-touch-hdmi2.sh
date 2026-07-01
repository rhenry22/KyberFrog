#!/usr/bin/env bash
set -euo pipefail

# Runnable from any shell (local terminal, SSH, ...): looks up the running
# session's own X11/Xwayland credentials instead of requiring the caller to
# have exported them — the Xwayland auth file name changes every login, so
# it can't be hardcoded either.
uid="$(id -u)"
runtime_dir="/run/user/$uid"
export XDG_RUNTIME_DIR="$runtime_dir"

xauth_file="$(find "$runtime_dir" -maxdepth 1 -name '.mutter-Xwaylandauth.*' 2>/dev/null | head -1)"
if [ -z "$xauth_file" ]; then
    echo "Pas de session Xwayland trouvée sous $runtime_dir — la machine est-elle en session graphique GNOME ?" >&2
    exit 1
fi
export XAUTHORITY="$xauth_file"
export DISPLAY="${DISPLAY:-:0}"

# xinput map-to-output only affects X11/Xwayland clients — a native-Wayland
# kyfrogmix window is invisible to it, so force Xwayland by hiding the
# Wayland display from winit's auto-detection.
unset WAYLAND_DISPLAY

cd "$(dirname "$0")/.."
cargo run -p kyfrogmix &
app_pid=$!

# The Xwayland touch device is (re)created per client and renumbered on
# every launch, so its xinput id can't be hardcoded — poll for it after the
# window maps.
touch_id=""
for _ in $(seq 1 50); do
    touch_id=$(xinput list 2>/dev/null | grep -i 'touch' | grep -oP 'id=\K[0-9]+' | head -1 || true)
    [ -n "$touch_id" ] && break
    sleep 0.2
done

if [ -z "$touch_id" ]; then
    echo "Aucun périphérique tactile trouvé via xinput — vérifie que l'écran tactile HDMI-2 est branché." >&2
else
    xinput map-to-output "$touch_id" HDMI-2
    echo "Tactile (xinput id=$touch_id) restreint à HDMI-2."
fi

wait "$app_pid"
