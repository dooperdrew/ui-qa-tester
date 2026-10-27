<#
.SYNOPSIS
    Guided Windows Setup Wizard for ui-qa-tester.

.DESCRIPTION
    Provides a zero-elevation graphical (Windows Forms) and interactive wizard to install
    ui-qa-tester into %LOCALAPPDATA%\Programs\ui-qa-tester, configure Start Menu and Desktop
    shortcuts, add to User PATH, register Windows "Installed apps" entry,
    and optionally launch the Live QA Dashboard UI upon completion.

.PARAMETER TargetDir
    Installation directory. Defaults to "$env:LOCALAPPDATA\Programs\ui-qa-tester".

.PARAMETER NoDesktopShortcut
    Disables creating the desktop shortcut.

.PARAMETER NoStartMenuShortcut
    Disables creating Start Menu shortcuts.

.PARAMETER NoPath
    Disables adding TargetDir to User PATH.

.PARAMETER NoLaunch
    Prevents launching ui-qa-ui.exe upon completion.

.PARAMETER Silent
    Runs installation unattended with no GUI or prompts.

.PARAMETER NoGui
    Forces interactive console wizard instead of Windows Forms GUI.
#>

[CmdletBinding()]
param(
    [string]$TargetDir = "$env:LOCALAPPDATA\Programs\ui-qa-tester",
    [switch]$NoDesktopShortcut,
    [switch]$NoStartMenuShortcut,
    [switch]$NoPath,
    [switch]$NoLaunch,
    [switch]$Silent,
    [switch]$NoGui
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $ScriptDir) { $ScriptDir = (Get-Location).Path }

$CandidateSources = @(
    $ScriptDir,
    (Join-Path $ScriptDir "..\target\release"),
    (Join-Path $ScriptDir "target\release")
)

$SourceDir = $null
foreach ($cand in $CandidateSources) {
    if (Test-Path (Join-Path $cand "ui-qa-ui.exe")) {
        $SourceDir = (Resolve-Path $cand).Path
        break
    }
}

if (-not $SourceDir) {
    throw "ui-qa-ui.exe was not found in $ScriptDir or target\release. Please build the workspace or run from an extracted release package."
}

$Version = "0.2.0"
$CargoToml = Join-Path $ScriptDir "..\Cargo.toml"
if (Test-Path $CargoToml) {
    $raw = Get-Content $CargoToml -Raw
    if ($raw -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1].Trim()
    }
}

