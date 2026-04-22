# Samwise - AI Coding Coworker

Matt's AI junior developer. Autonomous coding agent that lives on the Mac mini, reachable from anywhere. Tauri v2 + SvelteKit 5 + Rust.

Named after Samwise Gamgee (LOTR). Personality is Sam: loyal, proactive, "senior dev on Slack" tone. Takes tasks, ships PRs, answers questions.

See `docs/REVIVAL-CONTEXT.md` for the full vision and design rationale.

## What This Is

A desktop app plus a headless-capable worker loop with two interaction surfaces:
1. **Kanban Board** - Trello-style task management with drag-and-drop, comment threads with @mentions
2. **Chat Sidebar** - Direct conversation with Sam (task creation from plain language, status questions, @project tagging)

Planned additional surface: **Telegram bot** for reaching Sam from a phone. Inbound/outbound messages flow through the same `ae_messages` table as the desktop chat.

The backend worker picks up tasks from the board, writes code via Claude Code CLI, takes Playwright screenshots for visual QA, and opens PRs with before/after screenshots.

## Deployment

**Primary host:** Mac mini (macOS), 24/7. Accessed via screen sharing or Tailscale from Trenzalore (Windows workstation) and phone.

**Secondary host:** Trenzalore. The Tauri desktop app can run on either machine, both reading the same Supabase. The worker loop is single-active (enforced via `ae_workers` heartbeat).

## Commands

```bash
npx tauri dev          # Full app (Vite + Rust) - dev server
npx tauri build        # Production build (.app + .dmg on macOS, .exe + installers on Windows)
cd src-tauri && cargo check  # Rust only
npm run check          # Svelte type check
```

**Production binary (macOS):** `src-tauri/target/release/bundle/macos/Samwise.app`
**Production binary (Windows):** `src-tauri/target/release/agent-one.exe`

## Build Rules

- **Always rebuild after pushing changes.** Run `npx tauri build` after any commit/push so the production binary stays current on each host. The binary is used on machines without the dev environment (viewer mode).

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

### Key Patterns
- Stores use Svelte 5 runes and `safeInvoke` for Tauri IPC
- supabase.rs has public internal functions (for worker.rs) + Tauri command wrappers
- Worker posts personality-driven comments as it works (casual, like a senior dev on Slack)
- Visual QA: Playwright screenshots -> Claude Code vision eval -> JSON pass/fail
- `decorations: false` with custom TitleBar.svelte
- Before any task: workspace is reset (fetch, checkout main, hard reset, clean -fdx, new `sam/task-{id}` branch) to prevent state leakage between tasks
- Task prompt is explicit about committing, not just exploring. `max_turns` bounded low to prevent runaway reads.

### Cross-platform notes
- Supports both macOS (primary) and Windows (Trenzalore)
- Platform-specific code is gated with `cfg!(target_os = ...)` blocks, not assumed
- Screenshot directory resolves via `dirs::data_local_dir()`, not hardcoded
- Claude Code CLI lookup checks platform-appropriate paths: `~/.local/bin/claude` on macOS, `%USERPROFILE%\.local\bin\claude.exe` on Windows, PATH fallback on both
