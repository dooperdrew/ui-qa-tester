# 06 — QA & Validation

## Verification levels
1. **Automated unit/invariant tests:** Run by the verification gate (`cargo test --workspace`) on every change.
2. **Automated budget/soak tests:** Quantitative benchmarks verifying `NFR-P1` through `NFR-P5` (latency, memory, log throughput, target FPS impact).
3. **Real-environment QA:** Executed on real `iced`, `egui`, and `Slint` Windows applications connected to physical hardware.
4. **System edge cases:** Target process crash/panic, hung window (`IsHungAppWindow`), DPI scaling (`100%`–`200%`), occluded window capture, and AI endpoint failure fallback.

> A QA row is **Pass** only with empirical evidence (date, environment, who ran it, commit hash). Until then it stays **Pending**.
> Releases explicitly list which rows are still **Pending**.

## NFR budget results
| ID | Metric | Budget | Measured | Date / commit | Method |
|---|---|---|---|---|---|
| NFR-P1 | Background 1080p capture latency (p95) | `≤ 50 ms` | *Pending (M5.1)* | — | `crates/platform` capture benchmark |
| NFR-P2 | UIA tree extraction + heuristic check (200 nodes, p95) | `≤ 40 ms` | *Pending (M5.1)* | — | `crates/platform` UIA cache benchmark |
| NFR-P3 | Log tailing & correlation overhead (`1,000 lines/s`) | `< 2 ms` / line, `≤ 1%` CPU | *Pending (M5.1)* | — | `crates/core` correlator benchmark |
| NFR-P4 | RAM working set (Headless CLI / Dashboard) | `≤ 35 MB` / `≤ 120 MB` | *Pending (M5.1)* | — | 50-step crawl memory sampler |
| NFR-P5 | Target `wgpu` app FPS impact during 1 Hz capture | `< 5%` FPS drop | *Pending (M5.1)* | — | Target frame-time telemetry comparison |

## Real-environment test matrix
| Test ID | Area | Procedure | Expected | Result | Date / env / tester |
|---|---|---|---|---|---|
| QA-01 | Occluded Background Capture (`egui`/`iced`/`Slint`) | Place target Rust GUI app behind IDE window; run 20-step capture in `ui-qa-tester`. | All 20 screenshots render full target UI (non-black, non-cropped) without bringing target to foreground. | Pending | — |
| QA-02 | Non-Intrusive Background Actuation | Run autonomous crawl while typing in code editor and moving physical mouse. | Physical cursor never jumps; active editor never loses focus; target controls are actuated via UIA / `PostMessageW` / SDK. | Pending | — |
| QA-03 | UI ↔ Hardware Log Correlation | Trigger a UI control that communicates with connected hardware (and simulate a disconnect/error). | `ui-qa-tester` links the exact `stdout`/`stderr`/log lines to the UI step and flags any silent hardware error. | Pending | — |
| QA-04 | AI Visual & UX Critic (Gemini API) | Run critic against a target screen with truncated text or poor layout. | Critic returns structured JSON finding with accurate bounding box, severity, and actionable Rust GUI improvement suggestion. | Pending | — |
| QA-05 | AI Visual & UX Critic (Local VLM) | Point provider to local Ollama/LM Studio vision endpoint and run screen critique. | Critic completes analysis offline and populates dashboard findings. | Pending | — |
| QA-06 | Goal-Driven Scenario Runner | Provide a 3-step plain-English hardware test goal. | Agent navigates the UI, verifies log output at each step, and marks scenario Pass/Fail with evidence. | Pending | — |
| QA-07 | Agent-Ready Report & GitHub Issue Export | Export session report and trigger GitHub Issue creation on test repo. | `report.md`, `report.json`, and annotated `screenshots/` generated; GitHub Issue contains repro steps + log snippet. | Pending | — |

## Edge cases
| Test ID | Invariant | Procedure | Expected | Result |
|---|---|---|---|---|
| QA-SYS-01 | Target App Panic / Crash Recovery | Force target `.exe` to `panic!()` mid-crawl. | Watchdog captures exit code + panic backtrace from `stderr`, logs Critical finding, and terminates session cleanly without hanging. | Pending |
| QA-SYS-02 | Target Window Freeze (`IsHungAppWindow`) | Block target UI thread for `> 5 s`. | Watchdog detects hung window via `IsHungAppWindow` / `SendMessageTimeoutW`, captures frozen frame, and reports UI hang defect. | Pending |
| QA-SYS-03 | High-DPI Scaling (`125%` / `150%` / `200%`) | Run capture + bounding-box annotation on a high-DPI monitor. | UIA `BoundingRectangle` and Vision coordinates align within `±2 px` of rendered controls on the captured frame. | Pending |
| QA-SYS-04 | Minimized Target Window Prevention | Minimize target window during an active background QA run. | Runner detects `IsIconic(hwnd)`, restores window in background (`SW_SHOWNOACTIVATE`) without stealing focus, and continues capture. | Pending |
| QA-SYS-05 | AI Provider Network Failure / Rate Limit | Disconnect network or return HTTP 429 during Gemini critique. | Client retries with exponential backoff; if unreachable, degrades gracefully to deterministic local heuristics without aborting crawl. | Pending |

## Incident protocol
Any discrepancy: check `docs/m0-spike-findings.md` → if it's new, write a probe report + open an issue labelled `bug` → add a regression unit/integration test.
