-- Terminal work is immutable to automatic coding requeues. Review may continue
-- on a published PR, but repeating the original ask requires a new task.
create or replace function public.guard_ae_task_execution_state()
returns trigger language plpgsql as $$
declare
  review_only boolean := coalesce(new.task_type = 'qa-verify'
    or new.context->>'slack_review_workflow' = 'true'
    or new.context->>'plant_full_pr_review' = 'true'
    or new.context->>'full_pr_review_automation' = 'true'
    or new.source = 'github-kim-pr-review'
    or (new.source = 'slack' and new.title like 'Review PR%'), false);
begin
  -- Compact snapshot guards also catch manual edits that omit updated_at.
  if tg_op = 'UPDATE' then new.updated_at := clock_timestamp(); end if;
  if tg_op = 'UPDATE' and old.status in ('approved', 'done', 'cancelled')
     and new.status in ('queued', 'in_progress', 'testing')
     -- An explicit retry of a cancelled, never-started card has no work to replay.
     and not coalesce((old.status = 'cancelled' and new.status = 'queued'
       and new.context->>'resume_saved_work' = 'true'
       and nullif(old.pr_url,'') is null and nullif(old.branch,'') is null
       and old.claimed_at is null and old.completed_at is null
       and not (coalesce(old.context,'{}'::jsonb) ?| array['worker_checkpoint','recovery_required','start_commit'])
       and coalesce(old.context->>'push_to_existing_branch','false') <> 'true'
       and not exists (select 1 from jsonb_array_elements(case when jsonb_typeof(old.subtasks)='array' then old.subtasks else '[]'::jsonb end) s where s->>'done'='true')
     ), false)
     and not coalesce((old.status = 'approved' and new.status = 'in_progress'
       and nullif(new.pr_url, '') is not null
       and old.context->>'samwise_review_merge_status' = 'requested'
       and new.context->>'samwise_review_merge_status' = 'running'), false)
     and not coalesce((old.status = 'approved' and new.status = 'in_progress'
       and nullif(new.pr_url, '') is not null and new.pr_url = old.pr_url
       and old.context->>'samwise_merge_conflict_fix_status' = 'requested'
       and new.context->>'samwise_merge_conflict_fix_status' = 'running'), false) then
    if nullif(old.pr_url,'') is not null then
      raise exception 'This task has finished or stopped. Continue with a follow-up on its linked PR; its saved work stays here.';
    end if;
    raise exception 'This task has finished or stopped after starting work. Create a follow-up using its saved branch or checkpoint; its work stays here.';
  end if;
  if new.status = 'approved' and new.task_type = 'qa-verify' and nullif(new.pr_url, '') is null then
    -- A successful browser-only check has no code PR to publish. Finish it,
    -- before the legacy normalizer can turn its approval back into coding.
    new.status := 'done';
    new.completed_at := coalesce(new.completed_at, now());
    new.worker_id := null;
    new.claimed_at := null;
  elsif new.status = 'approved' and nullif(new.pr_url, '') is null then
    -- The old normalizer silently converted this to queued. Preserve work and
    -- surface the missing PR instead of launching the original coding prompt.
    new.status := 'failed';
    new.worker_id := null;
    new.claimed_at := null;
    new.failure_reason := 'This task has no linked PR. Link its finished work before marking it ready.';
  end if;
  -- Team authentication holds preserve a continuation, not the original ask.
  if new.status='queued' and new.on_hold and new.context ? 'worker_checkpoint'
     and not (new.context ? 'recovery_required') then
    new.context := coalesce(new.context,'{}'::jsonb)||'{"resume_saved_work":true}'::jsonb;
  end if;
  if new.status = 'queued' and not review_only and (
       nullif(new.pr_url, '') is not null or new.completed_at is not null
       or (new.context ? 'recovery_required' and not coalesce((new.context->>'resume_saved_work')::boolean,false))
       or (new.context ? 'worker_checkpoint' and not coalesce((new.context->>'resume_saved_work')::boolean,false) and not coalesce((
         jsonb_typeof(new.subtasks) = 'array'
         and exists (select 1 from jsonb_array_elements(case when jsonb_typeof(new.subtasks)='array' then new.subtasks else '[]'::jsonb end) s where coalesce((s->>'done')::boolean,false))
         and exists (select 1 from jsonb_array_elements(case when jsonb_typeof(new.subtasks)='array' then new.subtasks else '[]'::jsonb end) s where not coalesce((s->>'done')::boolean,false))
       ), false))
     ) then
    new.status := 'failed';
    new.worker_id := null;
    new.claimed_at := null;
    new.failure_reason := 'This task already has saved work. Resume that work or start a follow-up on its PR.';
  end if;
  return new;
end;
$$;
-- Run before normalize_ae_task_inbound_state_before_write (alphabetical order).
drop trigger if exists aa_guard_ae_task_execution_state on public.ae_tasks;
create trigger aa_guard_ae_task_execution_state before insert or update on public.ae_tasks
for each row execute function public.guard_ae_task_execution_state();
