param(
    [string]$Version = "",
    [switch]$SkipBuild = $false
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:PATH"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

if ([string]::IsNullOrWhiteSpace($Version)) {
    $CargoToml = Get-Content -Path (Join-Path $RepoRoot "Cargo.toml") -Raw
    if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        throw "Could not parse workspace version from Cargo.toml"
    }
}

if (-not $SkipBuild) {
    Write-Host "Building release workspace v$Version..."
    cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
}

$DistDir = Join-Path $RepoRoot "dist"
$StageDir = Join-Path $DistDir "ui-qa-tester-v$Version-windows-x64"
$ZipPath = Join-Path $DistDir "ui-qa-tester-v$Version-windows-x64.zip"

if (Test-Path $StageDir) { Remove-Item -Path $StageDir -Recurse -Force }
if (Test-Path $ZipPath) { Remove-Item -Path $ZipPath -Force }
New-Item -ItemType Directory -Path $StageDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "locales") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "scripts") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "docs") -Force | Out-Null

Copy-Item (Join-Path $RepoRoot "target\release\ui-qa-ui.exe") $StageDir
Copy-Item (Join-Path $RepoRoot "target\release\ui-qa-cli.exe") $StageDir
Copy-Item (Join-Path $RepoRoot "target\release\ui-qa-probe.exe") $StageDir
Copy-Item (Join-Path $RepoRoot "locales\*.json") (Join-Path $StageDir "locales")
Copy-Item (Join-Path $RepoRoot "README.md") $StageDir
Copy-Item (Join-Path $RepoRoot "LICENSE") $StageDir
Copy-Item (Join-Path $RepoRoot "docs\user-guide.md") $StageDir
Copy-Item (Join-Path $RepoRoot "docs\user-guide.md") (Join-Path $StageDir "docs\user-guide.md")
Copy-Item (Join-Path $RepoRoot "setup.cmd") $StageDir
Copy-Item (Join-Path $RepoRoot "scripts\setup-wizard.ps1") $StageDir
Copy-Item (Join-Path $RepoRoot "scripts\install.ps1") $StageDir
Copy-Item (Join-Path $RepoRoot "scripts\uninstall.ps1") $StageDir
Copy-Item (Join-Path $RepoRoot "scripts\*.ps1") (Join-Path $StageDir "scripts")

Compress-Archive -Path "$StageDir\*" -DestinationPath $ZipPath -Force
Write-Host "Packaged portable release archive: $ZipPath"
