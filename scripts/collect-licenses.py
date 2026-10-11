#!/usr/bin/env python3
"""Collect Cargo dependency notices into a distribution bundle."""
import json,subprocess,shutil,sys
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1']))
index=[]
for package in meta['packages']:
 if package['source'] is None:continue
 root=Path(package['manifest_path']).parent
 name=f"{package['name']}-{package['version']}"
 files=[p for p in root.iterdir() if p.is_file() and p.name.lower().startswith(('license','licence','copying','notice','copyright'))]
 if package.get('license_file'):
  p=root/package['license_file']
  if p.is_file() and p not in files:files.append(p)
 folder=out/name;folder.mkdir(exist_ok=True)
 for p in files:shutil.copy2(p,folder/p.name)
 index.append(f"{name}\nLicense: {package.get('license') or 'See included license file'}\nSource: {package.get('repository') or package['source']}\nNotices: {', '.join(p.name for p in files) or 'SPDX license expression above'}\n")
(out/'DEPENDENCIES.txt').write_text('\n'.join(index))
