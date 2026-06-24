# Samwise root-cause fixes, 2026-06-19

## Goal
Fix the two recurring worker failures that were only patched manually:

1. Claude Code OAuth 401 outage from stale copied credentials.
2. Post-merge deploy false negatives when superseded GitHub check-runs are cancelled.

## Task list
- [x] Confirm current worker state and open deployment window.
- [x] Scout the Claude Code spawn/auth path and deploy-green polling path.
- [x] Patch OAuth credential refresh at the worker root cause.
- [x] Patch cancelled check-run handling for superseded commits.
- [x] Add targeted tests where practical.
- [x] Run Rust checks/tests and frontend checks if touched.
- [x] Run reviewer pass.
- [x] Run codex-fix once at the end.
- [ ] Build production binary, verify embedded frontend assets.
- [ ] Deploy only if `in_progress = 0`.

## Evidence notes
- 2026-06-19: OAuth was restored by manually copying `~/.claude/.credentials.json` to `/home/mrchevyceleb/.config/autosam/claude-config/.credentials.json`, but no auto-refresh existed.
- 2026-06-19: `311e590d` had `md=failed` because `client-readiness` and `server-readiness` were `cancelled` on commit `033c662f`, while that commit was already an ancestor of current `origin/main`.
