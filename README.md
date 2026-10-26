# UI QA Tester (`ui-qa-tester`)

**Autonomous, Non-Intrusive Windows UI & Hardware-in-the-Loop (HIL) QA Agent for Rust Native GUI Applications (`iced`, `egui`, `Slint`)**

[![CI](https://github.com/DooperDrew/ui-qa-tester/actions/workflows/ci.yml/badge.svg)](https://github.com/DooperDrew/ui-qa-tester/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/DooperDrew/ui-qa-tester)](https://github.com/DooperDrew/ui-qa-tester/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`ui-qa-tester` launches or attaches to your Windows desktop application, explores screens and executes test goals **hands-free in the background without stealing your physical mouse cursor or keyboard focus**, correlates every UI interaction with real-time hardware logs (`stdout`, `stderr`, diagnostic log files), critiques visual layout and UX using deterministic geometry checks + multimodal AI (**Google Gemini API** or **local OpenAI-compatible Vision models**), and exports **agent-ready Markdown/JSON reports** that AI coding agents (Antigravity, Claude Code, GitHub Copilot, Cursor) can immediately use to fix bugs and improve your UI.

---

## ✨ Key Capabilities

1. **Non-Intrusive Background Capture (`PrintWindow(PW_RENDERFULLCONTENT)` + `WGC`):** Captures full-resolution screenshots of GPU-rendered `wgpu`/DirectX windows (`iced`, `egui`, `Slint`) even when covered by your IDE (`~16.5 ms` p50 latency, zero focus stealing).
2. **Three-Tier Hands-Free Actuation:**
   - **Tier 1:** Windows UI Automation (`IUIAutomationInvokePattern` / `AccessKit` nodes).
   - **Tier 2:** Direct `HWND` message injection (`PostMessageW` `WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_CHAR`) with zero physical cursor movement.
   - **Tier 3:** Optional lightweight `ui-qa-sdk` crate for white-box widget inspection and synthetic input.
3. **Real-Time UI ↔ Hardware Log Correlation:** Brackets every UI action with a `[-50 ms, +2000 ms]` log window across `stdout`, `stderr`, and diagnostic log files to catch silent hardware timeouts, `NACK`s, serial disconnects, panics, and UI thread hangs (`IsHungAppWindow`).
4. **Dual-Layer Visual & UX Critic:** Combines fast local checks (overlapping controls, clipped bounds, missing AccessKit labels, WCAG luminance contrast) with **Google Gemini Multimodal API** or **Local OpenAI-compatible VLMs** (Ollama, LM Studio, vLLM).
5. **Live QA Dashboard (`ui-qa-ui.exe`) & Headless CLI (`ui-qa-cli.exe`):** Watch live annotated captures, inspect synchronized hardware logs, compare visual baselines, and copy ready-to-paste AI Coding Agent Fix Prompts.

---

## 📥 Download & Installation

Pre-built Windows x64 binaries are published on [**GitHub Releases**](https://github.com/DooperDrew/ui-qa-tester/releases/latest) — no Rust toolchain required:

### 1. Guided Setup Wizard (Recommended)
Download [`ui-qa-tester-v0.1.0-windows-x64.zip`](https://github.com/DooperDrew/ui-qa-tester/releases/latest), extract it, and double-click **`setup.cmd`** (or run `.\setup-wizard.ps1`). Installs into `%LOCALAPPDATA%\Programs\ui-qa-tester` with zero administrator elevation required.

### 2. Portable ZIP Archive
Extract [`ui-qa-tester-v0.1.0-windows-x64.zip`](https://github.com/DooperDrew/ui-qa-tester/releases/latest) anywhere and run `ui-qa-ui.exe` or `ui-qa-cli.exe` directly.

### 3. Scoop
```powershell
scoop install packaging/scoop/ui-qa-tester.json
```

### 4. Microsoft WinGet
```powershell
winget install --manifest packaging/winget/manifests/d/DooperDrew/UiQaTester/0.1.0
```

---

## 🚀 Quick Usage

### Launch the Live QA Dashboard UI
```powershell
ui-qa-ui.exe
```
Click **"▶ Run Hands-Free Self-Test Demo"** in the top bar to watch `ui-qa-tester` spawn a background Hardware Controller window, capture and annotate its controls, actuate buttons without moving your cursor, correlate hardware logs, and generate an agent-ready report.

### Run the Headless CLI
```powershell
# Run built-in end-to-end self-test
ui-qa-cli.exe self-test

# Generate a starter qa-session.toml config
ui-qa-cli.exe init-config --path qa-session.toml

# Run autonomous background QA against your Rust GUI .exe or window title
ui-qa-cli.exe run --exe "C:\path\to\your_app.exe" --output ./qa-output

# List all discoverable desktop windows (HWND, PID, title, size)
ui-qa-cli.exe windows
```

---

## 📚 Documentation
- [User Guide (`docs/user-guide.md`)](docs/user-guide.md)
- [Vision (`docs/01-vision.md`)](docs/01-vision.md)
- [Requirements & NFR Budgets (`docs/02-requirements.md`)](docs/02-requirements.md)
- [Architecture (`docs/03-architecture.md`)](docs/03-architecture.md)
- [UX Specification (`docs/04-ux.md`)](docs/04-ux.md)
- [Roadmap (`docs/05-roadmap.md`)](docs/05-roadmap.md)
- [QA & Empirical Validation (`docs/06-qa-and-validation.md`)](docs/06-qa-and-validation.md)
- [Milestone 0 Spike Findings (`docs/m0-spike-findings.md`)](docs/m0-spike-findings.md)
