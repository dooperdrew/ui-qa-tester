# AGENTS.md — Mandatory Operating Guidelines for AI Agents

> You are working on **ui-qa-tester** (`https://github.com/DooperDrew/ui-qa-tester`).
> These rules are binding for every agent and every conversation in this repository.

## 1. Mission
**ui-qa-tester** is an autonomous, non-intrusive Windows UI and Hardware-in-the-Loop (HIL) QA agent built for Windows desktop applications (specifically targeting Rust native GUIs: `iced`, `egui`, and `Slint`). It launches or attaches to a target `.exe`, explores screens and executes plain-English test goals in the background without stealing the developer's physical mouse or keyboard focus, correlates every UI interaction with real-time hardware logs (`stdout`, `stderr`, diagnostic log files) and process health (CPU, RAM, hangs, crashes), uses multimodal AI (Gemini API or local OpenAI-compatible VLMs) plus Windows UI Automation (UIA) to detect visual/layout bugs and propose concrete UX improvements, and exports agent-ready Markdown/JSON reports, annotated screenshots, visual regression diffs, and GitHub Issues.

## 2. Mandatory reading order (before any implementation)
1. `docs/01-vision.md`: problem, principles, non-goals
2. `docs/02-requirements.md`: requirement IDs (`F…`, `NFR-…`) and budgets
3. `docs/03-architecture.md`: components, boundaries, key designs
4. `docs/04-ux.md`: UI/UX spec for the Tauri v2 Live QA Dashboard and CLI
5. `docs/05-roadmap.md`: **the single source of truth for tasks**
6. `docs/06-qa-and-validation.md`: verification matrices, NFR measurements, and empirical testing
7. `docs/m0-spike-findings.md` (created in M0.4): verified ground truth from M0 spikes. Don't contradict or guess beyond it.
8. `docs/decisions/`: ADRs. Changing a decision requires a new ADR.
9. `docs/multi-agent-methodology.md`: parallel worker orchestration, Copilot cloud integration, and reporting templates
10. `docs/lessons-learned.md` and `CHANGELOG.md`: history and pitfalls to avoid

## 3. Continuous Learning & Self-Correction Rule
Whenever the user corrects, redirects, or clarifies a requirement:
- **Do not silently apply the change.**
- You MUST explicitly formulate the takeaway as a rule and propose writing it into `docs/lessons-learned.md` (or `AGENTS.md` if binding).
- This ensures that lessons learned in one chat session immediately persist for all subsequent sessions.

## 4. Roles (multi-agent)
- **Lead agent (sole committer):** Antigravity interactive session. The **only** entity allowed to `git commit`, `git push`, tag, release, merge PRs, or close issues. Plans parallel work, creates bounded tasks in GitHub Issues, reviews worker/Copilot output, runs the verification gate, and syncs docs.
- **Worker subagents (`project-worker`):** Autonomous local subagents working in isolated branch workspaces (`Workspace: "branch"`) on **disjoint files**. **Forbidden:** `git commit`, `git push`, `git checkout main`, closing issues. They deliver results via the completion report in §8.
- **Cloud Worker agent (GitHub Copilot Agent):** Operates asynchronously in GitHub Cloud or IDE on bounded issues created via `.github/ISSUE_TEMPLATE/agent_task.yml`. Bound by `.github/copilot-instructions.md`. Delivers work exclusively via **Draft Pull Requests** on `task/<issue-id>-<description>`. **Forbidden:** pushing directly to `main`, merging PRs, or closing issues.
- Worker role names follow `<Tier>-<Domain>-<Milestone>` (e.g. `T1-Core-M1`).

## 5. Complexity tiers & model selection
| Tier | Scope | Model |
|---|---|---|
| T1 Lean | Pure logic (`crates/core`), schemas, report formatters, unit tests, i18n, docs, packaging scripts | `flash_lite` / `flash` |
| T2 Moderate | Tauri v2 UI (`crates/ui`), AI client (`crates/ai`), SDK IPC (`crates/sdk`), standard CLI | `flash` / `pro` |
| T3 Critical | Win32 / COM UIA (`IUIAutomation`), `Windows.Graphics.Capture`, `HWND` background input injection, process watchdog & concurrency (`crates/platform`) | `pro` / `inherit` |

Fallback order: `inherit` → `pro` → `flash` → `flash_lite`. Every fallback **must** be logged in
`docs/model-fallback-log.md`, and a review task for it goes into the hardening milestone.

