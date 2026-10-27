# Release & Versioning Methodology

This document defines the release engineering, versioning, packaging, and GitHub governance procedures for **`ui-qa-tester`**.

---

## 1. Single Source of Version Truth

The canonical version string is defined in [`Cargo.toml`](../Cargo.toml) under `[workspace.package].version` (currently `0.2.0`) and inherited by all 7 workspace crates (`ui-qa-core`, `ui-qa-platform`, `ui-qa-ai`, `ui-qa-sdk`, `ui-qa-cli`, `ui-qa-ui`, `ui-qa-probe`).

---

## 2. Release Verification Gate (Mandatory Before Every Tag)

Only the **Lead Agent** (Sole Committer) may execute release packaging and tagging after all 4 verification commands pass with zero warnings:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:ProgramFiles\GitHub CLI;$env:LOCALAPPDATA\Programs\GitHub CLI;$env:PATH"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

---

## 3. Automated Packaging & Manifest Generation

Run the release packaging script from the repository root:

```powershell
.\scripts\package-release.ps1
.\scripts\generate-manifests.ps1
```

This produces:
- `dist/ui-qa-tester-v<version>-windows-x64.zip` containing:
  - `ui-qa-cli.exe`, `ui-qa-ui.exe`, `ui-qa-probe.exe`
  - `setup.cmd`, `scripts/setup-wizard.ps1`, `scripts/install.ps1`, `scripts/uninstall.ps1`, `scripts/installer.iss`
  - `locales/en.json`, `locales/it.json`
  - `examples/magicbridge-qa-session.toml`
  - `README.md`, `LICENSE`, `CHANGELOG.md`, `docs/`
- Updated SHA-256 hashes and version strings in:
  - `packaging/winget/manifests/d/DooperDrew/UiQaTester/<version>/*.yaml`
  - `packaging/scoop/ui-qa-tester.json`

---

## 4. Publishing to GitHub Releases

```powershell
git tag v0.2.0
git push origin main --tags
gh release create v0.2.0 "dist/ui-qa-tester-v0.2.0-windows-x64.zip" `
  --title "ui-qa-tester v0.2.0 — UX Onboarding Wizard, Interactive Self-Test Matrix & Architecture Explorer" `
  --notes-file dist/release-notes.md
```
