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

Explicit production approval was given on 2026-07-25.

- Fast-forwarded `master` to code commit `055298afd172ecbe0d805cc083710736c989fe2e` and pushed `origin/master`.
- Re-ran the full Rust library suite: 32 passed.
- Re-ran Svelte checking: 0 errors, with the existing warning baseline.
- Built the production application with `npx tauri build --no-bundle`, preserving embedded frontend assets.
- Backed up the prior binary to `~/.local/share/autosam/backups/agent-one-20260725T145630Z`.
- Atomically replaced `/usr/bin/agent-one`; deployed SHA-256 is `e927990d7058b20d842b69a0b6ff9124fe572bec9b54e5af2ed8f3d40b0218c2` and the binary contains 20 embedded `_app/immutable` asset markers.
- Restarted `samwise-agent-one.service`. Final healthy PID is `1608414`, with exactly one `agent-one` process and no startup panic, fatal, OOM, or error signals.
- No environment or persisted setting overrides `AUTOSAM_MAX_REVIEW_CYCLES` / `autoFixMaxReviewCycles`, so the live effective limit is the new default of five.
- Startup recovery briefly reclaimed externally owned PR #1155. The duplicate coding child was stopped, the card was restored to clean `approved` state, and the worker was started again without claiming it. Final board state preserved the live external closeout.
