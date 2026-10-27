param(
    [string]$Version = "",
    [string]$ZipPath = ""
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$RepoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($Version)) {
    $CargoToml = Get-Content -Path (Join-Path $RepoRoot "Cargo.toml") -Raw
    if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1].Trim()
    } else {
        throw "Could not parse workspace version from Cargo.toml"
    }
}

if ([string]::IsNullOrWhiteSpace($ZipPath)) {
    $ZipPath = Join-Path $RepoRoot "dist\ui-qa-tester-v$Version-windows-x64.zip"
}

if (-not (Test-Path $ZipPath)) {
    throw "Release archive not found at $ZipPath"
}

$Sha256Upper = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToUpperInvariant()
$Sha256Lower = $Sha256Upper.ToLowerInvariant()
$ReleaseUrl = "https://github.com/DooperDrew/ui-qa-tester/releases/download/v$Version/ui-qa-tester-v$Version-windows-x64.zip"

$WingetDir = Join-Path $RepoRoot "packaging\winget\manifests\d\DooperDrew\UiQaTester\$Version"
New-Item -ItemType Directory -Path $WingetDir -Force | Out-Null

$VersionYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.1.6.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
"@

$LocaleYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.1.6.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
PackageLocale: en-US
Publisher: DooperDrew
PublisherUrl: https://github.com/DooperDrew
PackageName: UI QA Tester
PackageUrl: https://github.com/DooperDrew/ui-qa-tester
License: MIT
LicenseUrl: https://github.com/DooperDrew/ui-qa-tester/blob/main/LICENSE
ShortDescription: Autonomous, non-intrusive Windows UI and Hardware-in-the-Loop (HIL) QA Agent for Rust desktop apps.
Tags:
  - qa
  - testing
  - windows
  - ui-automation
  - rust
  - egui
  - iced
  - slint
ManifestType: defaultLocale
ManifestVersion: 1.6.0
"@

$InstallerYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.1.6.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: ui-qa-cli.exe
    PortableCommandAlias: ui-qa-cli
  - RelativeFilePath: ui-qa-ui.exe
    PortableCommandAlias: ui-qa-ui
  - RelativeFilePath: ui-qa-probe.exe
    PortableCommandAlias: ui-qa-probe
Installers:
  - Architecture: x64
    InstallerUrl: $ReleaseUrl
    InstallerSha256: $Sha256Upper
ManifestType: installer
ManifestVersion: 1.6.0
"@

[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.yaml"), $VersionYaml, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.locale.en-US.yaml"), $LocaleYaml, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.installer.yaml"), $InstallerYaml, [System.Text.UTF8Encoding]::new($false))

$ScoopJson = @"
{
  "version": "$Version",
  "description": "Autonomous, non-intrusive Windows UI and Hardware-in-the-Loop (HIL) QA Agent for Rust desktop apps.",
  "homepage": "https://github.com/DooperDrew/ui-qa-tester",
  "license": "MIT",
  "architecture": {
    "64bit": {
      "url": "$ReleaseUrl",
      "hash": "$Sha256Lower"
    }
  },
  "bin": [
    "ui-qa-cli.exe",
    "ui-qa-ui.exe",
    "ui-qa-probe.exe"
  ],
  "shortcuts": [
    [
      "ui-qa-ui.exe",
      "UI QA Tester"
    ]
  ],
  "checkver": "github",
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/DooperDrew/ui-qa-tester/releases/download/v`$version/ui-qa-tester-v`$version-windows-x64.zip"
      }
    }
  }
}
"@

$ScoopPath = Join-Path $RepoRoot "packaging\scoop\ui-qa-tester.json"
[System.IO.File]::WriteAllText($ScoopPath, $ScoopJson, [System.Text.UTF8Encoding]::new($false))
Write-Host "Updated WinGet ($WingetDir) and Scoop ($ScoopPath) manifests for v$Version (SHA256: $Sha256Upper)"
