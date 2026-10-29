param(
    [Parameter(Mandatory=$true)][string]$Installer,
    [Parameter(Mandatory=$true)][string]$PayloadDir,
    [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Installer smoke tests run only on an isolated GitHub runner, never on a developer PC.' }
Set-Location (Split-Path $PSScriptRoot -Parent)
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$PayloadDir = (Resolve-Path -LiteralPath $PayloadDir).Path
$testRoot = Join-Path $env:RUNNER_TEMP ('romforge-installer-test-' + [guid]::NewGuid().ToString('N'))
$install = Join-Path $testRoot 'Installed App'
$workspace = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::MyDocuments, [Environment+SpecialFolderOption]::DoNotVerify)) 'ROMForge'
$preferences = Join-Path $env:APPDATA 'ROMForge/builder.json'
$save = Join-Path $workspace 'Save Games/Installer Test/battery.srm'
$game = Join-Path $workspace 'artifacts/Installer Diagnostic.exe'
New-Item -ItemType Directory -Force $testRoot, (Split-Path $preferences), (Split-Path $save), (Split-Path $game) | Out-Null
[IO.File]::WriteAllText($preferences, '{"input":"keep-my-library"}')
[IO.File]::WriteAllText($save, 'preserve test save')
$preferencesHash = (Get-FileHash -LiteralPath $preferences).Hash
$saveHash = (Get-FileHash -LiteralPath $save).Hash
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{8C3A371C-F376-422F-9066-0BC47725C55F}_is1'
$startShortcut = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::Programs, [Environment+SpecialFolderOption]::DoNotVerify)) 'ROMForge.lnk'
$desktopShortcut = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::DesktopDirectory, [Environment+SpecialFolderOption]::DoNotVerify)) 'ROMForge.lnk'
function Install-App([string]$LogName) {
    $process = Start-Process -FilePath $Installer -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',"/DIR=`"$install`"",'/TASKS=desktopicon',"/LOG=`"$testRoot/$LogName`"") -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "Installer failed ($($process.ExitCode))" }
}
Install-App 'install.log'
foreach ($file in Get-ChildItem -LiteralPath $PayloadDir -Recurse -File) {
    $relative = [IO.Path]::GetRelativePath($PayloadDir, $file.FullName)
    $installed = Join-Path $install $relative
    if (!(Test-Path -LiteralPath $installed) -or (Get-FileHash -LiteralPath $installed).Hash -ne (Get-FileHash -LiteralPath $file.FullName).Hash) { throw "Installed payload differs from portable app: $relative" }
}
if ((Get-ItemProperty -LiteralPath $registry).DisplayVersion -ne $Version) { throw 'Installed Apps version missing or wrong' }
$shell = New-Object -ComObject WScript.Shell
foreach ($shortcut in @($startShortcut,$desktopShortcut)) {
    if (!(Test-Path -LiteralPath $shortcut)) { throw "Shortcut missing: $shortcut" }
    $link = $shell.CreateShortcut($shortcut)
    if ($link.TargetPath -ne (Join-Path $install 'romforge-gui.exe')) { throw 'Shortcut target is wrong' }
    if (!$link.IconLocation.StartsWith((Join-Path $install 'ROMForge.ico'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Shortcut icon is wrong' }
}
& python scripts/make-test-rom.py "$testRoot/Diagnostic.sfc"
if ($LASTEXITCODE -ne 0) { throw 'Diagnostic ROM generation failed' }
& "$install/romforge-builder.exe" --native "$install/romforge-runtime.exe" "$install/Contents/Frameworks/snes9x_libretro.dll" "$install/Contents/Resources/Snes9x-LICENSE.txt" "$testRoot/Diagnostic.sfc" $game
if ($LASTEXITCODE -ne 0) { throw 'Installed builder failed to make a game' }
$gameHash = (Get-FileHash -LiteralPath $game).Hash
Install-App 'upgrade.log'
if ((Get-FileHash -LiteralPath $game).Hash -ne $gameHash) { throw 'Upgrade modified the game' }
[IO.File]::WriteAllText((Join-Path $install 'user-added.txt'), 'preserve untracked user file')
$uninstaller = Join-Path $install 'unins000.exe'
$process = Start-Process -FilePath $uninstaller -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw 'Uninstall failed' }
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $startShortcut) -or (Test-Path -LiteralPath $desktopShortcut)) { throw 'Uninstall left registration or shortcuts behind' }
if (Test-Path -LiteralPath (Join-Path $install 'romforge-gui.exe')) { throw 'Uninstall left the app executable behind' }
if (!(Test-Path -LiteralPath (Join-Path $install 'user-added.txt'))) { throw 'Uninstall removed an untracked user file' }
if ((Get-FileHash -LiteralPath $preferences).Hash -ne $preferencesHash -or (Get-FileHash -LiteralPath $save).Hash -ne $saveHash -or (Get-FileHash -LiteralPath $game).Hash -ne $gameHash) { throw 'Install/upgrade/uninstall changed user data' }
$process = Start-Process -FilePath $game -ArgumentList '--headless' -Wait -PassThru -WindowStyle Hidden -RedirectStandardOutput "$testRoot/game.log" -RedirectStandardError "$testRoot/game-errors.log"
if ($process.ExitCode -ne 0) { throw 'Standalone game failed after ROMForge was uninstalled' }
Write-Host 'Installer checks passed: portable payload identity, shortcuts, upgrade, registration, uninstall, preserved data, independent game launch.'
