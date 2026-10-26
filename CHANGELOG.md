# Changelog & Development Log

Chronological record of milestones, decisions and discoveries. Newest first.

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
