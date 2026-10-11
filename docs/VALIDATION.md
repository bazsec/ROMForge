# Validation — October 11, 2026

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
