# Changelog

## v009

- Add native Windows x64 support with automatic host selection in the builder and batch workflow.
- Build one standalone Windows game executable with its ROM, emulator core, artwork icon and notices embedded.
- Remove private temporary cores on exit and clean abandoned runtime folders on the next launch while protecting running games.
- Share the macOS-style portable Save Games layout and in-game configuration menu on Windows.
- Add Windows build scripts, pinned native emulator builds, package checks and release downloads with checksums.
- Keep Windows update installation manual; Linux packaging remains planned.

## v008

- Show the release version beside macOS in the main window header.
- Clarify batch review with one Start batch action, progress, results and Retry failed.
- Check covers and existing builds in a background worker; add Missing cover, Not built and Needs update filters.
- Record build versions and identities; optionally rebuild older apps with launch verification, rollback and retained backups.
- Move cover management into a clickable preview to recover workspace space.
- Add NES cartridge detection and Nestopia packaging, using the existing fullscreen, controller, CRT and save-state runtime. Only the relevant core is included in each game.
- Include pinned Nestopia source/build instructions and license notices.

## v007

- Reduce the runtime by excluding unused SDL camera, dialog, tray, and GPU subsystems; retain audio, windowing, controller support, menu rendering, and wgpu graphics.
- Build Snes9x with size optimization, LTO, limited exports, and symbol stripping while keeping all emulation features.
- Add a developer probe for video/audio comparisons and save-state compatibility.

## v006

- Avoid a redundant 1024px upscaled icon representation when source artwork is 512px or smaller; retain standard icons through 512px.

## v005

- Add Exit Game to the pause menu and make Escape quit through the normal save-and-shutdown path.
- Show the existing Start + Select controller menu shortcut in the menu.

## v004

- Build and sign games on local storage before copying and verifying them on the destination, avoiding NAS metadata signing failures.

## v003

- Move the cover preview beside the game title and compact the icon controls to save vertical space.

## v002

- JPEG/JPG cover matching, previews and manual selection.
- JPEG-to-ICNS packaging alongside PNG and ICNS support.
- A deterministic cover-formatting utility for centered, black-backed square artwork.

## v001

- macOS Apple Silicon builder and fullscreen SNES game apps with controller support, F1 menu, save states and CRT-Lottes.
- Recursive cover matching with catalog aliases, artwork previews and manual overrides.
- Batch review, existing-output preservation and cancellation after the current game.
- Saved folder preferences and cover selections.
- Settings page inside the main window; charcoal/amber controls and auto-hiding scrollbars.
- Automatic launch check during builds; output-folder action aligned with build buttons.
- GitHub Release update checks and user-triggered installation with checksum and bundle verification.

Versions increment by one: v001, v002, v003, …
