# Windows validation — October 29, 2026

## Standalone executable revision

- Windows output is now one `Game Title.exe` with embedded ROM, emulator DLL,
  build metadata, license/shader notices and six sizes of native icon resources.
  Matched cover artwork supplies the icon; ROMForge artwork is the fallback.
- Current workspace checks: 30 tests passed, one optional GPU reference test
  ignored; formatting and Clippy with warnings denied passed.
- Updated integration checks build NES and SNES executables, move only each `.exe`
  into an isolated directory, run 120 headless frames, and export/verify embedded
  notices. Native NES input/battery and both cores' state probes also passed.
- The runtime checks embedded ROM/core hashes before loading a private temporary
  DLL. Normal exits removed the DLL directory, leaving only the startup lock.
  A subprocess test bypassed all Rust destructors; the next launch removed its
  abandoned directory. A separate test confirmed active locked directories and
  unrelated folders survive cleanup.
- Built the user's Super Metroid locally with the matched JPEG artwork. Windows
  shell icon extraction confirmed the embedded artwork is used. Its standalone
  executable passed 120 headless frames. Earlier in this revision, it also booted
  with the GPU and its F1 main/keybind menus were visually inspected. Final cleanup
  and keyboard-hint refinements received background checks; full controller and
  menu interaction testing remains user QA.
- Save location is shared with the previous Windows folder build and mirrors the
  Mac collection: `Save Games/<title> [ROM hash prefix]/`. Existing Super Metroid
  battery data and settings remained in that directory. Executable renaming uses
  the same ROM identity. Five state slots and per-ROM settings remain in the shared
  runtime; cross-platform state files still require an identical core hash.

macOS packaging is unchanged by the standalone Windows revision; local validation
was performed on Windows. Rerun macOS CI before a release.

## Initial Windows folder preview (superseded output format)

Host: Windows x64, Rust 1.95.0, Visual Studio 2022 Build Tools, AMD Radeon RX
7900 XT (wgpu Vulkan). Built both pinned native emulator DLLs from source.

- Workspace tests: 27 passed; the optional upstream shader equivalence test remains
  ignored. Formatting and Clippy with warnings denied passed.
- `scripts/test-windows.ps1 -SkipBuild -Smoke` generated original NES and SNES
  diagnostic ROMs, selected Windows packaging with `--native`, and built portable
  game folders under a path containing spaces.
- Both packaged `Game.exe` files ran 120 headless frames and presented 120 actual
  CRT-Lottes frames with audio. Portable save/settings folders were created beside
  the collection. No commercial ROMs were used or distributed.
- NES A/B, Start/Select, D-pad, button release and battery reload checks passed.
  Both cores passed state restoration, deterministic video replay, SRAM comparison
  and invalid-state recovery probes. The blank SNES fixture has no cartridge SRAM.
- Unit coverage includes Windows package payloads, core/resource placement,
  refusal to overwrite existing output, failed-build cleanup, portable save paths,
  and ICNS PNG preview/truncation handling.
- `dumpbin /dependents` showed no Visual C++ runtime DLL dependency in the shipped
  GUI/runtime or either emulator DLL. Windows GUI startup was confirmed.

Limits: diagnostic fixtures establish native startup and integration, not broad
gameplay/controller compatibility. Windows games are portable folders, not single
standalone executables. Selected cover artwork is retained but not embedded into
the executable's Explorer icon. Windows automatic updating and Linux packaging
are not implemented. macOS validation below predates this Windows change; rerun
the existing macOS CI checks before release.

# macOS validation — October 11, 2026

Host: macOS, Apple Silicon. Pinned Snes9x commit: `fae2fea08f74180759ef540ee94259213f503480`. SDL 3.4.18 via Rust `sdl3` 0.20.0, statically linked.

Verified:

