# Milestone 0 Spike Findings (`ui-qa-probe`)

> **Date:** 2026-10-26  
> **Probe Binary:** `crates/probe` (`ui-qa-probe.exe --self-test --iterations 20`)  
> **Status:** Empirical ground truth verified on Windows 10/11 (`x86_64-pc-windows-msvc`).

---

## 1. M0.2 Spike — Background Window Capture (`PrintWindow` vs `Windows.Graphics.Capture`)

### Empirical Measurements (Release Build, 640×420 Background Window `SW_SHOWNOACTIVATE`)
| Metric | Measured Result | NFR Budget | Status |
|---|---|---|---|
| `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` (`0x2`) | `true` (Succeeded while unfocused in background) | Must succeed without focus change | **PASS** |
| Capture Latency (`p50`, release) | **16.53 ms** | `≤ 50 ms` (`NFR-P1`) | **PASS** |
| Capture Latency (`p95`, release) | **20.82 ms** | `≤ 50 ms` (`NFR-P1`) | **PASS** |
| Non-black Pixel Ratio | **0.96** (`96.0%` valid painted pixels) | `> 0.50` | **PASS** |
| Foreground Focus Unchanged (`GetForegroundWindow`) | `true` | `NFR-S1` | **PASS** |
| `Windows.Graphics.Capture` OS Support (`IsSupported`) | `true` | — | Verified |

### Key Technical Findings & Architecture Decision
1. **`PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` (`0x00000002`)** asks DWM to composite the window's redirection surface (including DirectComposition / GPU surfaces) into a memory DC synchronously in **16.5 ms (p50)** / **20.8 ms (p95)** without requiring a WinRT `DispatcherQueue` or drawing yellow WGC capture borders around the window.
2. **Minimized Window Caveat Confirmed:** If a window is minimized (`IsIconic(hwnd) == true`), Windows DWM stops updating its surface. Calling `ShowWindow(hwnd, SW_SHOWNOACTIVATE)` restores the window behind existing windows without stealing focus.
3. **Production Pipeline (`crates/platform`):** Use `PrintWindow(..., PW_RENDERFULLCONTENT)` as the fast, synchronous, zero-border primary capture engine (`~17 ms`), with `BitBlt(SRCCOPY)` and `Windows.Graphics.Capture` as secondary fallbacks.

---

## 2. M0.3 Spike — AccessKit / `IUIAutomation` Tree & Non-Intrusive Input Injection

### Empirical Measurements
| Metric | Measured Result | NFR Budget | Status |
|---|---|---|---|
| `IUIAutomation` Tree Walk Latency (after filtering non-invokable MSAA titlebar queries) | **10.14 ms** | `≤ 40 ms` (`NFR-P2`) | **PASS** |
| Discovered Elements (`ControlViewWalker`) | Root window (`50032`), `"Calibrate Sensor"` button (`50000`), TitleBar (`50037`) | All visible controls discovered | **PASS** |
| Tier 1: `IUIAutomationInvokePattern::Invoke()` | `true` (`WM_COMMAND` counter incremented to `1` in background) | Must actuate control in background | **PASS** |
| Tier 2: `PostMessageW` (`WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_CHAR`) | `true` (Target window received all 4 events at exact `(120, 80)` client coords) | Must deliver input in background | **PASS** |
| Physical Cursor Position Unchanged (`GetCursorPos`) | `true` (`Δx = 0, Δy = 0`) | `NFR-S1` | **PASS** |
| Foreground Window Unchanged (`GetForegroundWindow`) | `true` | `NFR-S1` | **PASS** |

### Critical Discovery: Avoiding the 2,000 ms UIA MSAA Bridge Timeout
- **What happened:** In our first probe run, calling `el.GetCurrentPattern(UIA_InvokePatternId)` indiscriminately on **every** node—specifically the root `Window` (`50032`) and Win32 `TitleBar` (`50037`)—caused a **2,013 ms** delay (`2023.0 ms` total walk time) due to Windows UIA's legacy MSAA proxy timeout on non-client titlebar elements.
- **Fix verified:** Restricting `GetCurrentPattern(UIA_InvokePatternId)` strictly to interactive control types (`Button = 50000`, `CheckBox = 50002`, `Hyperlink = 50005`, `ListItem = 50007`, `MenuItem = 50011`, `RadioButton = 50013`, `TabItem = 50019`, `Custom = 50025`, `SplitButton = 50031`) reduced the entire UIA tree walk + invoke from **2,023.0 ms** down to **10.14 ms** (a **200× speedup**).
- **Rule adopted for `crates/platform`:** Never call `GetCurrentPattern` for action patterns on structural/non-client `ControlType` IDs (`Window = 50032`, `TitleBar = 50037`, `Pane = 50033`, `Group = 50026`).
