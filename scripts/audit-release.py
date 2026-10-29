#!/usr/bin/env python3
"""Fail release publication on game payloads, local preferences, or unexpected executables."""
import sys
import tarfile
import zipfile
from pathlib import Path, PurePosixPath

FORBIDDEN = {'.nes', '.sfc', '.smc', '.gb', '.gbc', '.gba', '.nds', '.n64', '.z64',
             '.nsp', '.xci', '.bin', '.rom', '.iso', '.gcm', '.wad', '.cia', '.fds',
             '.romforge', '.srm', '.state', '.sav', '.jpg', '.jpeg'}
EXECUTABLES = {'romforge-gui.exe', 'romforge-builder.exe', 'romforge-runtime.exe'}
def check(name):
    path = PurePosixPath(name)
    if path.is_absolute() or '..' in path.parts:
        raise SystemExit(f'Invalid archive path: {name}')
    if path.suffix.lower() in FORBIDDEN or path.name.lower() in {'project-root', 'builder.json', 'settings.json'}:
        raise SystemExit(f'Non-distribution content in archive: {name}')
    if path.suffix.lower() == '.exe' and path.name not in EXECUTABLES:
        raise SystemExit(f'Unexpected executable in archive: {name}')
    if path.suffix.lower() == '.dll' and path.name not in {'snes9x_libretro.dll', 'nestopia_libretro.dll'}:
        raise SystemExit(f'Unexpected DLL in archive: {name}')

if len(sys.argv) == 3 and sys.argv[1] == '--directory':
    root = Path(sys.argv[2])
    for path in root.rglob('*'):
        if path.is_file():
            check(path.relative_to(root).as_posix())
    print('Audited installer payload')
    sys.exit(0)

files = list(Path(sys.argv[1]).glob('*.zip')) + list(Path(sys.argv[1]).glob('*.tar.gz'))
if not files:
    raise SystemExit('No release archives to audit')
for archive in files:
    if archive.suffix == '.zip':
        with zipfile.ZipFile(archive) as package:
            for name in package.namelist():
                check(name)
    else:
        with tarfile.open(archive) as package:
            for member in package:
                check(member.name)
    print(f'Audited {archive.name}')
