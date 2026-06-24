# Change: Disable auto-merge → button-driven comprehensive review + fix + merge

Requested 2026-06-19 (temporary). Worker is stopped while this is built.

## Goal
1. No more auto-merge to main. Green cards park in **Ready to Merge** (`approved`).
2. A button on `approved` cards fires a **fire-and-forget** flow through Sam (Opus 4.8):
   comprehensive review (correctness, regressions, UI, UX, blind spots, security) → **fix everything broken** → **merge to main with `--admin`** + deploy.
3. If Sam hits a blocker it can't safely fix → **Stop, move card to Fixes Needed** (never merge broken code).
4. Decision: the new button **replaces** the plain "Merge + Deploy" button.

## Tasks
- [x] Disable both auto-merge paths
  - [x] `autoMergeOnApproved` default `true`→`false` in `spawn_pr_review_task` MergeNow branch (worker.rs ~9660)
  - [x] settings.json: `autoMergeEnabled=false`, `autoMergeOnApproved=false`
- [x] Backend: new `samwise_review_merge_*` context keys (worker.rs ~413)
- [x] Backend: `sweep_review_merge_requests` + `spawn_review_merge_task` + verdict parser (model on spawn_auto_fix_task; chain into start_merge_deploy_task)
- [x] Backend: register sweep in heartbeat loop (worker.rs ~2156)
- [x] Frontend: `review-actions.ts` keys + state + `requestReviewMergeContext` + label/busy helpers
- [x] Frontend: KanbanCard.svelte — replace Merge+Deploy button with "Review & Merge"
- [x] Frontend: TaskDetailModal.svelte — same replacement
- [x] cargo check + npm run check
- [~] UI: Tauri WebView (not Playwright-drivable headless); validated via svelte-check + logic mirror of existing button. Live click best verified by Matt.
- [x] Build + deploy to /usr/bin/agent-one + restart service (live)
- [x] Disabled external `codex-pr-review-batch` cron (id 410074cd, every 4h) — was the 3rd auto-merge path

## Verdict contract
Sam's comprehensive pass ends its final message with exactly:
`REVIEW_MERGE_VERDICT: ready`  (safe to merge after fixes) or
`REVIEW_MERGE_VERDICT: blocked`  (needs Matt; do not merge).
Worker defaults to **blocked** if no explicit `ready` line (safe default).

## Codex-fix result
- FIXED (critical): reviewed-head guard in spawn_review_merge_task — only admin-merge the exact local SHA Sam reviewed; fail closed if GitHub PR head differs (external push during review). worker.rs.
- FLAGGED (pre-existing, not changed): `gh_merge` retry on 'Head branch was modified' refetches+merges the new head (review.rs ~867-880), shared by all merge paths. Hardening follow-up; not touched to avoid regressing the legitimate self merge-in retry.
