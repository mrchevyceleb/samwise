-- Remember intentional task deletions so recovery sweeps do not recreate cards
-- Matt explicitly removed from the board.

create table if not exists public.ae_task_tombstones (
  id uuid primary key default gen_random_uuid(),
  task_id uuid,
  title text,
  source text,
  status text,
  project text,
  repo_url text,
  repo_path text,
  pr_url text,
  pr_number integer,
  head_ref text,
  orphan_short_id text,
  deleted_by text not null default 'matt',
  deleted_at timestamptz not null default now()
);

create index if not exists idx_ae_task_tombstones_pr_url
  on public.ae_task_tombstones(pr_url)
  where pr_url is not null;

create index if not exists idx_ae_task_tombstones_repo_short
  on public.ae_task_tombstones(repo_path, orphan_short_id)
  where orphan_short_id is not null;

create index if not exists idx_ae_task_tombstones_head_ref
  on public.ae_task_tombstones(head_ref)
  where head_ref is not null;
