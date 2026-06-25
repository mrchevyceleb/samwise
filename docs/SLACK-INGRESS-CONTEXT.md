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
- `attachments` includes Slack route metadata plus any materialized Slack files:
  - `{ source: "slack", route_id, slack: { channel, thread_ts, user, ... } }`
  - `{ source: "slack_file", url, name, mime_type, size?, slack_file_id?, message_ts? }`

AutoSam's worker polls `ae_messages` for `needs_response = true` in `check_remote_chat_messages()`, routes the text through Sam's normal chat/task prompt, and writes Sam's response back to the same `conversation_id`.

When the inbound message carries valid Slack routing metadata, AutoSam copies sanitized route metadata onto its agent response. `ASSISTANT-HUB` polls the same conversation for the matching agent response by `route_id` and posts it back into the originating Slack thread.

Slack files are handled differently from reply route metadata. `ASSISTANT-HUB` downloads files from Slack using `SLACK_SAMWISE_BOT_TOKEN`, uploads them into the public `task-attachments` Supabase Storage bucket, and puts only the public storage URL plus safe file metadata into `ae_messages.attachments`. AutoSam copies those `slack_file` entries onto any task created from that Slack turn, so the worker's existing `task.attachments[]` materialization path can download images/screenshots/PDFs before running Claude Code.

For threaded requests, `ASSISTANT-HUB` collects files from both the triggering mention/DM and earlier messages in the Slack thread before the mention. This supports the common flow where someone posts a screenshot, then replies `@Samwise can you fix this?`.

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

## Slack Follow-Ups

Slack route metadata also carries a signed task callback route when a shared callback secret is available in `ASSISTANT-HUB`. Resolution order is `SAMWISE_SLACK_CALLBACK_SECRET`, `SAM_CALLBACK_SECRET`, `AUTOSAM_TASK_WEBHOOK_SECRET`, then `TASK_WEBHOOK_SECRET`.

- `callback_url = https://matt-assistant-production.up.railway.app/webhook/slack/samwise-task-callback?...`
- `callback_secret = <shared secret>`

AutoSam copies these fields onto Slack-created `ae_tasks`. The existing `notify_callback()` path then fires on task status transitions. `ASSISTANT-HUB` verifies `X-Samwise-Signature` and posts important lifecycle updates back into the original Slack thread:

- PR created / Review status
- Review approved
- Fixes needed
- Done
- Failed

## Workflow Tags

Slack supports workflow hashtags separately from project hashtags:

- `#review` / `#pr-review` -> PR review workflow

When this tag is present, `ASSISTANT-HUB` writes `slack.workflow = "pr_review"` into the route metadata. AutoSam handles that before calling the chat model: it extracts GitHub PR links from the Slack request and thread context, then creates or revives Review-column cards with `pr_url`, `repo_path`, `project`, and `context.pr_review_required = true`. If no PR URL is found, Sam replies in Slack asking for the link. If a card already exists for a PR, Sam reports that instead of duplicating it.

Recommended team syntax:

`@Samwise #studio #review these PRs`

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
- `SAMWISE_SLACK_CALLBACK_SECRET` optional; falls back to existing `SAM_CALLBACK_SECRET`, `AUTOSAM_TASK_WEBHOOK_SECRET`, then `TASK_WEBHOOK_SECRET`
- `AUTOSAM_TASK_WEBHOOK_SECRET` remains used by the `/samwise` slash command/direct task path.

Required AutoSam behavior:

- Remote chat responses must preserve Slack metadata from inbound `ae_messages.attachments`.
- Remote chat task creation must preserve sanitized `slack_file` attachments on created `ae_tasks.attachments`.
- Remote chat task creation must preserve Slack `callback_url` / `callback_secret` on created `ae_tasks` so task lifecycle callbacks can post back to Slack.
- `slack.workflow = "pr_review"` must trigger deterministic PR review card creation/revival from PR links in the Slack turn, not a generic coding task.
- Remote chat processing must not be limited to the default desktop conversation UUID; Slack uses separate conversation IDs per channel/thread/DM route.
