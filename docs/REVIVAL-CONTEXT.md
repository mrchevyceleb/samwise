# Samwise Revival Context

Planning conversation that produced the revival direction. Preserved so the vision and design rationale aren't lost to chat history.

## The vision

A personal tool, not a product. Matt owns the machine, the repos, and the risk. No multi-tenancy, sandboxing, or security boilerplate required.

**Samwise is someone.** He has a name (LOTR reference, the gardener who carries the load), a personality ("senior dev on Slack" tone), and a role (Matt's AI junior developer). He lives on the Mac mini with persistent identity and memory.

### Core properties

- Claude Code CLI with Matt's subscription and full MCP setup
- Full filesystem access to Matt's repos, notes, OneDrive sync
- Accessible from anywhere via Tailscale, Telegram, and an eventual web app
- Chat mode (back-and-forth, no code changes) and Task mode (real work, ends in a PR)
- Knows Matt's codebases, conventions, and preferences

## Why this is different from Copilot / Railway ephemeral agents

Every cloud agent starts fresh. Sam doesn't.

When Matt messages Sam three days from now and Sam says "we talked about that Tuesday, you decided to use the Supabase approach, want me to pick it back up?" that's the continuity no cloud tool provides. Memory plus persistent workspace plus always-on presence equals coworker, not tool.

The Copilot cloud agent clones fresh, pokes around, answers, and is gone. Samwise has been in the Operly repo for weeks. His workspace is warm. His node_modules are cached. His git branches are still checked out. He remembers the Tuesday merge and has an opinion about the current design because he was there when Matt decided it.

## Why finish agent-one instead of rebuilding

- 48 commits of working Rust and Svelte already in
- Kanban plus drag-drop plus comments plus @mentions, days of work
- Chat sidebar with realtime Supabase subscriptions
- Doppler secrets (project `agent-one`, config `prd`)
- Playwright to Claude vision to JSON pass/fail visual QA pipeline
- Cron and trigger automation forms
- Six-table Supabase schema that is well-shaped

What stalled wasn't the architecture. It was the last-mile bugs in the worker loop that never got shaken out. Sam would pick up tasks but read without building, hit dependency issues, get confused about which repo, and not create PRs.

**The rebuild trap:** you rebuild 80% of what's there, rediscover the same 20% of hard problems, hit the same wall, lose weeks. The version in your head is always better than the version you built because the version in your head hasn't been pressure-tested yet. Agent-one has been. That's value.

## Why the Mac mini, not Trenzalore

- Trenzalore is Matt's active 4090 workstation, 10 hours a day of use. Sam and Matt would interrupt each other. Dev environment state leaks into Sam's runs.
- The mini is idle, dedicated, always on, and already has the full toolchain (Node, Rust, Claude Code, gh, Doppler).
- The OS port is small. No `windows-*` crates. `dev_server.rs` already has proper `cfg` blocks. Only a few hardcoded `C:\` paths need replacing.

Debugging Sam on the mini is faster than fixing bugs twice on two platforms.

## Why Telegram before the web app

- 4 hours of work vs a weekend
- Covers every device Matt owns
- Reuses the existing `ae_messages` table and persistent Claude session
- Outbound notifications are already wired. Only inbound ingestion is missing.
- Web app becomes additive later, not a blocker

The pain point isn't "I don't have a junior dev." It's "I don't have a way to do software work when I'm not at my desk." Mobile access turns Sam from "thing on the mini" into "thing in my pocket that happens to live on the mini."

## The failure modes Sam hit in the original build

1. **Just reading, not building.** Vague prompt plus high `max_turns` (50) let Claude explore forever.
2. **Dependency confusion.** No per-repo setup scripts, no workspace reset between tasks.
3. **Not creating PRs.** Combination of silent no-op detection, `git diff --cached --quiet` tripping after commit, and PR creation not wired to failure paths.
4. **Confusion on what repo to use.** Dirty workspace state from prior tasks, no clean-slate reset.

All tractable. All addressed in Phase 1 of the revival plan.

## Guardrails for now

- **One persona.** Sam. No multi-specialist split.
- **Proactive but opt-in.** Sam can ping Matt about things he notices, but only when configured to.
- **Task mode requires PR review before merge.** Sam opens PRs. Matt reviews. Loosen over time.
- **Junior dev tone.** Proactive, asks clarifying questions, flags assumptions, pushes back when something seems wrong. Not a yes-machine.

## Phased roadmap

### Phase 1: Finish Sam on the Mac mini

Get one real PR from one real task, end-to-end, on macOS, with visual QA passing.

- Fix hardcoded Windows paths (`C:\agent-one-screenshots`, `%USERPROFILE%\.local\bin\claude.exe`)
- Tighten the task prompt (explicit commit instruction, drop `max_turns` to ~20)
- Add workspace hygiene (fetch, reset, clean, new branch) before every task
- Fix the no-op failure mode (distinguish "nothing to change" from "wrong directory")
- Run against a deliberately simple task in a throwaway repo

### Phase 2: Telegram bidirectional chat

Matt DMs Sam from his phone. Sam answers. Task creation from DM works because `chat.rs` already parses `{"create_task": {...}}` JSON output.

- Complete the inbound path in `check_telegram_messages()` in `worker.rs`
- Route Sam's responses back to Telegram via the existing `send_telegram()`
- Reuse `ae_messages` table with Telegram metadata in `attachments`

### Phase 3 (future, not this cycle)

- **Headless worker.** Extract `commands/worker.rs` into a standalone binary, auto-start via `launchd` on the mini. Tauri UI on Trenzalore continues to work via the same Supabase.
- **Web app.** Extract Svelte kanban and chat components into a separate SvelteKit app, deploy to Vercel. Reachable from any browser.

Both are additive. They don't require changes to the Phase 1 or 2 work.

## Deferred (not this cycle)

- Identity and character file as loadable Markdown. Currently hardcoded in `chat.rs:427`. Valuable but not a blocker. Revisit once Sam ships PRs reliably.
- Journal and project memory files under `~/samwise/` (the OneDrive-synced memory structure)
- Multiple personas (junior dev plus marketing intern plus research assistant)
- Native mobile app. The web app will cover 95% of what Matt would want.
