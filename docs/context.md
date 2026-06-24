# Code Context: Tauri desktop app (src/) vs web board viewer (web/) divergence

Two separate SvelteKit 5 apps sharing Supabase tables. `review-actions.ts` is byte-for-byte identical in both, and both route cards through the shared `displayColumnStatus()` helper, so column grouping logic is in sync. The divergence is almost entirely in the **rendering surfaces and the action affordances** built on top of that shared logic. Below, "Tauri" = `src/`, "web" = `web/`.

Desktop-only surfaces (chat sidebar, settings, automation/cron editor, Tauri invoke commands, AppShell/TitleBar/StatusBar, CommandPalette) are intentionally excluded. Only shared board workflows are compared.

---

## Files Retrieved

1. `src/lib/types.ts` (full) and `web/src/lib/types.ts` (full) — shared type/enums + status/origin metadata.
2. `src/lib/utils/review-actions.ts` (full) and `web/src/lib/utils/review-actions.ts` (full) — identical merge/review pipeline logic.
3. `src/lib/components/kanban/KanbanCard.svelte` (full) vs `web/src/lib/components/KanbanCard.svelte` (full).
4. `src/lib/components/kanban/KanbanColumn.svelte` (full) vs `web/src/lib/components/KanbanColumn.svelte` (full).
5. `src/lib/components/kanban/KanbanBoard.svelte` (full) + `CardContextMenu.svelte` (full) vs `web/src/routes/+page.svelte` (full).
6. `src/lib/components/kanban/TaskDetailModal.svelte` (full) + `CommentThread.svelte` + `SubtaskChecklist.svelte` vs `web/src/lib/components/TaskDetail.svelte` (full).
7. `src/lib/components/kanban/NewTaskModal.svelte` (full) vs `web/src/lib/components/NewTaskModal.svelte` (full).
8. `src/lib/stores/tasks.svelte.ts` (full) vs `web/src/lib/stores/tasks.svelte.ts` (full) — action semantics + column sorting.

---

## 1. types.ts divergence (`src/lib/types.ts` vs `web/src/lib/types.ts`)

Enum unions are identical (`TaskStatus`, `TaskPriority`, `TaskType`, `OriginSystem`, `CronExecutionMode`, `CronRunStatus`). The `AeTask` shape and the status/origin metadata differ:

**Fields on Tauri `AeTask` missing from web:**
- `screenshots: unknown[] | null` — Tauri types it; web omits it (web only has `screenshots_before`/`screenshots_after`). Web card never reads a top-level `screenshots` so this is latent only.
- `last_pr_review_at` is on **web only**, not Tauri (web `AeTask`, ~line 90).

**Fields on web `AeTask` missing from Tauri:**
- `attachments: Attachment[] | null` + the `Attachment` interface (`{url,name,mime}`) — web-only feature.
- `last_pr_review_at?: string | null` — web-only.

**`AeComment` shape differs (shared workflow — comments thread):**
- Tauri `AeComment` has `mentions: string[]` (`src/lib/types.ts`). Web `AeComment` drops `mentions` entirely. The web comment renderer (`TaskDetail.svelte` `renderCommentHtml`) only auto-links URLs and does no @mention highlighting, so this is consistent with web's read-only display, but it means the types are out of sync for the same table.

**`source`/`task_type` typing:**
- Tauri: `source: TaskSource`, `task_type: TaskType` (narrow unions).
- Web: `source: string`, `task_type: string` (widened). Web also does not export `TaskSource`/`CommentAuthor`/`MessageRole`/`TriggerSourceType`/`WorkerStatus` types at all (those last ones are desktop-only and fine to omit).

**Status/origin metadata (the real surface divergence):**
- Tauri exports `KANBAN_COLUMNS[]` with per-column `color` (hex) + `glowColor` + `icon` + `label`, plus `PRIORITY_COLORS` (hex), `SOURCE_ICONS`, and `ORIGIN_BADGES` (label + hex color/bg/border) + `getOriginBadge()`.
- Web exports `STATUSES[]` (bare status strings), `STATUS_LABEL`, `PRIORITY_COLOR` (Tailwind class strings), `ORIGIN_LABEL` + `ORIGIN_BADGE_CLASS` (Tailwind classes) + `getOriginKey()`. Web has NO equivalent of `KANBAN_COLUMNS` (color/icon/glow) — it rebuilds column chrome locally.

The two use **different color systems**: Tauri uses theme hex tokens; web uses Tailwind utility classes. Some semantic colors disagree (see §3).

---

## 2. review-actions.ts — IN SYNC

