#!/bin/zsh
set -eu
ROOT="${0:A:h:h}"
cd "$ROOT"
export PATH="$ROOT/vendor/build-tools/bin:$HOME/.cargo/bin:$PATH"
APP="${ROMFORGE_APP:-$ROOT/artifacts/ROMForge.app}"
CORE="$ROOT/vendor/snes9x/libretro/snes9x_libretro.dylib"
if [[ ! -f "$CORE" ]]; then
  print 'Build the development Snes9x core first; see docs/DEVELOPMENT.md.'
  exit 1
fi
zsh scripts/build-macos-icon.sh
cargo build --workspace --locked
cargo build --release --workspace --locked
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp target/release/romforge-gui "$APP/Contents/MacOS/ROMForge"
cp target/release/romforge-runtime target/release/romforge-builder "$APP/Contents/MacOS/"
cp "$CORE" "$ROOT/vendor/nestopia/libretro/nestopia_libretro.dylib" "$APP/Contents/Frameworks/"
cp third-party/Nestopia-NOTICES.txt third-party/Nestopia-LICENSE.txt third-party/Snes9x-LICENSE.txt third-party/SDL-LICENSE.txt "$APP/Contents/Resources/"
rm -rf "$APP/Contents/Resources/shaders"
cp -R assets/shaders "$APP/Contents/Resources/"
cp assets/branding/ROMForge.icns "$APP/Contents/Resources/ROMForge.icns"
cp artifacts/native-icon/Assets.car "$APP/Contents/Resources/Assets.car"
if [[ "${ROMFORGE_DISTRIBUTION:-0}" == "1" ]]; then
  rm -f "$APP/Contents/Resources/project-root"
else
  print -r -- "$ROOT" > "$APP/Contents/Resources/project-root"
fi
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>ROMForge</string>
<key>CFBundleIdentifier</key><string>dev.romforge.builder</string>
<key>CFBundleName</key><string>ROMForge</string>
<key>CFBundleDisplayName</key><string>ROMForge</string>
<key>CFBundleIconFile</key><string>ROMForge.icns</string>
<key>CFBundleIconName</key><string>ROMForge</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
VERSION_NUMBER=$(python3 -c 'from pathlib import Path; print(int(Path("VERSION").read_text().strip()[1:]))')
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString 0.0.$VERSION_NUMBER" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION_NUMBER" "$APP/Contents/Info.plist"
# Local ad-hoc signing only. This does not confer Developer ID or notarization.
codesign --force --sign - "$APP/Contents/Frameworks/snes9x_libretro.dylib"
codesign --force --sign - "$APP/Contents/Frameworks/nestopia_libretro.dylib"
codesign --force --sign - "$APP/Contents/MacOS/romforge-runtime"
codesign --force --sign - "$APP/Contents/MacOS/romforge-builder"
codesign --force --sign - "$APP"
print "Created $APP"
