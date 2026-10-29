$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
& "$PSScriptRoot/build-windows-app.ps1" -Release
if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed' }
& "$PSScriptRoot/test-windows.ps1" -SkipBuild
if ($LASTEXITCODE -ne 0) { throw 'Windows package checks failed' }
$version = (Get-Content VERSION -Raw).Trim()
if ($version -notmatch '^v\d{3,}$') { throw 'Invalid release version' }
$staging = Join-Path $PWD ('artifacts/release-staging-' + [guid]::NewGuid().ToString('N'))
$app = Join-Path $staging 'ROMForge-Windows'
New-Item -ItemType Directory -Force "$app/Contents/Frameworks", "$app/Contents/Resources" | Out-Null
# Whitelist distribution inputs; never archive the development artifacts directory.
foreach ($binary in @('romforge-gui','romforge-builder','romforge-runtime')) {
    Copy-Item "target/release/$binary.exe" "$app/$binary.exe"
}
foreach ($core in @('snes9x','nestopia')) {
    Copy-Item "vendor/$core/libretro/${core}_libretro.dll" "$app/Contents/Frameworks/"
}
Copy-Item third-party/*.txt "$app/Contents/Resources/"
& python scripts/collect-licenses.py "$app/Contents/Resources/ThirdPartyLicenses"
if ($LASTEXITCODE -ne 0) { throw 'Dependency notice collection failed' }
Set-Content -Encoding utf8 "$app/READ ME.txt" @"
ROMForge $version for Windows x64
Extract this entire folder, then open romforge-gui.exe.
Build games from your own ROMs. Each game is one standalone .exe with its selected cover icon.
The game temporarily unpacks its embedded emulator core. Abandoned runtime folders are cleaned on the next launch.
Save Games beside the game collection contains battery saves, state slots and settings.
F1 opens the in-game menu. Escape exits and saves progress.
Windows builds are unsigned. Updates are installed manually.
Source, licenses and build instructions: https://github.com/bazsec/ROMForge
"@
New-Item -ItemType Directory -Force artifacts/releases | Out-Null
$archive = Join-Path $PWD "artifacts/releases/ROMForge-$version-Windows-x64.zip"
if (Test-Path -LiteralPath $archive) { throw "Archive already exists: $archive" }
Compress-Archive -LiteralPath $app -DestinationPath $archive -CompressionLevel Optimal
Write-Host "Release: $archive"
& "$PSScriptRoot/build-windows-installer.ps1" -PayloadDir $app -Version $version
if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
if ($env:GITHUB_ACTIONS -eq 'true') {
    & "$PSScriptRoot/test-windows-installer.ps1" -Installer (Join-Path $PWD "artifacts/releases/ROMForge-$version-Windows-x64-Setup.exe") -PayloadDir $app -Version $version
}
