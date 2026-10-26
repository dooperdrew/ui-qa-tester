param(
    [switch]$Silent = $false
)

$ErrorActionPreference = "Stop"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\ui-qa-tester"
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $InstallDir "locales") -Force | Out-Null

$SourceDir = $PSScriptRoot
if (-not (Test-Path (Join-Path $SourceDir "ui-qa-ui.exe"))) {
    $ParentDir = Split-Path -Parent $PSScriptRoot
    if (Test-Path (Join-Path $ParentDir "target\release\ui-qa-ui.exe")) {
        Copy-Item (Join-Path $ParentDir "target\release\ui-qa-ui.exe") $InstallDir -Force
        Copy-Item (Join-Path $ParentDir "target\release\ui-qa-cli.exe") $InstallDir -Force
        Copy-Item (Join-Path $ParentDir "target\release\ui-qa-probe.exe") $InstallDir -Force
        Copy-Item (Join-Path $ParentDir "locales\*.json") (Join-Path $InstallDir "locales") -Force
    }
} else {
    Copy-Item (Join-Path $SourceDir "ui-qa-ui.exe") $InstallDir -Force
    Copy-Item (Join-Path $SourceDir "ui-qa-cli.exe") $InstallDir -Force
    Copy-Item (Join-Path $SourceDir "ui-qa-probe.exe") $InstallDir -Force
    if (Test-Path (Join-Path $SourceDir "locales")) {
        Copy-Item (Join-Path $SourceDir "locales\*.json") (Join-Path $InstallDir "locales") -Force
    }
}

$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ui-qa-tester"
New-Item -Path $UninstallKey -Force | Out-Null
Set-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "UI QA Tester (Autonomous Windows & HIL QA Agent)"
Set-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $InstallDir
Set-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "DooperDrew"
Set-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -Type DWord

if (-not $Silent) {
    Write-Host "Installed ui-qa-tester to $InstallDir"
}
