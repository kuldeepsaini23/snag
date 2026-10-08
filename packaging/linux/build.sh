#!/usr/bin/env bash
# Builds the Linux release into target/linux/: Snag-x86_64.AppImage and snag_<version>_amd64.deb.
# Build on Ubuntu 22.04 (its glibc is the oldest the binaries run on) with:
#   apt install build-essential pkg-config curl file libxkbcommon-x11-0
#   cargo install cargo-deb --locked
set -euo pipefail
cd "$(dirname "$0")/../.."

version=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
out=target/linux
tools=target/linux-tools
mkdir -p "$out" "$tools"

cargo build --release -p rdm-app

# .deb: [package.metadata.deb] in crates/app/Cargo.toml (binary, .desktop file, icons).
cargo deb -p rdm-app --no-build --no-strip --output "$out/snag_${version}_amd64.deb"

# AppImage: linuxdeploy copies in the libraries Snag needs. (No GTK: the tray icon talks D-Bus
# itself, the folder picker asks the desktop portal.)
fetch() {
  [ -s "$tools/$2" ] || curl -fsSL --retry 3 -o "$tools/$2" "$1"
  chmod +x "$tools/$2"
}
fetch https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage linuxdeploy-x86_64.AppImage

appdir="$out/AppDir"
rm -rf "$appdir"
# The app's own icon exports, plus a 512 px one for big launchers (from docs/logo).
for size in 64 128 256 512; do
  src="crates/app/assets/logo/snag-$size.png"
  [ "$size" = 512 ] && src=packaging/linux/snag-512.png
  mkdir -p "$appdir/usr/share/icons/hicolor/${size}x${size}/apps"
  cp "$src" "$appdir/usr/share/icons/hicolor/${size}x${size}/apps/snag.png"
done

# No FUSE in containers and CI runners: the tools unpack themselves instead.
export APPIMAGE_EXTRACT_AND_RUN=1
# The name the updater looks for (crates/core/src/selfupdate.rs: APPIMAGE).
export LDAI_OUTPUT="$out/Snag-x86_64.AppImage" OUTPUT="$out/Snag-x86_64.AppImage"
export ARCH=x86_64 VERSION="$version"
rm -f "$LDAI_OUTPUT"
# The window's keyboard libraries are loaded at run time (dlopen), so linuxdeploy can't see them:
# named here, they travel in the AppImage for systems without them (libxkbcommon-x11 is often
# missing). X11 and Wayland themselves come from the system.
libs=()
for lib in libxkbcommon.so.0 libxkbcommon-x11.so.0; do
  path=$(ldconfig -p | awk -v l="$lib" '$1 == l && /x86-64/ {print $NF; exit}')
  [ -n "$path" ] || { echo "$lib not found (apt install libxkbcommon-x11-0)" >&2; exit 1; }
  libs+=(--library "$path")
done
PATH="$PWD/$tools:$PATH" "$tools/linuxdeploy-x86_64.AppImage" \
  --appdir "$appdir" \
  --executable target/release/snag \
  "${libs[@]}" \
  --desktop-file packaging/linux/snag.desktop \
  --icon-file "$appdir/usr/share/icons/hicolor/256x256/apps/snag.png" \
  --output appimage

(cd "$out" && sha256sum "Snag-x86_64.AppImage" "snag_${version}_amd64.deb" | tee "SHA256SUMS-linux-$version.txt")
ls -la "$out/Snag-x86_64.AppImage" "$out/snag_${version}_amd64.deb"
