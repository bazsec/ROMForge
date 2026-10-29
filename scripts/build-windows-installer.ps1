param(
    [Parameter(Mandatory=$true)][string]$PayloadDir,
    [string]$Version = '',
    [string]$OutputDir = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
if (!$Version) { $Version = (Get-Content VERSION -Raw).Trim() }
if ($Version -notmatch '^v\d{3,}$') { throw 'Invalid installer version' }
$PayloadDir = (Resolve-Path -LiteralPath $PayloadDir).Path
if (!$OutputDir) { $OutputDir = Join-Path $root 'artifacts/releases' }
New-Item -ItemType Directory -Force $OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
foreach ($file in @('romforge-gui.exe','romforge-builder.exe','romforge-runtime.exe',
    'Contents/Frameworks/snes9x_libretro.dll','Contents/Frameworks/nestopia_libretro.dll')) {
    if (!(Test-Path -LiteralPath (Join-Path $PayloadDir $file) -PathType Leaf)) { throw "Missing installer input: $file" }
}
& python scripts/audit-release.py --directory $PayloadDir
if ($LASTEXITCODE -ne 0) { throw 'Installer payload audit failed' }
$compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if ($compiler) { $iscc = $compiler.Source } else {
    $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe") |
        Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (!$iscc) { throw 'Install Inno Setup 6.3 or newer to compile the Windows installer.' }
$iconDir = Join-Path $root ('artifacts/installer-icon-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $iconDir | Out-Null
$icon = Join-Path $iconDir 'ROMForge.ico'
# Modern ICO files accept PNG images directly; retain the original 256px branding.
@'
import struct, sys
from pathlib import Path
png=Path('assets/branding/romforge-macos-icon.png').read_bytes()
assert png.startswith(b'\x89PNG\r\n\x1a\n')
w,h=struct.unpack('>II',png[16:24])
assert 1 <= w <= 256 and 1 <= h <= 256
header=struct.pack('<HHH',0,1,1)+struct.pack('<BBBBHHII',w%256,h%256,0,0,1,32,len(png),22)
Path(sys.argv[1]).write_bytes(header+png)
'@ | python - $icon
if ($LASTEXITCODE -ne 0) { throw 'Installer icon creation failed' }
$number = [int]$Version.Substring(1)
& $iscc "/DAppVersion=$Version" "/DFileVersion=0.0.$number.0" "/DPayloadDir=$PayloadDir" "/DReleaseDir=$OutputDir" "/DIconFile=$icon" scripts/windows-installer/ROMForge.iss
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
Write-Host "Installer: $OutputDir/ROMForge-$Version-Windows-x64-Setup.exe"
