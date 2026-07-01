#!/bin/bash
# Build a portable KyberFrog-<ver>-x86_64.AppImage for Ubuntu 24.04 (and any
# recent x86_64 Linux with glibc/FUSE).
#
# AppImage was chosen over .deb so iterating versions during dev needs no
# install/uninstall step: drop the new file next to the old one (or overwrite
# it), `chmod +x`, run. No root, no package database to update.
#
# Run directly on an Ubuntu 24.04 host (or any Linux with the Rust toolchain
# installed — see packaging/linux/README.md for the exact apt package list).
# Unlike the Windows build, this does NOT cross-compile: it builds the native
# x86_64-unknown-linux-gnu target on the machine running this script.
#
#   bash packaging/linux/build-appimage.sh
#
# Options:
#   -v <ver>   Version string (default: git describe, else Cargo.toml version).
#   -o <path>  Output dir for the .AppImage (default: <KyberFrog>/dist).
#   -s         Skip the cargo build; reuse an already-built target/release/kyberfrog.
#   -h         Help.
#
# KNOWN LIMITATION: this packages the KyberFrog supervisor/web-UI binary only.
# It does NOT bundle kycontroller/kyavserver/kyclient (the Kyber fork
# binaries) — whether the fork ships Linux builds of those is still
# unverified (see VIDEO_MATRIX_MIXER.md / project notes). Until confirmed,
# emission/reception on Ubuntu will fail to spawn real children; the web UI,
# supervisor, and config editing can still be exercised standalone.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KYBERFROG_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"   # repo root
TARGET="x86_64-unknown-linux-gnu"
BIN_REL="target/$TARGET/release/kyberfrog"

VERSION=""
OUTPUT_DIR="$KYBERFROG_DIR/dist"
SKIP_CARGO=false

usage() { sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

while getopts "v:o:sh" opt; do
    case $opt in
        v) VERSION="$OPTARG" ;;
        o) OUTPUT_DIR="$OPTARG" ;;
        s) SKIP_CARGO=true ;;
        h) usage 0 ;;
        *) usage 1 ;;
    esac
done

cargo_version() {
    grep -E '^version = ' "$KYBERFROG_DIR/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
}
if [ -z "$VERSION" ]; then
    if ! VERSION="$(git -C "$KYBERFROG_DIR" describe --tags --exact-match 2>/dev/null)"; then
        SHA="$(git -C "$KYBERFROG_DIR" rev-parse --short HEAD 2>/dev/null || echo nogit)"
        VERSION="$(cargo_version)-$SHA"
    fi
fi

echo "==> KyberFrog AppImage build"
echo "    version: $VERSION"
echo "    output:  $OUTPUT_DIR"

# --- 1) build kyberfrog (native release) ------------------------------------
if [ "$SKIP_CARGO" = false ]; then
    echo "==> Building kyberfrog (cargo build --release --target $TARGET)..."
    export KYBERFROG_VERSION="$VERSION"
    ( cd "$KYBERFROG_DIR" && cargo build --release --target "$TARGET" )
fi
if [ ! -f "$KYBERFROG_DIR/$BIN_REL" ]; then
    echo "ERROR: $KYBERFROG_DIR/$BIN_REL not found (build it, or drop -s)." >&2
    exit 1
fi

# --- 2) web UI (React app), same rule as the Windows installer --------------
UI_DIST="$KYBERFROG_DIR/ui/dist"
if [ ! -f "$UI_DIST/index.html" ]; then
    if command -v npm >/dev/null 2>&1; then
        echo "==> Building web UI (npm ci && npm run build)..."
        ( cd "$KYBERFROG_DIR/ui" && npm ci && npm run build )
    else
        echo "ERROR: web UI not built ($UI_DIST/index.html missing) and npm not found." >&2
        exit 1
    fi
fi

# --- 3) AppDir staging --------------------------------------------------------
APPDIR="$(mktemp -d)/KyberFrog.AppDir"
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" "$APPDIR/usr/bin/ui/dist"

cp "$KYBERFROG_DIR/$BIN_REL" "$APPDIR/usr/bin/kyberfrog"
# ui_dist() in web.rs resolves "ui/dist" relative to the running exe's own
# directory (current_exe().parent()) — inside the AppImage that's usr/bin/,
# not the AppDir root, so it must be staged there.
cp -a "$UI_DIST/." "$APPDIR/usr/bin/ui/dist/"
cp "$SCRIPT_DIR/kyberfrog.desktop" "$APPDIR/usr/share/applications/kyberfrog.desktop"
cp "$SCRIPT_DIR/kyberfrog.desktop" "$APPDIR/kyberfrog.desktop"

ICON_PNG="$KYBERFROG_DIR/kyberfrog/assets/kyberfrog.png"
if [ -f "$ICON_PNG" ]; then
    cp "$ICON_PNG" "$APPDIR/usr/share/icons/hicolor/256x256/apps/kyberfrog.png"
    cp "$ICON_PNG" "$APPDIR/kyberfrog.png"
else
    echo "WARNING: kyberfrog/assets/kyberfrog.png missing (only the .ico exists)." >&2
    echo "         Export a 256x256 PNG from kyberfrog.ico and re-run; shipping without an icon for now." >&2
fi

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/kyberfrog" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# --- 4) appimagetool ----------------------------------------------------------
APPIMAGETOOL="$SCRIPT_DIR/.appimagetool-x86_64.AppImage"
if [ ! -x "$APPIMAGETOOL" ]; then
    echo "ERROR: appimagetool not found at $APPIMAGETOOL." >&2
    echo "       Download once: curl -L -o '$APPIMAGETOOL' \\" >&2
    echo "         https://github.com/AppImage/appimagetool/releases/latest/download/appimagetool-x86_64.AppImage" >&2
    echo "       chmod +x '$APPIMAGETOOL'" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
OUTPUT_NAME="KyberFrog-$VERSION-x86_64.AppImage"
echo "==> Running appimagetool -> $OUTPUT_NAME"
ARCH=x86_64 "$APPIMAGETOOL" "$APPDIR" "$OUTPUT_DIR/$OUTPUT_NAME"

echo ""
echo "==> Done: $OUTPUT_DIR/$OUTPUT_NAME"
ls -lh "$OUTPUT_DIR/$OUTPUT_NAME"
