-- Idempotent intake: stop one upstream ticket from fanning out into several
-- OPEN Sam cards (the "same report POSTed 3x -> 3 cards" bug). A partial unique
-- index on (origin_system, origin_id) restricted to the OPEN status set makes a
-- concurrent-duplicate insert fail closed at the DB (race-proof, where the
-- edge-fn read-before-insert check cannot be). The task-webhook function catches
-- the resulting 23505 and returns the existing card instead of erroring.
--
-- Terminal cards (done/failed/cancelled and anything outside the OPEN set) are
-- excluded, so a fresh report after the prior one was resolved is still allowed.
-- The predicate lists OPEN statuses positively so it mirrors the edge function's
-- OPEN_STATUSES exactly and stays immutable (required for a partial index).
CREATE UNIQUE INDEX IF NOT EXISTS ae_tasks_open_origin_unique
  ON ae_tasks (origin_system, origin_id)
  WHERE origin_id IS NOT NULL
    AND origin_system IS NOT NULL
    AND status IN (
      'queued',
      'in_progress',
      'review',
      'fixes_needed',
      'approved',
      'on_hold',
      'testing',
      'pending_confirmation'
    );
