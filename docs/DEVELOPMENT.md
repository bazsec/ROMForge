# Development

## Windows development

Use Windows x64 with Rust's `x86_64-pc-windows-msvc` toolchain, Visual Studio
2022 C++ Build Tools (Desktop development with C++, including a Windows SDK),
and CMake. The script finds CMake bundled with Visual Studio when it is not on PATH.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-windows-app.ps1
# Optional optimized build:
powershell -ExecutionPolicy Bypass -File scripts/build-windows-app.ps1 -Release
& ./artifacts/ROMForge-Windows/romforge-gui.exe
```

`Launch ROMForge.ps1` builds and opens the app. The build script fetches the same
pinned Snes9x and Nestopia revisions as macOS and refuses modified source checkouts.
The native CMake build consumes their explicit upstream source manifests, keeps
all chips/mappers, and uses a small MSVC C/C++ data-linkage compatibility header.
Rust, SDL and emulator cores use static C runtimes. Cores are loaded as native DLLs.
Native code and packaged games must match the x64 app architecture.

The UI selects native packaging automatically. A Windows build is one
`Game Title.exe`, with its ROM, emulator DLL, build identity, notices and artwork
embedded as PE resources. Copy that executable alone to launch on another PC.
On launch the runtime verifies the ROM/core hashes, writes the bundled DLL into
a private `%TEMP%/ROMForge/runtime/game-*` directory, and loads it. An exclusive
lease protects each running game's directory. Normal exit removes it; the next
game launch removes unlocked abandoned runtime directories after a crash. Startup
cleanup and directory creation are serialized so concurrent games remain safe.
Only marked direct child directories are eligible for cleanup; saves are separate.

Saves live in `Save Games/<title> [ROM fingerprint]/` beside the `.exe` collection,
matching macOS's layout outside `.app` bundles. Renaming the executable keeps the
same save identity. Earlier Windows folder builds use the same location, so their
existing saves and settings are reused. Move Save Games too when moving progress.
Windows builder preferences live in `%APPDATA%/ROMForge/builder.json`.
PNG/JPEG covers and PNG-containing ICNS previews work on Windows. Artwork is
embedded as the Explorer executable icon at 16–256 pixels, preserving aspect ratio
with transparent padding. Games without a matched cover use the ROMForge icon.
`Game.exe --export-resources NEW_DIRECTORY` exports bundled licenses/shader sources.
F1 opens the shared in-game menu for keybinds, five save/load slots, CRT scanlines,
fullscreen/windowed mode and exit. Keyboard hints appear when no controller is connected.
The CLI accepts `--native` in place of `--windows` or `--macos` to select the
current host. The UI and builder share supported platforms and package paths;
Linux is excluded until its packager is implemented.

Automatic update installation remains macOS-only; Windows shows release links
for manual updates. Windows binaries are unsigned. Linux packaging remains planned.

```powershell
& ./target/debug/romforge-builder.exe --windows `
  ./target/debug/romforge-runtime.exe `
  ./vendor/snes9x/libretro/snes9x_libretro.dll `
  ./third-party/Snes9x-LICENSE.txt ./ROMs/Game.sfc ./artifacts/Game.exe
& ./artifacts/Game.exe
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
./scripts/test-windows.ps1 -SkipBuild
# Also exercise a real window, GPU shader, audio, and portable saves:
./scripts/test-windows.ps1 -SkipBuild -Smoke
```

Mac binaries and `.app` game bundles cannot run on Windows. Rebuild games from
the original ROMs on the destination platform. Battery saves are portable;
save states include the core hash and are rejected across different core binaries.

Rust stable (minimum 1.88), a C/C++ toolchain, and CMake are required. SDL3 is built from the Cargo-pinned source and linked statically. On this host CMake 3.31.6 is installed in a workspace-local Python virtual environment:

