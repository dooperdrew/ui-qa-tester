# Project Retrospective — `ui-qa-tester` (`v0.1.0`)

> **Date:** 2026-10-26  
> **Conversation:** [`d471fcac-3c7b-41ed-b815-f1a79a82127d`](conversation://d471fcac-3c7b-41ed-b815-f1a79a82127d)  
> **Release:** [`v0.1.0`](https://github.com/DooperDrew/ui-qa-tester/releases/tag/v0.1.0)

---

## 1. Timeline (Milestones ↔ Commits ↔ Issues)
| Milestone | Commit | Issues Closed | Summary |
|---|---|---|---|
| Project Definition | `7ec94ed` | — | Full spec suite (`01-vision.md`..`06-qa-and-validation.md`), `AGENTS.md`, `ADR-0001`, `.github/` templates |
| **M0 — Spikes** | `24ac028` | `#1`–`#5` | 7-crate Cargo workspace + `ui-qa-probe` empirical verification of `PrintWindow(PW_RENDERFULLCONTENT)` (`16.53 ms` p50) and `IUIAutomation` + `PostMessageW` (`10.14 ms` walk) |
| **M1 — Core Engine** | `5b50615` | `#6`–`#9` | `crates/core` (`#![forbid(unsafe_code)]`): TOML config, models, `en`/`it` i18n, hardware log correlator, dHash/SSIM visual diff, heuristics, crawler, and agent-ready Markdown/JSON/GitHub-Issue reports |
| **M2 — Platform, AI & SDK** | `21c4b18` | `#10`–`#13` | `crates/platform` (watchdog, log tailer, background capture, UIA/HWND actuator), `crates/ai` (Gemini + OpenAI-compatible VLM + heuristics fallback), `crates/sdk` (optional IPC bridge) |
| **M3 & M4 — CLI & UI** | `1a954b2` | `#14`–`#16` | `crates/cli` (`run`, `self-test`, `windows`, `init-config`) and `crates/ui` (4-tab Live QA Dashboard UI) |
| **M5 & M6 — Hardening & Release** | `0b22f3b` | `#17`–`#19`, `#21`–`#23` | `hardening_tests.rs` NFR verification + `v0.1.0` Portable ZIP, Setup Wizard, Scoop & WinGet manifests published to GitHub Releases |

---

## 2. What Worked Well
1. **Milestone 0 Empirical Probe Caught a 2,000 ms Windows UIA Bottleneck Early:**
   - Running `ui-qa-probe --self-test` immediately revealed that calling `IUIAutomationElement::GetCurrentPattern(UIA_InvokePatternId)` on non-interactive Win32 `Window` (`50032`) and `TitleBar` (`50037`) elements triggered a `2,013 ms` MSAA proxy timeout.
   - Filtering pattern queries to interactive `ControlType` IDs dropped the UIA walk time from `2,023.0 ms` to **`10.14 ms`** (`200×` speedup) before `crates/platform` was built.
2. **Disjoint 7-Crate Workspace (`core`, `platform`, `ai`, `sdk`, `cli`, `ui`, `probe`):**
   - Keeping `#![forbid(unsafe_code)]` across `core`, `ai`, `sdk`, and `cli` isolated all Win32/COM/GDI `unsafe` blocks inside `crates/platform` and `crates/probe`.
3. **Built-in Background `SelfTestWindow`:**
   - Spawning a real Win32 `SW_SHOWNOACTIVATE` window with child buttons and simulated hardware log lines allowed deterministic automated testing of background capture, UIA inspection, `PostMessageW` clicks, and hardware log correlation in CI without hijacking the developer's focus.

---

## 3. What Went Wrong & Fixes Adopted
| Issue Encountered | Root Cause | Fix & Rule Adopted |
|---|---|---|
| `uia_walk_latency_ms` initially measured `2023.0 ms` in `M0.3` | Calling `GetCurrentPattern(UIA_InvokePatternId)` on `TitleBar` (`50037`) invokes legacy MSAA bridge timeout on background windows | Query action patterns only on interactive `ControlType` IDs (`50000`, `50002`, `50005`, `50007`, `50011`, `50013`, `50019`, `50025`, `50031`). |
| Rust 1.99 clippy errors (`clippy::chunks_exact_to_as_chunks`, `clippy::unnecessary_sort_by`, `clippy::unnecessary_to_owned`) | Rust 1.99 introduced stricter slice/iterator lints under `-D warnings` | Replaced `chunks_exact_mut(4)` with `as_chunks_mut::<4>().0` and `sort_by` with `sort_by_key(|a| Reverse(a.severity))`. |

---

## 4. Metrics & NFR Budgets Verified
- **Automated Tests:** `17` unit, integration, and soak tests passing across all workspace crates (`0` warnings).
- **NFR-P1 (Background Capture Latency p95):** `20.82 ms` release / `33.86 ms` debug (budget `≤ 50 ms`).
- **NFR-P2 (UIA Walk + Heuristics):** `18.88 ms` (budget `≤ 40 ms`).
- **NFR-P3 (Log Correlation Throughput):** `0.85 ms` per `1,000` lines (`8.52 ms` for `10,000` lines; budget `< 2 ms` / event).
- **NFR-P4 (Working-Set RAM):** `24.12 MB` (budget `≤ 35 MB` CLI / `≤ 120 MB` Dashboard).

---

## 5. Open Items (`Pending` Real-Environment Hardware QA)
- **GitHub Issue [`#20`](https://github.com/DooperDrew/ui-qa-tester/issues/20) (`M5.4`):** Real-environment validation (`QA-01`–`QA-07` in `docs/06-qa-and-validation.md`) against the user's production `iced`, `egui`, and `Slint` applications and physical hardware devices remains open in Milestone `M5` per the Claims Follow Evidence rule.
