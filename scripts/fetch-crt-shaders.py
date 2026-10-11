#!/usr/bin/env python3
"""Fetch the exact upstream dependency closure for ROMForge's CRT-Lottes preset."""
import hashlib, json, posixpath, re, subprocess, urllib.request
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent / 'assets/shaders'
# Initial import pins HEAD in SOURCES.json; subsequent runs reuse that revision.
manifest = ROOT / 'SOURCES.json'
if manifest.exists():
    revision = json.loads(manifest.read_text())['revision']
else:
    revision = subprocess.check_output(['git', 'ls-remote', 'https://github.com/libretro/slang-shaders.git', 'HEAD'], text=True).split()[0]
BASE = f'https://raw.githubusercontent.com/libretro/slang-shaders/{revision}/'
files = {}
def fetch(path):
    path = posixpath.normpath(path)
    if path in files: return
    assert not path.startswith(('../', '/')), path
    destination = ROOT/path
    print(path, flush=True)
    data = urllib.request.urlopen(BASE+path, timeout=30).read()
    files[path] = hashlib.sha256(data).hexdigest()
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    if path.endswith(('.slang', '.slangp', '.h', '.inc')):
        source = data.decode('utf-8-sig')
        references = re.findall(r'^\s*#\s*(?:include|reference)\s+"([^"]+)"', source, re.M)
        if path.endswith('.slangp'):
            for line in source.splitlines():
                if '=' not in line or line.lstrip().startswith('#'): continue
                value = line.split('=',1)[1].split('#',1)[0].strip().strip('"')
                if value.endswith(('.slang','.slangp','.png','.jpg','.tga','.bmp')): references.append(value)
        for ref in references: fetch(posixpath.join(posixpath.dirname(path),ref))
for preset in ['crt/crt-lottes.slangp']:
    fetch(preset)
manifest.write_text(json.dumps({'repository':'https://github.com/libretro/slang-shaders','revision':revision,'files':files},indent=2)+'\n')
print(f'Fetched {len(files)} files ({sum((ROOT/p).stat().st_size for p in files)} bytes), revision {revision}')