```sh
python3 -m venv vendor/build-tools
vendor/build-tools/bin/pip install cmake==3.31.6
export PATH="$PWD/vendor/build-tools/bin:$PATH"
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

## Development core

Snes9x is used under its noncommercial development restrictions. Native files under `vendor/` are local, ignored build inputs. Reproduce the macOS core with:

```sh
git clone https://github.com/libretro/snes9x.git vendor/snes9x
git -C vendor/snes9x checkout fae2fea08f74180759ef540ee94259213f503480
python3 scripts/build-snes9x-macos.py
```

For NES, `scripts/build-nestopia-macos.sh` fetches the pinned Nestopia revision and builds the arm64 core with size optimization, LTO and the upstream `retro_*` export interface. No emulator features are removed. Both cores must exist before building the GUI bundle.

## Builder and game apps

Open `artifacts/ROMForge.app`. To rebuild it, double-click `Launch ROMForge.command` or run `scripts/build-macos-app.sh`. Choose a ROM and **Build macOS app**, then open the generated app in the output folder. The app contains its ROM, core, licenses and runtime; it does not need the source workspace to run. The builder preview remembers its development workspace via a local resource file.

Build from the CLI:

```sh
target/debug/romforge-builder --macos \
  target/debug/romforge-runtime \
  vendor/snes9x/libretro/snes9x_libretro.dylib \
  third-party/Snes9x-LICENSE.txt \
  'ROMs/Legend of Zelda, The - A Link to the Past.sfc' \
  'artifacts/Legend of Zelda, The - A Link to the Past.app'
```

The output must not already exist. Choose a new directory/name for revised builds; the GUI batch review offers verified replacement of older builds with a matching ROM. Previous apps are retained in `.romforge-backups`. Packaging stages the entire bundle in a temporary sibling directory, signs it locally, verifies it, then renames it into place. This is native macOS packaging, not cross-compilation or notarization.

## Runtime diagnostics

Play an external ROM:

```sh
target/debug/romforge-runtime vendor/snes9x/libretro/snes9x_libretro.dylib \
  'ROMs/Legend of Zelda, The - A Link to the Past.sfc'
```

Add `--headless` before the core to run 120 frames without a window. `Test ROMForge.command` does this too. `--smoke-frames 180 --windowed` runs an actual SDL video/audio session for 180 frames and exits; it exercises the normal save directory, so never run a second copy of the same game at once. Generated game binaries need no core or ROM arguments because they locate their bundle resources.

```sh
'artifacts/Legend of Zelda, The - A Link to the Past.app/Contents/MacOS/Game' \
  --smoke-frames 180 --windowed
cargo run --locked -p romforge-core --example check_sram -- \
  vendor/snes9x/libretro/snes9x_libretro.dylib \
  'ROMs/Legend of Zelda, The - A Link to the Past.sfc'