`src/lib/utils/review-actions.ts` and `web/src/lib/utils/review-actions.ts` are byte-identical (same `displayColumnStatus`, `isMergeInFlight`, `extractReviewActionPanel`, all merge-state helpers, button-label functions). No gap here. Both boards call `displayColumnStatus()` for column grouping (`src/lib/stores/tasks.svelte.ts:29` and `web/src/routes/+page.svelte` `byStatus`). Good.

---

## 3. KanbanCard divergence (`src/.../KanbanCard.svelte` vs `web/.../KanbanCard.svelte`)

### CRITICAL (workflow-affecting)

- **`canMergeDeploy` drops `reviewMergeState.status === 'failed'`.** This is the one behavioral bug in shared review/merge logic.
  - Tauri card (`src/.../KanbanCard.svelte:70`): `... || mergeDeployState.status === 'failed' || reviewMergeState.status === 'failed'`.
  - Web card (`web/.../KanbanCard.svelte:39`): `... || mergeDeployState.status === 'failed'` — **missing the `reviewMergeState.status === 'failed'` clause.**
  - Same omission in web detail (`web/.../TaskDetail.svelte:63`) vs Tauri detail (`src/.../TaskDetailModal.svelte:166`).
  - Effect: on web, when the "Review & Merge" pipeline fails at the **review phase** (`samwise_review_merge_status === 'failed'`) on a `review`/`fixes_needed` card, the action button falls back to **"Mark Done"** instead of the **"Retry Review & Merge"** label/button. Tauri shows the retry. Same data, different button. This is the single most concrete workflow divergence.

- **Card drop sets raw status without clearing stale claim fields** (web only, by consequence of how web drag works).
  - Web `KanbanColumn.svelte` `handleDrop` calls `tasksStore.setStatus(taskId, status)` which only flips `status` (+ `completed_at` on done). It does NOT clear `worker_id`/`claimed_at`/`failure_reason`.
  - The web store's `requeueTask` DOES clear those (`web/src/lib/stores/tasks.svelte.ts`), so the infrastructure exists, but a plain drag-to-column leaves a stale `worker_id` that the worker treats as "still claimed."
  - Tauri drag (`KanbanBoard.svelte` → `taskStore.moveTask`) has the same raw-flip behavior, so this is actually parity, not a gap. *(Documented for completeness — not a web-only defect.)*

### Cosmetic / missing indicators (web card is missing these Tauri card elements)

The Tauri card renders a rich bottom indicator row + working state; the web card is much sparser. On the web card, these **present in Tauri are absent**:

- **Visual QA badge** — Tauri renders a "QA Passed/Failed/Skipped" badge from `task.visual_qa_result` (KanbanCard `$derived(qaResult ...)`). Web card has no QA badge at all (QA only appears in the web detail modal).
- **Live working elapsed timer** — Tauri shows a ticking `m s`/`h m` timer from `task.claimed_at` for `in_progress`/`testing` cards (with `pulse-dot`). Web card shows no elapsed/working timer; it only shows `relTime(task.updated_at)`.
- **Latest agent comment preview** — Tauri shows the newest comment preview line for working/testing/review cards (what Sam is doing right now). Web card does not render any latest-comment preview.
- **Comment count indicator** — Tauri renders a comment-count bubble from `commentStore.getCommentCount`. Web card has no comment count.
- **PR link icon button** (bottom row) — Tauri renders a clickable GitHub-PR icon; web shows only a `PR #<n>` text badge (no icon-button), though it does expose PR via the review panel's "PR" button.
- **Report link icon** — Tauri renders a report icon linking `task.report_url` (tailnet). Web card omits the report link entirely.
- **Screenshot indicator** — Tauri renders a camera icon when `hasScreenshots`. Web card has no screenshot indicator.
- **Assignee indicator** (robot/Matt avatar) — Tauri renders an agent/matt icon. Web card shows no assignee indicator.
- **Branch pill** — Tauri shows `branch <x>` / `base <x>` pill in the badge row; web shows it in the bottom row (`⎇ branch`). Minor positional/wording difference.

### Extra on web card (not a defect, noted for parity awareness)
- Web card shows `commit_message` inline (`pre` block); Tauri card does not (Tauri shows commit_message only in the detail modal).
- Web card has inline **Hold/Release** (queued) and **Re-queue** buttons; Tauri exposes these via the right-click context menu (`CardContextMenu.svelte`) instead of inline. Functionally equivalent coverage.

---

## 4. KanbanColumn divergence (`KanbanColumn.svelte`)

