#!/bin/zsh
set -eu
ROOT="${0:A:h:h}"
cd "$ROOT"
ICON_BUILD="$ROOT/artifacts/native-icon"
mkdir -p "$ICON_BUILD"
# Apple renders the platform mask, padding, and compatibility icon from the
# editable Icon Composer source. Requires Xcode with Icon Composer support.
xcrun ictool assets/branding/ROMForge.icon \
  --compile "$ICON_BUILD" --platform macosx \
  --minimum-deployment-target 13.0 --app-icon ROMForge \
  --output-partial-info-plist "$ICON_BUILD/Info.plist"
iconutil -c iconset "$ICON_BUILD/ROMForge.icns" -o "$ICON_BUILD/ROMForge.iconset"
cp "$ICON_BUILD/ROMForge.icns" assets/branding/ROMForge.icns
# egui sets its own Dock image; use Apple's rendered 256px fallback there too.
cp "$ICON_BUILD/ROMForge.iconset/icon_128x128@2x.png" assets/branding/romforge-macos-icon.png
