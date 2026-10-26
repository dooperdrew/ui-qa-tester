param(
    [switch]$Silent = $false,
    [switch]$PurgeData = $false
)

$ErrorActionPreference = "Stop"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\ui-qa-tester"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ui-qa-tester"

if (Test-Path $UninstallKey) {
    Remove-Item -Path $UninstallKey -Recurse -Force
}
if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force
}
if ($PurgeData) {
    $DataDir = Join-Path $env:LOCALAPPDATA "ui-qa-tester"
    if (Test-Path $DataDir) {
        Remove-Item -Path $DataDir -Recurse -Force
    }
}

if (-not $Silent) {
    Write-Host "Uninstalled ui-qa-tester cleanly."
}
