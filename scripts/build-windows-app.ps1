param([switch]$Release)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed ($LASTEXITCODE)" }
}
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (!(Get-Command cmake -ErrorAction SilentlyContinue) -and (Test-Path $vswhere)) {
    $installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    $cmakeBin = Join-Path $installation 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin'
    if (Test-Path $cmakeBin) { $env:PATH = "$cmakeBin;$env:PATH" }
}
if (!(Get-Command cmake -ErrorAction SilentlyContinue)) { throw 'Install CMake and Visual Studio C++ Build Tools (Desktop development with C++).' }
$revisions = @{ snes9x = 'fae2fea08f74180759ef540ee94259213f503480'; nestopia = '60a325237ce3df3a42f8f58cec5ebf0d6be412eb' }
foreach ($core in @('snes9x','nestopia')) {
    $source = "vendor/$core"
    if (!(Test-Path "$source/.git")) { Invoke-Checked git @('clone', "https://github.com/libretro/$core.git", $source) }
    $dirty = & git -C $source status --porcelain --untracked-files=no
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw "Cannot use $source with local source changes" }
    Invoke-Checked git @('-C', $source, 'checkout', $revisions[$core])
}
Invoke-Checked cmake @('-S', 'scripts/windows-cores', '-B', 'vendor/windows-cores', '-A', 'x64')
Invoke-Checked cmake @('--build', 'vendor/windows-cores', '--config', 'Release', '--parallel', '8')
foreach ($core in @('snes9x','nestopia')) {
    Copy-Item "vendor/windows-cores/Release/${core}_libretro.dll" "vendor/$core/libretro/${core}_libretro.dll"
}
$cargoArgs = @('build','--workspace','--locked')
$profile = 'debug'
if ($Release) { $cargoArgs += '--release'; $profile = 'release' }
Invoke-Checked cargo $cargoArgs
$app = 'artifacts/ROMForge-Windows'
New-Item -ItemType Directory -Force "$app/Contents/Frameworks", "$app/Contents/Resources" | Out-Null
foreach ($binary in @('romforge-gui','romforge-builder','romforge-runtime')) {
    Copy-Item "target/$profile/$binary.exe" "$app/$binary.exe"
}
foreach ($core in @('snes9x','nestopia')) {
    Copy-Item "vendor/$core/libretro/${core}_libretro.dll" "$app/Contents/Frameworks/"
}
Copy-Item third-party/*.txt "$app/Contents/Resources/"
Copy-Item -Recurse -Force catalog "$app/"
[IO.File]::WriteAllText((Join-Path $PWD "$app/Contents/Resources/project-root"), (Get-Location).Path)
Write-Host "Built $app/romforge-gui.exe"
