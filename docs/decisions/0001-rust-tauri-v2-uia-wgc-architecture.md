# ADR 0001 — Rust Workspace + Tauri v2 Dashboard + Hybrid UIA/WGC/SDK Architecture

- **Status:** Accepted
- **Date:** 2026-10-26
- **Deciders:** User, Lead Agent (Antigravity)

## Context
We need an autonomous Windows UI and Hardware-in-the-Loop (HIL) QA agent (`ui-qa-tester`) capable of inspecting, driving, and visually critiquing Windows desktop applications built with **Rust native GUI frameworks (`iced`, `egui`, `Slint`)** while correlating every interaction with connected hardware logs (`stdout`, `stderr`, diagnostic files).
Key requirements from discovery (`docs/02-requirements.md`):
- Must run hands-free in the background without hijacking the developer's physical mouse or keyboard focus (`F2`, `F3`, `NFR-S1`).
- Must work out-of-the-box on black-box `.exe` binaries AND support an optional lightweight Rust SDK hook for deep white-box inspection (`F3`, `F9`).
- Must support both cloud multimodal LLMs (**Google Gemini API**) and **local OpenAI-compatible Vision endpoints** (`F6`).
- Must provide a rich **Live QA Dashboard UI** (`F7`) alongside a scriptable CLI (`F8`).

## Options
| Option | Pros | Cons |
|---|---|---|
| **A: Rust Workspace + Tauri v2 UI + Hybrid Win32 (UIA/WGC/PostMessageW) + Optional `ui-qa-sdk` (Chosen)** | • Native, zero-overhead access to `windows` crate (`IUIAutomation`, `Windows.Graphics.Capture`, `PostMessageW`, Job Objects).<br>• HTML5 `<canvas>` in Tauri v2 makes interactive bounding-box overlays, side-by-side visual diffing, and Markdown/JSON report rendering trivial.<br>• Shares the exact same Rust ecosystem as the user's target `iced`/`egui`/`Slint` apps, enabling a first-party `ui-qa-sdk` crate. | • Requires verifying `winit` background message handling (`PostMessageW`) and WGC occlusion behavior in M0 spikes. |
| **B: Python + Pywinauto / FlaUI + Web UI** | • Fast initial scripting for basic UIA calls. | • Poor performance for 1080p frame diffing; `pywinauto` relies heavily on foreground `SendInput`; cannot easily provide a native zero-dependency Rust SDK crate for `iced`/`egui`/`Slint`. |
| **C: Playwright / WebDriver / WinAppDriver** | • Standard WebDriver protocol. | • WinAppDriver is deprecated/unmaintained, forces foreground focus stealing, and fails completely on GPU-rendered `wgpu` canvas widgets without UIA IDs. |

## Decision
We choose **Option A**: a modular Rust Cargo workspace (`crates/core`, `crates/platform`, `crates/ai`, `crates/sdk`, `crates/cli`, `crates/ui`, `crates/probe`) pairing a **Tauri v2 Live QA Dashboard** with native Win32/COM/WinRT background capture (`Windows.Graphics.Capture` + `PrintWindow`), 3-tier non-intrusive input actuation (UIA Patterns → `PostMessageW` → `ui-qa-sdk` IPC), and a dual-provider Vision AI client (Gemini API + OpenAI-compatible local VLM).

## Consequences
- **Easier:**
  - Pure domain logic (`crates/core`) has `#![forbid(unsafe_code)]` and zero OS dependencies, enabling fast parallel multi-agent development.
  - Developers of `iced`, `egui`, and `Slint` apps get both zero-setup black-box testing and an optional 1-line `ui-qa-sdk` integration for 100% deterministic background testing.
  - Reports are generated in an agent-ready Markdown + JSON format that coding agents can immediately consume to fix discovered UI/UX and hardware-handling bugs.
- **Follow-up tasks:**
  - Execute Milestone 0 spikes (`M0.2` and `M0.3`) using `crates/probe` to empirically validate background capture and unfocused `PostMessageW` behavior on `winit`/`wgpu` windows.
