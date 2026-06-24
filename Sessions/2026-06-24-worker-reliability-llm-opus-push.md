# Session: /push of accumulated worker reliability + LLM Opus + close-pr WIP

Date: Tuesday, June 24, 2026
Repo: AutoSam (github.com/mrchevyceleb/samwise)
Branch: master. Pushed 4d5d34f..7150ccd (3 commits). master in sync with origin.

## Focus

Matt ran /push on a large, multi-concern working tree that had accumulated since the
06-22 session. No single "current task"; the WIP spanned the LLM/Opus switch, the
review-merge pipeline, worker self-sufficiency fixes, the merge-deploy stall redesign,
and the close-pr carryover. Goal: commit it in tight logical groups, push, then do the
full AutoSam deploy (binary + web + Supabase migration + edge function).

## What changed

### Commits (in order)
1. `53e26ff` fix(llm): Claude Code Opus 4.8 direct OAuth routing + proxy/credential sync
   - claude_code.rs (LlmProxyConfig, inject_proxy_env, scrub stale OAuth-blocker env,
     sync_claude_oauth_credentials_if_needed atomic refresh), health.rs (check_llm_proxy),
     SettingsModal.svelte + SetupWizard.svelte + settings.svelte.ts (LLM Proxy tab),
     AGENTS.md + CLAUDE.md (Opus 4.8 docs).
2. `e61b6b2` fix(intake): dedup duplicate Sam cards from repeated origin reports
   - task-webhook edge fn (read-before-insert dedup + 23505 catch), migration
     20260617210000 (partial UNIQUE index ae_tasks_open_origin_unique on OPEN statuses).
3. `7150ccd` feat(worker): review-merge pipeline, self-sufficiency, merge-deploy recovery,
   close-pr, board UI. The big one: worker.rs (813 lines), review.rs (kill_on_drop),
   git.rs (close_pr), lib.rs (register close_pr + check_llm_proxy), KanbanCard +
   TaskDetailModal (desktop), KanbanCard + TaskDetail + tasks store + page (web),
   tauri.conf.json (drop devUrl fallback), Cargo base64 dep, design docs moved to docs/.

### Deploy (full, per AGENTS.md)
- Binary: `npx tauri build --no-bundle` (39.5s release), deployed to /usr/bin/agent-one
  (frontend embedded: 20 _app/immutable refs), service restarted and active.
- Supabase migration: ae_tasks_open_origin_unique index applied live (meqtadfevxguishrlxyx, personal).
- Edge function: task-webhook redeployed via CLI with SUPABASE_ACCESS_TOKEN from Doppler
  (agent-one/prd). verify_jwt=false matches config.toml. Live and reachable (401 auth gate up).
- Web UI: `cd web && npx vercel --prod --yes` -> https://samwise-board.vercel.app (HTTP 200).

## Bugs found and root causes

- Desktop svelte-check had 1 error: SettingsModal passed `backend.id` (inferred `string`)
  into updateSetting('llmProxyBackend', ...) which expects the llmProxyBackend union.
  Root cause: the backend pill array literal widened `id` to string. Fix: wrap the array
  in `as const` so each id is a literal type; `backend.id` becomes the exact union. Now 0 errors.
- lib.rs carried two unrelated registration lines (close_pr + check_llm_proxy). Each commit
  must compile standalone, and close_pr's registration can't precede git.rs's definition.
  Solved by leaving lib.rs OUT of the LLM commit (unregistered command still compiles) and
  putting the whole lib.rs in the worker commit. No partial staging needed.

## Gotchas for future agents

- This tree was genuinely multi-concern and deeply interleaved in worker.rs (review-merge +
  self-sufficiency + stall redesign + cancelled check-runs + base64 all share helpers/context
  keys in one 813-line file). Splitting worker.rs by concern would require risky partial
  staging of interleaved hunks. Kept it as one cohesive worker commit on purpose.
- `litellm/` (autosam-litellm.service, proxy_config.yaml, setup.sh) is untracked and STAYS
  untracked. It is the superseded LiteLLM proxy setup (AGENTS.md marks it historical). Do not
  `git add` it unless Matt asks to preserve it.
- task-webhook edge function deploy needs the Supabase CLI (the assistant_supabase MCP cannot
  deploy functions). The CLI is at ~/.local/bin/supabase (v2.104.0) but is NOT linked/authed
  on its own. Get SUPABASE_ACCESS_TOKEN from Doppler project agent-one config prd, then:
  `SUPABASE_ACCESS_TOKEN=<tok> supabase functions deploy task-webhook --project-ref
  meqtadfevxguishrlxyx --no-verify-jwt`. The Doppler MCP workplace slug is the lowercase id
  `899cb032c21956a29df7`, but the simplest call is to OMIT the workplace param (defaults work).
- The Doppler 404 gotcha did NOT bite here: agent-one is a Personal project and Personal is the
  default Doppler workplace, so get_secret worked first try.
- review-actions.ts (both src/ and web/) already had the getReviewMergeState / requestReviewMergeContext
  / isReviewMergeBusy / reviewMergeButtonLabel helpers committed in an earlier commit; this push
  only wired the UI (KanbanCard, TaskDetailModal, web KanbanCard/TaskDetail) to use them.
- Web deploy: use `cd web && npx vercel --prod --yes`. The git-push auto-deploy from master
  fires but errors transiently (0ms build); the CLI deploy is the reliable path. The Vercel MCP
  tool cannot reach project samwise-board (team matts-projects-6632b735).

## Open items

- The 1 approved + 1 fixes_needed cards in Supabase are pre-existing, not touched by this push.
- Production LLM routing is direct Anthropic Opus 4.8 OAuth (systemd env has the proxy vars removed).
  If OAuth expires, sync_claude_oauth_credentials_if_needed now self-heals from ~/.claude.

## How to verify

- Binary live: `systemctl --user is-active samwise-agent-one.service` = active;
  `strings /usr/bin/agent-one | grep -c _app/immutable` = 20 (>0).
- Migration live: index ae_tasks_open_origin_unique exists on ae_tasks (confirmed via execute_sql).
- Edge fn live: `curl -o /dev/null -w '%{http_code}' -X POST .../functions/v1/task-webhook -d '{}'` = 401 (auth gate up).
- Web live: https://samwise-board.vercel.app = HTTP 200.
- Types: `npm run check` (desktop) = 0 errors; `cd web && npm run check` = 0 errors; `cd src-tauri && cargo check` = clean.

## Anything else

- 4 design/working notes (REVIEW-MERGE-BUTTON-CHANGE.md, SELF-SUFFICIENCY-FIXES.md,
  STALL-WATCHDOG-REDESIGN.md, context.md) were moved from repo root into docs/ and committed
  with the worker commit, so the rationale for the worker.rs redesigns is preserved in-repo.
- litellm/ deliberately left untracked (superseded approach).
