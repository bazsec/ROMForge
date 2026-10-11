#!/usr/bin/env python3
"""Original NROM diagnostic: changing background + joypad value in battery RAM.
No commercial game data. A/B/Select/Start/D-pad are sampled every frame.
"""
from pathlib import Path
import sys
prg=bytearray([0xea]*16384)
code=bytearray(); labels={}; fixups=[]
def emit(*data): code.extend(data)
def label(name): labels[name]=0x8000+len(code)
def branch(op,name): emit(op,0); fixups.append((len(code)-1,name,'relative'))
def jump(name): emit(0x4c,0,0); fixups.append((len(code)-2,name,'absolute'))
emit(0x78,0xd8,0xa2,0xff,0x9a,0xe8,0x8e,0x00,0x20,0x8e,0x01,0x20)
label('wait'); emit(0x2c,0x02,0x20); branch(0x10,'wait')
emit(0xa9,0x3f,0x8d,0x06,0x20,0xa9,0x00,0x8d,0x06,0x20,0xa9,0x21,0x8d,0x07,0x20)
label('loop'); emit(0x2c,0x02,0x20); branch(0x10,'loop')
emit(0xa9,0x01,0x8d,0x16,0x40,0xa9,0,0x8d,0x16,0x40,0xa2,8)
label('read');emit(0xad,0x16,0x40,0x4a,0x2e,0x00,0x60,0xca);branch(0xd0,'read')
# Frame counter in SRAM also demonstrates persisted battery data.
emit(0xee,0x01,0x60);jump('loop')
for pos,name,kind in fixups:
 target=labels[name]
 if kind=='relative': code[pos]=(target-(0x8000+pos+1))&255
 else: code[pos:pos+2]=target.to_bytes(2,'little')
prg[:len(code)]=code
for offset in (0x3ffa,0x3ffc,0x3ffe): prg[offset:offset+2]=(0x8000).to_bytes(2,'little')
header=b'NES\x1a'+bytes([1,1,2,0])+bytes(8)
out=Path(sys.argv[1] if len(sys.argv)>1 else 'artifacts/nes-tests/NES Diagnostic.nes')
out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(header+prg+bytes(8192))
print(out)
