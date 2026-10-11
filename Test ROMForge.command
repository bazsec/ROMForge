#!/bin/zsh
# Double-click in Finder to run the current headless SNES proof of concept.
set -eu
ROOT="${0:A:h}"
cd "$ROOT"
finish() {
  local result=$?
  if (( result != 0 )); then
    print '\nThe test failed. See the error above.'
  fi
  if [[ -t 0 && "${ROMFORGE_NO_PAUSE:-0}" != 1 ]]; then
    read '?Press Return to close this window. '
  fi
}
trap finish EXIT
print 'ROMForge — SNES development test'
print 'This runs 120 frames headlessly. There is no game window or GUI yet.\n'
export PATH="$ROOT/vendor/build-tools/bin:$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null; then
  print 'Rust is missing. Install Rust from https://rustup.rs and try again.'
  exit 1
fi
CORE="$ROOT/vendor/snes9x/libretro/snes9x_libretro.dylib"
if [[ ! -f "$CORE" ]]; then
  print 'The development core is missing.'
  print 'Follow docs/DEVELOPMENT.md to build the pinned Snes9x core, then retry.'
  exit 1
fi
roms=("$ROOT"/ROMs/**/*.(sfc|smc|SFC|SMC)(N))
if (( ${#roms} == 0 )); then
  print 'Put an uncompressed .sfc or .smc ROM in the ROMs folder and retry.'
  exit 1
fi
ROM="${roms[1]}"
if (( ${#roms} > 1 )); then
  print 'Choose a ROM:'
  select ROM in "${roms[@]}"; do
    [[ -n "$ROM" ]] && break
  done
fi
print "Testing: ${ROM:t}\n"
cargo run --locked -p romforge-runtime -- --headless "$CORE" "$ROM"
print '\nSuccess: the core loaded your ROM and produced video frames.'
print 'This test does not yet verify visible gameplay, sound playback, or controls.'
