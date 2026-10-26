# 02 — Requirements

> Status: **APPROVED**. Priority: **M**ust / **S**hould / **C**ould.
> 🔬 = depends on a Milestone 0 spike; it isn't verified yet.

## Functional requirements

### F1 — Target Process Lifecycle, Log Tailing & Crash/Hang Watchdog (M)
- **F1.1** Launch a target Windows `.exe` with configurable CLI arguments, working directory, and environment variables, or attach to an already-running process by executable name, PID, or window title.
- **F1.2** Capture `stdout` and `stderr` streams in real time with millisecond-precision timestamps and tail one or more configurable diagnostic log files (UTF-8 / UTF-16LE, rotation-aware).
- **F1.3** Monitor target process health at configurable intervals (default `250 ms`): working-set RAM, CPU usage %, thread count, process exit code, and Win32 window responsiveness (`IsHungAppWindow` + `SendMessageTimeoutW` with `SMTO_ABORTIFHUNG`).
- **F1.4** Correlate log entries and health anomalies with the active UI action using a configurable time window (default `[-50 ms, +2000 ms]` around each interaction) and flag `PANIC`, `ERROR`, `WARN`, stack traces, or custom regex patterns (e.g. hardware timeout / NACK) as linked defects.

### F2 — Non-Intrusive Window Capture & Visual Baseline Diffing 🔬 (M)
- **F2.1** 🔬 Capture full-resolution screenshots of the target application's `HWND` even when the window is occluded behind other windows or located on a secondary monitor, using `Windows.Graphics.Capture` (WGC) or `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`.
- **F2.2** Compute perceptual hashes (dHash/aHash) and structural similarity (SSIM) + pixel-level bounding-box diffs between consecutive states (to detect whether an action caused a visual change or spinner hang) and against saved baseline build snapshots.
- **F2.3** Generate annotated PNG images highlighting detected visual defects, changed regions, or interacted control bounding boxes (`[x, y, width, height]` in client coordinates).
- **F2.4 Fallback:** If a specific GPU swapchain (`wgpu` DX12/Vulkan) cannot be captured while minimized, automatically prevent minimization (keep window restored behind other windows or on a virtual/secondary display) and log a warning.

### F3 — Hybrid UI Inspection & Non-Intrusive Input Injection (`iced`, `egui`, `Slint`) 🔬 (M)
- **F3.1** 🔬 Inspect the target window's Windows UI Automation (`IUIAutomation`) tree exposed via `AccessKit` (`egui`, `Slint`, `iced`), extracting control IDs, names, localized control types, enabled/focused/checked states, values, and client-relative `BoundingRectangle`.
- **F3.2** 🔬 Execute UI interactions in **Background Focus Mode** (`focus_mode = "background"`) without moving the user's physical mouse cursor or stealing active foreground window focus, using a 3-tier dispatch strategy:
  1. **Tier 1 (UIA Patterns):** `IUIAutomationInvokePattern::Invoke`, `IUIAutomationValuePattern::SetValue`, `IUIAutomationTogglePattern::Toggle`, `IUIAutomationSelectionItemPattern::Select`, `IUIAutomationScrollPattern::Scroll`.
  2. **Tier 2 (Direct HWND Message Injection):** `PostMessageW` / `SendMessageTimeoutW` (`WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_MOUSEWHEEL`, `WM_KEYDOWN`, `WM_KEYUP`, `WM_CHAR`) sent to the target `HWND` at the control's center coordinate (from UIA `BoundingRectangle` or Vision LLM grounding).
  3. **Tier 3 (Optional Rust SDK Bridge):** Named-pipe IPC command to `ui-qa-sdk` embedded in the target `egui`/`iced`/`Slint` app.
- **F3.3 Fallback:** If `winit` in a specific target build filters out unfocused `WM_LBUTTONDOWN` messages and `ui-qa-sdk` is not linked, provide an opt-in **Isolated Desktop / Micro-Focus Fallback** (`CreateDesktopW` or `< 10 ms` cursor save → click → restore when user mouse is idle for `> 3 s`).

### F4 — Autonomous UI Crawler (M)
- **F4.1** Automatically explore the target application by constructing a directed state graph $G = (V, E)$, where each vertex $v \in V$ is a unique screen state (keyed by UIA structural hash + perceptual image hash) and each edge $e \in E$ is an interaction (click, toggle, tab switch, scroll, safe text input).
- **F4.2** Prioritize unvisited interactive controls using configurable safety rules (e.g. blocklist destructive buttons matching regex `Delete|Format|Factory Reset|Erase` unless explicitly allowed).
- **F4.3** Detect dead-ends, unresponsive buttons (clicked control produced zero UI change and zero expected log output), infinite loading spinners, and crashes during exploration.

### F5 — Goal-Driven AI Test Runner (M)
- **F5.1** Execute natural-language test scenarios defined in TOML/YAML or entered in the dashboard (e.g. *"Select COM3 in the device dropdown, click Connect, verify the log outputs 'Hardware handshake OK', and check that the Calibration tab becomes enabled"*).
- **F5.2** Maintain an observe → plan → act → verify loop where the AI planner selects the next control action using the combined UIA tree + window screenshot + recent hardware log tail, and evaluates step assertions against both visual state and log output.

