param(
    [switch]$Silent = $false,
    [switch]$PurgeData = $false
)

$ErrorActionPreference = "Stop"
Get-Process -Name "ui-qa-ui","ui-qa-cli","ui-qa-probe" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\ui-qa-tester"
foreach ($key in @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ui-qa-tester",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\UiQaTester"
)) {
    if (Test-Path $key) {
        Remove-Item -Path $key -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$StartMenuDir = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs\UI QA Tester"
if (Test-Path $StartMenuDir) {
    Remove-Item -Path $StartMenuDir -Recurse -Force -ErrorAction SilentlyContinue
}

$DesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "UI QA Tester.lnk"
if (Test-Path $DesktopShortcut) {
    Remove-Item -Path $DesktopShortcut -Force -ErrorAction SilentlyContinue
}

if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
}
if ($PurgeData) {
    $DataDir = Join-Path $env:LOCALAPPDATA "ui-qa-tester"
    if (Test-Path $DataDir) {
        Remove-Item -Path $DataDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if (-not $Silent) {
    Write-Host "Uninstalled ui-qa-tester cleanly."
}
