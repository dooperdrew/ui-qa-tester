# UI QA Tester — User Guide

## 1. Overview
`ui-qa-tester` is an autonomous Windows UI and Hardware-in-the-Loop (HIL) QA agent designed for developers building Windows desktop applications (especially Rust native GUIs: `iced`, `egui`, and `Slint`) that communicate with connected hardware.

It operates **hands-free in the background**:
- Captures the target window surface via `PrintWindow(PW_RENDERFULLCONTENT)` even when the window is behind your IDE or on a secondary monitor.
- Inspects the Windows UI Automation (`IUIAutomation` / `AccessKit`) tree and dispatches clicks/keys via UIA patterns, `PostMessageW`, or the optional `ui-qa-sdk` IPC bridge—never hijacking your physical mouse cursor or active window focus.
- Correlates every UI interaction with `stdout`, `stderr`, and diagnostic log files to catch silent hardware timeouts, `NACK`s, panics, and UI hangs.
- Critiques visual layout and UX using fast deterministic geometry checks + multimodal AI (Google Gemini API or local OpenAI-compatible Vision models).

---

## 2. Quick Start & Installation

### Channel 1: Guided Setup Wizard (`setup.cmd`)
1. Download `ui-qa-tester-v0.1.0-windows-x64.zip` from [GitHub Releases](https://github.com/DooperDrew/ui-qa-tester/releases/latest).
2. Extract the archive and run `setup.cmd` (or `powershell -ExecutionPolicy Bypass -File .\setup-wizard.ps1`).
3. Binaries are installed into `%LOCALAPPDATA%\Programs\ui-qa-tester` without requiring administrator privileges.

### Channel 2: Portable ZIP
Extract `ui-qa-tester-v0.1.0-windows-x64.zip` into any directory and run `ui-qa-ui.exe` or `ui-qa-cli.exe` directly.

### Channel 3: Scoop
```powershell
scoop install packaging/scoop/ui-qa-tester.json
```

### Channel 4: Microsoft WinGet
```powershell
winget install --manifest packaging/winget/manifests/d/DooperDrew/UiQaTester/0.1.0
```

---

## 3. Using the Live QA Dashboard (`ui-qa-ui.exe`)
1. Run `ui-qa-ui.exe`. The dashboard opens in a native app window (`http://127.0.0.1:17580`).
2. **Try the Self-Test Demo first:** Click **"▶ Run Hands-Free Self-Test Demo"** in the top-right bar.
3. **Test your own application:**
   - In **1. Session & Target Setup**, either select a running window from the dropdown (click **↻ Refresh** to list all visible desktop windows) or enter the path to your `.exe` binary.
   - Add any hardware diagnostic log file paths to tail alongside `stdout`/`stderr`.
   - Choose your AI Vision Critic backend:
     - **Google Gemini Multimodal API:** Set `$env:GEMINI_API_KEY = "your-key"` before launching.
     - **Local OpenAI-Compatible VLM:** Point the endpoint URL to Ollama (`http://localhost:11434/v1`) or LM Studio (`http://localhost:1234/v1`).
     - **Deterministic Local Heuristics Only:** Runs 100% offline with zero LLM calls.
   - Click **🚀 Start Hands-Free QA Session**.
4. **Inspect & Fix with AI Coding Agents:**
   - Switch to **3. Findings & AI UX Critique** or **4. Baseline Visual Diff & Export** and click **📋 Copy Agent Fix Prompt** to copy the complete Markdown report and paste it into Antigravity, Claude Code, Copilot, or Cursor.

---

## 4. Optional White-Box Rust SDK (`ui-qa-sdk`)
For `iced`, `egui`, and `Slint` applications you build yourself, you can optionally wire `ui-qa-sdk` (`crates/sdk`) by implementing the `QaAppAdapter` trait and starting `QaSdkServer::start("127.0.0.1:0", adapter)` to expose internal widget bounds/values and accept synthetic clicks directly in memory.

---

## 5. Known Limitations
- **Do not minimize the target window to the taskbar:** Windows Desktop Window Manager (DWM) stops rendering surfaces for minimized (`SW_MINIMIZE`) windows. Keep the target window in a normal/restored state (`SW_SHOWNOACTIVATE`)—it can be completely covered behind your IDE or on another monitor, and `ui-qa-tester` will automatically un-minimize it with `SW_SHOWNOACTIVATE` if needed.

---

## Uninstallation
Run the included uninstaller script:
```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\ui-qa-tester\uninstall.ps1" -PurgeData
```
Or run `.\scripts\uninstall.ps1 -PurgeData` from the repository root.
