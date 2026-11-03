-- /sam $pr-review tasks (context.full_pr_review_task) review an existing PR,
-- so they are review-only: a failed review may be re-queued. Without this the
-- guard blocked every retry with "already has saved work" (Nov 3 2026).
-- Applied live on Nov 3 2026; idempotent.
DO $do$
BEGIN
  IF pg_get_functiondef('public.guard_ae_task_execution_state'::regproc) NOT LIKE '%full_pr_review_task%' THEN
    EXECUTE replace(
      pg_get_functiondef('public.guard_ae_task_execution_state'::regproc),
      $r$or new.context->>'full_pr_review_automation' = 'true'$r$,
      $r$or new.context->>'full_pr_review_automation' = 'true'
    or new.context->>'full_pr_review_task' = 'true'$r$);
  END IF;
END $do$;
