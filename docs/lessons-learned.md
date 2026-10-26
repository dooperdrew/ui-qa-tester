# Lessons Learned

Living log of things that went wrong or surprisingly right, and the rule they produced.
Generic lessons (not tied to this project) are tagged **[KIT]** and get promoted into the
`project-kickstart` plugin during the retrospective.

| Date | Context (task / conversation) | What happened | Rule / fix adopted | Scope |
|---|---|---|---|---|
| 2026-10-26 | Project Kickstart Discovery | Target apps use Rust native GUIs (`iced`, `egui`, `Slint`) rendered via `wgpu`/`winit` rather than HTML DOMs, and user requires hands-free testing without stealing mouse/keyboard focus. | Designed 3-tier non-intrusive actuation (UIA patterns → `PostMessageW` to `HWND` → optional `ui-qa-sdk` named pipe) + background `Windows.Graphics.Capture` / `PrintWindow` instead of foreground `SendInput`. | project |
