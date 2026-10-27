---
name: "🤖 Agent Task (Markdown)"
about: "Bounded implementation task for GitHub Copilot Agent or local project-worker subagent"
title: "[Task] M<x>.<y>: <Short Summary>"
labels: ["agent-task"]
---

## Task ID & Complexity Tier
- **Roadmap ID:** `M<x>.<y>`
- **Tier:** `[T1 Lean | T2 Moderate | T3 Critical]`
- **Target Model:** `[flash_lite | flash | pro | inherit]`

## Owned Files (Strict Scope)
- `crates/...`

## Invariants & Hard Rules
- `#![forbid(unsafe_code)]` in `crates/core`, `crates/ai`, `crates/sdk`, and `crates/cli`.
- Every `unsafe` block in `crates/platform` must have a `// SAFETY:` comment.
- Never call `SetForegroundWindow`, `SetCursorPos`, or `SendInput` in default `focus_mode = "background"`.
- Deliver via Draft PR on `task/<issue-id>-<slug>` (Sole Committer Protocol).

## Acceptance Criteria
- [ ] Implementation complete in owned files
- [ ] Unit tests added/updated
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
