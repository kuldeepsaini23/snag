#!/usr/bin/env bash
# Builds target/installer/Snag-Setup-<version>.exe: the release app, the browser extension, and
# the Inno Setup installer around them. Needs Rust, Node and Inno Setup 6 (ISCC).
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
cargo build --release -p rdm-app
node extension/build.js

iscc=""
for candidate in "${LOCALAPPDATA:-}/Programs/Inno Setup 6/ISCC.exe" "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" "/c/Program Files/Inno Setup 6/ISCC.exe"; do
  [ -f "$candidate" ] && iscc="$candidate" && break
done
[ -n "$iscc" ] || { echo "Inno Setup 6 not found (winget install JRSoftware.InnoSetup)"; exit 1; }

"$iscc" //Q "//DAppVersion=$version" installer/snag.iss
ls -la "target/installer/Snag-Setup-$version.exe"
