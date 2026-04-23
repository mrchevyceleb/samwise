-- Tracks the last time Sam fired $samwise-pr-review for a task so the
-- re-review watcher can pick up cards that transitioned fixes_needed ->
-- review without re-running on every poll tick.
--
-- Also documents the new status value `fixes_needed` used by the
-- post-PR Codex review pipeline. ae_tasks.status is untyped text, so
-- the value is free-form and no CHECK constraint exists to extend.

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS last_pr_review_at timestamptz;
