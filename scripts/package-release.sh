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
# GPLv2 corresponding source and the exact build recipe accompany the core binary.
SOURCE_ROOT=$(mktemp -d)
trap 'rm -rf "$SOURCE_ROOT"' EXIT
SOURCE_NAME="ROMForge-$VERSION-Nestopia-source"
mkdir -p "$SOURCE_ROOT/$SOURCE_NAME/vendor/nestopia" "$SOURCE_ROOT/$SOURCE_NAME/scripts"
git -C vendor/nestopia archive 60a325237ce3df3a42f8f58cec5ebf0d6be412eb | tar -xf - -C "$SOURCE_ROOT/$SOURCE_NAME/vendor/nestopia"
cp scripts/build-nestopia-macos.sh "$SOURCE_ROOT/$SOURCE_NAME/scripts/"
print 'Corresponding Nestopia source for the ROMForge NES core. On macOS Apple Silicon with Xcode command-line tools installed, run: sh scripts/build-nestopia-macos.sh. No source changes were made; size optimization and export/link flags are in that script. Nestopia license: vendor/nestopia/COPYING; additional notices remain in the source tree.' > "$SOURCE_ROOT/$SOURCE_NAME/README.txt"
COPYFILE_DISABLE=1 tar -czf "artifacts/releases/$SOURCE_NAME.tar.gz" -C "$SOURCE_ROOT" "$SOURCE_NAME"
(cd artifacts/releases && shasum -a 256 "$ARCHIVE" "$SOURCE_NAME.tar.gz" > SHA256SUMS.txt)
print "Release: artifacts/releases/$ARCHIVE"
