-- Track every scheduler fire so recurring work is auditable from the UI.

create table if not exists public.ae_cron_runs (
  id uuid primary key default gen_random_uuid(),
  cron_id uuid not null references public.ae_crons(id) on delete cascade,
  status text not null default 'running',
  scheduled_for timestamptz,
  started_at timestamptz not null default now(),
  completed_at timestamptz,
  task_ids jsonb not null default '[]'::jsonb,
  task_count integer not null default 0,
  execution_mode text,
  summary text,
  error text,
  metadata jsonb not null default '{}'::jsonb
);

create index if not exists idx_ae_cron_runs_cron_started
  on public.ae_cron_runs(cron_id, started_at desc);

create index if not exists idx_ae_cron_runs_status_started
  on public.ae_cron_runs(status, started_at desc);

do $$
begin
  begin alter publication supabase_realtime add table public.ae_cron_runs; exception when duplicate_object then null; end;
end $$;
