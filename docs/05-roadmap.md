# 05 — Roadmap

> The single source of truth for tasks. Each task fits in **one conversation** (or one worker).
> Tick boxes with `(YYYY-MM-DD, <commit>)`. Don't start a milestone before the previous **Exit** is met.
> ⇉ = can run in parallel with the other ⇉ tasks in the same milestone (disjoint files).

## M0 — Spikes 🔬 (decision gate)
- [x] **M0.1** Project workspace skeleton (`Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, crate stubs for `core`, `platform`, `ai`, `sdk`, `cli`, `ui`, `probe`, `locales/`) + verification gate passing locally and in `.github/workflows/ci.yml`. *(2026-10-26, M0)*
- [x] **M0.2** ⇉ **Spike — Background Window Capture (`crates/probe/src/capture_probe.rs`):** Implement and benchmark `Windows.Graphics.Capture` vs `PrintWindow(..., PW_RENDERFULLCONTENT)` on occluded and background windows (`wgpu` / DirectX surfaces), measuring frame capture latency (`16.53 ms` p50 / `20.82 ms` p95) and verifying non-black pixel output (`96.0%`) when covered by other windows. *(2026-10-26, M0)*
- [x] **M0.3** ⇉ **Spike — AccessKit UIA Tree & Non-Intrusive Input (`crates/probe/src/input_probe.rs`):** Build a diagnostic probe that dumps `IUIAutomation` nodes/patterns (`10.14 ms` walk latency) and tests background input via Tier 1 (UIA `InvokePattern`/`ValuePattern`) and Tier 2 (`PostMessageW` `WM_MOUSEMOVE` + `WM_LBUTTONDOWN` + `WM_LBUTTONUP` + `WM_CHAR`) without moving the physical cursor or changing foreground focus. *(2026-10-26, M0)*
- [x] **M0.4** Run `ui-qa-probe` against target windows; record empirical results in `docs/m0-spike-findings.md`. *(2026-10-26, M0)*
- [x] **M0.5** Resolve all 🔬 items in `docs/02-requirements.md` and `docs/03-architecture.md` and record any design adjustments. *(2026-10-26, M0)*
**Exit:** Every 🔬 item is empirically verified in `docs/m0-spike-findings.md` or has an accepted, tested fallback. *(MET 2026-10-26)*

## M1 — Core Engine & Schemas (`crates/core`)
- [x] **M1.1** ⇉ **Session Config, Domain Models & i18n Loader (`crates/core/src/config.rs`, `model.rs`, `i18n.rs`):** Define `QaSessionConfig`, `UiElement`, `UiStateNode`, `UiAction`, `Finding`, `Severity`, and JSON locale loader (`locales/en.json`, `locales/it.json`) with 100% unit test coverage. *(2026-10-26, M1)*
- [x] **M1.2** ⇉ **Hardware Log Correlator & Anomaly Detector (`crates/core/src/correlator.rs`):** Implement millisecond-windowed log correlation (`[-50 ms, +2000 ms]`), regex/pattern matchers for hardware ACK/NACK/timeout/panic, and silent UI-vs-hardware mismatch detection. *(2026-10-26, M1)*
- [x] **M1.3** ⇉ **Visual Diffing, Perceptual Hash & Heuristic Geometry Analyzer (`crates/core/src/vision_heuristics.rs`, `diff.rs`):** Implement dHash screen deduplication, SSIM/pixel bounding-box diffing against baselines, bounding-box PNG annotation, and fast deterministic UIA checks (overlapping rects, clipped bounds, missing AccessKit names, contrast ratio). *(2026-10-26, M1)*
- [x] **M1.4** ⇉ **Autonomous Crawler State Machine & Agent-Ready Report Generator (`crates/core/src/crawler.rs`, `report.rs`):** Implement directed state-graph exploration $G=(V,E)$, destructive-control safety blocklist, dead-button detector, and Markdown/JSON/GitHub-Issue report formatters. *(2026-10-26, M1)*
**Exit:** `cargo test -p ui-qa-core` passes with `≥ 85%` coverage and zero OS/network dependencies (`#![forbid(unsafe_code)]`). *(MET 2026-10-26)*

## M2 — Platform Engine, AI Backend & Optional SDK (`crates/platform`, `crates/ai`, `crates/sdk`)
- [x] **M2.1** ⇉ **Process Watchdog & Real-Time Log Tailer (`crates/platform/src/process.rs`):** Implement `.exe` launching, PID/HWND attachment, async `stdout`/`stderr` streaming, rotation-aware diagnostic file tailing, RAM working-set sampler, and `IsHungAppWindow` watchdog. *(2026-10-26, M2)*
- [x] **M2.2** ⇉ **Background Window Capture & Hybrid UIA/HWND Actuator (`crates/platform/src/capture.rs`, `actuator.rs`, `selftest_window.rs`):** Productionize the M0-verified `PrintWindow(PW_RENDERFULLCONTENT)`/`BitBlt` capture pipeline, filtered `IUIAutomation` tree extractor, window enumerator, and 3-tier non-intrusive input dispatcher. *(2026-10-26, M2)*
- [x] **M2.3** ⇉ **Dual Vision LLM Client & Goal Planner (`crates/ai/src/lib.rs`):** Implement `UnifiedVisionClient` for Google Gemini API and local OpenAI-compatible VLM endpoints, structured UX/Visual Critic prompt + JSON parser, and natural-language goal step planner. *(2026-10-26, M2)*
- [x] **M2.4** ⇉ **Optional Lightweight Rust Target SDK (`crates/sdk/src/lib.rs`):** Implement the zero-unsafe IPC server/client and `QaAppAdapter` trait for `egui`, `iced`, and `Slint` apps to expose widget trees and accept synthetic background events. *(2026-10-26, M2)*
**Exit:** Full integration tests pass across `crates/platform`, `crates/ai`, and `crates/sdk`. *(MET 2026-10-26)*

## M3 — Headless CLI Runner & Orchestrator (`crates/cli`)
- [x] **M3.1** **Session Orchestrator & CLI Commands (`crates/cli/src/main.rs`, `crates/platform/src/orchestrator.rs`):** Wire `core` + `platform` + `ai` into the end-to-end execution loop (`run`, `self-test`, `windows`, `init-config`, and `--create-github-issues`), producing full agent-ready report bundles (`report.md`, `report.json`, `screenshots/`). *(2026-10-26, M3)*
**Exit:** Running `cargo run -p ui-qa-cli -- self-test` autonomously launches a test window, inspects controls in the background, correlates mock hardware logs, generates annotated screenshots, and writes a valid `report.md` and `report.json`. *(MET 2026-10-26)*

## M4 — Live QA Dashboard UI (`crates/ui`)
- [x] **M4.1** ⇉ **Dashboard Backend Commands & API Server (`crates/ui/src/main.rs`):** Expose session lifecycle endpoints (`/api/run`, `/api/windows`, `/api/i18n`) and launch native Windows Edge WebView2 (`--app`) shell. *(2026-10-26, M4)*
- [x] **M4.2** ⇉ **Dashboard Frontend Views & Interactive Overlay Canvas (`crates/ui/src/dashboard.html`):** Build the 4-screen responsive UI (Setup, Live Run with annotated frame preview & synchronized hardware log timeline, Findings & AI UX Critique inspector with "Copy Agent Fix Prompt", and Agent-Ready Report export) with `en`/`it` localization. *(2026-10-26, M4)*
**Exit:** `ui-qa-ui` builds cleanly in release mode and streams live background captures, hardware log correlations, and findings in the dashboard UI. *(MET 2026-10-26)*

## M5 — Hardening & Empirical Verification
- [x] **M5.1** Measure every NFR budget (`NFR-P1` through `NFR-P4`) with automated benchmarks and record empirical numbers in `docs/06-qa-and-validation.md`. *(2026-10-26, M5)*
- [x] **M5.2** Soak & edge-case tests (`10,000 lines/s` log flood, minimized target window `SW_SHOWNOACTIVATE` restore without focus change, offline AI endpoint fallback in `crates/platform/tests/hardening_tests.rs`). *(2026-10-26, M5)*
- [x] **M5.3** Audit any code produced by fallback models (`docs/model-fallback-log.md` — zero fallbacks occurred). *(2026-10-26, M5)*
- [ ] **M5.4** Real-environment QA run by the user against their real `iced`/`egui`/`Slint` applications and connected hardware (manual matrix `QA-01`..`QA-07` tracked in GitHub Issue `#20`).
**Exit:** Automated NFR budgets meet target thresholds in `docs/06-qa-and-validation.md`; manual hardware QA tracked openly in Issue `#20`. *(Automated gate MET 2026-10-26)*

## M6 — Packaging & Multi-Channel Release
- [x] **M6.1** Create release packaging & zero-elevation installer scripts (`scripts/package-release.ps1`, `scripts/setup-wizard.ps1`, `scripts/install.ps1`, `scripts/uninstall.ps1`, `scripts/generate-manifests.ps1`, `setup.cmd`). *(2026-10-26, v0.1.0)*
- [x] **M6.2** Write comprehensive `README.md` and `docs/user-guide.md` covering all 4 installation channels (WinGet, Scoop, Setup Wizard, Portable ZIP), AI provider setup, and target app (`iced`/`egui`/`Slint`) testing guides. *(2026-10-26, v0.1.0)*
- [x] **M6.3** Publish `v0.1.0` release archive (`dist/ui-qa-tester-v0.1.0-windows-x64.zip`) to GitHub Releases following `docs/release-methodology.md`. *(2026-10-26, v0.1.0)*
**Exit:** Pre-built `ui-qa-tester-v0.1.0-windows-x64.zip` and WinGet/Scoop manifests published and verified. *(MET 2026-10-26)*

## M7 — Retrospective
- [x] **M7.1** Run the `project-retrospective` skill, produce `docs/retrospective.md`, and record `[KIT]` lessons in `docs/lessons-learned.md`. *(2026-10-26, M7)*

## M8 — Multi-Agent Architecture & UI Explorer
- [x] **M8.1** Create comprehensive didactic multi-agent guide (`docs/multi-agent-methodology.md`), centralized specification hub (`docs/README.md`), and ecosystem growth strategy (`docs/08-community-and-growth-strategy.md`). *(2026-10-27, v0.2.0)*
- [x] **M8.2** Add dedicated **"🤖 Architecture"** tab in `ui-qa-ui` (`crates/ui/src/dashboard.html`) covering the non-intrusive engine, 3-tier actuation, and complexity tier taxonomy (`T1`/`T2`/`T3`). *(2026-10-27, v0.2.0)*

## M9 — User Experience Onboarding & Interactive QA Matrix
- [x] **M9.1** Guided Windows Installation Wizard (`scripts/setup-wizard.ps1`, `setup.cmd`, `scripts/installer.iss`): zero-elevation install into `%LOCALAPPDATA%\Programs\ui-qa-tester`, Desktop/Start Menu shortcuts, User PATH, and Windows "Installed apps" registration. *(2026-10-27, v0.2.0)*
- [x] **M9.2** First-Run Onboarding Setup Wizard in `ui-qa-ui` (`🪄 Setup Wizard` tab) with presets for `egui`/`MagicBridge`, `iced`, `Slint`, and `Win32`, plus live TOML generation. *(2026-10-27, v0.2.0)*
- [x] **M9.3** Responsive Dashboard Layout (`< 850px` Compact Mode vs `>= 850px` Full Mode) with dynamic scrollable rail navigation avoiding clipping on narrow windows and 4K displays. *(2026-10-27, v0.2.0)*
- [x] **M9.4** Interactive Subsystem Self-Test & NFR Budget Matrix (`🧪 Self-Test & QA` tab) with live green `#2ECC71` badges and 1-click JSON/Markdown QA certificate export. *(2026-10-27, v0.2.0)*
**Exit:** Comprehensive UX onboarding wizard, interactive QA self-test matrix, responsive dashboard UI, and v0.2.0 release packaged. *(MET 2026-10-27)*
