# AutoSam - AI Coding Coworker

Matt's AI junior developer. Autonomous coding agent that lives on the DGX Spark ("Moria"), reachable from anywhere. Tauri v2 + SvelteKit 5 + Rust.

Named after Samwise Gamgee (LOTR). Personality is Sam: loyal, proactive, "senior dev on Slack" tone. Takes tasks, ships PRs, answers questions.

See `docs/REVIVAL-CONTEXT.md` for the full vision and design rationale.

## What This Is

A desktop app plus a headless-capable worker loop with two interaction surfaces:
1. **Kanban Board** - Trello-style task management with drag-and-drop, comment threads with @mentions
2. **Chat Sidebar** - Direct conversation with Sam (task creation from plain language, status questions, @project tagging)

Planned additional surface: **Telegram bot** for reaching Sam from a phone. Inbound/outbound messages flow through the same `ae_messages` table as the desktop chat.

The backend worker picks up tasks from the board, writes code via Claude Code CLI, takes Playwright screenshots for visual QA, and opens PRs with before/after screenshots.

## LLM Backend

### The coding harness is selectable: `AUTOSAM_CODER=claude|pi`

Sam's code-writing step runs through one of two harnesses. Everything else in the
pipeline (worktrees, board, Slack/Telegram ingestion, crons, triggers, review
sweeps, merge/deploy) is harness-agnostic and untouched by the choice.

| | `claude` (default) | `pi` |
|---|---|---|
| Binary | Claude Code CLI | `pi -p --mode json` |
| Models | Anthropic IDs only, so non-Anthropic needs the LiteLLM proxy | any provider pi is configured for, natively |
| Vision | only if the proxy target accepts images | wherever the model does (`pi --list-models` has an `images` column) |
| Selection | `CLAUDE_MODEL` in `commands/claude_code.rs` + proxy rewrite | `AUTOSAM_PI_PROVIDER` / `AUTOSAM_PI_MODEL` / `AUTOSAM_PI_THINKING` |

Dispatch lives in `src-tauri/src/commands/coder.rs`; `run_claude_code_streaming`
and `run_claude_code_opts` in `worker.rs` are the two entrypoints that branch on
it. Both harnesses honour the same contract (PID slot, cancellation via
`TASK_CANCELLED`, progress comments, final-text return), so the swap is one env
var and rollback is the same.

**pi specifics.** Discovery deliberately avoids PATH: `/usr/bin/pi` on this host
is an unrelated 2024 ELF binary (a Lisp), so `find_pi_command()` resolves
`~/node_modules/@earendil-works/pi-coding-agent/dist/cli.js` under `node` and
only falls back to PATH last. The prompt goes on **stdin**, never argv. pi has no
`--max-turns` flag, so the cap is enforced by counting `turn_start` events in the
parser and SIGTERMing the child. Agent-one's LiteLLM env vars are scrubbed from
pi spawns (`strip_direct_oauth_blockers_async`) — pi has its own `anthropic`
provider that reads `ANTHROPIC_API_KEY`, and leaving them set would quietly route
pi back through the proxy it exists to remove. `AUTOSAM_PI_NO_EXTENSIONS=1`
disables pi extension loading if a nested-agent extension misbehaves unattended.

### Claude Code path (default) — LiteLLM proxy

**Live target is Fireworks GLM 5.2 (`accounts/fireworks/models/glm-5p2`).** Kimi
K3 Fast ran 2026-07-23 → 07-24 and was reverted the next day over a 78% zero-edit
session rate and tool-looping; the revert note is in the config header. The
`claude-opus-4-8` label agent-one hardcodes is only what Claude Code *thinks* it
is talking to — LiteLLM rewrites it before forwarding.

⚠️ **GLM 5.2 is text-only.** It rejects image blocks outright
(`400 "This model does not support image inputs"`), so on this path screenshots
must go through the local describe adapter — see the vision note below.

**Why a proxy at all:** Claude Code validates model names client-side and rejects non-Anthropic IDs, so it can't be pointed at Fireworks directly. LiteLLM sits in the middle, accepts the Anthropic name, rewrites it to the Fireworks model, and forwards.

