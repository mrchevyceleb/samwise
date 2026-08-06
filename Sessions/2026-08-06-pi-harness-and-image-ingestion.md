# Pluggable coding harness (pi) and image ingestion repair

## Context

Picked up a dead Pi session that had been planning autosam-pi work. That session had cut over to a standalone `autosam-pi` worker, which stopped `samwise-agent-one.service` and killed Slack ingestion, then reverted. The prior agent then scoped the fix as "build autosam-pi to full parity with agent-one" and its scouts died looking for the source on Windows.

Matt corrected the scope directly: change the harness agent-one uses to write code from Claude Code to pi so any model can be used, with the existing pipeline intact. Not a new worker. The standalone `autosam-pi` daemon stays stopped and out of scope.

The second thread was images. Matt's position was that Slack screenshots were crucial and had regressed. Investigation showed they had never worked.

## Changes

### Harness

- Added `src-tauri/src/commands/coder.rs`: `CoderBackend` enum, `AUTOSAM_CODER=claude|pi` selection defaulting to Claude, pi entrypoint discovery, and model resolution.
- `run_claude_code_streaming` and `run_claude_code_opts` became dispatchers; the existing bodies moved to `_impl` so all 21 call sites and the Claude Code path are byte-for-byte unchanged.
- Added a pi NDJSON parser mapping `toolcall_end` / `turn_end` / `agent_end` onto the same progress, result, and error contract the stream-json parser produced.
- pi has no `--max-turns`, so the cap is enforced by counting `turn_start` events and SIGTERMing the child, reusing the existing PID-slot abort path.
- Extracted `tool_progress_message` and `CommandLoopDetector` so both harnesses render identical board comments from one source of truth. Claude Code's inline formatting and loop detection now route through them.
- Scrubbed LiteLLM env from pi spawns; pi's own `anthropic` provider reads `ANTHROPIC_API_KEY` and would otherwise route back through the proxy the harness exists to remove.
- Prompt is delivered on stdin, never argv.
- Discovery deliberately avoids PATH: `/usr/bin/pi` on this host is an unrelated 2024 ELF binary. `find_pi_command()` resolves the pi package entrypoint under `node`. The spawned process renames itself to `pi`, so `pgrep -f cli.js` finds nothing.
- Harness name now appears in board comments and auto-fix messages instead of a hardcoded "Claude Code".

### Images

- `AUTOSAM_CODER_HANDLES_VISION` is now derived from the active harness and model in `coder::coder_handles_vision()` rather than hand-set. The manual flag had drifted: set for vision-capable Kimi on 07-23, the model was reverted to text-only GLM 5.2 the next day, and it stayed on for 13 days.
- Attachments are content-validated on download. Bytes claiming to be an image but sniffing as HTML are rejected instead of reaching the model. The Telegram fetch path carries the same guard.
- `download_attachment_for_task` used `name_hint` verbatim, so signed storage URLs put a JWT in the filename and failed with `ENAMETOOLONG`. Now query-stripped and length-capped with the extension preserved.
- Failed attachments are reported on the card instead of being swallowed.
- In `assistant-mcp` (deployed to Railway, not version controlled): `downloadSlackFile` now rejects HTML responses, `materializeSlackFiles` returns failures, and Samwise posts a warning in-thread naming what did not download.

## Root causes

**Slack images never worked.** Not a regression and not the cutover. Of 233 Slack-sourced tasks, 26 carried images, the first on 2026-06-25. Fetched stored bytes across the full range (06-25 through 08-06): every one begins with `<!DOCTYPE html`. Zero real images, ever.

The cause was a missing `files:read` scope on the Samwise bot token. Slack does not error on an unauthorized file fetch, it returns `200 OK` with its sign-in page, so an uploader checking only `res.ok` stored a login page as `image/png`. Nothing ever logged an error.

What hid it: the `size` recorded in attachment metadata is what Slack reported for the real screenshot, so it varied plausibly from 31KB to 2.5MB, while every stored object was the same ~62KB login page. The board looked healthy.

Matt added the scope. Verified after: `files.info` returns ok, the download returns `image/png`, and the bytes are a real PNG.

**Secondary, compounding:** the coder was GLM 5.2, which rejects image blocks outright (`400 "This model does not support image inputs"`, reproduced against the live proxy), while the vision-adapter fallback was disabled by the stale flag. So even a valid image would not have been seen.

## Validation

- Rust library suite: 68 passed, 0 failed. 26 tests added covering the pi event parsers, harness-agnostic tool naming, loop detection, filename sanitisation, and the two real-world regressions (the signed-URL filename and the Slack sign-in HTML).
- `cargo check`: passed with the pre-existing warning baseline only.
- Verified the exact pi invocation by hand before wiring it, and confirmed `pi --mode json` emits clean NDJSON on stdout with boot chatter on stderr.
- Confirmed pi's `read` tool feeds images to the model: kimi-k3 read the calendar screenshot accurately, including the full Zoom URL, guest list, and RSVP state.
- Audited every ingestion surface for corrupt bytes. Slack was the only broken one; operly-triage, qa-hub, telegram, and web-board all store real images. The bug class only affects surfaces that fetch a remote file, which is Slack and Telegram; both now carry the guard.
- Production build: `npx tauri build --no-bundle`, 20 embedded `_app/immutable` markers on both the built and installed binary.

## Rollout

- Deployed `/usr/bin/agent-one`, SHA-256 `de3bc71f613bcba9f9148f0766fea07ace94cfb1f2038c2ea772a93f470b0fa6`, service active with 0 restarts.
- `AUTOSAM_CODER=pi` on `accounts/fireworks/routers/kimi-k3`, provider `fireworks` (the main account, not `fireworks-personal` or the unkeyed `fireworks-secondary`), thinking `xhigh`.
- Deployed `assistant-mcp` to Railway via the structured upload. Post-deploy check confirmed 65 tools and 15 `supabase_*` tools, guarding against the known stale-tree regression.
- Commits `5c66671` (harness and image fixes) and `715a368` (Codex auth doc correction), pushed to `origin/master`.

Verified live on three real cards: a `$pipeline-rescue` that correctly identified a GitHub Actions outage and declined to invent a fix; an email-composer bug that reached Review with `review_cycle_count: 0`; and an operly-triage card where kimi read the error string `Could not verify correction status` out of a PNG and grepped for it. That string appears nowhere in the task text.

## Notes and open items

- The 2026-07-24 verdict on Kimi ("78% zero-edit session rate, tool-looping") came from running it through LiteLLM, where `drop_params: true` is mandatory and the param it drops is `thinking`. Kimi was reasoning-disabled that week. pi passes thinking natively, so that result does not carry over. Early runs support this, but the sample is small.
- Codex PR review is on the ChatGPT subscription, not an API key. Both work; the Doppler `agent-one/prd` key is valid but has a $0 balance (`credit_balance_exhausted`, a credit error rather than a model error, so it will work once credits are added). AGENTS.md previously claimed subscription auth could not serve `gpt-5.6-sol`; that is false and was corrected.
- Restarting agent-one to apply `xhigh` killed an in-flight Codex review. The 25-minute stale guard recovered it and the review re-ran unattended. Check for in-flight reviews, not just in-flight tasks, before restarting.
- No Slack message with an image has yet flowed through the redeployed webhook. Every component is verified independently; the end-to-end link proves itself on the next screenshot.
- Historical Slack attachments remain login pages. All those tasks are closed, so nothing is recoverable or needed.
- The desktop Tauri app cannot attach images at all (no file input, no paste handler, and `chat.rs` never sets attachments). Raised and explicitly declined: Matt works from the `web/` viewer, which has full upload support.
