alter table ae_tasks
  add column if not exists review_scores jsonb,
  add column if not exists review_summary text,
  add column if not exists auto_merged boolean not null default false,
  add column if not exists auto_merge_blocked_reason text;

create table if not exists ae_review_log (
  id uuid primary key default gen_random_uuid(),
  task_id text not null,
  pr_url text,
  scores jsonb,
  blockers jsonb,
  ci_passed boolean,
  decision text not null,
  reason text,
  created_at timestamptz not null default now()
);

create index if not exists ae_review_log_task_id_idx on ae_review_log(task_id);
create index if not exists ae_review_log_created_at_idx on ae_review_log(created_at desc);
