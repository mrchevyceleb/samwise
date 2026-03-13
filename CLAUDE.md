# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Banana Code IDE. A desktop code editor (like VS Code/Cursor) built with Tauri v2 + SvelteKit + Rust. It has an AI chat panel, integrated terminal (PTY), file explorer, git integration, live preview, and MCP tool support.

## Commands

### Development
```bash
# Run the full Tauri desktop app (starts Vite dev server + Rust backend)
npx tauri dev

# Frontend-only dev server (no Rust backend, limited functionality)
npm run dev

# Type-check the Svelte frontend
npm run check
```

### Building
```bash
# Production build (frontend + Rust binary)
npx tauri build

# Frontend-only build
npm run build
```

### Rust Backend
```bash
# Check Rust compilation
cd src-tauri && cargo check

# Build Rust only
cd src-tauri && cargo build
```

No test suite exists yet.

## Architecture

### Two-Process Model
- **Frontend** (SvelteKit + Tailwind v4): Renders the IDE UI. Uses `adapter-static` to produce a static build that Tauri loads in a webview. All Svelte files use **runes mode** (Svelte 5 `$state`, `$derived`, etc.).
- **Backend** (Rust/Tauri v2): Handles filesystem, PTY terminals, git operations, AI streaming, preview servers, and MCP. The frontend calls Rust via `@tauri-apps/api/core` `invoke()`.

### Frontend Structure (`src/`)
- `routes/+page.svelte` - Single-page app entry point
- `lib/components/shell/AppShell.svelte` - Main layout with resizable panels
- `lib/stores/*.svelte.ts` - Svelte 5 rune-based stores (workspace, file-tree, git, preview, terminals, agents, settings, layout)
- `lib/ai/` - AI chat system:
  - `chat/chat-engine.ts` - Orchestrates AI conversations with tool use loops
  - `providers/` - Anthropic, OpenRouter, OpenAI Codex streaming providers
  - `tools/tool-definitions.ts` + `tool-executor.ts` - AI tool calling (file read/write/edit, terminal, search)
  - `tools/mcp-manager.ts` - MCP server integration (both HTTP and stdio transports)
  - `prompts/base-prompts.ts` - System prompts; reads workspace `AGENTS.md`/`CLAUDE.md` for project-specific instructions
- `lib/components/editor/CodeEditor.svelte` - CodeMirror 6 editor
- `lib/components/terminal/` - xterm.js terminal connected to Rust PTY
- `lib/components/preview/` - Live preview panel using Tauri webview

### Backend Structure (`src-tauri/src/`)
- `lib.rs` - Tauri app setup, plugin registration, command handler registration
- `commands/` - All Tauri IPC commands organized by domain (files, git, ai, terminal, mcp, preview, settings, orchestrator)
- `preview/` - Three-tier preview engine:
  - **DirectServe** (Tier 1): Static file server via axum for plain HTML/CSS/JS
  - **EsbuildBundle** (Tier 2): Runs esbuild to bundle JSX/TSX then serves output
  - **ManagedProcess** (Tier 3): Spawns the project's own dev server (e.g., `npm run dev`)
  - `orchestrator.rs` - Coordinates tier detection, server lifecycle, and file watching
  - `tier_detector.rs` - Analyzes project files (package.json, file extensions) to pick the right tier
- `state.rs` - Shared app state (project root, terminal sessions, MCP sessions)

### Key Patterns
- All frontend-to-backend communication goes through Tauri `invoke()` calls. Commands are defined in `src-tauri/src/commands/` and registered in `lib.rs`.
- Stores use Svelte 5 runes (`$state`, `$derived`) and export getter functions (e.g., `getWorkspace()`, `getPreviewStore()`).
- The AI chat engine supports an agentic tool-use loop: send message, receive tool calls, execute tools, send results back, repeat until the model responds with text.
- The preview system emits Tauri events (`preview:status`, `preview:file-changed`) that the frontend listens to for real-time updates.
- The app window uses `decorations: false` with a custom `TitleBar.svelte` for the title bar.
