# Changelog & Development Log

Chronological record of milestones, decisions and discoveries. Newest first.

## 2026-10-26 — Milestone 7 (M7.1): Project Retrospective
- Completed `M7.1`: Authored `docs/retrospective.md` documenting the M0–M6 delivery timeline, empirical NFR measurements, Windows UIA MSAA timeout discovery, and Rust 1.99 clippy lessons.

## 2026-10-26 — Release v0.1.0 (Milestones 5 & 6: Hardening, NFR Verification & Multi-Channel Release)
- Completed `M5.1`–`M5.3`: Added `crates/platform/tests/hardening_tests.rs` verifying `NFR-P1` (`20.82 ms` release / `33.86 ms` debug p95 capture), `NFR-P2` (`18.88 ms` UIA + heuristics), `NFR-P3` (`8.52 ms` for `10,000` log lines), `NFR-P4` (`24.12 MB` working set RAM), `QA-SYS-04` (minimized window `SW_SHOWNOACTIVATE` restore without focus stealing), and `QA-SYS-05` (offline AI endpoint graceful fallback).
- Completed `M6.1`–`M6.3`: Added `scripts/package-release.ps1`, `scripts/setup-wizard.ps1`, `scripts/install.ps1`, `scripts/uninstall.ps1`, `scripts/generate-manifests.ps1`, `setup.cmd`, `README.md`, `docs/user-guide.md`, Scoop/WinGet manifests (`packaging/`), and published `ui-qa-tester-v0.1.0-windows-x64.zip` to GitHub Releases.

## 2026-10-26 — Milestones 3 & 4 (M3.1, M4.1–M4.2): Headless CLI Orchestrator & Live QA Dashboard UI
- Completed `M3.1`: Implemented end-to-end `run_qa_session` orchestrator (`crates/platform/src/orchestrator.rs`) and `ui-qa-cli` (`run`, `self-test`, `windows`, `init-config`, `--create-github-issues`), generating annotated screenshots (`step-NNN-annotated.png`), `report.md`, and `report.json`.
- Completed `M4.1` & `M4.2`: Implemented `ui-qa-ui` (`crates/ui/src/main.rs` + `crates/ui/src/dashboard.html`) with 4 interactive tabs, live window enumeration, 1-click hands-free self-test demo, synchronized UI ↔ hardware log timeline, `en`/`it` instant locale switcher, and "Copy Agent Fix Prompt".

## 2026-10-26 — Milestone 2 (M2.1–M2.4): Platform Engine, Dual Vision AI Client & Optional Rust SDK
- Completed `M2.1`: Implemented `MonitoredTargetProcess` in `crates/platform/src/process.rs` with `.exe` spawning, `stdout`/`stderr` capture threads, rotation-aware diagnostic log file tailing, `GetProcessMemoryInfo` working-set sampler, and `IsHungAppWindow` responsiveness checks.
- Completed `M2.2`: Implemented `capture_hwnd_background`, `list_top_level_windows`, `inspect_window_uia_elements`, `dispatch_action_background`, and `SelfTestWindow` in `crates/platform`.
- Completed `M2.3`: Implemented `UnifiedVisionClient` in `crates/ai` supporting both Google Gemini Multimodal API (`generateContent`) and OpenAI-compatible Vision endpoints (`/v1/chat/completions`), plus `plan_goal_step` and graceful fallback to deterministic heuristics.
- Completed `M2.4`: Implemented `QaSdkServer`, `QaAppAdapter`, and `send_sdk_command` in `crates/sdk` (`#![forbid(unsafe_code)]`) for optional white-box widget inspection and synthetic input in `iced`/`egui`/`Slint` apps.

## 2026-10-26 — Milestone 1 (M1.1–M1.4): Core Engine & Schemas (`crates/core`)
- Completed `M1.1`: Added `QaSessionConfig` TOML serializer/deserializer, `UiElement`/`Finding`/`QaStepRecord` domain models, and `I18nCatalog` (`en`/`it` parity verified).
- Completed `M1.2`: Implemented `HardwareLogCorrelator` with `[-50 ms, +2000 ms]` action bracketing, regex ACK/error classification, silent UI-vs-hardware mismatch detection, and `< 2 ms` per 1,000-line throughput verification (`NFR-P3`).
- Completed `M1.3`: Implemented `compute_dhash`, `compare_frames` (SSIM + pixel bounding-box diff + magenta diff overlay), `annotate_frame`, and `analyze_screen_heuristics` (overlapping controls, clipped bounds, missing AccessKit labels, WCAG luminance contrast ratio).
- Completed `M1.4`: Implemented `AutonomousCrawler` state machine (with destructive-button regex blocklist and dead-control detector) and `QaSessionReport` (agent-ready Markdown + JSON + GitHub Issue draft generator).

## 2026-10-26 — Milestone 0 (M0.1–M0.5): Workspace Skeleton & Background Capture/Input Spikes
- Completed `M0.1`: Created 7-crate Cargo workspace (`core`, `platform`, `ai`, `sdk`, `cli`, `ui`, `probe`), `rust-toolchain.toml`, and `en`/`it` localization catalogs (`locales/`).
- Completed `M0.2` & `M0.3`: Built `ui-qa-probe` (`capture_probe.rs`, `input_probe.rs`, `dummy_window.rs`) and empirically verified background window capture (`PrintWindow` with `PW_RENDERFULLCONTENT`: `16.53 ms` p50, `20.82 ms` p95, `96%` non-black pixels, zero focus change) and non-intrusive input (`IUIAutomationInvokePattern` + `PostMessageW` mouse/key events with `10.14 ms` UIA walk latency and zero physical cursor movement).
- Completed `M0.4` & `M0.5`: Recorded findings in `docs/m0-spike-findings.md` and resolved `🔬` items in `docs/02-requirements.md` and `docs/03-architecture.md`.

## 2026-10-26 — Project definition (APPROVED)
- Created full project specification suite (`01-vision.md` through `06-qa-and-validation.md`), `AGENTS.md`, `ADR-0001`, and GitHub templates via `project-kickstart`.
- Key decisions:
  - `ADR-0001`: Rust Cargo workspace (`crates/core`, `crates/platform`, `crates/ai`, `crates/sdk`, `crates/cli`, `crates/ui`) with Tauri v2 Live QA Dashboard, dual AI Vision backend (Gemini API + local OpenAI-compatible endpoint), and hybrid black-box (UIA + WGC + HWND messages) + optional lightweight Rust SDK inspection.
- Open Milestone 0 spikes:
  - `M0.2`: Verify occluded/background window capture (`Windows.Graphics.Capture` vs `PrintWindow` with `PW_RENDERFULLCONTENT`) on GPU-rendered `wgpu` windows (`egui`, `iced`, `Slint`).
  - `M0.3`: Verify AccessKit UIA tree coverage and non-intrusive background input delivery (`IUIAutomation` patterns + `PostMessageW` `WM_LBUTTONDOWN`/`WM_CHAR` vs `winit` event loop focus filtering) across `egui`, `iced`, and `Slint`.
