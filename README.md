# ROMForge — v004

A Rust desktop builder that packages user-provided retro ROMs as game applications.

[Download the latest macOS build](https://github.com/bazsec/ROMForge/releases/latest).

The downloadable app runs on Apple Silicon Macs and includes the builder/runtime/core. No Rust, Xcode, or separate emulator installation is required to run it. It uses local ad-hoc signing and is not Apple-notarized; macOS may require Privacy & Security → Open Anyway after downloading. Intel Mac builds are not included.

Settings lives inside the main window. Folder choices and per-game cover overrides are saved in `~/Library/Application Support/ROMForge/builder.json`. Matching ignores common region/revision tags and punctuation, supports leading/trailing “The”, and uses the catalog's reviewed aliases. Multiple possible covers require selection. PNG, JPEG and ICNS covers can be previewed and overridden.

Settings → Check for updates reads public GitHub Releases. Install update and restart downloads the matching Apple Silicon ZIP, checks SHA-256 and bundle signing, stages beside the current app, then replaces it after exit. The previous app remains in the hidden staging folder for recovery. The app's containing folder must be writable. Folder preferences, ROMs, and saves are not replaced. Version tags increment `v001`, `v004`, `v004`.

**Current preview:** playable SNES apps on macOS Apple Silicon. Open `artifacts/ROMForge.app` and drop one `.sfc` or `.smc` ROM anywhere in its window to build automatically, or select a ROM and click **Build macOS app**. Drops use the current Game apps and Game icons folders, even for ROMs outside the library. Existing apps are preserved; choose another output folder to build a second copy. Wait for a running job before dropping another ROM. Open the output folder and double-click the generated game. `Launch ROMForge.command` rebuilds and opens the builder.

Each game app includes its ROM and Snes9x core. SDL3 is statically linked; wgpu/Metal presents video and the embedded CRT-Lottes shader. No separate emulator installation, ROM picker, or runtime extraction is needed. Game apps open fullscreen by default, retain their aspect ratio, play audio, support a keyboard or SDL-compatible gamepad, and persist battery saves. They use local ad-hoc signatures, not Developer ID notarization.

**Create all** first previews the entire batch, showing ready games, existing outputs, missing covers and ambiguous matches. Confirm to build ready games; Cancel stops after the current game. It builds every ROM in the library (including subfolders and games hidden by search), one at a time. It uses the selected output and icon folders, skips existing app names, continues after individual failures, and reports created/skipped/failed totals with per-game activity details. ROMs with the same filename stem share an output name; subsequent copies are skipped once that app exists.

Place PNG, JPEG or ICNS icons in your icons folder (subfolders are searched). Exact filenames work: `Game.sfc` matches `Icons/Game.png`. The GUI detects this folder automatically, lets you choose another icons folder, and displays the selected ROM's match. Icons are embedded in the generated app for Finder and the Dock. If no icon matches, packaging uses the default application icon.

| Action | Keyboard |
|---|---|
| Move | Arrow keys |
| SNES B / A | Z / X |
| SNES Y / X | A / S |
| SNES L / R | Q / W |
| Start / Select | Return / Right Shift |
| Fullscreen / windowed | F11 (Fn+F11 if macOS uses the function key) |
| Open / close paused menu | F1 (Fn+F1 on some Macs); Escape resumes |
| Close game | Command-Q or window close |

The paused F1 menu has exactly five options: **Keybinds**, **Save State**, **Load State**, **Scanlines**, and **Fullscreen / Windowed**. It uses SDL's built-in pixel font; no font files or font-renderer dependencies are added. Scanlines defaults to CRT-Lottes, fullscreen defaults on, and each game's settings persist. In **Scanlines**, choose **Off** or **CRT-Lottes**, then Enter/Circle to apply. Wait for `[Active]`, then F1/Escape to resume. CRT-Lottes is translated from the pinned upstream source during development and embedded in the runtime; no downloads or RetroArch installation are needed. Switching is immediate, and the menu stays sharp and unfiltered.

For a PS5 DualSense, Cross/Circle/Square/Triangle map to SNES B/A/Y/X; Options is Start, Create is Select, and L1/R1 are L/R. Use the D-pad or left stick to move. **Create + Options** toggles the menu. Within it, D-pad/left stick navigate, Circle (Nintendo A) selects, and Cross (Nintendo B) goes back. The game window must have focus; switching away pauses gameplay and audio. Controllers can connect after launch.

In **Keybinds**, Left/Right switches keyboard/controller bindings. Select a SNES control and press Enter/Circle, then press its new key/button or move a controller axis. Assigning a binding already in use swaps the two controls. F1, Escape, F11, and the controller menu shortcut remain reserved. Backspace goes back; F1/Escape closes the menu, including during capture. Five state slots are available per ROM. Saving replaces the chosen slot; loading checks the ROM, core version, size, and checksum before restoring. Slots persist outside the app alongside battery saves.

Windows/Linux packaging, other consoles, and incremental rebuilds remain planned. Current builds are for local noncommercial development using the Snes9x license. Bluetooth DualSense menu/input response was confirmed by the user; broader controller and game compatibility still need testing.

- [Architecture and feasibility](docs/ARCHITECTURE.md)
- [Implementation milestones](docs/PLAN.md)
- [Build and test instructions](docs/DEVELOPMENT.md)
- [Current validation](docs/VALIDATION.md)

| Crate | Responsibility |
|---|---|
| `romforge-gui` | Native desktop builder, folder selection, ROM list, background jobs |
| `romforge-format` | Bounded/versioned ROM payload and corruption checks |
| `romforge-core` | Libretro FFI, software frames, audio, input, save memory |
| `romforge-runtime` | SDL3 input/audio, wgpu CRT rendering, menu, saves, diagnostics |
| `romforge-builder` | Template packaging and locally signed macOS game bundles |

ROMs, native dependencies, saves, and generated game artifacts stay local. User ROMs are excluded from source releases and CI fixtures.

Packaged games store progress in a shared **Save Games** folder beside the `.app` collection. Each game gets a readable title folder with a short ROM fingerprint. It contains battery.srm, five state slots, and settings.json. Existing Library saves are copied on first launch without deleting originals or replacing portable saves. Move or back up the collection together with Save Games to keep progress. Apps must be in a writable folder; a read-only location reports an error instead of silently storing progress elsewhere.

Rectangular PNG artwork is fitted inside a transparent square during packaging; originals are never modified.