```

`cargo run -p romforge-core --example check_states -- CORE ROM` checks native serialization, SRAM restoration, deterministic video after restore, and recovery from an invalid state, entirely in memory. `target/debug/romforge-runtime --controllers` prints SDL's detected controllers and mappings.

The SRAM example tests write/read/reload in memory without touching player save files. `scripts/make-test-rom.py` generates original minimal 65816 code when a commercial ROM is unavailable.

## Saves and controls

Packaged macOS apps use `<collection>/Save Games/<title> [<first 12 ROM SHA-256 characters>]/`. A full `rom.sha256` identity file prevents mismatches. Existing matching folders are reused after ROM renaming. Development binaries continue to use `~/Library/Application Support/ROMForge/games/<ROM SHA-256>/`. Packaged games copy legacy battery data, states, and settings on first launch, preserve originals, never overwrite destination saves, and mark completed migration to avoid resurrecting deleted slots. `battery.srm` holds the game's own battery save; `settings.json` stores fullscreen, selected CRT shader, and keyboard/controller bindings. Legacy `scanlines: true` migrates to CRT-Lottes unless a shader choice is present. `slot-1.state` through `slot-5.state` store snapshots with a versioned header, ROM/core hashes and a payload checksum. Snapshots from a different core binary are rejected. Writes are atomic, changed SRAM is flushed every five seconds and on exit, and an exclusive session lock prevents concurrent copies from racing. The folder is outside the bundle; move Save Games along with the apps when relocating the collection. The collection must be writable. Per-game and legacy session locks prevent concurrent save writers. Battery saves require the game's normal save action; they are not save states.

Keyboard and gamepad mappings are in the README. The app pauses and clears queued audio when focus is lost. A connected gamepad uses positional face-button mapping (South=B, East=A, West=Y, North=X), D-pad or left stick, shoulders, Start and Back. F1 opens the six-option menu; Create + Options does the same on DualSense. Bindings are customizable in Keybinds and persist per ROM. SDL's built-in bitmap text supplies the menu font without another font dependency. Escape exits gameplay or the menu; Command-Q/window close exits.

The original development overlay packager remains available with three positional arguments. Its output requires an external core and is for diagnostics only. Production macOS games use bundle resources rather than executable overlays.

## Game icons

The GUI defaults to the selected ROM directory's `Icons` subfolder. Choose another directory with **Choose icons…**. Matching indexes subfolders, normalizes region/revision tags and punctuation, and applies catalog aliases; ambiguous matches require a selection. PNG/ICNS extensions are case-insensitive. If both formats match, ICNS takes precedence. Missing icons are optional; unreadable folders and invalid images report errors.

The CLI accepts `--icon PATH` after the output path. PNGs must be 16–8192 pixels per side; rectangular artwork receives transparent padding and at most 32 MiB. macOS `sips` and `iconutil` generate the standard icon sizes without changing the source. The resulting `Contents/Resources/Game.icns` is referenced by `CFBundleIconFile` and included before bundle signing. Adding an icon requires rebuilding the app; the source file is not needed at launch.

Verified automatic matching and a GUI build using the supplied 1254×1254 Zelda PNG. The resulting bundle contains Game.icns, passes strict signature verification and a headless launch check. The earlier Zelda app is preserved under `artifacts/previous-builds`. Fullscreen remains the default; the windowed preference left by the earlier Zelda test was reset to fullscreen.

## Compact CRT runtime

SDL owns the window, audio, gamepads, and built-in pixel menu font. wgpu 30 owns the GPU surface (Metal on macOS) and runs the upstream CRT-Lottes fragment shader, translated into WGSL during development, into an offscreen GPU texture. The final composite preserves aspect ratio and draws an unfiltered, cached SDL bitmap menu above it. There is no GPU readback during gameplay. The fixed shader pipeline is created once at startup; switching Off/Lottes is immediate. Paused gameplay stays frozen. State loads and viewport changes refresh the filtered image. Lottes has no temporal history.

The comparison submenu is inside Scanlines, preserving five top-level entries. It contains Off and CRT-Lottes. Older Guest/Royale preferences migrate to Lottes; new games default to Lottes. Each choice persists per ROM.

`assets/shaders/SOURCES.json` pins the upstream revision and dependency hashes. `scripts/fetch-crt-shaders.py` refetches that pinned dependency closure. Original sources, notices and licenses are bundled at packaging time. The builder embeds these assets and writes them into Resources/shaders before signing. The runtime embeds the generated WGSL. Regenerate it with `cargo run --manifest-path tools/compile-shader/Cargo.toml --locked` after any source update, and rerun the explicit GPU equivalence test: `cargo test -p romforge-runtime matches_upstream_lottes -- --ignored --nocapture`. The reference librashader engine is a development-only dependency; it is not linked into shipped apps. The separate generator has its own locked dependency graph. Release builds use size optimization (`opt-level = "z"`), full LTO, one codegen unit, and symbol stripping.

GPU checks can select a preset without changing saved preferences:

```sh
target/release/romforge-runtime --smoke-frames 120 --windowed --shader lottes \
  vendor/snes9x/libretro/snes9x_libretro.dylib \
  'ROMs/Legend of Zelda, The - A Link to the Past.sfc'