- All seven user-provided ROMs loaded through the native core and produced software video/audio: Contra III, Donkey Kong Country, Zelda: A Link to the Past, Street Fighter II Turbo, Super Mario World, Super Metroid, Super Punch-Out.
- All seven game bundles passed a 120-frame headless launch check using their own embedded payload and bundled core.
- Zelda's packaged SDL smoke test rendered 180 frames and queued 191,772 interleaved audio samples.
- Visually inspected the generated Zelda app: original startup/title sequence, Start to file selection, directional keyboard navigation, and fullscreen/windowed toggle.
- Visually exercised the builder's **Build macOS app** action for Contra III; packaging and launch check succeeded.
- Zelda's 8,192-byte save RAM passed write/read and core-unload/reload restoration checks. Atomic save replacement and aspect-preserving letterboxing have unit coverage.
- Unit tests cover payload bounds/corruption, all three software pixel formats, pitch handling, duplicate frames, and XML-safe game titles. Workspace tests, formatting, and Clippy pass.
- `codesign --verify --deep --strict` passed for the Zelda game bundle. `otool -L` showed only macOS system dependencies for the runtime and core; there is no external SDL dylib. Zelda bundle size is approximately 11 MiB in this development build.

Not yet verified: controller reconnect cycling and controller remapping with physical hardware, subjective audio quality, extended gameplay or in-game battery-save interactions across all games, Intel macOS, Windows, clean-machine Gatekeeper behavior, notarization, or firmware-dependent/special cartridges. Loading and callback checks do not establish full compatibility.

F1-menu update:

- Visually checked the built-in SDL pixel font, five-option overlay, darkened/paused gameplay, fullscreen/windowed toggle, keyboard navigation, save/load confirmations, and keyboard remapping (restored original binding afterward).
- Zelda's native state test saved 823,407 bytes and verified SRAM restoration, identical next-frame pixels after repeated restore, and continued execution after rejecting an invalid state. Unit tests reject wrong ROM/core, truncated and corrupted snapshots, and cover legacy settings defaults and binding swaps.
- SDL enumerated the Bluetooth DualSense. The user confirmed controller response after focusing the game window. Menu mapping was subsequently changed at the user's request to Nintendo layout: East/Circle confirms and South/Cross backs out; a physical retest of that change remains for the user.
- Scanlines were visible during gameplay and settings persisted. This initially used a simple line overlay; the comparison update below replaces it with actual CRT presets.
- All seven game bundles were rebuilt with the new menu/runtime and passed strict signature verification and 120-frame headless launch checks. ROMForge.app now embeds that runtime for future builds. Existing bundles are preserved under artifacts/previous-builds; user saves remain outside the bundles.
- No additional font file or font-rendering dependency was added. Shader research led to the three-preset comparison update below.

Windows single-executable templates and production batch/incremental behavior remain future milestones.

CRT comparison update (same date):

- Bundled the actual CRT-Lottes, CRT-Guest Advanced and CRT-Royale upstream presets from `libretro/slang-shaders` revision `e1d75632a205c70f14a4cc947c46d5abb7b3f7f1`: 56 shader/source/LUT files, plus provenance and licenses. Total resources approximately 1.38 MB.
- Added wgpu 30 / librashader 0.12; the detected adapter was Apple M5 Max using Metal. The four rendering choices (including Off) each passed 120-presented-frame GPU checks. Rendering uses native GPU textures; the built-in SDL pixel menu is composited afterward and remains unfiltered.
- Visually verified Off, Lottes, Guest, and Royale in Zelda, switching presets while paused, background compilation feedback, restoring an existing save state, and Royale at fullscreen resolution. The menu shows the active preset and persists the choice. Nintendo-style Circle confirm / Cross back is present in the final menu and prompts.
- Workspace tests and strict Clippy pass. Native state serialization/restoration tests from the menu milestone remain applicable; the core binary is unchanged.
- Optimized runtime builds are now used in generated apps and ROMForge.app, instead of distributing the larger shader-development binary. All game apps retain their icons and external save locations.

These are local Apple Silicon checks, not performance guarantees for other GPUs or long-duration gameplay. Full shader/controller/platform compatibility still needs broader testing.


CRT-Lottes selection (2026-10-11): removed Guest Advanced and Royale from shipping assets and menu; legacy preset preferences deserialize to Lottes. New games default to Lottes with Off available. All 9 workspace tests and strict Clippy pass. Rebuilt and signature-verified all seven game bundles; each passed headless boot. Packaged Zelda passed 120 presented frames using Lottes on Metal. Zelda bundle is now 20,152,547 bytes (previously 21,426,593 bytes). Prior comparison builds and assets remain in artifacts/previous-builds.