- **Column status-dot color scheme disagrees.** Tauri pulls each column's dot/glow/border from `KANBAN_COLUMNS.color` (e.g. `in_progress=#6366f1` indigo, `review=#3fb950` green, `approved=#58a6ff` blue). Web hardcodes its own `statusDot` map (`web/.../KanbanColumn.svelte`) with **different colors**: `in_progress=bg-sky-400`, `review=bg-violet-400`, `approved=bg-emerald-400`. So the same status renders with visibly different accent colors across the two apps. Cosmetic, but it is a documented "shared-shaped data must stay in sync" surface.
- **Column label/icon** — Tauri shows an `icon` token + uppercase label + count pill with `in_progress` glow animation + "No tasks"/"Drop here" empty state. Web shows `STATUS_LABEL` + count + "nothing here" empty state, no icon, no working-glow animation. Cosmetic.
- Collapse behavior is equivalent (both collapse `done`/`failed`, persist to localStorage).

---

## 5. KanbanBoard / `+page.svelte` board chrome

**Card ordering within active columns diverges (concrete):**
- Tauri sorts active columns by **explicit priority** (`critical>high>medium>low`) then **created_at descending** (`src/lib/stores/tasks.svelte.ts:39-54`).
- Web does **no client-side sort** in `+page.svelte` `byStatus`; it relies on the refresh query's `.order('priority', { ascending: true })` which orders priority **alphabetically** (`critical, high, low, medium` — i.e. **`low` before `medium`**, wrong) and then `created_at` **ascending** (`web/src/lib/stores/tasks.svelte.ts` `refresh()`).
- Net effect on the same data: (a) a `low`-priority card renders above a `medium` card in the same active column on web but below it on Tauri; (b) equal-priority cards are in **opposite** chronological order between the two apps. Cosmetic but visible.

**Board header chrome:**
- Web header (`+page.svelte`) has: Samwise title, live/connected indicator, build-version watcher (auto-reload on deploy), **Refresh** button, **Reports** link, **Schedules** (cron) button, **search** input, **project filter** select.
- Tauri board (`KanbanBoard.svelte`) header has only: "Tasks" title, total count, add button. No search, no project filter on the board (search/filter/search not present in the shell either — `src/lib/components/shell/` has no projectFilter/query).
- So search + project filter are **web-only extras** (not a web gap). The web's extra "Schedules" board button corresponds to Tauri's Settings>automation (desktop-only surface) — borderline; flagged for awareness.
- Tauri board has a **drag ghost** (floating card following cursor) + Ctrl+N new-task shortcut; web uses native HTML5 drag (no ghost, no shortcut). Cosmetic/interaction difference.

---

## 6. TaskDetail modal vs TaskDetail (`TaskDetailModal.svelte` vs `TaskDetail.svelte`)

This is the largest functional gap. Tauri's modal is a full editor; web's is a read-mostly viewer.

### CRITICAL (workflow-affecting, shared review/merge)
- **`canMergeDeploy` missing `reviewMergeState.status === 'failed'`** (same as §3) — `web/.../TaskDetail.svelte:63` vs `src/.../TaskDetailModal.svelte:166`. Retry-Review-&-Merge button mislabeled as Mark Done when review phase failed.

### HIGH (comment thread workflow)
- **Comments are read-only on web.** Tauri `TaskDetailModal` embeds `CommentThread.svelte`, which is fully interactive: post as `matt`, Enter-to-send, @mention highlighting, markdown/code rendering, scroll-to-bottom. Web `TaskDetail.svelte` "Activity" section only **renders** existing comments (escaped text + URL autolinking via `renderCommentHtml`) — **there is no input box; you cannot post a comment from the web board.** This breaks the same "comment on a card" workflow that works on desktop.

### HIGH (task editing workflow)
- **Inline title editing** — Tauri: click title to edit (`editingTitle`). Web: title is static.
- **Inline description editing** — Tauri: click to edit with markdown rendering. Web: description is read-only plain text.
- **Priority selector** — Tauri: full priority button-list in the right sidebar (`changePriority`). Web: priority is a non-interactive badge only; no way to change priority.
- **Status selector** — both can change status (Tauri sidebar button-list `changeStatus`→`moveTask`; web header `<select>`→`setStatus`). Parity here, though web's `setStatus('done')` has an extra side effect (see below).

### MEDIUM
- **Subtask interactivity** — Tauri embeds `SubtaskChecklist.svelte` (toggle/add/edit/drag-reorder, persists via `updateTask`). Web renders subtasks **read-only** (checkbox emoji + title, no toggle/add). You cannot manage subtasks from the web board.
- **Restart Task action missing on web** — Tauri detail has a "Restart Task" button for `failed` (`isRestartable`, `handleRestart`). Web detail has only "Stop Task"; no Restart.
- **Report tab missing on web** — Tauri detail has Details/Report tabs; the Report tab fetches the report artifact via `supabase_fetch_artifacts` and renders markdown (`renderMarkdown`). Web has no report tab (it links `report_url` externally via `LinkRow`, but does not render the artifact content inline).
- **Mark-Done side-effect divergence:** web `setStatus('done')` calls `closeOriginTicket()` (`web/src/lib/stores/tasks.svelte.ts`), closing the Operly/Banana/etc. origin ticket via `/api/close-origin-ticket`. Tauri `moveTask('done')` does **not** close the origin ticket (`src/lib/stores/tasks.svelte.ts`). Same "Mark Done" gesture → different external behavior.

