-- Counts auto-fix attempts Sam has made on a task in response to Codex
-- $samwise-pr-review landing in Fixes Needed. Caps the auto-fix loop so a
-- task whose blockers Sam can't resolve doesn't thrash the PR forever.
-- Reset to 0 when the task leaves the review/fixes_needed cycle (moved to
-- done/approved/failed/cancelled or requeued fresh).

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS review_cycle_count integer NOT NULL DEFAULT 0;
