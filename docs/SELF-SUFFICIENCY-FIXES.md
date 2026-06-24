# Sam self-sufficiency fixes (2026-06-17) — kill the 3 recurring babysitter taxes

Goal: the 3 manual interventions that cost Matt a babysitter session today, encoded so they self-heal.

## Fix 1 — cap-merge re-review unblock (the #1 tax, cleared ~6x by hand)
`cap_merge_attempted` blocks re-evaluation of an rc>=3 card forever. When a fix is PUSHED to a capped card (PR head changes), it never re-reviews/merges.
- [ ] When setting `cap_merge_attempted`, also store `cap_merge_attempted_sha` = current PR head.
- [ ] In the rc>=3 sweep path: if current PR head != cap_merge_attempted_sha (new commit pushed), clear cap flags + review/blocked state, reset to review (rc=1), so the pushed fix re-reviews + cap-merges. Else stay parked.

## Fix 2 — zombie-review self-heal (#596 sat pending 21m today)
A `review`-status card whose `samwise_pr_review_status` is pending/running but never progresses (stale, no live process) sits forever.
- [ ] Cover the 'pending-never-started' case (started_at null + review status stale via updated_at), not just RUNNING_STALE.
- [ ] In the review sweep: if stale beyond threshold AND no live review proc, clear pr_review status/started_at so it re-fires.

## Fix 3 — intake dedup (recording x3, webinar-disappear x3 today)
Triage spawns N cards from one origin ticket (same origin_id).
- [ ] At card creation from an origin report: if an OPEN card with same origin_id (or origin_url) already exists, skip/merge instead of creating a duplicate.
- [ ] LOCATE intake first — may be worker.rs, an edge function, or external triage.

## Implemented
- Fix 1 (worker.rs maybe_spawn_auto_fix): stamp cap_merge_attempted_sha at attempt; in the already-tried branch, fetch current PR head and if it moved, clear cap+review+blocked state and reset to review rc=1 (status-guarded). Self-heals the ~6x/day manual flag clear.
- Fix 2a (worker.rs run_git): on a .lock collision error, clear_stale_git_locks (>=30s old, under --git-common-dir, skips objects/) and retry once. Root-cause fix for the #596 zombie.
- Fix 2b (worker.rs pr_review_should_run_now): pr_review_is_stale_inflight backstop re-fires any running/pending review whose worker went stale, instead of waiting the 30-min timer.
- Fix 3 (task-webhook edge fn): dedup before insert on (origin_system, origin_id) then identical title+description, against OPEN statuses only; fail-open on query error so no report is lost.

## Process
- [x] scout exact locations
- [x] implement (root-cause, minimal, no em dashes)
- [x] cargo check (clean, no new warnings)
- [x] Codex-Fix: 3 P1 + 1 P2 found, ALL fixed:
  - P1-1 lock sweep too broad -> clear_named_stale_git_locks removes only the exact lock path git named (no walk), 30s-gated
  - P1-2 read-before-insert race -> partial unique index ae_tasks_open_origin_unique + edge fn catches 23505 and returns existing card
  - P1-3 content fallback too broad -> scoped by source + project
  - P2-1 pending re-fire spam -> pr_review_is_stale_inflight narrowed to running-only
  - Fix 1 SHA-reset path: NO findings (status guard + reviewed-head guard hold)
- [x] re-check compile after fixes (clean)
- [x] DB index applied to meqtadfevxguishrlxyx (verified present; 0 pre-existing dups blocked it)
- [x] task-webhook edge function deployed to meqtadfevxguishrlxyx
- [x] build --release + atomic-swap (/usr/bin/agent-one) + restart at safe window (no md=running, no PR-less in_progress)
- [x] startup verified: PID 2666146, 0 panics, failover shim intact (glmServed 3007)

## SHIPPED 2026-06-17. worker.rs Fixes 1+2 deployed-uncommitted (like the rest of the local agent-one). Fix 3 (edge fn + index) live on the board project.
