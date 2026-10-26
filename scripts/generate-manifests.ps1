param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$RepoRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($Version)) {
    $CargoToml = Get-Content -Path (Join-Path $RepoRoot "Cargo.toml") -Raw
    if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        throw "Could not parse workspace version from Cargo.toml"
    }
}

$ZipPath = Join-Path $RepoRoot "dist\ui-qa-tester-v$Version-windows-x64.zip"
if (-not (Test-Path $ZipPath)) {
    throw "Release ZIP not found at $ZipPath. Run package-release.ps1 first."
}

$Sha256 = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
$DownloadUrl = "https://github.com/DooperDrew/ui-qa-tester/releases/download/v$Version/ui-qa-tester-v$Version-windows-x64.zip"

# 1. Scoop Manifest
$ScoopDir = Join-Path $RepoRoot "packaging\scoop"
New-Item -ItemType Directory -Path $ScoopDir -Force | Out-Null
$ScoopJson = @{
    version = $Version
    description = "Autonomous Windows UI & Hardware-in-the-Loop QA Agent for Rust Native GUI Apps (iced, egui, Slint)"
    homepage = "https://github.com/DooperDrew/ui-qa-tester"
    license = "MIT"
    architecture = @{
        "64bit" = @{
            url = $DownloadUrl
            hash = $Sha256
        }
    }
    bin = @("ui-qa-ui.exe", "ui-qa-cli.exe", "ui-qa-probe.exe")
} | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText((Join-Path $ScoopDir "ui-qa-tester.json"), $ScoopJson, [System.Text.UTF8Encoding]::new($false))

# 2. WinGet Manifests (1.9.0)
$WingetDir = Join-Path $RepoRoot "packaging\winget\manifests\d\DooperDrew\UiQaTester\$Version"
New-Item -ItemType Directory -Path $WingetDir -Force | Out-Null

$VersionYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.1.9.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.9.0
"@

$LocaleYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
PackageLocale: en-US
Publisher: DooperDrew
PackageName: UI QA Tester
PackageUrl: https://github.com/DooperDrew/ui-qa-tester
License: MIT
ShortDescription: Autonomous Windows UI and Hardware-in-the-Loop QA Agent for Rust Native GUI Apps
ManifestType: defaultLocale
ManifestVersion: 1.9.0
"@

$InstallerYaml = @"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.1.9.0.schema.json
PackageIdentifier: DooperDrew.UiQaTester
PackageVersion: $Version
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: ui-qa-ui.exe
    PortableCommandAlias: ui-qa-ui
  - RelativeFilePath: ui-qa-cli.exe
    PortableCommandAlias: ui-qa-cli
  - RelativeFilePath: ui-qa-probe.exe
    PortableCommandAlias: ui-qa-probe
Installers:
  - Architecture: x64
    InstallerUrl: $DownloadUrl
    InstallerSha256: $($Sha256.ToUpperInvariant())
ManifestType: installer
ManifestVersion: 1.9.0
"@

[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.yaml"), $VersionYaml, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.locale.en-US.yaml"), $LocaleYaml, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText((Join-Path $WingetDir "DooperDrew.UiQaTester.installer.yaml"), $InstallerYaml, [System.Text.UTF8Encoding]::new($false))

Write-Host "Generated Scoop and WinGet manifests for v$Version (SHA256: $Sha256)"
