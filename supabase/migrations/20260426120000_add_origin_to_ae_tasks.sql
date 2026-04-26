-- Origin tracking for tasks that come from external systems (Operly triage,
-- Banana Code triage, Sentry alerts). After Sam ships and the merge+deploy
-- succeeds, the worker calls back to the source system to close the original
-- ticket so Matt does not have to do morning cleanup by hand. NULL means the
-- task was created manually and needs no closeout.

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS origin_system text,
  ADD COLUMN IF NOT EXISTS origin_id text,
  ADD COLUMN IF NOT EXISTS origin_url text;

CREATE INDEX IF NOT EXISTS ae_tasks_origin_system_idx
  ON ae_tasks (origin_system)
  WHERE origin_system IS NOT NULL;
