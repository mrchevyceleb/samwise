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

**Claude Code now runs on real Anthropic Opus 4.8 (`claude-opus-4-8`), reasoning effort `xhigh`.** This is experimental ("for now"), set up to compare results against the prior GLM setup, and it is fully reversible. The worker runs the Claude Code CLI exactly as before (same agent loop: tool use, streaming, file ops), but the model is genuine Anthropic Opus 4.8. The `claude-opus-4-8` label is now literal: it resolves to real Anthropic Opus 4.8, NOT to GLM. Model and effort constants `CLAUDE_MODEL` / `CLAUDE_EFFORT` live in `src-tauri/src/commands/claude_code.rs`.

**Account (do not mix up):** Claude Code authenticates against the **KG Claude account, `mtjohnston42@gmail.com`** (a Claude Max subscription), via OAuth. It must NOT use the Personal account `mjohnst@gmail.com`. OAuth creds live in `CLAUDE_CONFIG_DIR=/home/mrchevyceleb/.config/autosam/claude-config/.credentials.json` (copied from `~/.claude/.credentials.json`).

**Routing mechanism (lives in systemd env, NOT in the repo):** the worker's `load_llm_proxy()` reads `AUTOSAM_LLM_PROXY_URL` and injects it as `ANTHROPIC_BASE_URL`. With those env vars removed, Claude Code hits `api.anthropic.com` directly and authenticates via the OAuth creds above. The switch (done 2026-06-19) removed the Z.ai/GLM proxy env vars (`AUTOSAM_LLM_PROXY_URL`, `AUTOSAM_LLM_PROXY_API_KEY`, `AUTOSAM_DEFAULT_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, `ANTHROPIC_DEFAULT_HAIKU_MODEL`) from `~/.config/systemd/user/samwise-agent-one.service`, deleted the `samwise-agent-one.service.d/failover.conf` drop-in (it pointed the proxy at a local `http://127.0.0.1:8788`), and removed `CLAUDE_CODE_SIMPLE=1`.

⚠️ **CLAUDE_CODE_SIMPLE gotcha:** with `CLAUDE_CODE_SIMPLE=1` set, Claude Code forces API-key auth and REJECTS OAuth subscription login, dying with `Not logged in · Please run /login`. That flag was needed for the GLM/Z.ai API-key setup but is incompatible with the KG Max OAuth auth, so it must stay removed for Opus.

No rebuild was needed for this switch: the model/effort were already baked into the deployed `/usr/bin/agent-one`, so it was a pure runtime-config change.

**Revert path:** a backup of the prior GLM config sits at `~/.config/autosam/glm-revert-backup-20260619/`. Restore the `.service` file plus `service.d/` drop-in, `systemctl --user daemon-reload`, then restart the service. A clean revert to GLM also requires restoring `CLAUDE_CODE_SIMPLE=1`.

This supersedes both the earlier Z.ai coding plan / GLM 5.2 Max routing and the older Fireworks GLM 5.1 LiteLLM proxy approach documented in `LLM-PROXY-SWAP.md`. Treat both of those as historical/superseded.

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
