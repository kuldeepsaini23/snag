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

# Store packages: Firefox (checked by Mozilla's linter first) and Chrome/Edge.
mkdir -p target/store
(cd extension && npx --yes web-ext@latest lint --source-dir dist/firefox --output text | tail -6)
(cd extension && npx --yes web-ext@latest build --source-dir dist/firefox --artifacts-dir ../target/store --overwrite-dest >/dev/null)
python - "$version" <<'PY'
import os, sys, zipfile
out = f"target/store/snag-chrome-{sys.argv[1]}.zip"
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for root, _, files in os.walk("extension/dist/chrome"):
        for name in files:
            path = os.path.join(root, name)
            z.write(path, os.path.relpath(path, "extension/dist/chrome"))
PY
(cd target && sha256sum "installer/Snag-Setup-$version.exe" store/*.zip > "SHA256SUMS-$version.txt")
# Also under a name without the version, so .../releases/latest/download/Snag-Setup.exe (the
# website's Download button) always gets the newest one. Upload both to the release.
cp "target/installer/Snag-Setup-$version.exe" target/installer/Snag-Setup.exe
ls -la "target/installer/Snag-Setup-$version.exe" target/installer/Snag-Setup.exe target/store/