## 6. Task lifecycle
**Before:** find the task in the roadmap, restate the task + tier + target files + acceptance criteria, and check that `git status` is clean. Ensure toolchain PATH is initialized.
**During:** respect the hard rules (§7). Unverified assumptions are marked 🔬 and probed, not guessed. Decouple headless tests from physical QA (keep manual QA as Pending).
**Draft PR Review (for Copilot / Worker tasks):** Lead pulls branch locally, runs the full verification gate, inspects diff for ownership and safety invariants, refines if needed, merges to `main`, and closes the corresponding issue.
**Gate (must pass before finishing):**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```
**After (lead only):** tick the roadmap box (date + commit), add a CHANGELOG entry, update docs/ADRs if the design changed,
add any lesson to `docs/lessons-learned.md`, commit (`M<x>.<y>: <summary>`), push, and update the issue.

## 7. Hard rules
- **Core Purity (`crates/core` & `crates/sdk`):** `#![forbid(unsafe_code)]` is mandatory in `crates/core`, `crates/ai`, `crates/sdk`, and `crates/cli`. `crates/core` must have zero Win32 or network dependencies so all state-machine, log-correlation, diffing, and report-generation logic is 100% unit-testable on any host.
- **Win32 & COM Safety (`crates/platform`):** `unsafe` is permitted **only** inside `crates/platform` (and `crates/probe` during M0) for Win32/COM/WGC calls, and every `unsafe` block MUST carry a `// SAFETY:` comment explaining pointer validity, thread affinity (STA/MTA COM initialization), and handle lifetime invariants.
- **Non-Intrusive Focus Rule:** By default (`focus_mode = "background"`), `ui-qa-tester` MUST NOT call `SetForegroundWindow`, `SetCursorPos`, or `SendInput` that hijacks the user's physical cursor or active window focus. Foreground input simulation is strictly opt-in per test run (`focus_mode = "foreground_fallback"`).
- **Performance Budgets:** Adhere strictly to NFR-P1 through NFR-P5 in `docs/02-requirements.md` (capture latency ≤ 50 ms, idle RAM ≤ 80 MB for engine + UI, log correlation overhead < 2 ms per event, zero target-app frame drops > 5%).
- **Secret Hygiene:** Never log, commit, or embed API keys (`GEMINI_API_KEY`, `OPENAI_API_KEY`, `GITHUB_TOKEN`) in reports, screenshots, or config files committed to git. Read credentials strictly from environment variables or OS credential storage.
- **GitHub Governance (Milestones, Issues & Releases):** Every roadmap milestone (`M0`..`Mn`) MUST have a corresponding GitHub Milestone and tracked GitHub Issues. Completed tasks close their issue with a commit/release reference; pending hardware/manual QA stays open in its milestone. Every release MUST attach a pre-built downloadable bundle (`.zip` / installer) on GitHub Releases via `gh release create` even while physical hardware QA remains `Pending`.
- **Claims follow evidence:** Never claim a test, benchmark, or QA item passed without having run it. Hardware/manual QA stays `Pending` until the user confirms.
- **Keep claims in docs factual:** Avoid marketing language ("pioneering", "zero bugs") in technical specs while QA is pending.
- **Environment resiliency:** Never assume standard toolchains are in `$PATH`. Prepend `$env:USERPROFILE\.cargo\bin` (or relevant toolchain) in PowerShell commands to guard against PATH omissions.
- **Test concurrency:** Tests touching global OS resources (named pipes, temp log files, dummy windows) must use unique UUID names and timeout protection to prevent test hangs.
- **Windows / PowerShell hygiene:** When writing files from PowerShell, force UTF-8 without BOM (`[System.IO.File]::WriteAllText(..., [System.Text.UTF8Encoding]::new($false))` or `Set-Content -Encoding utf8`) and set `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` before capturing external CLI stdout (`gh`, `git`). Prefer dedicated file-writing tools over shell redirection. Use `;` rather than `&&` on Windows PowerShell 5. Avoid unconstrained recursive searches over dependency caches (`node_modules`, `target`, `.venv`).
- User-facing strings go through i18n files (`locales/en.json`, `locales/it.json`). Don't hardcode UI text.
- Files are UTF-8. Never commit build artifacts (e.g. `dist/`, `target/`); releases go to GitHub Releases.

## 8. Worker completion report (required format)
```markdown
### WORKER COMPLETION REPORT: <Role>
- Task: <ID + description>
- Model used: <model> (fallback? yes/no + reason)
- Files created/modified: <list>
- Verification: <each gate command: PASS/FAIL, test counts>
- Decisions & rationale: <numbered>
- Open issues / risks / follow-ups: <list>
```

## 9. Escalation
If reality contradicts the docs (tests, hardware, external API behaviour): **stop**, report the evidence to the user,
propose a probe, and update the docs only after the user agrees.

## 10. Build & run
```powershell
# Initialize toolchain PATH
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:APPDATA\npm;$env:ProgramFiles\nodejs;$env:PATH"

# Run M0 diagnostic probe against a target executable or window title
cargo run -p ui-qa-probe -- --exe "path\to\target_app.exe"

# Run headless CLI QA session
cargo run -p ui-qa-cli -- run --config qa-session.toml

# Launch Tauri v2 Live QA Dashboard UI
cargo run -p ui-qa-ui
```
