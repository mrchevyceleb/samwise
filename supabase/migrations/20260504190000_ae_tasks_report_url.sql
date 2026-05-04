-- Public-readable URL where the rendered HTML report for a research task
-- is served. Today the server is the local Tauri binary on the Mac mini,
-- bound to the Tailscale IP; only devices on Matt's tailnet can resolve it.

alter table public.ae_tasks
  add column if not exists report_url text;
