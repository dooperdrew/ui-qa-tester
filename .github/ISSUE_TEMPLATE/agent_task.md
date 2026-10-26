---
name: "🤖 Agent Implementation Task"
about: Structured implementation task designed for GitHub Copilot Agent or autonomous worker subagents
title: "[Task]: "
labels: ["agent-task", "enhancement"]
assignees: []
---

## 🎯 Task Objective & Scope
<!-- A clear 1-2 sentence description of what needs to be implemented and why -->

## 📊 Complexity Tier & Target
- **Complexity Tier:** Tier 1 (Lean) | Tier 2 (Moderate) | Tier 3 (Complex / Critical)
- **Target Module / Package:** `crates/core` | `crates/platform` | `crates/ai` | `crates/sdk` | `crates/cli` | `crates/ui`
- **Target Files (Explicit Ownership):**
  - `path/to/target/file`
  - `path/to/test/file`

## 🛡️ Architectural & Performance Invariants
<!-- Specify any critical constraints (check all that apply) -->
- [ ] **Pure Logic:** Core logic isolated from side effects, 100% unit-tested (`#![forbid(unsafe_code)]`).
- [ ] **Performance Budgets:** Comply with non-functional budgets defined in `docs/02-requirements.md`.
- [ ] **Safety & Invariants:** Any unsafe operations in `crates/platform` must have documented `// SAFETY:` invariants.
- [ ] **Localization:** User-facing strings must reside in `locales/en.json` and `locales/it.json`.
- [ ] **Claims Follow Evidence:** Never claim a test or benchmark passed without running it.

## 📝 Implementation Instructions
<!-- Step-by-step guidance for the coding agent -->
1. 
2. 
3. 

## ✅ Verifiable Acceptance Criteria
<!-- Checkboxes that can be verified mechanically -->
- [ ] Unit tests added covering all new logic, branches, and error cases.
- [ ] `cargo fmt --all -- --check` passes cleanly.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` produces zero warnings or errors.
- [ ] `cargo test --workspace` passes 100%.
- [ ] `cargo build --workspace --release` succeeds.

## 🔄 Sole Committer Protocol Reminder
> **Note for AI Agents:** Deliver your changes via a **Draft Pull Request** on branch `task/<issue-id>-<description>`. Do not commit directly to `main` or attempt to close this issue. The Lead Agent (Antigravity) will review, run the integration verification gate, and merge.
