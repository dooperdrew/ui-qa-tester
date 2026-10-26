# Contributing to ui-qa-tester

Thank you for your interest in contributing to **ui-qa-tester**! This project follows a spec-first, agent-ready development methodology designed to support seamless collaboration between human developers, local AI pair programmers (Antigravity), and cloud worker agents (GitHub Copilot).

---

## 🧭 Core Principles

1. **Specs Come First:** No application code should be modified or added without a corresponding task in [`docs/05-roadmap.md`](docs/05-roadmap.md) and requirements defined in [`docs/02-requirements.md`](docs/02-requirements.md).
2. **Claims Follow Evidence:** Never submit or approve pull requests claiming performance gains or hardware compatibility without empirical benchmark results recorded in [`docs/06-qa-and-validation.md`](docs/06-qa-and-validation.md).
3. **Continuous Learning:** When resolving bugs, edge cases, or tool friction, document the root cause and the rule created in [`docs/lessons-learned.md`](docs/lessons-learned.md).
4. **Sole Committer Protocol:** To prevent merge collisions and broken release states, only the Lead Agent / repository maintainer merges to `main` and creates releases.

---

## 🛠️ Development & Pair Programming Workflow

### 1. Branch Strategy
- Work on feature branches branched from `main`.
- Naming convention: `<type>/<milestone>-<short-description>` (e.g., `feat/m1-skeleton`, `fix/m2-log-correlator`).
- When working with AI worker agents (Copilot / subagents), branches follow `task/<issue-id>-<description>`.

### 2. Task Lifecycle & Cloud Agent Collaboration
```
[ Maintainer / Lead Agent ] ──► Creates GitHub Issue (agent_task.yml)
                                              │
                                              ▼
                             [ GitHub Copilot Agent (Cloud) ]
                                              │
                                              ▼
                             Opens Draft PR (task/<issue-id>-<desc>)
                                              │
                                              ▼
[ Maintainer / Lead Agent ] ──► Runs Verification Gate locally & Merges to main
```

Before starting any task:
1. Identify the task ID in [`docs/05-roadmap.md`](docs/05-roadmap.md).
2. Confirm the complexity tier (T1 Lean, T2 Moderate, T3 Critical) defined in [`AGENTS.md`](AGENTS.md).
3. Ensure target files are disjoint from other concurrently active tasks.

### 3. Verification Gate
Before committing or submitting a pull request, the project verification gate must pass cleanly:
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

### 4. Commits & Changelog
- Use Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- For milestone progress commits, prefix with the milestone task ID: `M<x>.<y>: <summary>` (e.g., `M1.1: workspace skeleton and core traits`).
- Add notable changes to [`CHANGELOG.md`](CHANGELOG.md).

---

## 🤖 Guidelines for AI Agents

All AI assistants (Antigravity, GitHub Copilot, Claude Code, Cursor, etc.) must follow [`AGENTS.md`](AGENTS.md) and [`.github/copilot-instructions.md`](.github/copilot-instructions.md) as binding instructions:
- **Lead Agent (Antigravity):** Operates as the **sole committer** when interacting with Git remotes. Manages issues and merges pull requests.
- **Worker Subagents & Copilot Agent:** Strictly forbidden from running `git commit`, `git push`, or merging directly to `main`. Deliver work exclusively via **Draft Pull Requests** on `task/<issue-id>-<description>`.
- **Maintain documentation integrity:** Do not remove existing comments, docstrings, or structure templates unless instructed.
- **Windows / PowerShell hygiene:** Force UTF-8 encoding when writing files on Windows (`Set-Content -Encoding utf8`). Avoid unconstrained recursive searches over dependency caches (`node_modules`, `target`, `.venv`).
