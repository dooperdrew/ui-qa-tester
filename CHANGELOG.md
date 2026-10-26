# Changelog & Development Log

Chronological record of milestones, decisions and discoveries. Newest first.

## 2026-10-26 — Project definition (APPROVED)
- Created full project specification suite (`01-vision.md` through `06-qa-and-validation.md`), `AGENTS.md`, `ADR-0001`, and GitHub templates via `project-kickstart`.
- Key decisions:
  - `ADR-0001`: Rust Cargo workspace (`crates/core`, `crates/platform`, `crates/ai`, `crates/sdk`, `crates/cli`, `crates/ui`) with Tauri v2 Live QA Dashboard, dual AI Vision backend (Gemini API + local OpenAI-compatible endpoint), and hybrid black-box (UIA + WGC + HWND messages) + optional lightweight Rust SDK inspection.
- Open Milestone 0 spikes:
  - `M0.2`: Verify occluded/background window capture (`Windows.Graphics.Capture` vs `PrintWindow` with `PW_RENDERFULLCONTENT`) on GPU-rendered `wgpu` windows (`egui`, `iced`, `Slint`).
  - `M0.3`: Verify AccessKit UIA tree coverage and non-intrusive background input delivery (`IUIAutomation` patterns + `PostMessageW` `WM_LBUTTONDOWN`/`WM_CHAR` vs `winit` event loop focus filtering) across `egui`, `iced`, and `Slint`.
