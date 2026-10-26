# Release, Packaging & Multi-Channel Distribution Methodology

## 1. Semantic Versioning (SemVer)
- **MAJOR (`X.0.0`):** breaking changes (APIs, protocols, config format) or graduating to production `1.0.0`.
- **MINOR (`0.X.0`):** backward-compatible features, UX enhancements, installers, or new AI/hardware integrations.
- **PATCH (`0.X.Y`):** bug fixes, translations, timing tuning.

## 2. Single Source of Truth for the Version
Root `Cargo.toml` `[workspace.package] version`. All workspace members use `version.workspace = true`, and all packaging scripts (`scripts/package-release.ps1`, `scripts/generate-manifests.ps1`, `scripts/setup-wizard.ps1`) read the version dynamically from `Cargo.toml` and never hardcode it.

## 3. Multi-Channel Distribution Standard (Zero-Build User Experience)
Every release milestone MUST provide pre-built, zero-elevation distribution channels so end users never have to compile from source:
1. **Portable Release Archive (`.zip` on GitHub Releases):** Produced by `scripts/package-release.ps1`, containing compiled release binaries (`ui-qa-ui.exe`, `ui-qa-cli.exe`, `ui-qa-probe.exe`), localization files (`locales/`), documentation (`README.md`, `user-guide.md`, `LICENSE`), and setup scripts.
2. **Guided Setup Wizard (`setup.cmd` / `scripts/setup-wizard.ps1`):** Zero-elevation graphical installer (with `-Silent` and `-NoGui` modes) that installs into `%LOCALAPPDATA%\Programs\ui-qa-tester`, creates Start Menu and Desktop shortcuts, and registers in Windows **Installed apps** (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ui-qa-tester`).
3. **User-Space CLI Installer & Uninstaller (`scripts/install.ps1` & `scripts/uninstall.ps1`):** Supports `-Silent` and `-PurgeData` flags.
4. **Package Manager Manifests (`scripts/generate-manifests.ps1`):** Automatically calculates the `.zip` SHA256 hash and generates:
   - **Microsoft WinGet (`1.9.0`):** `packaging/winget/manifests/...`
   - **Scoop:** `packaging/scoop/ui-qa-tester.json`

> **Documentation Parity Rule:** Whenever packaging or installation capabilities are added or updated, `README.md` (`## 📥 Download & Installation`) and `docs/user-guide.md` (`## 2. Quick Start & Installation` + `## Uninstallation`) MUST be updated in the same commit to document all 4 installation channels (WinGet, Scoop, Guided Setup Wizard, Portable ZIP).

## 4. Official 5-Step Release Procedure (Lead Agent Only)
1. Bump the version in root `Cargo.toml` + write the `CHANGELOG.md` entry (+ update `README.md` and `docs/user-guide.md` links).
2. Run the full verification gate (formatting, strict linter, automated test suite, release build).
3. Package the portable `.zip` and generate WinGet/Scoop manifests:
   ```powershell
   .\scripts\package-release.ps1 -Version "X.Y.Z" -SkipBuild
   .\scripts\generate-manifests.ps1 -Version "X.Y.Z"
   ```
4. `git commit -m "release: vX.Y.Z"` → `git tag -a vX.Y.Z -m "Release vX.Y.Z"` → `git push origin main --tags`.
5. Publish to GitHub Releases via `gh release create vX.Y.Z "dist/ui-qa-tester-vX.Y.Z-windows-x64.zip" --title "..." --notes "..." --latest`. Build artifacts (`target/`, `dist/`) go to GitHub Releases, **never into git**.

## 5. Pre-Release Checklist
- [ ] Clean working tree (`git status`).
- [ ] NFR budgets measured for this version (see `docs/06-qa-and-validation.md`).
- [ ] Real-environment QA status listed in the release notes (`Pass` / `Pending`).
- [ ] No hardcoded UI strings; translations (`en`, `it`) complete.
- [ ] Portable `.zip` archive verified to contain executables, `locales/`, `setup.cmd`, `setup-wizard.ps1`, `install.ps1`, `uninstall.ps1`, and `user-guide.md`.

## 6. Hotfix
Ship a patch version through the same pipeline and flag the defective release on GitHub Releases.
