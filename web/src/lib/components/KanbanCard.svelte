<script lang="ts">
  import type { AeTask } from '$lib/types';
  import { PRIORITY_COLOR, ORIGIN_LABEL, ORIGIN_BADGE_CLASS } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import {
    extractReviewActionPanel,
    getUiStamp,
    getMergeDeployState,
    getMergeConflictFixState,
    isMergeConflictError,
    isMergeConflictFixBusy,
    isReviewActionStatus,
    isMergeDeployBusy,
    mergeConflictFixButtonLabel,
    mergeDeployButtonLabel,
    nextManualInProgressStampContext,
    requestMergeConflictFixContext,
    requestMergeDeployContext
  } from '$lib/utils/review-actions';
  let { task, onOpen }: { task: AeTask; onOpen: (t: AeTask) => void } = $props();

  let comments = $derived(tasksStore.comments[task.id] ?? []);
  let reviewPanel = $derived(extractReviewActionPanel(task, comments));
  let uiStamp = $derived(getUiStamp(task));
  let mergeDeployState = $derived(getMergeDeployState(task));
  let mergeConflictFixState = $derived(getMergeConflictFixState(task));
  let mergeDeployRequestError = $state<string | null>(null);
  let mergeConflictFixRequestError = $state<string | null>(null);
  let showReviewActions = $derived(isReviewActionStatus(task.status) && !!(reviewPanel || task.pr_url));
  let canMergeDeploy = $derived(!!task.pr_url && (task.status === 'approved' || mergeDeployState.status === 'failed'));
  let canRequestMergeConflictFix = $derived(
    !!task.pr_url &&
    mergeDeployState.status === 'failed' &&
    isMergeConflictError(mergeDeployState.error) &&
    !isMergeConflictFixBusy(mergeConflictFixState)
  );
  let originKey = $derived(
    task.origin_system && task.origin_system !== 'manual'
      ? (task.origin_system as 'operly_triage' | 'banana_triage' | 'sentry')
      : null
  );

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

  async function toggleManualInProgressStamp(e: MouseEvent) {
    e.stopPropagation();
    await tasksStore.updateTask(task.id, { context: nextManualInProgressStampContext(task) });
  }

  async function requestMergeDeploy(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!canMergeDeploy || isMergeDeployBusy(mergeDeployState)) return;
    mergeDeployRequestError = null;
    const ok = await tasksStore.updateTask(task.id, { context: requestMergeDeployContext(task) });
    if (!ok) {
      mergeDeployRequestError = tasksStore.error || 'Could not queue Merge + Deploy.';
    }
  }

  async function requestMergeConflictFix(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!canRequestMergeConflictFix) return;
    mergeConflictFixRequestError = null;
    const ok = await tasksStore.updateTask(task.id, { context: requestMergeConflictFixContext(task) });
    if (!ok) {
      mergeConflictFixRequestError = tasksStore.error || 'Could not queue Sam conflict recovery.';
    }
  }

  async function markDone(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    await tasksStore.setStatus(task.id, 'done');
  }

  async function toggleHold(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    await tasksStore.updateTask(task.id, { on_hold: !task.on_hold });
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
    <div class="mb-2 rounded-xl border border-orange-300/70 bg-gradient-to-r from-orange-400/30 via-amber-300/15 to-rose-300/20 px-2.5 py-2 shadow-lg shadow-orange-950/30">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[11px] font-black uppercase tracking-[0.16em] text-orange-50">
          Manual In Progress
        </span>
        <span class="rounded-full border border-orange-200/40 bg-black/25 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wide text-orange-100">
          Mine
        </span>
      </div>
    </div>
  {/if}

  {#if task.on_hold}
    <div class="mb-2 rounded-xl border border-slate-300/60 bg-gradient-to-r from-slate-400/30 via-slate-300/15 to-slate-500/20 px-2.5 py-2 shadow-lg shadow-slate-950/30">
      <div class="flex items-center justify-between gap-2">
        <span class="inline-flex items-center gap-1.5 text-[11px] font-black uppercase tracking-[0.16em] text-slate-50">
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M11.5 1.75C11.5 .784 12.284 0 13.25 0a1.75 1.75 0 011.75 1.75v12.5A1.75 1.75 0 0113.25 16a1.75 1.75 0 01-1.75-1.75V1.75zm-7 0C4.5.784 5.284 0 6.25 0A1.75 1.75 0 018 1.75v12.5A1.75 1.75 0 016.25 16 1.75 1.75 0 014.5 14.25V1.75z"/>
          </svg>
          On Hold
        </span>
        <span class="rounded-full border border-slate-200/40 bg-black/25 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wide text-slate-100">
          Sam Skips
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

  {#if task.project || originKey}
    <div class="mt-2 flex flex-wrap items-center gap-1.5 text-xs text-slate-400">
      {#if task.project}
        <span>📦 {task.project}</span>
      {/if}
      {#if originKey}
        {#if task.origin_url}
          <a
            href={task.origin_url}
            target="_blank"
            rel="noopener"
            onclick={(e) => e.stopPropagation()}
            class="rounded-md border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide hover:opacity-90 {ORIGIN_BADGE_CLASS[originKey]}"
            title="Open source ticket in {ORIGIN_LABEL[originKey]}"
          >
            {ORIGIN_LABEL[originKey]}
          </a>
        {:else}
          <span
            class="rounded-md border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide {ORIGIN_BADGE_CLASS[originKey]}"
            title="From {ORIGIN_LABEL[originKey]}"
          >
            {ORIGIN_LABEL[originKey]}
          </span>
        {/if}
      {/if}
    </div>
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

  {#if task.commit_message && task.commit_message.trim()}
    <pre
      onmousedown={(e) => e.stopPropagation()}
      onclick={(e) => e.stopPropagation()}
      class="mt-2 max-h-52 overflow-y-auto whitespace-pre-wrap break-words rounded-md border-l-2 border-indigo-400/45 bg-indigo-500/5 px-2.5 py-2 font-mono text-[10px] leading-snug text-slate-300"
    >{task.commit_message}</pre>
  {/if}

  {#if task.status === 'queued'}
    <div class="mt-2 flex justify-end">
      <button
        type="button"
        onclick={toggleHold}
        class="rounded-lg border px-2 py-1 text-[10px] font-black uppercase tracking-wide transition {task.on_hold ? 'border-emerald-300/40 bg-emerald-400/10 text-emerald-100 hover:bg-emerald-400/15' : 'border-slate-300/30 bg-slate-400/10 text-slate-100 hover:bg-slate-400/15'}"
      >
        {task.on_hold ? 'Release' : 'Hold'}
      </button>
    </div>
  {/if}

  {#if showReviewActions && task.status !== 'done'}
    <div class="mt-2 grid grid-cols-2 gap-1.5">
      <button
        type="button"
        onclick={toggleManualInProgressStamp}
        class="rounded-lg border px-2 py-1.5 text-[10px] font-black text-orange-100 transition {uiStamp ? 'border-orange-300/50 bg-orange-400/20' : 'border-orange-300/25 bg-orange-400/10 hover:bg-orange-400/15'}"
      >
        {uiStamp ? 'Clear Stamp' : 'In Progress'}
      </button>
      <button
        type="button"
        onclick={canMergeDeploy ? requestMergeDeploy : markDone}
        disabled={isMergeDeployBusy(mergeDeployState)}
        class="rounded-lg border px-2 py-1.5 text-[10px] font-black transition {canMergeDeploy ? 'border-cyan-300/35 bg-cyan-400/10 text-cyan-100 hover:bg-cyan-400/15' : 'border-emerald-300/30 bg-emerald-400/10 text-emerald-100 hover:bg-emerald-400/15'} {isMergeDeployBusy(mergeDeployState) ? 'opacity-70 cursor-wait' : ''}"
      >
        {canMergeDeploy ? mergeDeployButtonLabel(mergeDeployState) : 'Mark Done'}
      </button>
    </div>
    {#if mergeDeployRequestError || mergeDeployState.error}
      <div class="mt-2 line-clamp-3 rounded-lg border border-rose-400/35 bg-rose-500/10 px-2 py-1.5 text-[10px] font-bold leading-snug text-rose-100">
        Merge + Deploy failed: {mergeDeployRequestError || mergeDeployState.error}
      </div>
    {/if}
    {#if canRequestMergeConflictFix || isMergeConflictFixBusy(mergeConflictFixState) || mergeConflictFixRequestError || mergeConflictFixState.error}
      <button
        type="button"
        onclick={requestMergeConflictFix}
        disabled={isMergeConflictFixBusy(mergeConflictFixState)}
        class="mt-2 w-full rounded-lg border border-amber-300/40 bg-gradient-to-r from-orange-400/20 to-teal-400/10 px-2 py-1.5 text-[10px] font-black text-orange-100 transition hover:bg-orange-400/20 {isMergeConflictFixBusy(mergeConflictFixState) ? 'opacity-70 cursor-wait' : ''}"
      >
        {mergeConflictFixButtonLabel(mergeConflictFixState)}
      </button>
    {/if}
    {#if mergeConflictFixRequestError || mergeConflictFixState.error}
      <div class="mt-2 line-clamp-3 rounded-lg border border-amber-400/35 bg-orange-500/10 px-2 py-1.5 text-[10px] font-bold leading-snug text-orange-100">
        Sam conflict recovery failed: {mergeConflictFixRequestError || mergeConflictFixState.error}
      </div>
    {/if}
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