### Vision / image attachments

`AUTOSAM_CODER_HANDLES_VISION` means "the coder can see images itself, skip the
local describe pass". It must match the actual coder, and drifting apart is a
silent failure — Sam keeps working, just blind, and probes screenshots with
`file` instead of reading them.

- **Coder can see images** (pi on kimi-k3, or a vision-capable proxy target) → set `1`.
- **Coder is text-only** (GLM 5.2 today) → set `0`, and images are described by
  the local model at `AUTOSAM_VISION_MODEL_URL` (LM Studio) before being pasted
  into the prompt.

Set in the drop-in `~/.config/systemd/user/samwise-agent-one.service.d/vision.conf`.
It was left at `1` for 13 days after the GLM revert, which is how Sam went blind
on Slack screenshots (task `8740cede`, 2026-08-06).

Attachments are also validated on download: a file claiming to be an image whose
bytes are HTML is rejected rather than handed to the model. Slack answers an
unauthenticated file fetch with `200` + its sign-in page, and an upstream
uploader that only checks `res.ok` will store that as `image/png`.

**Routing mechanism (lives in systemd env + a second service, NOT in the repo):**
- `samwise-agent-one.service` sets `AUTOSAM_LLM_PROXY_URL=http://127.0.0.1:9876` and `AUTOSAM_LLM_PROXY_API_KEY=sk-litellm-autosam-master`. The worker's `load_llm_proxy()` reads these and injects them as `ANTHROPIC_BASE_URL` / `ANTHROPIC_API_KEY` per Claude Code spawn. `CLAUDE_CODE_SIMPLE=1` is set (fine here — auth is the proxy master key, not OAuth; see the gotcha below).
- **`autosam-litellm.service`** runs LiteLLM on port 9876, config `~/.config/autosam/litellm_config.yaml`. It maps `claude-opus-4-8` (plus `glm-5.2`, `claude-sonnet-4-6`, `claude-3-5-sonnet` aliases) → `fireworks_ai/accounts/fireworks/routers/kimi-k3-fast`, `api_base https://api.fireworks.ai/inference/v1`, Fireworks key inline. The repo copy under `litellm/` (`proxy_config.yaml`, `setup.sh`, unit file) is a reference/scaffold; the **live** config is the one under `~/.config/autosam/`.

⚠️ **`drop_params: true` is REQUIRED** in the LiteLLM config's `litellm_settings`. Without it, LiteLLM translates Claude Code's `thinking` param into `reasoning_effort`, which the Fireworks provider rejects for the Kimi router, 400-ing *every* request (`litellm.UnsupportedParamsError`). drop_params discards the Anthropic-only params the Fireworks provider doesn't accept instead of erroring.

**To change the coding model:** edit `~/.config/autosam/litellm_config.yaml` (the `model:` lines), then `systemctl --user restart autosam-litellm.service`. **No agent-one restart or rebuild needed** — agent-one only points at the proxy URL (unchanged), and new Claude Code spawns pick up the rewrite immediately. Smoke test through the proxy the way Claude Code calls it: `curl http://127.0.0.1:9876/v1/messages -H "Authorization: Bearer sk-litellm-autosam-master" -H "anthropic-version: 2023-06-01" -H "Content-Type: application/json" -d '{"model":"claude-opus-4-8","max_tokens":50,"messages":[{"role":"user","content":"ping"}]}'` (add `?beta=true` + `"stream":true` to exercise the streaming path). The Fireworks/Kimi model catalog (router ids) lives in the Pi config `~/GLOBAL-AGENT-CONFIGS/models.json`.

⚠️ **CLAUDE_CODE_SIMPLE gotcha:** with `CLAUDE_CODE_SIMPLE=1` set, Claude Code forces API-key auth and REJECTS OAuth subscription login, dying with `Not logged in · Please run /login`. That flag is correct for any API-key/proxy setup (GLM, Kimi) but is incompatible with the OAuth-direct Opus path below, where it must be removed.