### F6 — AI Visual & UX Critic (Dual Backend: Gemini API + Local VLM) (M)
- **F6.1** Support both **Google Gemini API** (e.g. `gemini-2.5-flash`, `gemini-2.5-pro`) and **Local OpenAI-compatible Vision endpoints** (e.g. Ollama, LM Studio, vLLM at `http://localhost:11434/v1/chat/completions`) via a unified `VisionProvider` trait.
- **F6.2** Analyze each discovered screen (screenshot + UIA accessibility tree + recent hardware logs) to identify and classify:
  - **Visual bugs:** clipped/truncated text, overlapping controls, broken alignment, unreadable contrast, raw debug strings/NaNs displayed in UI.
  - **UX & Accessibility gaps:** missing disabled/loading feedback during hardware operations, ambiguous icons without labels, missing AccessKit roles/names, confusing layout hierarchy, unhandled hardware error states.
  - **Concrete Improvement Suggestions:** specific, actionable recommendations for layout, widget choice, error handling, and Rust GUI (`egui`/`iced`/`Slint`) code improvements.
- **F6.3** Run local deterministic heuristic checks (zero-size bounding boxes, overlapping UIA rects, off-screen controls, unnamed interactive controls, high-contrast ratio checks) automatically before invoking the LLM to save tokens and latency.

### F7 — Live QA Dashboard UI (Tauri v2) (M)
- **F7.1** Provide a desktop dashboard to configure target `.exe`/logs, select Crawl or Goal mode, watch live background captures with interactive bounding-box overlays, inspect the synchronized UI action ↔ hardware log timeline, and browse prioritized findings (Critical / High / Medium / Suggestion).
- **F7.2** Allow pausing/resuming runs, comparing current screens side-by-side against baseline snapshots, and filtering findings by category (Hardware/Log Error, Visual Bug, UX Suggestion, Crash/Hang, Accessibility).

### F8 — Agent-Ready Export & GitHub Issue Creation (M)
- **F8.1** Export every session to a structured directory containing `report.md` (formatted specifically for AI coding agents like Antigravity/Claude/Copilot to locate and fix issues in the target app's repository), `report.json` (machine-readable schema), and `screenshots/` (raw + annotated PNGs + baseline diffs).
- **F8.2** Optionally create deduplicated GitHub Issues in the target application's repository via `gh issue create` or GitHub REST API with embedded repro steps, hardware log snippets, and UX recommendations.

### F9 — Optional Lightweight Rust Target SDK (`crates/sdk`) (S)
- **F9.1** Provide a lightweight, zero-unsafe Rust crate (`ui-qa-sdk`) that developers of `egui`, `iced`, or `Slint` apps can optionally enable behind a `#[cfg(feature = "qa-hook")]` flag to expose custom widget metadata, internal app state assertions, and direct synthetic input reception over a local Windows named pipe (`\\.\pipe\ui-qa-tester-<pid>`).

---

## Non-functional requirements

### Performance (hard budget, verified in the hardening milestone M5)
| ID | Metric | Budget | How it's measured |
|---|---|---|---|
| NFR-P1 | Background single-frame capture latency (1080p window) | `≤ 50 ms` (p95) | Benchmark test in `crates/platform` measuring WGC / `PrintWindow` + PNG/RGBA buffer read |
| NFR-P2 | UIA tree extraction + local heuristic geometry check | `≤ 40 ms` (p95 for 200 nodes) | Benchmark test on synthetic/real AccessKit window |
| NFR-P3 | Log tailing & action-correlation overhead | `< 2 ms` per log line; `≤ 1%` CPU at `1,000 lines/sec` | Criterion / timed throughput unit test in `crates/core` |
| NFR-P4 | QA Engine + Dashboard RAM footprint (excluding external LLM) | `≤ 120 MB` working set (Dashboard) / `≤ 35 MB` (Headless CLI) | Process memory measurement during a 50-step crawl in M5.1 |
| NFR-P5 | Target application frame-rate impact during background QA | `< 5%` FPS drop on target `wgpu` app | Measured via target app frame-time telemetry during 1 Hz capture in M5.1 |

### Safety, security & compatibility
- **NFR-S1 (No Focus Hijack):** In default `background` mode, physical mouse cursor position and foreground window focus must remain 100% untouched across an entire test run.
- **NFR-S2 (No Invasive Injection):** Zero kernel drivers, zero global hooks, zero DLL injection (`CreateRemoteThread`). Uses standard user-space Win32 (`PrintWindow`, `PostMessageW`), COM (`IUIAutomation`), WinRT (`Windows.Graphics.Capture`), and named pipes.
- **NFR-S3 (Graceful Target Cleanup):** If `ui-qa-tester` crashes or is aborted (`Ctrl+C`), any child target process spawned by the runner must be cleanly terminated (via Win32 Job Object `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` when configured) and no ghost locks left on log files.
- **NFR-S4 (Credential Isolation):** API keys for Gemini or GitHub are read from environment variables or Windows Credential Manager, never written to logs, session configs, or exported reports.

### Quality
- **NFR-Q1** `crates/core`, `crates/ai`, `crates/sdk`, and `crates/cli` enforce `#![forbid(unsafe_code)]`; `crates/core` has `≥ 85%` line coverage with deterministic unit tests.
- **NFR-Q2** All user-facing dashboard and CLI strings are loaded from localization files (`locales/en.json`, `locales/it.json`).
