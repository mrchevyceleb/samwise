# Review cycle cap reliability

## Incident

A status-independent 24-hour board audit found 16 cards at `review_cycle_count >= 3`, including 14 internal cards. The hardcoded three-pass limit was operating as normal queue pressure rather than a rare safety stop. Samcheck also failed to surface the cluster because it only treated current `fixes_needed` rows as actionable and the recurring Pi loop had expired.

## Changes

- Raised the automatic review/fix limit from 3 to 5 by default.
- Added `AUTOSAM_MAX_REVIEW_CYCLES` and persisted `autoFixMaxReviewCycles` overrides, bounded to 1–10.
- Threaded the selected limit through verdict-driven fixes, stale-card recovery, comments, Telegram messages, and generated commit messages.
- Added explicit boundary coverage proving counts 3 and 4 remain eligible while 5 is capped.
- Scoped cap-floor metadata to the active configured limit so old three-cycle markers cannot suppress cycles 4 and 5.
- Cleared stale cap failure state when a new fix cycle is claimed.
- Prevented repeated terminal cap messages after a completed cap-floor attempt.
- For auto-merge-off caps, deduplicate notifications through an atomic top-level `failure_reason` marker written only after the board comment succeeds. No context read-modify-write is used.
- Updated the global samcheck skill to verify watcher liveness and perform a status-independent 24-hour cap-pressure audit.
- Re-armed the five-minute `/samcheck` loop.

## Validation

- Full Rust library suite: 32 passed.
- `cargo check`: passed with existing warnings only.
- Frontend production build: passed.
- `git diff --check`: passed.
- Parent Codex-Fix reviewed the cap boundary, metadata compatibility, notification races, and deduplication. All P1 findings were resolved.

## Rollout

This branch is not merged and the live worker has not been rebuilt or restarted. The current production worker remains unchanged until an explicit merge/deploy decision.
