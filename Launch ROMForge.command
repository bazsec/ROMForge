#!/bin/zsh
set -eu
ROOT="${0:A:h}"
finish() {
  local result=$?
  if (( result != 0 )); then
    print '\nROMForge could not launch. See the error above.'
    if [[ -t 0 ]]; then read '?Press Return to close. '; fi
  fi
}
trap finish EXIT
print 'Building ROMForge. The first build may take a few minutes.'
"$ROOT/scripts/build-macos-app.sh"
open "$ROOT/artifacts/ROMForge.app"