### PR review backend: Codex CLI → OpenRouter (2026-08-08)

The coder and the reviewer are billed to different accounts on purpose. Sam **codes** with pi
on Fireworks kimi-k3 (above); he **reviews** with the Codex CLI pointed at OpenRouter
`openai/gpt-5.6-sol` at `model_reasoning_effort="xhigh"`, paid from `OPENROUTER_API_KEY` in
Doppler `agent-one/prd` rather than this machine's ChatGPT login.

- Wiring is `CODEX_MODEL` / `CODEX_REASONING_CONFIG` / `CODEX_PROVIDER_ARGS` in
  `src-tauri/src/commands/review.rs`, applied to all three codex spawns (the scored auto-merge
  review, `$samwise-pr-review`, and the full `$pr-review` merge/deploy run).
- The provider and model are pinned **inline via `-c`**, not in `~/.codex/config.toml`, so a
  host-config edit cannot silently repoint reviews at another model or account. This pins the
  provider/model only — `--ignore-user-config` is not passed, so Codex still reads trust levels,
  hooks and MCP servers from whatever `CODEX_HOME` it inherits.
  (`~/.codex/config.toml` carries the same block for manual debug runs only. agent-one sets no
  `CODEX_HOME`, so a hand-run `codex` reproduces it; an interactive shell here exports
  `CODEX_HOME=~/.codex-personal` and does not.)
- ⚠️ **Codex 0.144 dropped `wire_api = "chat"`.** Any provider swapped in here must speak the
  OpenAI **Responses** API. OpenRouter does; a chat-completions-only endpoint needs a LiteLLM
  bridge.
- The model slug must carry the `openai/` prefix. A bare `gpt-5.6-sol` is a valid ChatGPT-plan
  model but not a valid OpenRouter id.
- `resolve_openrouter_key()` resolves the key per spawn — agent-one's env first, then
  `doppler secrets get ... --project agent-one --config prd --scope $HOME` — and sets it on the
  **codex child only** via `cmd.env`. It is deliberately NOT exported into agent-one's
  environment: every child inherits that env, including the coding harness running
  model-authored shell commands against untrusted task text. (`--scope` is required; a bare
  `doppler secrets get` resolves its token from the cwd, and per-task worktrees carry their own
  scoped tokens that 404 this project.) A resolution failure fails that one review with a
  readable message rather than blocking the service from starting.
- An OpenRouter 401/402 (dead key, no credits) is classified as Inconclusive + requires-human,
  not as a verdict on the PR.
- Cost note: a codex turn ships ~78K tokens of system prompt + tools and a real review lands
  around 125-160K, so at $5/Mtok in / $30 out a full review is dollars, not cents — and
  OpenRouter doubles the rate above a 272K-token prompt. This is per-token billing now, not a
  flat plan, and there is no local spend cap beyond the wall-clock timeouts.
- Unaffected and still on the ChatGPT OAuth account: interactive `codex`, the `/codex-fix` pass
  inside the worker (that runs through the *coder* harness, `worker.rs`), and Rivendell's
  `pr-review-batch` forge cron.

