#!/bin/zsh
set -eu
ROOT="${0:A:h:h}"
cd "$ROOT"
ROMFORGE_DISTRIBUTION=1 zsh scripts/build-macos-app.sh
APP="$ROOT/artifacts/ROMForge.app"
python3 scripts/collect-licenses.py "$APP/Contents/Resources/ThirdPartyLicenses"
codesign --force --sign - "$APP"
codesign --verify --deep --strict "$APP"
VERSION=$(tr -d '\n' < VERSION)
mkdir -p artifacts/releases
ARCHIVE="ROMForge-$VERSION-macOS-arm64.zip"
ditto -c -k --keepParent "$APP" "artifacts/releases/$ARCHIVE"
(cd artifacts/releases && shasum -a 256 "$ARCHIVE" > SHA256SUMS.txt)
print "Release: artifacts/releases/$ARCHIVE"
