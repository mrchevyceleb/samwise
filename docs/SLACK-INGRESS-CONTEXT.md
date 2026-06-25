# Slack Ingress Context

This repo does not own the Slack app or the public Slack webhook. Those live in:

`/home/mrchevyceleb/ASSISTANT-HUB/assistant-mcp/server`

Important files there:

- `src/routes/slack-webhook.ts` handles Slack Events API callbacks for the Samwise Slack app.
- `src/lib/samwise-bot.ts` posts replies as the Samwise bot using `SLACK_SAMWISE_BOT_TOKEN`.
- `src/lib/samwise-project-resolver.ts` contains legacy project aliases used by strict Slack commands.
- `src/index-railway.ts` captures raw Slack request bodies for HMAC verification and mounts `/webhook/slack`.

The Slack webhook already verifies Slack signatures, dedupes retry events, handles `app_mention`, bot DMs, and `/samwise`, replies in Slack threads, and can pull prior thread messages as context.

## Current Integration Shape

Slack mention and DM events are normalized in `ASSISTANT-HUB` and inserted into AutoSam's `ae_messages` table with:

- `conversation_id = a stable UUID derived from the Slack channel/thread, or from the DM channel/user`
- `role = user`
- `needs_response = true`
- `attachments = [{ source: "slack", route_id, slack: { channel, thread_ts, user, ... } }]`

AutoSam's worker polls `ae_messages` for `needs_response = true` in `check_remote_chat_messages()`, routes the text through Sam's normal chat/task prompt, and writes Sam's response back to the same `conversation_id`.

When the inbound message carries valid Slack routing metadata, AutoSam copies sanitized route metadata onto its agent response. `ASSISTANT-HUB` polls the same conversation for the matching agent response by `route_id` and posts it back into the originating Slack thread.

## Project Matching

Slack supports explicit project hashtags as the most reliable routing convention:

- `#operly` -> `operly`
- `#studio` / `#r-link` -> `r-link-studio-rebuild`
- `#banana-code` -> `banana-code`
- `#pixa` -> `pixa-app`
- `#fiscal` -> `FiscalPilot`
- `#wecare` -> `wecare-dash`
- `#mj-site` -> `MJ-site`

`ASSISTANT-HUB` resolves a single recognized hashtag before separator parsing or channel-name inference and writes the canonical project into Slack route metadata. AutoSam treats that route as authoritative and forces it onto tasks created from that Slack turn, which avoids cards getting stuck in project confirmation when the team includes a hashtag.

## Why This Exists

The older Slack path posted directly to `supabase/functions/task-webhook` and required strict syntax such as:

`@Samwise operly: fix the login crash`

That was reliable, but it bypassed Sam's conversational reasoning. Natural mention handling should use the same brain as desktop chat and remote chat so Matt can write:

`@Samwise can you fix this?`

inside a Slack thread and have Sam use the thread context, infer the project when possible, ask a clarification when needed, or create a task when the request requires work.

## Deployment Notes

The Slack webhook is deployed with `assistant-mcp` on Railway. AutoSam is deployed separately on Moria as `/usr/bin/agent-one`.

Required `ASSISTANT-HUB` env:

- `SLACK_SIGNING_SECRET`
- `SLACK_SAMWISE_BOT_TOKEN`
- `SAMWISE_SERVICE_ROLE_KEY`
- `AUTOSAM_TASK_WEBHOOK_SECRET` remains used by the `/samwise` slash command/direct task path.

Required AutoSam behavior:

- Remote chat responses must preserve Slack metadata from inbound `ae_messages.attachments`.
- Remote chat processing must not be limited to the default desktop conversation UUID; Slack uses separate conversation IDs per channel/thread/DM route.
