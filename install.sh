#!/usr/bin/env bash
# Installs CYS from a downloaded tarball or from a local release build.
set -euo pipefail

PREFIX="${CYS_INSTALL_PREFIX:-$HOME/.local}"
SYSTEM=0
UNINSTALL=0

usage() {
    cat <<'EOF'
Usage: ./install.sh [options]

Options:
  --system          install system-wide into /usr/local (needs root)
  --prefix=PATH     override the installation prefix
  --uninstall       remove files installed by this script
  -h, --help        show this help

The default prefix is ~/.local. Game data is NOT stored in the prefix:
saves and settings live in ${XDG_DATA_HOME:-~/.local/share}/cys
(or %APPDATA%/CYS on Windows).
EOF
}

for arg in "$@"; do
    case "$arg" in
        --system) SYSTEM=1 ;;
        --prefix=*) PREFIX="${arg#*=}" ;;
        --uninstall) UNINSTALL=1 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $arg" >&2; usage >&2; exit 2 ;;
    esac
done

if (( SYSTEM )); then
    if [[ $EUID -ne 0 ]]; then
        echo "Run with sudo: sudo ./install.sh --system" >&2
        exit 1
    fi
    PREFIX="${CYS_INSTALL_PREFIX:-/usr/local}"
fi

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/cys"
APPLICATIONS_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/512x512/apps"

if (( UNINSTALL )); then
    rm -f "$BIN_DIR/cys" "$APPLICATIONS_DIR/cys.desktop" "$ICON_DIR/cys.png"
    rm -rf "$SHARE_DIR"
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$APPLICATIONS_DIR" >/dev/null 2>&1 || true
    echo "CYS removed from $PREFIX (saves in the user data directory were kept)."
    exit 0
fi

SRC_BIN=""
SRC_ASSETS=""
if [[ -x ./cys && -d ./assets ]]; then
    SRC_BIN=./cys
    SRC_ASSETS=./assets
elif [[ -x ./target/release/cys && -d ./assets ]]; then
    SRC_BIN=./target/release/cys
    SRC_ASSETS=./assets
else
    echo "No CYS binary found." >&2
    echo "Run this script from the extracted release tarball, or build first with:" >&2
    echo "  cargo build --release" >&2
    exit 1
fi

SRC_DESKTOP=./packaging/cys.desktop
[[ -f ./cys.desktop ]] && SRC_DESKTOP=./cys.desktop
SRC_ICON=./img/icon.png
[[ -f ./cys.png ]] && SRC_ICON=./cys.png

if [[ ! -f "$SRC_DESKTOP" || ! -f "$SRC_ICON" ]]; then
    echo "Missing cys.desktop or icon next to the installer." >&2
    exit 1
fi

mkdir -p "$BIN_DIR" "$SHARE_DIR" "$APPLICATIONS_DIR" "$ICON_DIR"
install -m 755 "$SRC_BIN" "$SHARE_DIR/cys"
mkdir -p "$SHARE_DIR/assets"
cp -a "$SRC_ASSETS/." "$SHARE_DIR/assets/"
ln -sfn "$SHARE_DIR/cys" "$BIN_DIR/cys"
install -m 644 "$SRC_DESKTOP" "$APPLICATIONS_DIR/cys.desktop"
install -m 644 "$SRC_ICON" "$ICON_DIR/cys.png"

command -v update-desktop-database >/dev/null 2>&1 && \
    update-desktop-database "$APPLICATIONS_DIR" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && \
    gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true

echo "CYS installed into $SHARE_DIR"
echo "Launch: cys (or find 'Create your Shop' in the application menu)"
echo "Saves:  ${XDG_DATA_HOME:-$HOME/.local/share}/cys/data"