### LOW / cosmetic
- **Metadata sidebar** — Tauri detail has a right sidebar with Assignee, Project, Source, Origin (+origin_id), Repo URL, Repo Path, Branch/Base, and Created/Claimed/Done dates. Web detail shows project in the header and branch/report/pr/preview/repo as `LinkRow`s, but no assignee/source/origin-id/dates panel.
- **Visual QA** — both render it (parity). 
- **Screenshots before/after** — both render them (parity).
- **Failure reason** — **web shows it** (`TaskDetail.svelte` Failure section); **Tauri detail does not** show `failure_reason`. (Reverse gap: web has the extra here.)
- **Attachments** — web-only (`TaskDetail.svelte` Attachments grid + `NewTaskModal` upload). Tauri has none. (Web extra.)
- **Close PR & Mark Done** — both have it (parity); web routes through `/api/close-pr`, Tauri through the `close_pr` Tauri command.
- **Copy PR link** — both have it (parity).

---

## 7. NewTaskModal (`NewTaskModal.svelte`)

Largely in sync. Both: repo mode (Single/None/Multiple), project picker grouped by client optgroup, base-branch (optional, hidden for qa-verify), prompt, mode (Coding/Research/QA Verify), QA environment (Staging/Production).

Differences:
- **Priority** — neither exposes a priority picker (both hardcode `medium`); consistent (no gap).
- **Schedule** — neither NewTaskModal has a schedule field. (Web's separate `ScheduleModal.svelte` handles cron from the board header; Tauri handles cron in Settings>automation, a desktop surface.)
- **Attachments** — web NewTaskModal supports image/PDF upload (`/api/upload-attachment`); Tauri NewTaskModal does not. (Web extra, consistent with web's attachments feature.)
- Submission path differs by necessity: web posts to `/api/create-task`; Tauri calls `taskStore.createTask` → Tauri command. Equivalent.

No shared-workflow gap in NewTaskModal itself.

---

## Summary: ranked defects (web missing/different vs Tauri for the SAME shared workflow)

**Critical / workflow-breaking**
1. `canMergeDeploy` drops `reviewMergeState.status === 'failed'` — Review-&-Merge retry button mislabeled as "Mark Done" on web when the review phase failed. `web/.../KanbanCard.svelte:39` + `web/.../TaskDetail.svelte:63` (Tauri: `src/.../KanbanCard.svelte:70` + `TaskDetailModal.svelte:166`).
2. Web comment thread is **read-only** — no way to post a comment on a card from the web board (`web/.../TaskDetail.svelte` Activity vs `src/.../CommentThread.svelte`).

**High (workflow present on Tauri, absent on web)**
3. No inline title/description editing in web detail.
4. No priority change in web detail (badge only).
5. Subtasks read-only on web (no toggle/add/edit/reorder).
6. No "Restart Task" action on web detail.

**Medium**
7. Mark-Done side-effect divergence: web closes origin ticket on done, Tauri does not.
8. Card ordering within active columns differs (web alphabetical-priority + created_at-asc vs Tauri correct-priority + created_at-desc).
9. Report tab (inline rendered report artifact) missing on web.
10. Card indicators missing on web: Visual QA badge, working elapsed timer, latest-comment preview, comment count, report icon, screenshot icon, assignee icon.

**Cosmetic**
11. Column status-dot colors disagree between the two apps (Tauri KANBAN_COLUMNS hex vs web hardcoded Tailwind `statusDot`).
12. Web has no drag ghost / Ctrl+N shortcut; web empty-state copy differs ("nothing here" vs "No tasks").
13. Web has extras not on Tauri board: search, project filter, Refresh button, build-version auto-reload, attachments, failure-reason section, ScheduleModal board button.

**Reverse gaps (Tauri missing something web has — for awareness)**
- Tauri detail does not show `failure_reason`; web does.
- Tauri NewTaskModal/detail have no attachments; web does.
- Tauri board has no search/project-filter; web does.

## Start Here
Open `web/src/lib/components/KanbanCard.svelte:39` and `web/src/lib/components/TaskDetail.svelte:63` and add the missing `|| reviewMergeState.status === 'failed'` clause to `canMergeDeploy` (mirroring `src/lib/components/kanban/KanbanCard.svelte:70`). That is the single highest-value, lowest-risk fix. For the read-only comment thread gap, the work is in `web/src/lib/components/TaskDetail.svelte` (Activity section) plus adding a `postComment` to `web/src/lib/stores/tasks.svelte.ts`.
