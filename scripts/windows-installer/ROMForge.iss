; Packaging-only installer: application payload is identical to the portable ZIP.
#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef PayloadDir
  #error PayloadDir is required
#endif

[Setup]
AppId={{8C3A371C-F376-422F-9066-0BC47725C55F}
AppName=ROMForge
AppVersion={#AppVersion}
AppVerName=ROMForge {#AppVersion}
AppPublisher=ROMForge
AppPublisherURL=https://github.com/bazsec/ROMForge
AppSupportURL=https://github.com/bazsec/ROMForge/issues
AppUpdatesURL=https://github.com/bazsec/ROMForge/releases/latest
VersionInfoVersion={#FileVersion}
DefaultDirName={localappdata}\Programs\ROMForge
DefaultGroupName=ROMForge
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#ReleaseDir}
OutputBaseFilename=ROMForge-{#AppVersion}-Windows-x64-Setup
SetupIconFile={#IconFile}
UninstallDisplayIcon={app}\ROMForge.ico
UninstallDisplayName=ROMForge
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#IconFile}"; DestDir: "{app}"; DestName: "ROMForge.ico"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\ROMForge"; Filename: "{app}\romforge-gui.exe"; WorkingDir: "{app}"; IconFilename: "{app}\ROMForge.ico"
Name: "{autodesktop}\ROMForge"; Filename: "{app}\romforge-gui.exe"; WorkingDir: "{app}"; IconFilename: "{app}\ROMForge.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\romforge-gui.exe"; Description: "Launch ROMForge"; Flags: nowait postinstall skipifsilent

; No wildcard cleanup or user-data deletion. Inno removes only installed files.
