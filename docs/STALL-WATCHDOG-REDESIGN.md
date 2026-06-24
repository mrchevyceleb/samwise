# Merge+Deploy stall recovery — redesign (2026-06-17)

Codex flagged 3 P1s on the first "external watchdog" (requeue-to-`requested`) approach:
- P1: requeue races a still-alive in-process task holding the per-repo `MERGE_DEPLOY_LOCKS` mutex, AND abandons the 90-min `running` backstop → deadlock.
- P1: 12-min window false-positives — pre-merge does `wait_for_ci` TWICE (≤15m each) so a legit run can take ~32m; `mergeable` ≠ CI-green.
- P1: retry dropped `expected_head_sha` → could merge an unreviewed post-review commit. (Also a pre-existing hole: the queue picker always passed `None`.)

## Correct design (replaces the watchdog)

1. [x] Delete external watchdog: consts `MERGE_DEPLOY_PREMERGE_STALL_SECS`/`MERGE_DEPLOY_MAX_RETRIES`(old), helpers `merge_deploy_premerge_stalled`/`requeue_stalled_merge_deploy`/`merge_deploy_retry_count`, and the sweep-loop block. Revert sweep to original (90-min `stale_running` only).
2. [x] Internal pre-merge timeout: wrap ONLY the pre-merge phase of `run_merge_deploy_workflow` in `tokio::time::timeout(MERGE_DEPLOY_PREMERGE_TIMEOUT_SECS = 40*60)`. Above the ~32m double-CI-wait ceiling, well under the 90-min backstop. On elapse → `MergeDeployError::premerge_timeout(...)` (pr_merged=false). Lock releases when the spawned task ends.
3. [x] Auto re-request ONLY on `PremergeTimeout`, capped at `MERGE_DEPLOY_MAX_RETRIES=2`, done in the Err handler AFTER the lock is dropped (no race). Sets status=approved + merge_deploy_status=requested + bumps `samwise_merge_deploy_retries` + preserves `samwise_merge_deploy_expected_head_sha`. Other errors park as before. At cap → park in approved w/ "needs a human" note.
4. [x] Persist + thread `expected_head_sha`: store in context (`samwise_merge_deploy_expected_head_sha`) when a request is created with Some; queue picker reads it and passes Some(sha) instead of None.

## Codex re-review round 2 (4 P1 + 1 P2) — ALL FIXED
- P1-1 timeout left git/gh child running (no kill_on_drop) → added `.kill_on_drop(true)` to run_git, run_git_capture, gh_merge (all `.output().await`, never detached; global rejected because dev_server spawns a detached npm child).
- P1-2 40min < real ~55min ceiling (2x CI 15m + conflict-Claude 20m) → raised to 70min (still < 90min backstop).
- P1-3 not every queue path carried the reviewed SHA → (a) autoMergeOnApproved stamp now fetches+persists head SHA; (b) conflict-fix retry passes resolved head; (c) picker reads context SHA; (d) Err retry preserves it. UI/manual merges keep relying on first-parent guard + gh --match-head-commit (not fail-closed, to not break the manual button).
- P1-4 retry forced status=approved (could upgrade deploy-only card) → retry now omits status (preserves row) + only re-fires {approved,fixes_needed,review}.
- P2 ignored update result → retry matches Result; Ok→comment+return, Err→log+fall through to failed park.

## Codex round 3 verdict: P1-1/P1-2/P1-4/P2 CLOSED; P1-3 partial → now finished:
- P1-3 QA-pass stamp (~3164): added SHA fetch+persist.
- P1-3 conflict-fix (~10120): persist resolved SHA in the SAME requested-write (closes picker-race window).
- P1-3 auto-approve stamp (~9567): best-effort SHA fetch (residual: queues on gh-fetch failure, defended by gh_merge --match-head-commit, staging-only — ACCEPTED).
- Round-3 new minor P2 (~11572, premerge-retry preserves reviewed SHA before self-merge-in head): non-destructive, 90-min backstop covers — ACCEPTED.

## BONUS (live bug found via samcheck 2026-06-17): post-merge-finalize hang
- 5f84febd / PR #581 merged 19:25Z but its merge-deploy died during post-merge finalize, leaving md=running 51min, wedging the r-link slot and starving 104a1900/c84e28cb. Hand-fixed (finalized→done) AND added permanent fix: the 90-min stale_running handler now gh_pr_is_merged-checks; if merged → close Done+succeeded (deploy is automatic via Vercel action) instead of mislabeling failed. Releases the slot either way.

## Codex round 4 (delta verify) — 2 P1 + 1 P2, ALL FIXED
- P1-A conflict-fix read the head SHA twice (stamp + start) → could pass a newer unreviewed SHA. Fixed: fetch ONCE, reuse for both.
- P1-B 90-min stale threshold < real max budget (70 pre + 60 post = 130) → could race a live post-merge. Fixed: bumped MERGE_DEPLOY_RUNNING_STALE_SECS to 140 min (provably-dead).
- P2 stale-done branch skipped notify_callback + close_origin_ticket → upstream stale. Fixed: mirrored the success-path closeout.

## Status
- [x] all code edits (redesign + R2 5 fixes + R3 P1-3 finish + post-merge recovery + R4 3 fixes)
- [x] cargo check clean, no em-dashes
- [x] Codex rounds 2/3/4 (converged; R4 fixes mechanical + compiled, no R5 needed)
- [x] release build (16:22) + atomic-swap STAGED to /usr/bin/agent-one
- [x] **RESTART DONE** 2026-06-17 16:29 EDT at a restart-safe window (no md=running, no PR-less in_progress). New binary (PID 2240630) live; startup sweep cleaned 10 merged/cancelled worktrees, kept 7; NO panics; failover shim intact (glmServed 1961/fellBack 76). Board sane (7 fixes_needed). No dup PRs (waited for safe window).

## SHIPPED ✅ — stall watchdog redesign live on Moria. Deployed-uncommitted (see memory autosam-worker-fixes-moria-local item 7).
