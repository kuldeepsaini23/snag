#!/usr/bin/env bash
# Builds the macOS release into target/macos/: Snag.app (universal: Apple silicon and Intel),
# Snag-macOS.zip (what Snag's updater installs) and Snag-macOS.dmg (drag Snag to Applications).
# Run on a Mac with Xcode's command line tools (xcode-select --install) and both Rust targets:
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
# SNAG_TOOLS_DIR=<folder> also puts the yt-dlp, gallery-dl, ffmpeg and ffprobe found there into
# Snag.app/Contents/Resources/bin (Snag uses those before downloading its own).
set -euo pipefail
cd "$(dirname "$0")/../.."

version=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
out=target/macos
app="$out/Snag.app"
rm -rf "$out"
mkdir -p "$out"

# Info.plist's LSMinimumSystemVersion.
export MACOSX_DEPLOYMENT_TARGET=11.0
for target in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release -p rdm-app --target "$target"
done

mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/snag" \
  target/aarch64-apple-darwin/release/snag target/x86_64-apple-darwin/release/snag
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"

# The icon: the logo on macOS's icon grid (packaging/macos/icon-*.png, from docs/logo), the bold
# mark at the two smallest sizes where the engraving's fine lines blur.
iconset="$out/Snag.iconset"
mkdir -p "$iconset"
icon() { # <pixels> <file name>
  local src=packaging/macos/icon-1024.png
  [ "$1" -le 32 ] && src=packaging/macos/icon-small-1024.png
  sips -z "$1" "$1" "$src" --out "$iconset/$2" >/dev/null
}
for size in 16 32 128 256 512; do
  icon "$size" "icon_${size}x${size}.png"
  icon $((size * 2)) "icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Snag.icns"
rm -rf "$iconset"

if [ -n "${SNAG_TOOLS_DIR:-}" ]; then
  mkdir -p "$app/Contents/Resources/bin"
  for tool in yt-dlp gallery-dl ffmpeg ffprobe; do
    if [ -f "$SNAG_TOOLS_DIR/$tool" ]; then
      cp "$SNAG_TOOLS_DIR/$tool" "$app/Contents/Resources/bin/"
      chmod 755 "$app/Contents/Resources/bin/$tool"
    fi
  done
fi

# Ad-hoc signature: Apple silicon runs only signed code. Not notarized, so the first launch is
# right-click → Open (see the README).
xattr -cr "$app"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

# The zip keeps the bundle as it is (ditto: permissions, symlinks, the signature).
ditto -c -k --sequesterRsrc --keepParent "$app" "$out/Snag-macOS.zip"

# The disk image: Snag.app next to a link to Applications.
stage="$out/dmg"
mkdir -p "$stage"
ditto "$app" "$stage/Snag.app"
ln -s /Applications "$stage/Applications"
hdiutil create -volname Snag -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$out/Snag-macOS.dmg"
rm -rf "$stage"

(cd "$out" && shasum -a 256 Snag-macOS.zip Snag-macOS.dmg | tee "SHA256SUMS-macos-$version.txt")
ls -la "$out/Snag-macOS.zip" "$out/Snag-macOS.dmg"
