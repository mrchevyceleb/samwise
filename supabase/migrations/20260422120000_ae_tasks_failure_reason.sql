-- Capture why a task ended in `failed` status (build errors, QA exhaustion, etc.)
-- Worker writes a short machine-extractable reason when it has to bail out
-- between codex-fix and PR creation.
ALTER TABLE ae_tasks
    ADD COLUMN IF NOT EXISTS failure_reason text;