### History (superseded)
- **2026-08-06:** added the pluggable `AUTOSAM_CODER` harness (pi alongside Claude Code) so model choice no longer depends on what survives a proxy rewrite.
- **2026-07-24:** Kimi K3 Fast reverted to **GLM 5.2** after a 78% zero-edit session rate and tool-looping. The `AUTOSAM_CODER_HANDLES_VISION=1` drop-in from the Kimi swap was left behind, blinding image tasks until 2026-08-06.
- **2026-07-23:** switched to **Fireworks Kimi K3 Fast** (`accounts/fireworks/routers/kimi-k3-fast`). Snapshot: `litellm_config.yaml.bak-kimi-20260723`.
- **2026-06-29:** Opus 4.8 hit the KG monthly spend limit → switched to **Fireworks GLM 5.2** (`accounts/fireworks/models/glm-5p2`, max thinking) via this same LiteLLM proxy on :9876. This established the proxy machinery still in use.
- **2026-06-19 → 06-29 (Opus era, now off):** ran **real Anthropic Opus 4.8, effort `xhigh`**, OAuth-direct (no proxy). Auth was the **KG Claude account `mtjohnston42@gmail.com`** (Claude Max), NOT Personal `mjohnst@gmail.com`; OAuth creds at `CLAUDE_CONFIG_DIR=/home/mrchevyceleb/.config/autosam/claude-config/.credentials.json` (copied from `~/.claude/.credentials.json`). OAuth-direct mode requires REMOVING `AUTOSAM_LLM_PROXY_*` env (so `load_llm_proxy()` returns None and Claude Code hits `api.anthropic.com`) **and** removing `CLAUDE_CODE_SIMPLE=1`. To restore Opus: revert those env changes; the old GLM env backup is at `~/.config/autosam/glm-revert-backup-20260619/`.
- Both the earlier Z.ai coding-plan / GLM 5.2 Max routing and the older Fireworks GLM 5.1 LiteLLM approach in `LLM-PROXY-SWAP.md` are historical/superseded.

## Deployment

**Primary host:** DGX Spark "Moria" (Ubuntu 24.04, aarch64), 24/7. Worker id / hostname `spark-3065`. Accessed via screen sharing or Tailscale from Trenzalore (Windows workstation) and phone. Migrated here from the old Mac mini on 2026-05-29 (mini retired; its `ae_workers` row still lingers but stopped heartbeating at cutover).

**Secondary host:** Trenzalore. The Tauri desktop app can run on either machine, both reading the same Supabase. The worker loop is single-active (enforced via `ae_workers` heartbeat).

**Linux host requirement (Spark):** Codex's PR-review sandbox uses bubblewrap, which needs unprivileged user namespaces. Ubuntu 24.04 blocks these by default via AppArmor, which silently breaks `$samwise-pr-review` (every review returns INCONCLUSIVE and cards stick in Review). Fix is `kernel.apparmor_restrict_unprivileged_userns=0` (persisted in `/etc/sysctl.d/60-unprivileged-userns.conf`). The sandbox also needs network for `gh`, set via `[sandbox_workspace_write] network_access = true` in `~/.codex/config.toml` and the `-c sandbox_workspace_write.network_access=true` flag in `review.rs`.

## Commands

```bash
npx tauri dev          # Full app (Vite + Rust) - dev server
npx tauri build        # Production build — ALWAYS use this for prod, never `cargo build --release` alone (see Build Rules)
cd src-tauri && cargo check  # Rust only
npm run check          # Svelte type check
```

**Production binary (macOS):** `src-tauri/target/release/bundle/macos/Samwise.app`
**Production binary (Windows):** `src-tauri/target/release/agent-one.exe`

## Build Rules

