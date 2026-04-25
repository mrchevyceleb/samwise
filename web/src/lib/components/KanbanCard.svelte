<script lang="ts">
  import type { AeTask } from '$lib/types';
  import { PRIORITY_COLOR } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import {
    extractReviewActionPanel,
    getUiStamp,
    isReviewActionStatus,
    nextCopilotStampContext
  } from '$lib/utils/review-actions';
  let { task, onOpen }: { task: AeTask; onOpen: (t: AeTask) => void } = $props();

  let comments = $derived(tasksStore.comments[task.id] ?? []);
  let reviewPanel = $derived(extractReviewActionPanel(task, comments));
  let uiStamp = $derived(getUiStamp(task));
  let showReviewActions = $derived(isReviewActionStatus(task.status) && !!(reviewPanel || task.pr_url));

  function relTime(iso: string) {
    const d = Date.now() - new Date(iso).getTime();
    const m = Math.floor(d / 60000);
    if (m < 1) return 'just now';
    if (m < 60) return `${m}m ago`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h ago`;
    const days = Math.floor(h / 24);
    return `${days}d ago`;
  }

  function verdictColor(verdict: string | undefined) {
    if (verdict === 'merge') return '#34d399';
    if (verdict === 'fix' || verdict === 'blocked') return '#fb923c';
    if (verdict === 'errored') return '#fb7185';
    return '#60a5fa';
  }

  function openPr(e: MouseEvent) {
    e.stopPropagation();
    if (task.pr_url) window.open(task.pr_url, '_blank', 'noopener');
  }

  async function toggleCopilotStamp(e: MouseEvent) {
    e.stopPropagation();
    await tasksStore.updateTask(task.id, { context: nextCopilotStampContext(task) });
  }

  async function markDone(e: MouseEvent) {
    e.stopPropagation();
    await tasksStore.setStatus(task.id, 'done');
  }
</script>

<div
  role="button"
  tabindex="0"
  draggable="true"
  ondragstart={(e) => {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('application/x-samwise-task', task.id);
    // Plain text fallback for browsers that ignore custom MIME types.
    e.dataTransfer.setData('text/plain', task.id);
  }}
  onclick={() => onOpen(task)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onOpen(task); }}
  class="group w-full text-left rounded-xl border border-white/10 bg-white/5 p-3 backdrop-blur hover:bg-white/10 hover:scale-[1.01] hover:-translate-y-0.5 active:scale-[0.99] transition-all shadow-sm cursor-grab active:cursor-grabbing"
>
  {#if uiStamp}
    <div class="mb-2 rounded-xl border border-sky-300/70 bg-gradient-to-r from-sky-400/25 via-cyan-300/15 to-emerald-300/20 px-2.5 py-2 shadow-lg shadow-sky-950/30">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[11px] font-black uppercase tracking-[0.16em] text-sky-50">
          Waiting on Copilot Review
        </span>
        <span class="rounded-full border border-sky-200/40 bg-black/25 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wide text-sky-100">
          Stamped
        </span>
      </div>
    </div>
  {/if}

  <div class="flex items-start justify-between gap-2">
    <div class="text-sm font-medium text-slate-100 line-clamp-2">{task.title}</div>
    <span class="shrink-0 text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 {PRIORITY_COLOR[task.priority]}">
      {task.priority}
    </span>
  </div>

  {#if reviewPanel && isReviewActionStatus(task.status)}
    <div
      class="mt-2 rounded-xl border p-2 shadow-inner"
      style="border-color: {verdictColor(reviewPanel.verdict)}66; background: linear-gradient(135deg, {verdictColor(reviewPanel.verdict)}22, rgba(14, 165, 233, 0.08));"
    >
      <div class="flex items-center gap-2">
        <span class="text-[10px] font-black uppercase tracking-wide" style="color: {verdictColor(reviewPanel.verdict)};">
          {reviewPanel.label}
        </span>
        <span class="flex-1"></span>
        {#if task.pr_url}
          <button
            type="button"
            onclick={openPr}
            class="rounded-md border border-white/15 bg-black/20 px-2 py-0.5 text-[10px] font-bold text-slate-100 hover:bg-black/30"
          >
            PR
          </button>
        {/if}
      </div>
      <p class="mt-1 line-clamp-3 text-xs leading-snug text-slate-200">{reviewPanel.why}</p>
      <p class="mt-1 line-clamp-2 text-[10px] font-bold leading-snug {reviewPanel.hasDeploymentCallout ? 'text-amber-200' : 'text-slate-400'}">
        Deploy: {reviewPanel.deployment}
      </p>
    </div>
  {/if}

  {#if task.project}
    <div class="mt-2 text-xs text-slate-400">📦 {task.project}</div>
  {/if}

  {#if task.subtasks && task.subtasks.length > 0}
    {@const done = task.subtasks.filter((s) => s.done).length}
    <div class="mt-2 flex items-center gap-1.5">
      <div class="h-1 flex-1 rounded-full bg-white/10 overflow-hidden">
        <div class="h-full bg-emerald-400/70" style="width:{(done / task.subtasks.length) * 100}%"></div>
      </div>
      <span class="text-[10px] text-slate-400">{done}/{task.subtasks.length}</span>
    </div>
  {/if}

  {#if showReviewActions && task.status !== 'done'}
    <div class="mt-2 grid grid-cols-2 gap-1.5">
      <button
        type="button"
        onclick={toggleCopilotStamp}
        class="rounded-lg border px-2 py-1.5 text-[10px] font-black text-sky-100 transition {uiStamp ? 'border-sky-300/50 bg-sky-400/20' : 'border-sky-300/25 bg-sky-400/10 hover:bg-sky-400/15'}"
      >
        {uiStamp ? 'Unstamp' : 'Copilot Review'}
      </button>
      <button
        type="button"
        onclick={markDone}
        class="rounded-lg border border-emerald-300/30 bg-emerald-400/10 px-2 py-1.5 text-[10px] font-black text-emerald-100 transition hover:bg-emerald-400/15"
      >
        Mark Done
      </button>
    </div>
  {/if}

  <div class="mt-2 flex flex-wrap items-center gap-1.5 text-[10px] text-slate-400">
    {#if task.pr_number}
      <span class="rounded bg-violet-500/15 border border-violet-500/30 text-violet-200 px-1.5 py-0.5">PR #{task.pr_number}</span>
    {/if}
    {#if task.branch}
      <span class="rounded bg-white/5 border border-white/10 px-1.5 py-0.5 truncate max-w-[140px]">⎇ {task.branch}</span>
    {/if}
    <span class="ml-auto">{relTime(task.updated_at)}</span>
  </div>
</div>
