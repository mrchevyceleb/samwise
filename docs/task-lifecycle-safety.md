# Task lifecycle safety

CEG: Every job finishes or clearly says it failed, keeps its own PR and saved work, and never repeats finished work.

Independent outcome score: 7/10 on real overnight tasks and PRs. The named retry/requeue and error-message findings were corrected before activation.

A live tracked task owns its Tokio runner, not a stale updated_at timestamp. Coder, build, review and publication phases have finite deadlines. If the runner exits or panics with an unfinished task, retain the worktree and actual local commit, mark the task failed, and clear its claim. A stopped task stays stopped. A worker-poll panic keeps independently running jobs and their slots.

Published and terminal tasks cannot restart the original ask. The database guard runs before the existing approval normalizer. An approval without a PR becomes a visible failure; browser-only QA can finish without a code PR. Explicit merge/review continuations keep their existing paths. Both board retry actions request saved-work continuation and use the canonical response. Only an explicitly retried, never-started cancelled card with no branch, claim, checkpoint, recovery record, completed subtask or PR can queue again.

Existing-PR rounds verify the actual open PR, repository, branch and start-commit ancestry. Each round uses its own detached worktree. It pushes only to the assigned branch with a fast-forward push and never creates a second PR. Merged, closed, wrong-branch and fork targets give distinct reasons and preserve work.

The independent reviewer is pinned to Opus5.5 at medium effort. A timeout, process error, incomplete answer or wrong-HEAD receipt cannot pass. A completed findings report gets one bounded repair and a fresh independent review on the resulting commit. Review findings and recovery context remain available to a continuation. Publication saves a durable Git receipt before updating the task with optimistic ownership/status guards.

## Activation

Max approved a brief admission drain, not a rolling worker replacement. Samwise coordinates the queue.

1. Stop admissions with the existing native `worker_stop` command in the running desktop (the current Settings UI does not expose the worker Stop control). This stops new claims while existing task runners finish. Do not use Stop Task, quit the app or stop its service while runners remain. Keep the window open to avoid frontend autostart. Check active database rows, process children and the desktop pool; do not infer a drain from an old heartbeat.
2. When there are zero live task runners, stop samwise-agent-one.service. Keep the approved team-account shim and systemd drop-in. Never restore an older unguarded binary.
3. Apply supabase/migrations/20261022093000_preserve_task_lifecycle.sql to the existing AutoSam project (meqtadfevxguishrlxyx). Verify the guard trigger precedes the live inbound normalizer. No data rows are reset or requeued.
4. Install this exact full Tauri release binary. Check embedded frontend assets and --check-claude-account before starting. Preserve refusal exit78, hooks, global/project/local rules, Node options and normal TLS.
5. Start the service, which deliberately resumes claims. Verify the new executable hash, guarded team account and first task ownership/lifecycle behavior. Deploy the matching existing web project so its retry buttons use the same continuation behavior.
6. Only after installed proof, Samwise removes pg_cron ae-liveness-keepalive (job3). Resume/release critical held follow-ups using their existing cards and branches. Never requeue completed owning tasks.

## Validation and limits

Full native build embeds20 frontend assets; desktop and web checks pass. Exact migration bodies were exercised against TEMP copies of real overnight rows with the live normalizer. Real PR checks bind overnight duplicate rounds to PR2444 and preserve separate worktrees. Real vendor timeout, local runner exit, cancellation, stale ownership snapshots and durable publication receipts were exercised on an owned held fixture. A large real context row is supported through compact timestamp/owner guards, with the trigger timestamp changing even when a manual edit omits it.

Outcome verification is on real data before activation. The first live admitted job is a further rollout check, not a claim made by pre-deploy fixtures. Remaining capped-review notes are retained in the PR. No production customer task, commit or PR is reset to make the proof pass.
