-- Sam's structured commit message (Root Cause / Fixes Made / CS Message + Deployment).
-- Captured right after Claude Code's main commit lands, before codex-fix or
-- merge-conflict-resolution can add follow-up commits with auto-generated
-- messages. Persisted on the task row so the kanban card can render it
-- without git access (worktrees get reaped on merge).

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS commit_message text;