```

Use `off` or `lottes`. Smoke mode counts actual presented frames only after the selected preset becomes active and has a 90-second shader timeout. Its finite rendering check continues if the test window loses focus; ordinary gameplay still pauses on focus loss.

## Releases

For v009 and later, update VERSION and CHANGELOG.md, commit, then push the matching
version tag. The Release workflow checks Windows and macOS, builds optimized native
downloads, tests packaged diagnostic games, audits archives, and publishes both ZIPs,
the corresponding emulator source archive, and SHA256SUMS.txt only when both jobs pass.
The Windows ZIP contains the builder and its dependencies; generated games are not
release inputs. `scripts/package-windows-release.ps1` reproduces Windows packaging
locally. `scripts/audit-release.py` rejects ROMs, game executables, saves and local
workspace/preferences files in distribution archives.

Set the root `VERSION` file to the next integer tag (`v001`, `v002`, …). Run `zsh scripts/package-release.sh` on Apple Silicon with Xcode (including Icon Composer tools). This builds optimized binaries, removes the development workspace marker, collects third-party notices, verifies local signatures, and creates a ZIP plus SHA256SUMS.txt under artifacts/releases. Upload both to the matching public GitHub Release; the updater expects those exact asset names. Never include ROMs, covers, saves, or generated game apps.

The builder's numeric bundle version follows VERSION. Developer ID signing and notarization are not configured. The first release is locally tested; a future-version replacement should also be verified on a disposable app copy before each release.


### Size-optimized macOS core

The core build script uses the pinned upstream source with `-Oz`, full LTO,
limited public exports, dead-code removal, and local-symbol stripping. It keeps
all emulation chips and features. SDL camera, native dialog, tray, and SDL GPU
subsystems are disabled in the runtime; wgpu supplies GPU rendering. SDL window,
audio, software rendering (menu), joystick, HID, haptics, and sensors remain.

Use `cargo run --release -p romforge-core --example compare -- CORE ROM FRAMES
STATE_OUT STATE_IN` for frame/audio fingerprints, save-state replay, and optional
cross-build state loading. Each invocation loads one trusted core. The probe
compares all video and audio samples, not just frame counts; its elapsed time
includes hashing overhead and is not an isolated CPU benchmark.

Validation on Apple Silicon: 717 catalog ROMs compared over 120 boot frames.
716 passed matching video/audio, state replay, and original-to-new state loading.
The Blues Brothers had identical initial output but the same replay mismatch in
both original and optimized builds; this is a pre-existing limitation, not a
clean save-state pass. Longer representative boot sequences also matched.
This is regression sampling, not full gameplay coverage of every title.

Seven representative 1,800-frame runs (30 seconds of emulated time) matched
video/audio and original-build states: Super Mario World, Super Mario Kart,
Star Fox, Super Mario RPG, Mega Man X2, Super Metroid, and Donkey Kong Country.
The size-focused build took more CPU time in this probe (about 1.2–2.5 seconds
including hashing for those sequences on the test Mac). Full-game performance
on older Macs remains unverified. Do not describe this as a speed optimization.

## Isolated UI and NES checks

Set `ROMFORGE_APP` to a separate absolute `.app` path when running the build script to avoid replacing an active builder. Set `ROMFORGE_PREFERENCES` to a separate JSON file when launching the GUI for isolated testing.

```sh
python3 scripts/make-nes-test-rom.py
cargo run -p romforge-core --example check_nes -- vendor/nestopia/libretro/nestopia_libretro.dylib 'artifacts/nes-tests/NES Diagnostic.nes'
cargo run -p romforge-core --example check_states -- vendor/nestopia/libretro/nestopia_libretro.dylib 'artifacts/nes-tests/NES Diagnostic.nes'
```

The NES diagnostic is original NROM code. It verifies A/B, Start/Select, all D-pad directions, button release and battery-memory reload. It is not a substitute for a mapper/game compatibility suite.
