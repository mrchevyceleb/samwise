-- Track when we last paged Matt about a stale worker heartbeat.
-- The worker-heartbeat-alarm edge function reads/writes this so we
-- alert on transitions and re-alert no more than every 10 min.
ALTER TABLE ae_workers
  ADD COLUMN IF NOT EXISTS last_stale_alert_at TIMESTAMPTZ;
