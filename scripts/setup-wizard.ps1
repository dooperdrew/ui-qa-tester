param(
    [switch]$Silent = $false,
    [switch]$NoGui = $false
)

$ErrorActionPreference = "Stop"
$InstallScript = Join-Path $PSScriptRoot "install.ps1"
if (-not (Test-Path $InstallScript)) {
    $InstallScript = Join-Path $PSScriptRoot "scripts\install.ps1"
}

& $InstallScript -Silent:$Silent
if (-not $Silent -and -not $NoGui) {
    Write-Host "Setup Wizard completed: UI QA Tester is installed in %LOCALAPPDATA%\Programs\ui-qa-tester."
}
