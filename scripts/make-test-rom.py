#!/usr/bin/env python3
"""Generate original 65816 test code, never a commercial game ROM.
The program disables interrupts and loops forever; the PPU stays blank.
This tests core initialization and callbacks, not gameplay compatibility.
"""
from pathlib import Path
import sys
rom = bytearray(32768)
rom[:3] = bytes([0x78, 0x80, 0xfe])  # SEI; BRA -2
rom[0x7fc0:0x7fd5] = b'ROMFORGE TEST'.ljust(21, b' ')
rom[0x7fd5:0x7fdc] = bytes([0x20, 0, 5, 0, 1, 0, 0])
for offset in (0x7fea, 0x7fee, 0x7ffa, 0x7ffc, 0x7ffe):
    rom[offset:offset + 2] = bytes([0, 0x80])
checksum = (sum(rom) + 510) & 0xffff
rom[0x7fdc:0x7fde] = (checksum ^ 0xffff).to_bytes(2, 'little')
rom[0x7fde:0x7fe0] = checksum.to_bytes(2, 'little')
output = Path(sys.argv[1] if len(sys.argv) > 1 else 'artifacts/test.sfc')
output.parent.mkdir(parents=True, exist_ok=True)
output.write_bytes(rom)
print(output)
