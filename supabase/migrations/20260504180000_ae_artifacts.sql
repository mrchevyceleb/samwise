-- Research-task reports and other long-form outputs that don't fit in a
-- comment. The TaskDetailModal renders these via its "Report" tab.
-- Without this table, the worker's create_artifact call returns an error
-- and research tasks fall back to a truncated summary in a comment.

create table if not exists public.ae_artifacts (
  id uuid primary key default gen_random_uuid(),
  task_id uuid not null references public.ae_tasks(id) on delete cascade,
  title text not null,
  content text not null,
  artifact_type text not null default 'report',
  created_at timestamptz not null default now()
);
create index if not exists idx_ae_artifacts_task on public.ae_artifacts(task_id, created_at);

alter publication supabase_realtime add table public.ae_artifacts;