- ⚠️ **NEVER build prod with `cargo build --release` — the resulting binary has NO embedded frontend, so the WebView falls back to the dev server URL.** `tauri.conf.json` sets `devUrl: http://localhost:5890` and `frontendDist: ../build`. Only the Tauri CLI (`npx tauri build`) actually compiles+embeds the built `../build` assets into the binary and registers the WebView custom protocol. A plain `cargo build --release` skips that embedding step entirely, so at runtime the WebView has nothing to serve and tries to load `http://localhost:5890` (the non-running Vite dev server), showing a blank page with **"Could not connect to localhost: Connection refused."** This is the #1 recurring AutoSam deploy bug.
  - **How to verify a binary is a correct prod build:** `strings <binary> | grep -c _app/immutable` must return a number **> 0** (≈20). That counts the embedded SvelteKit asset refs. A broken (cargo-only) binary returns **0**. Do NOT use `grep localhost:5890` to test — that string is present in BOTH good and bad binaries (it's just the embedded config blob) and proves nothing. A correct binary is also visibly larger (~27.4MB vs ~26.7MB) because of the embedded assets.
  - (This was first fixed in commit `6d5c6a2`, but only in AGENTS.md — the AGENTS.md note's `target/aarch64-unknown-linux-gnu/release/` path is WRONG for this host; see step 3.)
- **Always rebuild AND deploy after pushing changes.** Building alone is NOT enough — the running production process must be replaced, or the binary stays stale while the source moves on.
  - **Spark "Moria" (primary, Linux) — current reality:** prod runs as `/usr/bin/agent-one`, managed by the systemd user service `samwise-agent-one.service`. Deploy steps:
    1. **Build (from project root):** `doppler run --project agent-one --config prd -- npx tauri build --no-bundle` (`--no-bundle` skips the slow .deb/.rpm/AppImage step; assets still embed correctly). This also runs `beforeBuildCommand` (`npm run build`), so the frontend is rebuilt for you. On this host the binary is written to `src-tauri/target/release/agent-one` (the host target triple IS the default, so there is NO separate `target/aarch64-unknown-linux-gnu/` output — that dir holds only stale binaries; ignore it).
    2. `systemctl --user stop samwise-agent-one.service`
    3. `sudo cp src-tauri/target/release/agent-one /usr/bin/agent-one` — then confirm before starting: `strings /usr/bin/agent-one | grep -c _app/immutable` (must be > 0).
    4. `systemctl --user start samwise-agent-one.service`
    To restart without rebuilding: `systemctl --user restart samwise-agent-one.service`. ⚠️ Do NOT use `pkill` + manual launch — the WebView (Tauri UI) requires the systemd user session environment to initialize properly (WebKitWebProcess, WebKitNetworkProcess). A bare shell launch starts the worker but not the GUI.
    Note: changes that only touch `~/.codex/config.toml` or other CLI configs take effect on the next child-process spawn WITHOUT a rebuild.
  - **Mac (legacy/`bin/deploy.sh`):** `doppler run -- npx tauri build` → stop the running instance → replace `/Applications/SamWise.app` → `launchctl kickstart`.
- The frontend source of truth for board columns is `src/lib/types.ts` (Tauri app) AND `web/src/lib/types.ts` (separate SvelteKit viewer under `web/`). Changes to statuses or labels must be applied to BOTH. Same rule for any other shared-shaped data — treat `web/` as its own app with its own types.

## Architecture

### Two-Process Model
- **Frontend** (SvelteKit 5 + Tailwind v4): Board + Chat UI. Adapter-static, SSR disabled. Svelte 5 runes.
- **Backend** (Rust/Tauri v2): Claude Code CLI, Supabase REST, Playwright screenshots, Git, worker loop, Telegram bridge.

### Frontend (`src/`)
- `routes/+page.svelte` - Single-page entry, renders AppShell
- `lib/components/shell/` - AppShell (Board + Chat layout), TitleBar, StatusBar
- `lib/components/kanban/` - KanbanBoard, KanbanColumn, KanbanCard, TaskDetailModal, NewTaskModal, CommentThread
- `lib/components/chat/` - ChatPanel, ChatMessage, ChatInput, AgentAvatar
- `lib/components/settings/` - SettingsModal, DopplerTab
- `lib/components/automation/` - CronForm, CronList, TriggerForm, TriggerList (inside Settings)
- `lib/components/playful/` - FloatingBananas, ClickEasterEgg
- `lib/stores/` - tasks, comments, chat, worker, automation, settings, layout, workspace
- `lib/utils/tauri.ts` - Typed wrappers for Tauri invoke (includes `safeInvoke`)
- `lib/supabase.ts` - Supabase JS client for realtime subscriptions
- `lib/types.ts` - TypeScript types matching Supabase tables

### Backend (`src-tauri/src/`)
- `commands/files.rs` - File read/write/create/delete, search
- `commands/git.rs` - Git status, diff, stage, commit, branch, push/pull
- `commands/claude_code.rs` - Spawn/manage Claude Code CLI processes
- `commands/chat.rs` - Sam's chat engine (persistent Claude session, system prompt, task creation from DM)
- `commands/supabase.rs` - Supabase REST (two layers: internal API + Tauri wrappers)
- `commands/worker.rs` - Worker loop: poll tasks, run Claude Code, visual QA, create PRs, Telegram bridge
- `commands/playwright.rs` - Screenshot capture (desktop + mobile)
- `commands/settings.rs` - Settings persistence
- `models/` - Rust structs (FileNode, etc.)
- `state.rs` - Shared app state

### Supabase (project `meqtadfevxguishrlxyx`)
Tables: `ae_tasks`, `ae_comments`, `ae_messages`, `ae_workers`, `ae_crons`, `ae_triggers`, `ae_projects`
Secrets in Doppler project `agent-one`, config `prd`

### Sam's Persona
Defined inline in `chat.rs::build_system_prompt()` (around line 427). Tone: proactive, competent, casual Slack. Asks clarifying questions, flags assumptions, pushes back when something seems wrong. Not a yes-machine. Eventually this will load from a Markdown character file under `~/samwise/` but for now it's a Rust string.

### Task Lifecycle — Where Sam Stops
**Sam's job ends at `approved` (Ready to Merge). Sam never merges or deploys himself.**

The full lifecycle is:
1. Task picked up → Sam codes → opens PR (GitHub)
2. `$samwise-pr-review` (Codex skill) runs automatically via `sweep_pr_review_queue`
3. Verdict: `MergeNow` → card moves to **`approved`** (Ready to Merge) — **SAM STOPS HERE**
   - A fix verdict can run up to **5** automatic review/fix cycles by default. Override with `AUTOSAM_MAX_REVIEW_CYCLES` or persisted `autoFixMaxReviewCycles`; values are bounded to 1–10.
4. Merge + deploy is handled externally by one of:
   - **`pr-review-batch` cron** at 12 past the hour — runs via Rivendell's Codex forge, does the full review → merge → deploy
   - **"Merge and Deploy" button** in the AutoSam UI — Matt clicks it, which stamps `samwise_merge_deploy_status: "requested"` in context, and `sweep_merge_deploy_requests` in the worker executes it

Post-merge deploy (when triggered): Railway server deploy, Supabase migrations (`supabase db push`), Supabase Edge Functions (`supabase functions deploy` — requires `SUPABASE_ACCESS_TOKEN` in Doppler). **Vercel is NOT triggered by Sam** — Vercel auto-deploys from the repo's configured GitHub branch. Do not assume that branch is `main`; for protected Operly work, ordinary PRs target `dev`.

⚠️ **Critical**: Do NOT auto-stamp `samwise_merge_deploy_status: "requested"` at the `approved` transition in `spawn_pr_review_task`. Doing so bypasses the intended human/cron control point and causes Sam to merge PRs he shouldn't. The `sweep_merge_deploy_requests` gate is intentional and correct — it only fires when the button or cron explicitly requests it.

### Key Patterns
- Stores use Svelte 5 runes and `safeInvoke` for Tauri IPC
- supabase.rs has public internal functions (for worker.rs) + Tauri command wrappers
- Worker posts personality-driven comments as it works (casual, like a senior dev on Slack)
- Visual QA: Playwright screenshots -> Claude Code vision eval -> JSON pass/fail
- `decorations: false` with custom TitleBar.svelte
- Before any task: workspace is reset (fetch, checkout main, hard reset, clean -fdx, new `sam/task-{id}` branch) to prevent state leakage between tasks
- Task prompt is explicit about committing, not just exploring. `max_turns` bounded low to prevent runaway reads.

### Cross-platform notes
- Runs on Linux (primary — the DGX Spark), macOS (legacy mini), and Windows (Trenzalore)
- Platform-specific code is gated with `cfg!(target_os = ...)` blocks, not assumed
- Screenshot directory resolves via `dirs::data_local_dir()`, not hardcoded
- Claude Code CLI lookup checks platform-appropriate paths: `~/.local/bin/claude` on macOS/Linux, `%USERPROFILE%\.local\bin\claude.exe` on Windows, PATH fallback on all