function Invoke-UiQaInstall {
    param(
        [string]$DestDir,
        [bool]$CreateDesktop,
        [bool]$CreateStartMenu,
        [bool]$AddPath,
        [bool]$LaunchDashboard
    )

    Get-Process -Name "ui-qa-ui","ui-qa-cli","ui-qa-probe" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $DestDir "locales") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $DestDir "examples") -Force | Out-Null

    foreach ($bin in @("ui-qa-ui.exe", "ui-qa-cli.exe", "ui-qa-probe.exe")) {
        $srcBin = Join-Path $SourceDir $bin
        if (Test-Path $srcBin) {
            Copy-Item $srcBin $DestDir -Force
        }
    }

    foreach ($locCand in @((Join-Path $ScriptDir "locales"), (Join-Path $ScriptDir "..\locales"))) {
        if (Test-Path $locCand) {
            Copy-Item (Join-Path $locCand "*.json") (Join-Path $DestDir "locales") -Force -ErrorAction SilentlyContinue
            break
        }
    }

    foreach ($exCand in @((Join-Path $ScriptDir "examples"), (Join-Path $ScriptDir "..\examples"))) {
        if (Test-Path $exCand) {
            Copy-Item (Join-Path $exCand "*.toml") (Join-Path $DestDir "examples") -Force -ErrorAction SilentlyContinue
            break
        }
    }

    $uninstSrc = Join-Path $ScriptDir "uninstall.ps1"
    if (Test-Path $uninstSrc) {
        Copy-Item $uninstSrc $DestDir -Force
    }

    if ($AddPath) {
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($null -eq $UserPath) { $UserPath = "" }
        if ($UserPath -notlike "*$DestDir*") {
            $NewPath = if ([string]::IsNullOrWhiteSpace($UserPath)) { $DestDir } else { "$UserPath;$DestDir" }
            [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        }
    }

    $WshShell = New-Object -ComObject WScript.Shell
    if ($CreateStartMenu) {
        $StartMenuDir = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs\UI QA Tester"
        New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
        $Shortcut = $WshShell.CreateShortcut((Join-Path $StartMenuDir "UI QA Tester.lnk"))
        $Shortcut.TargetPath = Join-Path $DestDir "ui-qa-ui.exe"
        $Shortcut.WorkingDirectory = $DestDir
        $Shortcut.Description = "Autonomous Non-Intrusive Windows UI & Hardware QA Agent"
        $Shortcut.Save()
    }

    if ($CreateDesktop) {
        $DesktopDir = [Environment]::GetFolderPath("Desktop")
        $Shortcut = $WshShell.CreateShortcut((Join-Path $DesktopDir "UI QA Tester.lnk"))
        $Shortcut.TargetPath = Join-Path $DestDir "ui-qa-ui.exe"
        $Shortcut.WorkingDirectory = $DestDir
        $Shortcut.Description = "Autonomous Non-Intrusive Windows UI & Hardware QA Agent"
        $Shortcut.Save()
    }

    # Register in Windows Settings -> Installed apps (HKCU, zero elevation)
    $UninstRegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\UiQaTester"
    New-Item -Path $UninstRegPath -Force | Out-Null
    Set-ItemProperty -Path $UninstRegPath -Name "DisplayName" -Value "UI QA Tester (Autonomous Windows UI & HIL QA Agent)"
    Set-ItemProperty -Path $UninstRegPath -Name "DisplayVersion" -Value $Version
    Set-ItemProperty -Path $UninstRegPath -Name "Publisher" -Value "DooperDrew"
    Set-ItemProperty -Path $UninstRegPath -Name "InstallLocation" -Value $DestDir
    Set-ItemProperty -Path $UninstRegPath -Name "DisplayIcon" -Value (Join-Path $DestDir "ui-qa-ui.exe")
    $uninstCmd = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$DestDir\uninstall.ps1`""
    Set-ItemProperty -Path $UninstRegPath -Name "UninstallString" -Value $uninstCmd
    Set-ItemProperty -Path $UninstRegPath -Name "QuietUninstallString" -Value "$uninstCmd -Silent"
    Set-ItemProperty -Path $UninstRegPath -Name "NoModify" -Value 1 -Type DWord
    Set-ItemProperty -Path $UninstRegPath -Name "NoRepair" -Value 1 -Type DWord

    if ($LaunchDashboard) {
        Start-Process -FilePath (Join-Path $DestDir "ui-qa-ui.exe") -WorkingDirectory $DestDir
    }
}

$OptDesktop = -not $NoDesktopShortcut
$OptStartMenu = -not $NoStartMenuShortcut
$OptPath = -not $NoPath
$OptLaunch = -not $NoLaunch

if ($Silent -or $NoGui) {
    Invoke-UiQaInstall -DestDir $TargetDir -CreateDesktop $OptDesktop -CreateStartMenu $OptStartMenu -AddPath $OptPath -LaunchDashboard ($OptLaunch -and (-not $Silent))
    Write-Host "UI QA Tester v$Version installed to $TargetDir"
    exit 0
}

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Form = New-Object System.Windows.Forms.Form
$Form.Text = "UI QA Tester v$Version — Guided Setup Wizard"
$Form.Size = New-Object System.Drawing.Size(520, 360)
$Form.StartPosition = "CenterScreen"
$Form.FormBorderStyle = "FixedDialog"
$Form.MaximizeBox = $false

$TitleLbl = New-Object System.Windows.Forms.Label
$TitleLbl.Text = "Install UI QA Tester v$Version (Zero-Elevation User Space)"
$TitleLbl.Font = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Bold)
$TitleLbl.Location = New-Object System.Drawing.Point(20, 18)
$TitleLbl.Size = New-Object System.Drawing.Size(470, 26)
$Form.Controls.Add($TitleLbl)

$SubLbl = New-Object System.Windows.Forms.Label
$SubLbl.Text = "Autonomous, non-intrusive Windows UI & Hardware-in-the-Loop QA Agent."
$SubLbl.Location = New-Object System.Drawing.Point(20, 46)
$SubLbl.Size = New-Object System.Drawing.Size(470, 22)
$Form.Controls.Add($SubLbl)

$DirLbl = New-Object System.Windows.Forms.Label
$DirLbl.Text = "Installation Directory:"
$DirLbl.Location = New-Object System.Drawing.Point(20, 80)
$DirLbl.Size = New-Object System.Drawing.Size(200, 20)
$Form.Controls.Add($DirLbl)

$DirBox = New-Object System.Windows.Forms.TextBox
$DirBox.Text = $TargetDir
$DirBox.Location = New-Object System.Drawing.Point(20, 102)
$DirBox.Size = New-Object System.Drawing.Size(460, 24)
$Form.Controls.Add($DirBox)

$ChkPath = New-Object System.Windows.Forms.CheckBox
$ChkPath.Text = "Add ui-qa-cli, ui-qa-ui, and ui-qa-probe to User PATH"
$ChkPath.Checked = $OptPath
$ChkPath.Location = New-Object System.Drawing.Point(20, 140)
$ChkPath.Size = New-Object System.Drawing.Size(450, 24)
$Form.Controls.Add($ChkPath)

$ChkStart = New-Object System.Windows.Forms.CheckBox
$ChkStart.Text = "Create Start Menu Shortcut"
$ChkStart.Checked = $OptStartMenu
$ChkStart.Location = New-Object System.Drawing.Point(20, 168)
$ChkStart.Size = New-Object System.Drawing.Size(450, 24)
$Form.Controls.Add($ChkStart)

$ChkDesktop = New-Object System.Windows.Forms.CheckBox
$ChkDesktop.Text = "Create Desktop Shortcut"
$ChkDesktop.Checked = $OptDesktop
$ChkDesktop.Location = New-Object System.Drawing.Point(20, 196)
$ChkDesktop.Size = New-Object System.Drawing.Size(450, 24)
$Form.Controls.Add($ChkDesktop)

$ChkLaunch = New-Object System.Windows.Forms.CheckBox
$ChkLaunch.Text = "Launch Live QA Dashboard UI when setup finishes"
$ChkLaunch.Checked = $OptLaunch
$ChkLaunch.Location = New-Object System.Drawing.Point(20, 224)
$ChkLaunch.Size = New-Object System.Drawing.Size(450, 24)
$Form.Controls.Add($ChkLaunch)

$BtnInstall = New-Object System.Windows.Forms.Button
$BtnInstall.Text = "Install Now"
$BtnInstall.Location = New-Object System.Drawing.Point(280, 272)
$BtnInstall.Size = New-Object System.Drawing.Size(100, 32)
$BtnInstall.Add_Click({
    Invoke-UiQaInstall -DestDir $DirBox.Text -CreateDesktop $ChkDesktop.Checked -CreateStartMenu $ChkStart.Checked -AddPath $ChkPath.Checked -LaunchDashboard $ChkLaunch.Checked
    [System.Windows.Forms.MessageBox]::Show("UI QA Tester v$Version has been installed to $($DirBox.Text).", "Setup Complete", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
    $Form.Close()
})
$Form.Controls.Add($BtnInstall)

$BtnCancel = New-Object System.Windows.Forms.Button
$BtnCancel.Text = "Cancel"
$BtnCancel.Location = New-Object System.Drawing.Point(390, 272)
$BtnCancel.Size = New-Object System.Drawing.Size(90, 32)
$BtnCancel.Add_Click({ $Form.Close() })
$Form.Controls.Add($BtnCancel)

[void]$Form.ShowDialog()
