# Model Fallback Log

Every time a task ran on a lower-tier model than its tier required (quota, rate limit, unavailability),
it gets logged here. Each entry creates an audit task in the hardening milestone.

| Date | Task | Target model | Fallback used | Reason | Audit status | Notes |
|---|---|---|---|---|---|---|
| — | — | — | — | — | N/A | No fallbacks yet. |

## Audit workflow
1. Log the entry. 2. Add an audit task to the roadmap. 3. A higher-tier model reviews edge cases, budgets and safety.
4. Set the status to `AUDITED & PASSED` or `REFACTORED`.
