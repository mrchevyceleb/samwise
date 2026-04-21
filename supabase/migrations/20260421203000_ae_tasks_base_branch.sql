-- Add nullable base_branch column so tasks can target feature branches rather than
-- always basing off the repo's default branch (main/master). Null = use default.
alter table public.ae_tasks
  add column if not exists base_branch text;
