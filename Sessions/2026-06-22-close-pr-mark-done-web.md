# Session: Close PR & Mark Done + Copy PR link (web app)

Date: Monday, June 22, 2026 14:51 EDT
Repo: AutoSam (github.com/mrchevyceleb/samwise)
Commits pushed to origin/master: 3e10102 (feat), 4791544 (session log). master is in sync with origin.

## Focus

Matt asked for a "Close PR and Mark Done" button on the card detail view, then expanded it: the web app needs it too, plus a one-click "copy a PR link" on the web app. The desktop (Tauri) app got the same two buttons in an earlier step of the session.

## What changed

### Web app (web/) + shared Supabase backend
- `web/src/lib/components/TaskDetail.svelte`: added "Copy PR link" (indigo) and "Close PR & Mark Done" (rose) buttons to the action footer. `copiedPr` shows a 2s "Copied!" state. `canClosePr` = task has a pr_url and is not already done. Closing the PR succeeds, then the task is set to Done (which also fires origin-ticket closeout for Operly/Banana/etc. cards via the existing closeOriginTicket path).
- `web/src/lib/stores/tasks.svelte.ts`: new `closePr(taskId)` method that POSTs to `/api/close-pr`. Returns `{ok, error?}`. Does NOT touch task status; the caller decides to setStatus done after.
- `web/src/routes/api/close-pr/+server.ts` (new): web backend route. Loads the task row to resolve `pr_url` (so a caller can't close an arbitrary PR by guessing a URL), then delegates to the `close-pr` edge function using `SUPABASE_SERVICE_ROLE_KEY` (same env var already used by close-origin-ticket, so no new Vercel env vars needed).
- `supabase/functions/close-pr/index.ts` (new): Supabase Edge Function. Auth: compares caller's Authorization/apikey header against the auto-injected `SUPABASE_SERVICE_ROLE_KEY` (constant-time). Parses `https://github.com/{owner}/{repo}/pull/{n}` (+ /files /commits suffixes), then `PATCH https://api.github.com/repos/{owner}/{repo}/pulls/{n}` with `{"state":"closed"}` using a `GH_TOKEN` secret. Already-closed/already-merged (GitHub 422) is treated as success.
- `supabase/config.toml`: registered `[functions.close-pr] verify_jwt = false` (it authenticates callers itself, like the other Samwise functions).

### Desktop app (src/), done earlier in session, deployed live
- `src-tauri/src/commands/git.rs`: new `close_pr` Tauri command (runs `gh pr close <url>` via the already-authed gh CLI; already-closed/already-merged treated as success).
- `src-tauri/src/lib.rs`: registered `commands::git::close_pr`.
- `src/lib/components/kanban/TaskDetailModal.svelte`: PR section now has View PR + Copy link + Close PR & Mark Done buttons.
- Built `npx tauri build --no-bundle`, deployed to `/usr/bin/agent-one`, restarted `samwise-agent-one.service` (verified active, frontend embedded, close_pr in binary).

## Bugs found and root causes

- None functional. One deploy-path gotcha: the Tauri binary embeds frontend assets compressed, so `strings <binary> | grep "Close PR & Mark Done"` returns 0 even though the string IS present in the rebuilt `build/_app/immutable/nodes/*.js`. Do not use `strings` to verify UI copy in the Tauri binary; grep the `build/` dir instead, and use `strings | grep -c _app/immutable` (>0) to confirm the frontend is embedded at all.

## Gotchas for future agents

- The working tree has a lot of pre-existing uncommitted WIP (review-merge feature, llm-proxy health check, etc.) that is NOT part of this task. When committing, stage only the 5 web/close-pr files by name. `lib.rs` specifically has TWO uncommitted hunks: mine (`commands::git::close_pr`) and a pre-existing `commands::health::check_llm_proxy` line. The desktop `lib.rs` change was NOT committed in this push (it's entangled with that WIP); only the web app + edge function were pushed. The desktop binary is live regardless (deployed directly).
- `GH_TOKEN` is an OAuth token (`gho_...`) from `gh auth login` on the Spark (mrchevyceleb, repo scope). If it expires/gets revoked, the web "Close PR" returns 401. Refresh by re-running `gh auth login` on the Spark and re-setting the `GH_TOKEN` Supabase secret on project meqtadfevxguishrlxyx.
- The web app (samwise-board on Vercel, project name `samwise-board` NOT `samwise-web`) is deployed via `npx vercel --prod` from the `web/` directory. The Vercel CLI is authenticated on the Spark as `mtjohnston42-7236` (token in `~/.local/share/com.vercel.cli/auth.json`). Do NOT rely on the Vercel MCP tool, it cannot reach this project (it's under team `matts-projects-6632b735`, project `samwise-board`, id `prj_fkrhH6Zrz0jUd4CYyNetUBU5kQTF`). The git-push auto-deploy from master DID fire but errored transiently (0ms build); always follow up with a manual `cd web && npx vercel --prod --yes` to guarantee the deploy lands.
- The Vercel project already has `SUPABASE_SERVICE_ROLE_KEY` set as an env var (used by the pre-existing `/api/close-origin-ticket` route). The new `/api/close-pr` route reuses it, so no new Vercel env vars were needed.

## Open items

- ~~Web app Vercel deploy not done from here (tooling unavailable).~~ RESOLVED: deployed via `npx vercel --prod` from `web/` (CLI is authenticated as mtjohnston42-7236 on the Spark). Live at https://samwise-board.vercel.app . The git-push auto-deploy fired but errored transiently (0ms build); the CLI deploy is the reliable path.
- Desktop `lib.rs` + `git.rs` + `TaskDetailModal.svelte` changes are live via the deployed binary but remain uncommitted in the working tree (entangled with unrelated WIP). Commit them when the rest of that WIP is ready.

## How to verify

- Edge function live: `curl -s -o /dev/null -w "%{http_code}\n" -X POST https://meqtadfevxguishrlxyx.supabase.co/functions/v1/close-pr -H "content-type: application/json" -d '{}'` returns 401 (auth gate up). With the Samwise service key + a real pr_url it returns 200 and closes the PR.
- Desktop app: open a card with a PR in the Tauri app, the PR section shows View Pull Request / Copy link / Close PR & Mark Done.
- Web app (after Vercel deploy): same three controls in the task detail action footer.
- Type checks: `npm run check` (Tauri app, 1 pre-existing unrelated error in SettingsModal) and `cd web && npm run check` (0 errors). `cd web && npm run build` passes (adapter-vercel).

## Anything else

- The `close-pr` edge function, `GH_TOKEN` secret, and `supabase/config.toml` entry are all live on the Samwise Supabase project (meqtadfevxguishrlxyx).
- The web app is deployed and live at https://samwise-board.vercel.app . Verified: site returns 200, `/api/close-pr` returns 400 for empty body and 404 "task has no PR to close" for a fake task_id (proving the Supabase service key is wired), and the existing `/api/close-origin-ticket` route still works.
- Deploy command that works on the Spark: `cd web && npx vercel --prod --yes` (authenticated as mtjohnston42-7236).
