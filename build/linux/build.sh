#!/usr/bin/env bash
# Builds a portable tarball and, if cargo-deb is available, a .deb package.
set -euo pipefail

cd "$(dirname "$0")/../.."

VERSION="$(sed -n 's/^[[:space:]]*version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
ARCH="$(uname -m)"
DIST=dist
STAGE="$DIST/cys-$VERSION-linux-$ARCH"

if [[ -z "$VERSION" ]]; then
    echo "Cannot read version from Cargo.toml" >&2
    exit 1
fi

echo "Building CYS $VERSION for $ARCH..."
cargo build --release

rm -rf "$STAGE"
mkdir -p "$STAGE" "$DIST"

install -m 755 target/release/cys "$STAGE/cys"
cp -a assets "$STAGE/assets"
cp build/linux/install.sh "$STAGE/install.sh"
cp build/linux/cys.desktop "$STAGE/cys.desktop"
cp build/icons/icon.png "$STAGE/cys.png"
cp LICENSE README.md "$STAGE/"

TARBALL="$DIST/cys-$VERSION-linux-$ARCH.tar.gz"
tar -czf "$TARBALL" -C "$DIST" "$(basename "$STAGE")"
echo "Tarball: $TARBALL"

DEB_TOOL=""
if command -v cargo-deb >/dev/null 2>&1; then
    DEB_TOOL="$(command -v cargo-deb)"
elif [[ -x "$HOME/.cargo/bin/cargo-deb" ]]; then
    DEB_TOOL="$HOME/.cargo/bin/cargo-deb"
fi

if [[ -n "$DEB_TOOL" ]]; then
    case "$ARCH" in
        x86_64) DEB_ARCH=amd64 ;;
        aarch64) DEB_ARCH=arm64 ;;
        *) DEB_ARCH="$ARCH" ;;
    esac
    DEB="$DIST/cys_${VERSION}_${DEB_ARCH}.deb"
    "$DEB_TOOL" -p cys --no-build --output "$DEB"
    echo "Debian package: $DEB"
else
    echo "cargo-deb not found, skipping .deb (install with: cargo install cargo-deb)"
fi
