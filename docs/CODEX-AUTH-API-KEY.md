# AutoSam Codex Auth — Raw OpenAI API Key (2026-08-05)

## Current setup

AutoSam's PR reviewer is the **Codex CLI** (`$samwise-pr-review`, model `gpt-5.6-sol`, ultra reasoning), spawned per review by the worker as the local user. Since 2026-08-05 it authenticates with a **raw OpenAI API key**, NOT the ChatGPT subscription OAuth it used before.

```
AutoSam worker → spawns `codex exec ... $samwise-pr-review` → reads ~/.codex/auth.json (auth_mode=apikey) → OpenAI API (billed to API key)
```

The worker spawns Codex fresh for every review, so `~/.codex/auth.json` is re-read each time. Auth changes take effect on the next review with **no service restart and no rebuild**.

## Where the key lives

| Location | Role |
|----------|------|
| Doppler `agent-one` project, `prd` config, `OPENAI_API_KEY` (also mirrored in `dev`/`stg`) | Canonical secret store (sk-proj key, API-billed) |
| `~/.codex/auth.json` on Moria | What the Codex CLI actually reads. Must be `auth_mode = "apikey"` with `OPENAI_API_KEY` populated |
| `~/.codex/auth.json.bak-chatgpt-20260805` | Pre-switch ChatGPT OAuth backup. Restore only if deliberately reverting |

## Rotate / re-wire procedure

```bash
doppler secrets get OPENAI_API_KEY --project agent-one --config prd --plain | codex login --with-api-key
```

Gotchas learned the hard way:

- **`codex login --api-key <key>` no longer works** (flag removed in Codex v0.144). Pipe the key on stdin with `--with-api-key`.
- Verify with a cheap live probe: `cd /tmp && codex exec --sandbox read-only "Reply with exactly: AUTH_OK"`.
- No `systemctl --user restart samwise-agent-one.service` needed (Codex spawns per review).

## Why the switch (2026-08-05)

The ChatGPT subscription auth was failing in production:

- `gpt-5.6-sol` (the review model) is not served to ChatGPT-account subscription auth — reviews 500'd or died `exit status 1` inside a minute, landing INCONCLUSIVE and burning review cycles (card 635c9d0e capped cyc5 this way on 2026-08-05).
- Subscription OAuth also rate-limited mid-review on large PRs (1200s/1800s timeouts observed 2026-08-03/04).

API-key auth bills per token but serves the model reliably. Validated same-day: live probe passed and the first post-switch review spawned cleanly (pid observed running `gpt-5.6-sol` ultra on PR #1222 instead of dying instantly).

## Symptoms the auth regressed

- Board comments like `Codex review exited with exit status: 1` shortly after `Running $samwise-pr-review`.
- `Codex says: **INCONCLUSIVE**` with no real review body.
- Rapid cycle burn on cards whose diffs are otherwise reviewable.

Check `python3 -c "import json; print(json.load(open('/home/mrchevyceleb/.codex/auth.json')).get('auth_mode'))"` — it must print `apikey`.

## Troubleshooting (learned 2026-08-05)

- **"Codex review exited with exit status: 1", empty stdout, on every review:** probe
  with `echo "Reply with exactly: PROBE_OK" | codex exec --sandbox read-only -`. If the
  error is `You have no credits remaining`, the API billing org is empty. Add credits at
  https://platform.openai.com/settings/organization/billing/ (requires Matt). Symptom on
  the board: cards burn all 5 auto-fix cycles on EMPTY blocker lists (reviews never
  produce a verdict) and cap as fixes_needed with no actionable blockers.
- **Rate-limit (`hit your ChatGPT usage limit`):** you are on `auth_mode=chatgpt`.
  Switch to the API key per the steps above.
