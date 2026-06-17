# AutoSam LLM Backend — Z.ai Coding Plan (GLM 5.2 Max)

## Current Setup

AutoSam still runs the **Claude Code CLI**, but the model behind it is **GLM 5.2 Max (thinking mode)** served by the **Z.ai coding plan**, not Anthropic Claude.

The Z.ai coding plan exposes an **Anthropic-compatible endpoint**, so Claude Code talks to it natively — there is **no translation layer and no LiteLLM**. AutoSam points Claude Code at Z.ai by injecting two env vars into every spawned CLI process:

- `ANTHROPIC_BASE_URL` → the Z.ai coding-plan Anthropic base URL (e.g. `https://api.z.ai/api/anthropic`)
- `ANTHROPIC_API_KEY`  → the Z.ai coding-plan key

Same Claude Code agent loop (tool use, streaming, file ops); only the upstream endpoint and model change.

### Architecture

```
AutoSam → Claude Code CLI → (ANTHROPIC_BASE_URL + ANTHROPIC_API_KEY) → Z.ai coding plan → GLM 5.2 Max (thinking)
              ↓
         Thinks it's talking to Anthropic.
         Z.ai answers in the Anthropic Messages API shape, so no proxy/translation is needed.
```

No LiteLLM. No Fireworks. Any `claude-*` model name AutoSam sends (e.g. `claude-opus-4-8`) is just the upstream label — it resolves to GLM 5.2 Max on the Z.ai plan.

### Where it's wired (code)

| File | Role |
|------|------|
| `src-tauri/src/commands/claude_code.rs` | `LlmProxyConfig` struct + `inject_proxy_env()`. Both `spawn_claude_code` and `claude_code_prompt` inject `ANTHROPIC_BASE_URL` + `ANTHROPIC_API_KEY` when configured. |
| `src-tauri/src/commands/worker.rs` | `load_llm_proxy()` helper. Both `run_claude_code_opts` and `run_claude_code_streaming` inject the env vars. Fallback: reads `ANTHROPIC_BASE_URL` from the process env (systemd deployments). |
| `src-tauri/src/commands/health.rs` | `check_llm_proxy` Tauri command — health check from the Settings UI. |
| `src-tauri/src/lib.rs` | Registers `check_llm_proxy`. |
| `src/lib/stores/settings.svelte.ts` | `llmProxyEnabled`, `llmProxyBaseUrl`, `llmProxyApiKey`, `llmProxyBackend` settings fields. |
| `src/lib/components/settings/SettingsModal.svelte` | "LLM Proxy" tab: enable toggle, base URL, key, health check. |
| `src/lib/components/settings/SetupWizard.svelte` | Step 3 also checks endpoint health when enabled. |

> The struct/field names still say "proxy" for historical reasons — the same env-var injection mechanism now points Claude Code directly at the Z.ai endpoint instead of a local proxy.

### Configure

**Settings → LLM Proxy tab:**
- Enable it
- Base URL → the Z.ai coding-plan Anthropic endpoint
- API Key → the Z.ai coding-plan key
- Click "Check Proxy Health"

**Or via env vars** (no settings UI needed — the worker picks these up automatically):
```bash
export ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic
export ANTHROPIC_API_KEY=YOUR_ZAI_CODING_PLAN_KEY
```

## History (superseded)

Earlier, AutoSam routed Claude Code through a local **LiteLLM proxy** (`localhost:9876`) that translated the Anthropic Messages API → OpenAI format and hit **Fireworks GLM** (5.1, then 5.2). That LiteLLM hop is **no longer used** — the Z.ai coding plan speaks the Anthropic API natively, so Claude Code points straight at it. The `litellm/` folder (`proxy_config.yaml`, `autosam-litellm.service`, `setup.sh`) is kept for reference only and is not part of the live path.
