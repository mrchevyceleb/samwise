-- Keep externally-created task rows out of impossible board states.
--
-- Some hand-off agents can write directly to ae_tasks instead of going through
-- task-webhook. A code task cannot be Ready to Merge until Sam has created a PR,
-- and project-bound tasks need their repo fields filled from the registry.

create or replace function public.normalize_ae_task_inbound_state()
returns trigger
language plpgsql
as $$
declare
  project_row record;
begin
  if new.project is not null and (
    nullif(new.repo_path, '') is null
    or nullif(new.repo_url, '') is null
    or nullif(new.preview_url, '') is null
  ) then
    select name, repo_url, repo_path, preview_url
      into project_row
      from public.ae_projects
      where lower(name) = lower(new.project)
      limit 1;

    if found then
      new.project := project_row.name;
      if nullif(new.repo_url, '') is null and project_row.repo_url is not null then
        new.repo_url := project_row.repo_url;
      end if;
      if nullif(new.repo_path, '') is null and project_row.repo_path is not null then
        new.repo_path := project_row.repo_path;
      end if;
      if nullif(new.preview_url, '') is null and project_row.preview_url is not null then
        new.preview_url := project_row.preview_url;
      end if;
    end if;
  end if;

  if new.status = 'approved' and nullif(new.pr_url, '') is null then
    new.status := 'queued';
    new.worker_id := null;
    new.claimed_at := null;
    new.failure_reason := null;
    new.context := coalesce(new.context, '{}'::jsonb) || jsonb_build_object(
      'samwise_normalized',
      jsonb_build_object(
        'reason', 'approved_without_pr',
        'normalized_at', now()
      )
    );
  end if;

  return new;
end;
$$;

drop trigger if exists normalize_ae_task_inbound_state_before_write on public.ae_tasks;

create trigger normalize_ae_task_inbound_state_before_write
before insert or update on public.ae_tasks
for each row
execute function public.normalize_ae_task_inbound_state();
