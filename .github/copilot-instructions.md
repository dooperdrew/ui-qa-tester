# Operating Rules & Instructions for GitHub Copilot

> **Repository:** `https://github.com/DooperDrew/ui-qa-tester`  
> **Role of GitHub Copilot:** Worker Subagent / Fast Draft Implementer  
> **Lead Orchestrator & Sole Committer:** Antigravity (Local Lead Agent)

Whenever working on this repository — whether via Copilot Agent, Copilot in VS Code / IDE, or GitHub Pull Request review — you MUST strictly comply with the guidelines, architecture, performance budgets, and Sole Committer protocol defined below and in [`AGENTS.md`](../AGENTS.md).

---

## 1. Division of Labor & Sole Committer Protocol

This project enforces a strict **Hub-and-Spoke (Lead Orchestrator & Worker)** multi-agent model:

- **Fast Draft Implementer:** Consume bounded, self-contained GitHub Issues created from `.github/ISSUE_TEMPLATE/agent_task.yml`, generate scaffolding, implement core logic, create UI views, and write comprehensive unit tests.
- **Draft Pull Requests Only:** Work strictly on dedicated feature branches (`task/<issue-id>-<description>`) and submit **Draft Pull Requests**.
- **PROHIBITED:**
  - ❌ Never execute `git push origin main` or attempt to commit directly to `main`.
  - ❌ Never merge pull requests into `main`.
  - ❌ Never close GitHub Issues.
  - Only the Lead Agent (Antigravity) is authorized to run the final release gate, merge branches to `main`, update project tracking, and close issues.

---

## 2. Architecture & Module Separation Boundaries

To allow parallel development across multiple agents without merge conflicts:
1. **Disjoint Module Boundaries:**
   - Independent tasks must touch disjoint crates (`crates/core`, `crates/platform`, `crates/ai`, `crates/sdk`, `crates/cli`, `crates/ui`, `crates/probe`). Check the **Target Files & Ownership** section of your assigned GitHub Issue.
   - Do not edit files outside your assigned ownership scope without explicit instruction.
2. **Pure Core Logic vs. Platform/IO:**
   - Keep `crates/core`, `crates/ai`, `crates/sdk`, and `crates/cli` `#![forbid(unsafe_code)]`.
   - `crates/core` must remain 100% pure, deterministic, and isolated from Win32 / network I/O.
3. **Zero Hardcoded Strings:**
   - All user-facing UI text, error messages, and labels must go through localization / i18n dictionaries (`locales/en.json`, `locales/it.json`).

---

## 3. Non-Negotiable Performance & Safety Budgets

Every contribution by Copilot MUST respect the non-functional requirements defined in [`docs/02-requirements.md`](../docs/02-requirements.md):

1. **Performance Budgets (NFR-P1..P5):**
   - Background 1080p capture latency `≤ 50 ms` (p95); UIA tree walk `≤ 40 ms`; log correlation `< 2 ms` per line; zero focus stealing in default `background` mode.
2. **Safety & Invariants (NFR-S1..S4):**
   - Avoid `unwrap()` / `expect()` on fallible runtime calls; return typed `Result<T, Error>`.
   - `unsafe` is permitted ONLY in `crates/platform` (and `crates/probe` during M0) and MUST carry a `// SAFETY:` comment documenting preconditions, COM apartment state, and handle lifetimes.
3. **Claims Follow Evidence Rule:**
   - Mark automated unit, integration, and invariant tests as **PASS** only when verified by the verification gate.
   - Never mark physical hardware, third-party cloud credentials, or manual QA as passed without empirical proof. Keep manual QA rows marked **`Pending`**.

---

## 4. Verification Gate & Environment Hygiene

Before submitting a Draft PR or marking an implementation complete, run the project verification gate:
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

---

## 5. Draft PR Submission Checklist

When opening your Draft Pull Request:
- [ ] PR title formatted as `task/<issue-id>: <concise description>`
- [ ] Base branch is set to `main`
- [ ] PR marked as **Draft**
- [ ] PR description links the originating issue (`Closes #N`)
- [ ] All verification gate commands pass cleanly
- [ ] No files outside the assigned ownership scope were touched
