# Lessons Learned

Living log of things that went wrong or surprisingly right, and the rule they produced.
Generic lessons (not tied to this project) are tagged **[KIT]** and get promoted into the
`project-kickstart` plugin during the retrospective.

| Date | Context (task / conversation) | What happened | Rule / fix adopted | Scope |
|---|---|---|---|---|
| 2026-10-26 | Project Kickstart Discovery | Target apps use Rust native GUIs (`iced`, `egui`, `Slint`) rendered via `wgpu`/`winit` rather than HTML DOMs, and user requires hands-free testing without stealing mouse/keyboard focus. | Designed 3-tier non-intrusive actuation (UIA patterns → `PostMessageW` to `HWND` → optional `ui-qa-sdk` named pipe) + background `Windows.Graphics.Capture` / `PrintWindow` instead of foreground `SendInput`. | project |
| 2026-10-26 | M0.3 UIA Spike (`crates/probe`) | Calling `IUIAutomationElement::GetCurrentPattern(UIA_InvokePatternId)` on structural/non-client Win32 elements (`Window = 50032`, `TitleBar = 50037`) triggered a 2,013 ms MSAA proxy timeout. | Filter `GetCurrentPattern` queries by interactive `UIA_ControlType` IDs (`50000`, `50002`, `50005`, `50007`, `50011`, `50013`, `50019`, `50025`, `50031`), reducing UIA walk latency from `2023.0 ms` to `10.14 ms`. | [KIT] |
| 2026-10-26 | M0.2 Capture Spike (`crates/probe`) | Rust 1.99 clippy enforces `clippy::chunks_exact_to_as_chunks` under `-D warnings` when iterating over BGRA pixel buffers by 4 bytes. | Use `bgra_buf.as_chunks_mut::<4>().0` instead of `chunks_exact_mut(4)` for zero-warning RGBA swizzling. | [KIT] |
