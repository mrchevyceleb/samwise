-- Samwise agent schema. All tables prefixed ae_ to avoid collision
-- with other apps sharing this Supabase project.

create table if not exists public.ae_tasks (
  id uuid primary key default gen_random_uuid(),
  title text not null,
  description text,
  status text not null default 'queued',
  priority text not null default 'medium',
  project text,
  source text not null default 'manual',
  task_type text not null default 'code',
  repo_url text,
  repo_path text,
  branch text,
  preview_url text,
  pr_url text,
  pr_number integer,
  screenshots jsonb,
  screenshots_before jsonb,
  screenshots_after jsonb,
  visual_qa_result jsonb,
  assignee text not null default 'agent',
  worker_id text,
  trigger_id uuid,
  cron_id uuid,
  context jsonb,
  subtasks jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  claimed_at timestamptz,
  completed_at timestamptz
);
create index if not exists idx_ae_tasks_status on public.ae_tasks(status);
create index if not exists idx_ae_tasks_priority_created on public.ae_tasks(priority, created_at);

create table if not exists public.ae_comments (
  id uuid primary key default gen_random_uuid(),
  task_id uuid not null references public.ae_tasks(id) on delete cascade,
  author text not null,
  content text not null,
  mentions jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now()
);
create index if not exists idx_ae_comments_task on public.ae_comments(task_id, created_at);

create table if not exists public.ae_messages (
  id uuid primary key default gen_random_uuid(),
  conversation_id uuid not null,
  role text not null,
  content text not null,
  task_id uuid references public.ae_tasks(id) on delete set null,
  attachments jsonb,
  needs_response boolean not null default false,
  created_at timestamptz not null default now()
);
create index if not exists idx_ae_messages_convo on public.ae_messages(conversation_id, created_at);

create table if not exists public.ae_workers (
  id text primary key,
  machine_name text not null unique,
  status text not null default 'offline',
  current_task_id uuid references public.ae_tasks(id) on delete set null,
  last_heartbeat timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table if not exists public.ae_crons (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  schedule text not null,
  task_template jsonb not null default '{}'::jsonb,
  enabled boolean not null default true,
  last_run timestamptz,
  next_run timestamptz,
  created_at timestamptz not null default now()
);

create table if not exists public.ae_triggers (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  source_type text not null,
  source_config jsonb not null default '{}'::jsonb,
  task_template jsonb not null default '{}'::jsonb,
  enabled boolean not null default true,
  last_checked timestamptz,
  created_at timestamptz not null default now()
);

create table if not exists public.ae_trigger_events (
  id uuid primary key default gen_random_uuid(),
  trigger_id uuid not null references public.ae_triggers(id) on delete cascade,
  payload jsonb not null default '{}'::jsonb,
  processed boolean not null default false,
  created_at timestamptz not null default now()
);
create index if not exists idx_ae_trigger_events_unprocessed on public.ae_trigger_events(trigger_id, processed);

create table if not exists public.ae_projects (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  repo_url text,
  repo_path text,
  preview_url text,
  client text,
  deploy_method text,
  dev_command text,
  created_at timestamptz not null default now()
);

do $$
begin
  begin alter publication supabase_realtime add table public.ae_tasks; exception when duplicate_object then null; end;
  begin alter publication supabase_realtime add table public.ae_comments; exception when duplicate_object then null; end;
  begin alter publication supabase_realtime add table public.ae_messages; exception when duplicate_object then null; end;
  begin alter publication supabase_realtime add table public.ae_workers; exception when duplicate_object then null; end;
end $$;

insert into storage.buckets (id, name, public)
values ('agent-one-screenshots', 'agent-one-screenshots', true)
on conflict (id) do nothing;
