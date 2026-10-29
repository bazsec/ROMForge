#!/bin/zsh
set -eu
ROOT="${0:A:h:h}"
cd "$ROOT"
APP="${ROMFORGE_APP:-$ROOT/artifacts/ROMForge.app}"
export ROMFORGE_APP="$APP"
ROMFORGE_DISTRIBUTION=1 zsh scripts/build-macos-app.sh
python3 scripts/collect-licenses.py "$APP/Contents/Resources/ThirdPartyLicenses"
codesign --force --sign - "$APP"
codesign --verify --deep --strict "$APP"
VERSION=$(tr -d '\n' < VERSION)
mkdir -p artifacts/releases
ARCHIVE="ROMForge-$VERSION-macOS-arm64.zip"
ditto -c -k --keepParent "$APP" "artifacts/releases/$ARCHIVE"
# Corresponding emulator sources and exact platform integration recipes.
python3 scripts/package-core-sources.py artifacts/releases
(cd artifacts/releases && shasum -a 256 "$ARCHIVE" "ROMForge-$VERSION-emulator-sources.tar.gz" > SHA256SUMS.txt)
print "Release: artifacts/releases/$ARCHIVE"
