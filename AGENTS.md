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

## Deployment

**Primary host:** DGX Spark "Moria" (Ubuntu 24.04, aarch64), 24/7. Worker id / hostname `spark-3065`. Accessed via screen sharing or Tailscale from Trenzalore (Windows workstation) and phone. Migrated here from the old Mac mini on 2026-05-29 (mini retired; its `ae_workers` row still lingers but stopped heartbeating at cutover).

**Secondary host:** Trenzalore. The Tauri desktop app can run on either machine, both reading the same Supabase. The worker loop is single-active (enforced via `ae_workers` heartbeat).

**Linux host requirement (Spark):** Codex's PR-review sandbox uses bubblewrap, which needs unprivileged user namespaces. Ubuntu 24.04 blocks these by default via AppArmor, which silently breaks `$samwise-pr-review` (every review returns INCONCLUSIVE and cards stick in Review). Fix is `kernel.apparmor_restrict_unprivileged_userns=0` (persisted in `/etc/sysctl.d/60-unprivileged-userns.conf`). The sandbox also needs network for `gh`, set via `[sandbox_workspace_write] network_access = true` in `~/.codex/config.toml` and the `-c sandbox_workspace_write.network_access=true` flag in `review.rs`.

## Commands

```bash
npx tauri dev          # Full app (Vite + Rust) - dev server
npx tauri build        # Production build (embeds frontend assets + bundles .deb/.rpm/AppImage). ALWAYS use this for prod, never cargo build --release alone.
cd src-tauri && cargo check  # Rust only
npm run check          # Svelte type check
```

**Production binary (Linux/aarch64):** `src-tauri/target/release/agent-one` (the host target triple is the default, so `npx tauri build` writes here — there is NO separate `target/aarch64-unknown-linux-gnu/` output on this host; that dir holds only stale binaries). Verify a build embedded the frontend with `strings <binary> | grep -c _app/immutable` (must be > 0).
**Production binary (macOS):** `src-tauri/target/release/bundle/macos/Samwise.app`
**Production binary (Windows):** `src-tauri/target/release/agent-one.exe`

## Build Rules

- **Always rebuild AND deploy after pushing changes.** A build alone is NOT enough — the running production process must be replaced, or the binary stays stale while the source moves on.
  - **Spark "Moria" (primary, Linux) — current reality:** prod runs as `/usr/bin/agent-one` (native Linux binary, built to `src-tauri/target/release/agent-one`), managed by the systemd user service `samwise-agent-one.service`. Deployment:
    1. **Build:** `npx tauri build --no-bundle` from the project root (`--no-bundle` skips the slow .deb/.rpm/AppImage step; assets still embed). This runs `beforeBuildCommand` (npm run build) and properly embeds frontend assets via Tauri's protocol handler. **Do NOT use `cargo build --release` alone** — it skips the asset-embedding step, so the binary has NO frontend and the WebView falls back to `devUrl` (`http://localhost:5890`), showing a blank "Could not connect to localhost: Connection refused" page. The binary is written to `src-tauri/target/release/agent-one`.
    2. **Deploy:**
       ```bash
       systemctl --user stop samwise-agent-one.service
       sudo cp src-tauri/target/release/agent-one /usr/bin/agent-one
       strings /usr/bin/agent-one | grep -c _app/immutable   # must be > 0 before starting
       systemctl --user start samwise-agent-one.service
       ```
    To restart without rebuilding: `systemctl --user restart samwise-agent-one.service`. To stop completely (prevents auto-restart): `systemctl --user stop samwise-agent-one.service` (the `Restart=on-failure` policy only restarts on crashes, not clean stops).
    - **Systemd restart policy:** The service uses `Restart=on-failure` with `RestartSec=15` and `TimeoutStopSec=30`. This means crashes auto-recover, but clean `systemctl --user stop` or tray "Quit" exits stay stopped. The 15-second restart delay gives WebKit compositor processes time to clean up before a new WebView is created.
    - **Crash-loop guard:** `src-tauri/src/lib.rs` writes a timestamp to `/tmp/samwise-startup` on launch. If the previous start was <30s ago, it pauses to let WebKit processes fully die. This prevents the blank/error window from rapid force-kill-then-restart cycles.
    - ⚠️ Do NOT use `pkill agent-one` + manual launch — the WebView (Tauri UI) requires the systemd user session environment to initialize properly (WebKitWebProcess, WebKitNetworkProcess). A bare shell launch starts the worker but not the GUI, causing "connection refused" in the app.
    - Note: changes that only touch `~/.codex/config.toml` or other CLI configs take effect on the next child-process spawn WITHOUT a rebuild.
  - **Mac (legacy/`bin/deploy.sh`):** `doppler run -- npx tauri build` → stop the running instance → replace `/Applications/SamWise.app` → `launchctl kickstart`.
- **When updating Tauri UI, also update and deploy the web UI.** The desktop app under `src/` and the separate web app under `web/` must stay visually/functionally in sync for shared board workflows. Any UI affordance, label, status action, stamp, review/deploy panel, or task interaction added to the Tauri UI needs the equivalent web UI change in the same task unless explicitly impossible. After pushing, deploy both surfaces: the desktop binary (see deploy note above — `bin/deploy.sh` only on Mac) and the web UI via the repo's Vercel workflow/CLI.
- **Merge-to-Done must include post-merge deployments.** Do not add a UI path that marks PR-backed cards Done immediately after merge. The worker must check the PR file list, run any required Railway server deploys, Supabase migrations, and Supabase Edge Function deploys, then mark Done only after those steps succeed. If any deployment is needed or skipped, make that explicit in Sam comments, PR/commit text, and UI copy.
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

### Task Lifecycle — Sam Merges to Main

**Sam now auto-merges approved PRs to `main`.** Main isn't production (Matt manually promotes), so the merge gate is just a formality. No separate cron review needed.

The full lifecycle is:
1. Task picked up → Sam codes → opens PR (GitHub)
2. `$samwise-pr-review` (Codex skill) runs automatically via `sweep_pr_review_queue`
3. Verdict: `MergeNow` → card moves to **`approved`** and auto-stamps merge request
4. `sweep_merge_deploy_requests` picks it up on the next worker cycle → merges PR to `main`
5. Matt manually promotes `main` to production when ready

Post-merge deploy (when triggered): Railway server deploy, Supabase migrations (`supabase db push`), Supabase Edge Functions (`supabase functions deploy` — requires `SUPABASE_ACCESS_TOKEN` in Doppler). **Vercel is NOT triggered by Sam** — Vercel auto-deploys from the repo's configured GitHub branch.

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