## Compact runtime, 2026-10-11

Replaced the shipping librashader engine with a direct wgpu pass using machine-generated WGSL from the same pinned CRT-Lottes source and original default parameters. Shader compiler dependencies remain only in a separate development generator and dev-only reference test. Generator output was reproduced byte-for-byte. Release optimization is z with full LTO; the signed runtime is 4,330,192 bytes versus 12,791,472 bytes previously. The complete Zelda bundle is 11,691,403 bytes versus 20,152,547 bytes (42.0% smaller). Icons and emulator binaries were verified byte-identical to the preceding bundles, preserving existing state identity.

Nine ordinary workspace tests and strict all-target Clippy pass. Explicit GPU equivalence test passed at 256x224 -> 768x672, 512x448 -> 1024x896, and 256x239 -> 853x641. Maximum channel difference was 1/255; mean absolute channel differences were 0.000016, 0.000069 and 0.000091 in byte units. This covers color, bloom, mask, curvature, source resolution and uneven output scaling against the prior engine.

A 300-frame windowed Lottes run took 5.544 seconds with the previous runtime and 5.423 seconds with the compact z build, including startup on Apple M5 Max / Metal. This is a bounded playback check, not an exhaustive benchmark. All seven updated apps passed strict code signatures and headless boot. Packaged Zelda passed 120 presented frames each with Off and Lottes. Native UI verified fullscreen gameplay, F1 menu and loading existing slot 5. Controller/audio feature code remains unchanged; physical controller input and subjective audio were not re-tested in this size pass. Other operating systems remain unvalidated. Backups are under artifacts/previous-builds/before-runtime-trim-20261011-080801.


## Portable collection saves, 2026-10-11

All seven game apps and ROMForge builder updated to use Save Games beside the collection. Twelve ordinary workspace tests passed, including bundle-parent resolution, migration, non-overwrite behavior, no resurrection of removed slots, renamed-ROM reuse, name sanitization, ROM separation, and session locking. Strict all-target Clippy passed. All seven signed apps completed a real windowed GPU launch using the portable location. Zelda battery.srm and both existing state files were verified byte-identical to the original Library files after migration. Legacy originals remain in place. Existing icons and core binary retained.

## v008 — batch polish and NES, 2026-10-11

- 25 workspace tests pass, with the existing GPU-reference test intentionally ignored in the ordinary suite; strict all-target Clippy and formatting pass. New coverage includes cancellation/retry paths, version classification, duplicate output names, matching-ROM replacement, retained backups, rollback after rename or final verification failure, and truncated NES headers/trainers.
- Isolated GUI testing used separate preferences and original NES/SNES diagnostic ROMs. Verified Create all → Start batch, progress, a successful two-console batch, enlarged cover preview, Not built filtering, older-version rebuild, failure continuation, and Retry failed after repairing the fixture. Previous-app backup and new v008 manifests were inspected.
- Nestopia revision `60a325237ce3df3a42f8f58cec5ebf0d6be412eb`: original NROM diagnostic verified A/B, Start/Select, every D-pad direction, button release and 8,192-byte battery RAM reload. State test verified SRAM restore, deterministic next-frame video and invalid-state recovery (21,813-byte state). Both baseline and size-optimized builds pass these checks.
- A packaged NES game rendered 181 frames (180 CRT-Lottes frames) on Apple M5 Max/Metal and queued 288,996 interleaved audio samples. NES and SNES packaged diagnostics both passed 120-frame headless checks, delivery to a temporary NAS folder, final rename and strict signature checks. Test data was isolated from the user's collection and removed from the NAS afterward.
- Runtime is 4,062,976 bytes, 32 bytes larger than v007. Snes9x binary is byte-identical to v007, preserving its state identity. Nestopia is 3,745,392 bytes after size optimization/LTO/dead stripping, versus a 5,900,016-byte baseline; no emulation features were removed. Only the selected console's core goes into each generated game app.
- Current 717-game batch and running builder were not replaced or interrupted. The new distribution is staged separately. NES specialty peripherals, FDS and broad commercial-game/mapper coverage remain unverified/unsupported as documented; diagnostic success does not establish complete NES compatibility.
