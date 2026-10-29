#!/usr/bin/env python3
"""Archive pinned emulator sources and integration recipes, excluding binary test fixtures."""
import io
import subprocess
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VERSION = (ROOT / 'VERSION').read_text().strip()
REVISIONS = {'nestopia': '60a325237ce3df3a42f8f58cec5ebf0d6be412eb',
             'snes9x': 'fae2fea08f74180759ef540ee94259213f503480'}
EXCLUDED = {'.nes', '.sfc', '.smc', '.gb', '.gbc', '.gba', '.n64', '.z64', '.nds',
            '.bin', '.rom', '.iso', '.gcm', '.wad', '.cia', '.nsp', '.xci', '.fds',
            '.png', '.jpg', '.jpeg', '.bmp', '.ico', '.exe', '.dll', '.dylib', '.o', '.obj'}
OUTPUT = Path(sys.argv[1])
OUTPUT.mkdir(parents=True, exist_ok=True)
PREFIX = f'ROMForge-{VERSION}-emulator-sources'
archive = OUTPUT / f'{PREFIX}.tar.gz'
if archive.exists():
    raise SystemExit(f'Archive already exists: {archive}')
with tarfile.open(archive, 'w:gz') as destination:
    for core, revision in REVISIONS.items():
        data = subprocess.check_output(['git', '-C', str(ROOT / 'vendor' / core), 'archive', revision])
        with tarfile.open(fileobj=io.BytesIO(data)) as source:
            for member in source:
                if not member.isfile() or Path(member.name).suffix.lower() in EXCLUDED:
                    continue
                contents = source.extractfile(member).read()
                item = tarfile.TarInfo(f'{PREFIX}/vendor/{core}/{member.name}')
                item.size = len(contents)
                destination.addfile(item, io.BytesIO(contents))
    recipes = ['scripts/build-nestopia-macos.sh', 'scripts/build-snes9x-macos.py',
               'scripts/snes9x-exports-macos.txt', 'scripts/windows-cores/CMakeLists.txt',
               'scripts/windows-cores/snes9x-linkage.h']
    for recipe in recipes:
        destination.add(ROOT / recipe, arcname=f'{PREFIX}/{recipe}')
    readme = (f'ROMForge {VERSION} emulator sources\n' +
              '\n'.join(f'{core}: https://github.com/libretro/{core} revision {rev}' for core, rev in REVISIONS.items()) +
              '\n\nCode and licenses match the pinned upstream revisions. Binary test ROM/image fixtures are excluded.\n'
              'Windows x64: from this folder, run cmake -S scripts/windows-cores -B build -A x64, then '
              'cmake --build build --config Release. Requires Visual Studio C++ Build Tools and CMake.\n'
              'macOS: use upstream Git checkouts at the revisions above with the included scripts; '
              'the Snes9x script verifies Git revision and cleanliness.\n'
              'Full ROMForge source/build instructions: https://github.com/bazsec/ROMForge\n').encode()
    item = tarfile.TarInfo(f'{PREFIX}/README.txt')
    item.size = len(readme)
    destination.addfile(item, io.BytesIO(readme))
print(archive)
