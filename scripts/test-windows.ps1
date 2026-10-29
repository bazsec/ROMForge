param([switch]$SkipBuild, [switch]$Smoke)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (!$SkipBuild) { & "$PSScriptRoot/build-windows-app.ps1" }
function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed ($LASTEXITCODE)" }
}
$tests = Join-Path $PWD ('artifacts/Windows Test Games/' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $tests | Out-Null
Invoke-Checked python @('scripts/make-test-rom.py', "$tests/SNES.sfc")
Invoke-Checked python @('scripts/make-nes-test-rom.py', "$tests/NES.nes")
$app = Join-Path $PWD 'artifacts/ROMForge-Windows'
foreach ($spec in @(@('SNES','sfc','snes9x','Snes9x'), @('NES','nes','nestopia','Nestopia'))) {
    $name, $extension, $core, $license = $spec
    $package = "$tests/$name.exe"
    Invoke-Checked "$app/romforge-builder.exe" @('--native', "$app/romforge-runtime.exe",
        "$app/Contents/Frameworks/${core}_libretro.dll", "$app/Contents/Resources/$license-LICENSE.txt",
        "$tests/$name.$extension", $package)
    # Move only the executable into a new directory: no ROM, core or sidecars.
    $isolated = "$tests/Isolated Games"
    New-Item -ItemType Directory -Force $isolated | Out-Null
    $standalone = "$isolated/$name.exe"
    Move-Item -LiteralPath $package -Destination $standalone
    $package = $standalone
    if (!(Test-Path -LiteralPath $package -PathType Leaf)) { throw 'Expected one executable' }
    $process = Start-Process -FilePath $package -ArgumentList '--headless' -Wait -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput "$tests/$name-headless.log" -RedirectStandardError "$tests/$name-headless-errors.log"
    if ($process.ExitCode -ne 0) { throw "Packaged $name failed: $(Get-Content "$tests/$name-headless-errors.log" -Raw)" }
    $export = "$tests/$name notices"
    $process = Start-Process -FilePath $package -ArgumentList "--export-resources `"$export`"" -Wait -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput "$tests/$name-export.log" -RedirectStandardError "$tests/$name-export-errors.log"
    if ($process.ExitCode -ne 0 -or !(Test-Path "$export/shaders/SOURCES.json")) { throw "Embedded notices export failed for $name" }
    if ($Smoke) {
        $process = Start-Process -FilePath $package -ArgumentList '--smoke-frames 120 --windowed --shader lottes' -Wait -PassThru `
            -RedirectStandardOutput "$tests/$name-smoke.log" -RedirectStandardError "$tests/$name-smoke-errors.log"
        if ($process.ExitCode -ne 0) { throw "Rendered $name failed: $(Get-Content "$tests/$name-smoke-errors.log" -Raw)" }
    }
}
Invoke-Checked cargo @('run', '--locked', '-p', 'romforge-core', '--example', 'check_nes', '--',
    "$app/Contents/Frameworks/nestopia_libretro.dll", "$tests/NES.nes")
foreach ($spec in @(@('NES','nes','nestopia'), @('SNES','sfc','snes9x'))) {
    $name, $extension, $core = $spec
    Invoke-Checked cargo @('run', '--locked', '-p', 'romforge-core', '--example', 'check_states', '--',
        "$app/Contents/Frameworks/${core}_libretro.dll", "$tests/$name.$extension")
}
Write-Host "Windows packages passed. Diagnostic games and logs: $tests"
