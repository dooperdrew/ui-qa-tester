# 05 — Roadmap

> The single source of truth for tasks. Each task fits in **one conversation** (or one worker).
> Tick boxes with `(YYYY-MM-DD, <commit>)`. Don't start a milestone before the previous **Exit** is met.
> ⇉ = can run in parallel with the other ⇉ tasks in the same milestone (disjoint files).

## M0 — Spikes 🔬 (decision gate)
- [ ] **M0.1** Project workspace skeleton (`Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, crate stubs for `core`, `platform`, `ai`, `sdk`, `cli`, `ui`, `probe`, `locales/`) + verification gate passing locally and in `.github/workflows/ci.yml`.
- [ ] **M0.2** ⇉ **Spike — Background Window Capture (`crates/probe/src/capture_probe.rs`):** Implement and benchmark `Windows.Graphics.Capture` vs `PrintWindow(..., PW_RENDERFULLCONTENT)` on occluded and background windows (`wgpu` / DirectX surfaces), measuring frame capture latency and verifying non-black pixel output when covered by other windows.
- [ ] **M0.3** ⇉ **Spike — AccessKit UIA Tree & Non-Intrusive Input (`crates/probe/src/input_probe.rs`):** Build a diagnostic probe that dumps `IUIAutomation` nodes/patterns and tests background input via Tier 1 (UIA `InvokePattern`/`ValuePattern`) and Tier 2 (`PostMessageW` `WM_MOUSEMOVE` + `WM_LBUTTONDOWN` + `WM_LBUTTONUP` + `WM_CHAR`) without moving the physical cursor or changing foreground focus.
- [ ] **M0.4** Run `ui-qa-probe` against real target windows (`egui`/`iced`/`Slint` or Windows apps) in the user's environment; record empirical results in `docs/m0-spike-findings.md`.
- [ ] **M0.5** Resolve all 🔬 items in `docs/02-requirements.md` and `docs/03-architecture.md` and record any design adjustments as an ADR.
**Exit:** Every 🔬 item is empirically verified in `docs/m0-spike-findings.md` or has an accepted, tested fallback.

## M1 — Core Engine & Schemas (`crates/core`)
- [ ] **M1.1** ⇉ **Session Config, Domain Models & i18n Loader (`crates/core/src/config.rs`, `model.rs`, `i18n.rs`):** Define `QaSessionConfig`, `UiElement`, `UiStateNode`, `UiAction`, `Finding`, `Severity`, and JSON locale loader (`locales/en.json`, `locales/it.json`) with 100% unit test coverage.
- [ ] **M1.2** ⇉ **Hardware Log Correlator & Anomaly Detector (`crates/core/src/correlator.rs`):** Implement millisecond-windowed log correlation (`[-50 ms, +2000 ms]`), regex/pattern matchers for hardware ACK/NACK/timeout/panic, and silent UI-vs-hardware mismatch detection.
- [ ] **M1.3** ⇉ **Visual Diffing, Perceptual Hash & Heuristic Geometry Analyzer (`crates/core/src/vision_heuristics.rs`, `diff.rs`):** Implement dHash screen deduplication, SSIM/pixel bounding-box diffing against baselines, bounding-box PNG annotation, and fast deterministic UIA checks (overlapping rects, clipped bounds, missing AccessKit names, contrast ratio).
- [ ] **M1.4** ⇉ **Autonomous Crawler State Machine & Agent-Ready Report Generator (`crates/core/src/crawler.rs`, `report.rs`):** Implement directed state-graph exploration $G=(V,E)$, destructive-control safety blocklist, dead-button detector, and Markdown/JSON/GitHub-Issue report formatters.
**Exit:** `cargo test -p ui-qa-core` passes with `≥ 85%` coverage and zero OS/network dependencies (`#![forbid(unsafe_code)]`).

## M2 — Platform Engine, AI Backend & Optional SDK (`crates/platform`, `crates/ai`, `crates/sdk`)
- [ ] **M2.1** ⇉ **Process Watchdog & Real-Time Log Tailer (`crates/platform/src/process.rs`, `log_tailer.rs`):** Implement `.exe` launching (with Win32 Job Object cleanup), PID/HWND attachment, async `stdout`/`stderr` streaming, rotation-aware diagnostic file tailing, CPU/RAM sampler, and `IsHungAppWindow` watchdog.
- [ ] **M2.2** ⇉ **Background Window Capture & Hybrid UIA/HWND Actuator (`crates/platform/src/capture.rs`, `uia.rs`, `actuator.rs`):** Productionize the M0-verified WGC/`PrintWindow` capture pipeline, cached `IUIAutomation` tree extractor, and 3-tier non-intrusive input dispatcher.
- [ ] **M2.3** ⇉ **Dual Vision LLM Client & Goal Planner (`crates/ai/src/lib.rs`, `gemini.rs`, `openai_compat.rs`, `critic.rs`, `planner.rs`):** Implement `VisionProvider` for Google Gemini API and local OpenAI-compatible VLM endpoints, visual control grounding fallback, structured UX/Visual Critic prompt + JSON parser, and natural-language goal step planner.
- [ ] **M2.4** ⇉ **Optional Lightweight Rust Target SDK (`crates/sdk/src/lib.rs`):** Implement the zero-unsafe named-pipe server/client (`\\.\pipe\ui-qa-tester-<pid>`) and adapter traits for `egui`, `iced`, and `Slint` apps to expose widget trees and accept synthetic background events.
**Exit:** Full integration tests pass across `crates/platform`, `crates/ai` (with mock HTTP server), and `crates/sdk` (named pipe round-trip).

## M3 — Headless CLI Runner & Orchestrator (`crates/cli`)
- [ ] **M3.1** **Session Orchestrator & CLI Commands (`crates/cli/src/main.rs`, `runner.rs`, `github.rs`):** Wire `core` + `platform` + `ai` into the end-to-end execution loop (`crawl`, `goal`, `diff`, `probe`, `self-test`, and `--create-github-issues`), producing full agent-ready report bundles (`report.md`, `report.json`, `screenshots/`).
**Exit:** Running `cargo run -p ui-qa-cli -- self-test` autonomously launches a test window, inspects controls in the background, correlates mock hardware logs, generates annotated screenshots, and writes a valid `report.md` and `report.json`.

## M4 — Live QA Dashboard UI (`crates/ui` — Tauri v2)
- [ ] **M4.1** ⇉ **Tauri v2 Backend Commands & Event Stream (`crates/ui/src/lib.rs`, `commands.rs`, `state.rs`):** Expose session lifecycle commands (`start_session`, `pause_session`, `stop_session`, `list_windows`, `test_ai_provider`, `export_report`, `create_github_issues`) and real-time event emitters for captured frames, logs, telemetry, and findings.
- [ ] **M4.2** ⇉ **Dashboard Frontend Views & Interactive Overlay Canvas (`crates/ui/dist/`):** Build the 4-screen responsive UI (Setup, Live Run with bounding-box `<canvas>` overlay & synchronized hardware log timeline, Findings & AI UX Critique inspector with "Copy Agent Fix Prompt", and Baseline Visual Diff comparator) with `en`/`it` localization.
**Exit:** `ui-qa-ui` builds cleanly in release mode and streams live background captures, hardware log correlations, and findings in the dashboard UI.

## M5 — Hardening & Empirical Verification
- [ ] **M5.1** Measure every NFR budget (`NFR-P1` through `NFR-P5`) with automated benchmarks and record empirical numbers in `docs/06-qa-and-validation.md`.
- [ ] **M5.2** Soak & edge-case tests (target crash mid-crawl, hung target window, rapid log flood `10,000 lines/s`, occluded/resized target window, offline AI endpoint fallback).
- [ ] **M5.3** Audit any code produced by fallback models (see `docs/model-fallback-log.md`).
- [ ] **M5.4** Real-environment QA run by the user against their real `iced`/`egui`/`Slint` applications and connected hardware (manual matrix in `docs/06-qa-and-validation.md`).
**Exit:** All NFR budgets meet target thresholds in `docs/06-qa-and-validation.md`; zero high-severity open bugs.

## M6 — Packaging & Multi-Channel Release
- [ ] **M6.1** Create release packaging & zero-elevation installer scripts (`scripts/package-release.ps1`, `scripts/setup-wizard.ps1`, `scripts/install.ps1`, `scripts/uninstall.ps1`, `scripts/generate-manifests.ps1`, `setup.cmd`).
- [ ] **M6.2** Write comprehensive `README.md` and `docs/user-guide.md` covering all 4 installation channels (WinGet, Scoop, Setup Wizard, Portable ZIP), AI provider setup, and target app (`iced`/`egui`/`Slint`) testing guides.
- [ ] **M6.3** Publish `v0.1.0` release archive to GitHub Releases following `docs/release-methodology.md`.
**Exit:** Pre-built `ui-qa-tester-v0.1.0-windows-x64.zip` and WinGet/Scoop manifests published and verified.

## M7 — Retrospective
- [ ] **M7.1** Run the `project-retrospective` skill, produce `docs/retrospective.md`, and promote generic `[KIT]` lessons into `project-kickstart`.
