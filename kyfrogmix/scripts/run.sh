#!/usr/bin/env bash
set -euo pipefail

# Runnable from any shell (local terminal, SSH, ...) regardless of what that
# shell inherited: looks up the running graphical session's own variables
# instead of requiring the caller to have exported them.
uid="$(id -u)"
runtime_dir="/run/user/$uid"
export XDG_RUNTIME_DIR="$runtime_dir"

wayland_socket="$(find "$runtime_dir" -maxdepth 1 -name 'wayland-[0-9]*' ! -name '*.lock' 2>/dev/null | head -1)"
if [ -z "$wayland_socket" ]; then
    echo "Aucune session Wayland trouvée sous $runtime_dir — la machine est-elle en session graphique ?" >&2
    exit 1
fi
export WAYLAND_DISPLAY="$(basename "$wayland_socket")"

cd "$(dirname "$0")/.."
exec cargo run -p kyfrogmix
