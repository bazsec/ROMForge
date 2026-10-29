$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/scripts/build-windows-app.ps1"
if ($LASTEXITCODE -ne 0) { throw 'ROMForge build failed' }
Start-Process -FilePath "$PSScriptRoot/artifacts/ROMForge-Windows/romforge-gui.exe"
