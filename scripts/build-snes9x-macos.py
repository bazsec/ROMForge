#!/usr/bin/env python3
"""Build the pinned Snes9x core with size optimization and all emulation features."""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'vendor/snes9x'
REVISION = 'fae2fea08f74180759ef540ee94259213f503480'

def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)

def main():
    if sys.platform != 'darwin':
        raise SystemExit('This script builds the macOS core; run it on macOS.')
    revision = subprocess.check_output(['git', '-C', str(SOURCE), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != REVISION:
        raise SystemExit(f'Expected Snes9x revision {REVISION}, found {revision}')
    run('git', '-C', str(SOURCE), 'diff', '--exit-code', 'HEAD', '--')
    with tempfile.TemporaryDirectory(prefix='romforge-core-') as temp:
        checkout = Path(temp) / 'source'
        shutil.copytree(SOURCE, checkout, ignore=shutil.ignore_patterns('.git', '*.o', '*.dylib'))
        makefile = checkout / 'libretro/Makefile'
        original = makefile.read_text()
        if original.count(' -O3 -DNDEBUG') != 2:
            raise SystemExit('Unexpected optimization flags in upstream Makefile')
        makefile.write_text(original.replace(' -O3 -DNDEBUG', ' -Oz -DNDEBUG'))
        exports = checkout / 'libretro/romforge.exports'
        shutil.copyfile(ROOT / 'scripts/snes9x-exports-macos.txt', exports)
        run('make', '-C', str(checkout / 'libretro'), '-j8', 'platform=osx', 'LTO=-flto',
            'SHARED=-dynamiclib -Wl,-dead_strip -Wl,-exported_symbols_list,romforge.exports')
        core = checkout / 'libretro/snes9x_libretro.dylib'
        run('strip', '-x', str(core))
        run('codesign', '--force', '--sign', '-', str(core))
        destination = SOURCE / 'libretro/snes9x_libretro.dylib'
        pending = destination.with_suffix('.pending')
        shutil.copy2(core, pending)
        os.replace(pending, destination)
        print(f'Built {destination}: {destination.stat().st_size:,} bytes')

if __name__ == '__main__':
    main()
