# macOS builder icon

`ROMForge.icon` is the editable Apple Icon Composer source. It combines the
original cartridge artwork at 76% scale with a warm ivory (#F4EBDD) background.
Apple's compiler supplies the macOS mask and compatibility rendering.

`zsh scripts/build-macos-icon.sh` requires Xcode with `ictool`. It regenerates
`ROMForge.icns`, the 256px `romforge-macos-icon.png` used by egui, and the native
`artifacts/native-icon/Assets.car`. The macOS app build runs this automatically
and bundles both the native asset catalog and the compatibility ICNS.

The header continues to use `romforge-logo.png` and `romforge-wordmark.png`.
This branding belongs to the builder, not to the generated game applications.
