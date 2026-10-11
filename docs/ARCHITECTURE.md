# Architecture and technical feasibility

Research date: October 11, 2026. This document records the initial feasibility research. The implementation has since advanced to a playable Apple Silicon SNES runtime and macOS game packager; see DEVELOPMENT.md and VALIDATION.md for current behavior. Statements below describing the initial host or future SDL/macOS work are historical design context.

## Core candidates and licensing

| System | Candidate | License considerations | Decision |
|---|---|---|---|
| SNES | [Snes9x Libretro](https://github.com/libretro/snes9x/blob/master/LICENSE) | Custom noncommercial license; the Libretro port expressly restricts commercial rights; component exceptions need review | Development POC only; no assumption of commercial permission |
| SNES | [bsnes Libretro](https://github.com/libretro/bsnes/blob/master/LICENSE.txt) | GPL-3.0-only plus component notices | Candidate for a GPL-compatible production distribution; benchmark CPU use and verify memory/static integration |
| NES | [Nestopia](https://github.com/libretro/nestopia) | GPL; audit exact revision and bundled components before distribution | Compatibility-focused candidate |
| GB/GBC | [SameBoy](https://github.com/LIJI32/SameBoy/blob/master/LICENSE) | Expat/MIT-style license with component exceptions | Preferred candidate; verify Libretro revision and boot-ROM behavior |
| GBA | [mGBA](https://github.com/mgba-emu/mgba/blob/master/LICENSE) | MPL-2.0, with dependency notices and file-level source obligations | Preferred candidate; test Libretro build and BIOS-free behavior |
| Genesis | [Genesis Plus GX](https://github.com/libretro/Genesis-Plus-GX/blob/master/LICENSE.txt) | Noncommercial restrictions; modified distributions require source | Compatibility baseline, unsuitable for assumed commercial use |
| Genesis | [PicoDrive](https://github.com/libretro/picodrive) | Multiple component licenses; inspect source inventory | Alternative to investigate, not automatically cleared |

The release process must pin commits, compiler versions, options, and dependency hashes. Supply license texts, notices, corresponding source and build scripts as required by the actual binary composition. Do not assume dynamic linking removes GPL obligations. The project has not adopted an overall license yet. Never bundle user ROMs in source releases or CI fixtures. Firmware-dependent cartridges and sidecar content need explicit capability validation rather than silently downloading firmware.

## Runtime boundaries

Builder GUI → packaging service → target/core template → shared runtime → Libretro core.

`format` is shared by packaging and runtime. `core` owns the unsafe ABI and lifecycle. The runtime will own SDL, frame timing, persistence, and the menu. The builder will never initialize an emulator during bulk packaging.

The implemented core host is synchronous and allows one session per process. It keeps ROM memory and the native library alive through unload/deinit, rejects `need_fullpath`, validates API version, and supplies software video, stereo audio, and neutral input callbacks. Unknown environment capabilities return false. Callbacks currently count output and discard pixels/samples. It is not a complete general-purpose Libretro frontend: option values, directories, logging, hardware rendering, AV changes, serialization, SRAM, and input routing remain work items. Native libraries are trusted code, not sandboxed plugins.

[Libretro documents memory versus path-based loading](https://docs.libretro.com/development/cores/developing-cores/). A core needing a full path cannot meet the no-ROM-extraction contract through this host. Static library support must be built and tested independently for each core, compiler, and target. The chosen development Snes9x revision has a static-build path in its Makefile, but that does not establish a working Windows/MSVC integration.

## SDL3

Use SDL3 for windows, software-frame upload, aspect-preserving letterboxing, audio streams, events, and gamepads. Rust [sdl3 0.20.0](https://docs.rs/sdl3/0.20.0/sdl3/) is the evaluated high-level binding. Its source/static build features require a native toolchain and CMake; CMake was absent on this host during initial setup. SDL is deliberately not required by the headless workspace yet. [SDL's CMake guide](https://github.com/libsdl-org/SDL/blob/main/docs/README-cmake.md) describes static builds.

Do not pace the game with a fixed 60 Hz assumption: use `retro_get_system_av_info`, support PAL, bound audio buffering, resample to the device rate, and avoid callback allocation. Copy video row-by-row using core-provided pitch and negotiated RGB format. Preserve a duplicate frame when a core sends null data only if that capability is advertised. Use reported aspect ratio, including hi-res/interlace transitions. Keep native system-library dependencies distinct from separately installed runtimes.

## Embedding and self-containment

Current prototype: executable prefix, metadata JSON, ROM bytes, then a 56-byte footer containing two little-endian lengths, SHA-256, and `ROMFRG01`. Limits are 64 KiB metadata and 64 MiB ROM. Validate lengths before allocation and integrity before loading. SHA-256 detects corruption; it is not authentication. The native core is still a separate file.

Windows release proposal: a core-specific prelinked PE runtime with SDL and core statically linked, static CRT where supported, ROM appended as an overlay or inserted into a reserved resource. Overlay packing avoids compilation per game and is validated here only on a macOS development executable. Verify PE loader behavior, antivirus handling, CRT/C++ dependencies, and signature tooling on Windows. Inject before Authenticode signing; a signature appended after the ROM footer requires a reader aware of the PE certificate table or a different payload location. The current EOF reader does not support signed artifacts. Do not call an embedded-and-extracted DLL solution genuinely dependency-free.

macOS release proposal: `.app/Contents/MacOS/Game` plus ROM and metadata in `Contents/Resources`, and license/source information in resources. Static linking is preferred; bundled signed dylibs under Frameworks remain a self-contained app bundle, although not a single binary. Do not append data to a signed Mach-O. Finalize resources, sign nested code, sign the bundle, then notarize and staple. [Apple describes distribution notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution). Every game bundle is a new signed artifact; a signed template cannot simply retain its original signature after changing contents. Developer ID credentials and network notarization are release inputs, not something an offline builder can invent.

## Templates, batch building, and platform strategy

Future template manifests identify payload ABI, core ID/commit/license, OS, architecture, binary hash, extension list, memory-loading support, firmware requirements, and source archive provenance. Reject platform/core mismatches before packaging. The current development CLI does not validate target executable formats and must not be used as a production cross-platform packager.

Build Windows x64 on Windows runners; build arm64 and x86_64 macOS on macOS runners and combine verified architectures when a universal bundle is needed. Cross-compiling packaging metadata is easy; cross-compiling Rust/C/C++/SDL with the correct SDK, deployment target and signing is a separate problem. A builder can package any already-available validated template, regardless of its host OS. Windows execution requires a Windows test machine; native Apple Silicon execution does not establish Intel compatibility.

Incremental keys must include ROM content hash, template/core hashes, payload ABI, metadata and packaging options. Verify output integrity before declaring a hit. Stream ROMs sequentially or with bounded workers instead of retaining thousands in memory. Create per-job temporary output, atomically commit complete artifacts, report per-ROM failures, and use deterministic sanitized names plus hash suffixes to prevent collisions. Current CLI handles a single ROM, refuses existing output and removes partial files on failure; batch, cache and progress events are planned.

## Shared game experience

F1 pauses core execution, freezes/darkens the last frame and opens exactly: Keybinds; Save State; Load State; Scanlines (On/Off); Fullscreen / Windowed. F1/Escape closes it. Slots and binding capture are subviews, not extra top-level options. Default scanlines off, fullscreen on. Implement the menu once in the runtime, not in cores. Store settings and SRAM under OS application-data directories keyed by ROM hash. Save states also identify core revision and reject incompatible data; use atomic writes and bounds. No extra shader settings or emulator branding.
