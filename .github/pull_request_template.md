## Description
<!-- Provide a brief summary of the changes introduced in this PR and their purpose. -->
<!-- Link the roadmap task (e.g. M1.2) and/or issue number (#N). -->

Closes #<!-- issue number -->

## Type of Change
- [ ] 🤖 Agent implementation task (Draft PR)
- [ ] 🐛 Bug fix (non-breaking change fixing an issue)
- [ ] ✨ New feature (non-breaking change adding functionality)
- [ ] 📝 Documentation update
- [ ] ⚡ Performance improvement / hardening
- [ ] ♻️ Refactor (no behavioral changes)

## Requirements Touched
<!-- Specify requirement IDs from docs/02-requirements.md e.g., F1.2, NFR-P3 -->

## Contributor & Agent Checklist
Before submitting or merging this pull request, verify:
- [ ] **Verification Gate:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace --release` pass cleanly.
- [ ] **Budgets Unaffected:** Non-functional performance and memory budgets in `docs/02-requirements.md` are respected.
- [ ] **Safety & Invariants:** `#![forbid(unsafe_code)]` preserved outside `crates/platform`; any `unsafe` blocks in `crates/platform` carry `// SAFETY:` comments.
- [ ] **No Hardcoded Strings:** User-facing strings use `locales/en.json` and `locales/it.json`.
- [ ] **Claims Follow Evidence:** Automated unit tests pass; manual or hardware QA is marked `Pending` until confirmed.
- [ ] **Sole Committer Protocol:** AI agents deliver changes as a **Draft PR** on a `task/<issue-id>-<description>` branch. Only the Lead Agent merges to `main`.

## Documentation Updates
- [ ] Roadmap task ticked in `docs/05-roadmap.md` (upon lead merge)
- [ ] CHANGELOG entry added
- [ ] Architecture / ADR updated (if design changed)
- [ ] Lessons learned recorded in `docs/lessons-learned.md` (if any friction occurred)
