-- Webhook callback plumbing. External systems posting to task-webhook can
-- now include `callback_url` (+ optional `callback_secret`) in the body;
-- the worker fires a signed POST to that URL on every notable status
-- transition (in_progress, review, done, failed).

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS callback_url text,
  ADD COLUMN IF NOT EXISTS callback_secret text;
