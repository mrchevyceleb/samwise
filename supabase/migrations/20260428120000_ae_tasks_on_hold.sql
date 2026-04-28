-- Manual hold flag for queued tasks. When `on_hold` is true, the worker skips
-- the row in its claim path even though status is still 'queued'. Matt clicks
-- the HOLD stamp on the card to toggle. Lets him pin a card he wants Sam to
-- get to later without dragging it out of the queue or relying on memory.

ALTER TABLE ae_tasks
  ADD COLUMN IF NOT EXISTS on_hold boolean NOT NULL DEFAULT false;
