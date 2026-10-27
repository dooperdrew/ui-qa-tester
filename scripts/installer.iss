; Inno Setup Script for UI QA Tester (Zero-Elevation User-Space Installer)
#define MyAppName "UI QA Tester"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "DooperDrew"
#define MyAppURL "https://github.com/DooperDrew/ui-qa-tester"
#define MyAppExeName "ui-qa-ui.exe"

[Setup]
AppId={{8E9B7D2F-5A14-4C93-9B2C-6F1A4E8D3B10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\ui-qa-tester
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=ui-qa-tester-v{#MyAppVersion}-setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\ui-qa-ui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\ui-qa-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\ui-qa-probe.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\locales\*.json"; DestDir: "{app}\locales"; Flags: ignoreversion
Source: "..\examples\*.toml"; DestDir: "{app}\examples"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
