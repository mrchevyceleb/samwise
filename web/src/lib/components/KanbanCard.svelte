<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { AeTask } from '$lib/types';
  import { PRIORITY_COLOR, ORIGIN_LABEL, ORIGIN_BADGE_CLASS, getOriginKey } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import {
    extractReviewActionPanel,
    getUiStamp,
    getMergeDeployState,
    getMergeConflictFixState,
    getReviewMergeState,
    isMergeConflictError,
    isMergeConflictFixBusy,
    isReviewActionStatus,
    isMergeDeployBusy,
    isReviewMergeBusy,
    mergeConflictFixButtonLabel,
    reviewMergeButtonLabel,
    nextManualInProgressStampContext,
    requestMergeConflictFixContext,
    requestReviewMergeContext
  } from '$lib/utils/review-actions';
  let { task, onOpen }: { task: AeTask; onOpen: (t: AeTask) => void } = $props();

  let comments = $derived(tasksStore.comments[task.id] ?? []);
  let reviewPanel = $derived(extractReviewActionPanel(task, comments));
  let uiStamp = $derived(getUiStamp(task));
  let mergeDeployState = $derived(getMergeDeployState(task));
  let mergeConflictFixState = $derived(getMergeConflictFixState(task));
  let mergeDeployRequestError = $state<string | null>(null);
  let mergeConflictFixRequestError = $state<string | null>(null);
  let reviewMergeRequestError = $state<string | null>(null);
  let reviewMergeState = $derived(getReviewMergeState(task));
  let showTesterPicker = $state(false);
  let testers = $state<{ name: string; role: string }[]>([]);
  let selectedTester = $state('');
  let sendingToQa = $state(false);
  let sendToQaError = $state<string | null>(null);
  let showReviewActions = $derived(isReviewActionStatus(task.status) && !!(reviewPanel || task.pr_url));
  let canMergeDeploy = $derived(!!task.pr_url && (task.status === 'approved' || mergeDeployState.status === 'failed' || reviewMergeState.status === 'failed'));
  let canRequestMergeConflictFix = $derived(
    !!task.pr_url &&
    mergeDeployState.status === 'failed' &&
    isMergeConflictError(mergeDeployState.error) &&
    !isMergeConflictFixBusy(mergeConflictFixState)
  );
  let originKey = $derived(getOriginKey(task.origin_system));

  // Status-derived flags
  let isWorking = $derived(task.status === 'in_progress' || task.status === 'testing');
  let isAgent = $derived(task.assignee === 'agent');
  let hasScreenshots = $derived(
    (task.screenshots_before && task.screenshots_before.length > 0) ||
    (task.screenshots_after && task.screenshots_after.length > 0)
  );
  let commentCount = $derived(comments.length);

  // Latest agent/system comment for activity preview on working/review cards
  let latestComment = $derived((() => {
    if (comments.length === 0) return null;
    if (!isWorking && task.status !== 'review') return null;
    const agentComments = comments.filter((c) => c.author === 'agent' || c.author === 'system');
    const src = agentComments.length > 0 ? agentComments[agentComments.length - 1] : comments[comments.length - 1];
    return src.content;
  })());

  // Visual QA
  let qaResult = $derived(task.visual_qa_result);
  let qaVerdict = $derived(qaResult ? (qaResult.verdict || (qaResult.pass ? 'PASS' : 'FAIL')).toUpperCase() : '');
  let qaIsPass = $derived(qaVerdict === 'PASS');
  let qaIsSkip = $derived(qaVerdict === 'SKIP');
  let qaBadgeLabel = $derived(qaIsSkip ? 'QA Skipped' : qaIsPass ? 'QA Passed' : 'QA Failed');

  // Live working timer
  let nowTick = $state(Date.now());
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let workingElapsed = $derived((() => {
    if (!isWorking || !task.claimed_at) return '';
    const start = new Date(task.claimed_at).getTime();
    const diff = Math.max(0, nowTick - start);
    const secs = Math.floor(diff / 1000);
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    const remSecs = secs % 60;
    if (mins < 60) return `${mins}m ${remSecs}s`;
    const hrs = Math.floor(mins / 60);
    const remMins = mins % 60;
    return `${hrs}h ${remMins}m`;
  })());

  onMount(() => {
    if (isWorking) timerInterval = setInterval(() => { nowTick = Date.now(); }, 1000);
  });

  onDestroy(() => {
    if (timerInterval) clearInterval(timerInterval);
  });

  $effect(() => {
    if (isWorking && !timerInterval) {
      timerInterval = setInterval(() => { nowTick = Date.now(); }, 1000);
    } else if (!isWorking && timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  });

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

  async function requestReviewMerge(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!canMergeDeploy || isReviewMergeBusy(reviewMergeState, mergeDeployState)) return;
    reviewMergeRequestError = null;
    const ok = await tasksStore.updateTask(task.id, { context: requestReviewMergeContext(task) });
    if (!ok) {
      reviewMergeRequestError = tasksStore.error || 'Could not queue Review & Merge.';
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

  async function handleRequeue(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    await tasksStore.requeueTask(task.id);
  }

  async function openSendToQa(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    sendToQaError = null;
    showTesterPicker = true;
    if (testers.length === 0) {
      try {
        const res = await fetch('/api/qa-testers');
        if (res.ok) {
          const data = await res.json();
          testers = data.testers || [];
          if (testers.length > 0 && !selectedTester) selectedTester = testers[0].name;
        } else {
          sendToQaError = `Could not load testers (${res.status})`;
        }
      } catch (err) {
        sendToQaError = err instanceof Error ? err.message : String(err);
      }
    }
  }

  function cancelSendToQa(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    showTesterPicker = false;
    sendToQaError = null;
  }

  async function confirmSendToQa(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!selectedTester) { sendToQaError = 'Pick a tester first'; return; }
    sendingToQa = true;
    sendToQaError = null;
    try {
      const res = await fetch('/api/send-to-qa', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ task_id: task.id, tester: selectedTester })
      });
      if (!res.ok) {
        const body = await res.text();
        throw new Error(body.slice(0, 200) || `HTTP ${res.status}`);
      }
      showTesterPicker = false;
    } catch (err) {
      sendToQaError = err instanceof Error ? err.message : String(err);
    } finally {
      sendingToQa = false;
    }
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
    e.dataTransfer.setData('text/plain', task.id);
  }}
  onclick={() => onOpen(task)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onOpen(task); }}
  class="group w-full text-left rounded-xl border p-3 backdrop-blur transition-all shadow-sm cursor-grab active:cursor-grabbing hover:scale-[1.01] hover:-translate-y-0.5 active:scale-[0.99] hover:bg-white/10 {isWorking ? 'border-indigo-500/50 bg-indigo-500/5' : 'border-white/10 bg-white/5'}"
>
  {#if uiStamp}
    <div class="mb-2 rounded-xl border border-orange-300/70 bg-gradient-to-r from-orange-400/30 via-amber-300/15 to-rose-300/20 px-2.5 py-2 shadow-lg shadow-orange-950/30">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[11px] font-black uppercase tracking-[0.16em] text-orange-50">Manual In Progress</span>
        <span class="rounded-full border border-orange-200/40 bg-black/25 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wide text-orange-100">Mine</span>
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
        <span class="rounded-full border border-slate-200/40 bg-black/25 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wide text-slate-100">Sam Skips</span>
      </div>
    </div>
  {/if}

  <!-- Title + priority -->
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
        <span class="text-[10px] font-semibold px-1.5 py-0.5 rounded-md bg-indigo-500/10 text-indigo-300">📦 {task.project}</span>
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

  <!-- Latest agent comment preview -->
  {#if latestComment}
    <div class="mt-2 flex items-start gap-1.5 rounded-md border-l-2 border-indigo-400/35 bg-indigo-500/5 px-2 py-1.5 text-[11px] leading-snug text-slate-400">
      {#if isWorking}
        <span class="mt-px h-1.5 w-1.5 shrink-0 rounded-full bg-indigo-400 animate-pulse"></span>
      {/if}
      <span class="line-clamp-2">{latestComment}</span>
    </div>
  {/if}

  <!-- Visual QA badge -->
  {#if qaResult}
    <div class="mt-2">
      <span class="inline-flex items-center gap-1 text-[10px] font-bold px-2 py-0.5 rounded-md border
        {qaIsPass ? 'border-emerald-400/40 bg-emerald-500/10 text-emerald-300' : qaIsSkip ? 'border-amber-400/40 bg-amber-500/10 text-amber-300' : 'border-rose-400/40 bg-rose-500/10 text-rose-300'}">
        {qaBadgeLabel}
      </span>
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
        onclick={canMergeDeploy ? requestReviewMerge : markDone}
        disabled={isReviewMergeBusy(reviewMergeState, mergeDeployState)}
        class="rounded-lg border px-2 py-1.5 text-[10px] font-black transition {canMergeDeploy ? 'border-cyan-300/35 bg-cyan-400/10 text-cyan-100 hover:bg-cyan-400/15' : 'border-emerald-300/30 bg-emerald-400/10 text-emerald-100 hover:bg-emerald-400/15'} {isReviewMergeBusy(reviewMergeState, mergeDeployState) ? 'opacity-70 cursor-wait' : ''}"
      >
        {canMergeDeploy ? reviewMergeButtonLabel(reviewMergeState, mergeDeployState) : 'Mark Done'}
      </button>
    </div>
    {#if reviewMergeRequestError || reviewMergeState.error}
      <div class="mt-2 line-clamp-3 rounded-lg border border-rose-400/35 bg-rose-500/10 px-2 py-1.5 text-[10px] font-bold leading-snug text-rose-100">
        Review &amp; Merge: {reviewMergeRequestError || reviewMergeState.error}
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

  {#if task.status === 'failed' || task.status === 'fixes_needed' || task.status === 'pending_confirmation' || task.status === 'review' || task.status === 'approved'}
    <button
      type="button"
      onclick={handleRequeue}
      class="mt-2 w-full rounded-lg border border-indigo-300/35 bg-indigo-400/10 px-2 py-1.5 text-[10px] font-black uppercase tracking-wide text-indigo-100 transition hover:bg-indigo-400/15"
    >
      Re-queue
    </button>
  {/if}

  {#if task.status === 'approved'}
    {#if !showTesterPicker}
      <button
        type="button"
        onclick={openSendToQa}
        class="mt-2 w-full rounded-lg border border-teal-300/45 bg-gradient-to-r from-teal-500/20 to-cyan-500/15 px-2 py-1.5 text-[10px] font-black uppercase tracking-wide text-teal-50 transition hover:from-teal-500/30 hover:to-cyan-500/25"
      >
        Send to QA
      </button>
    {:else}
      <div
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
        role="presentation"
        class="mt-2 rounded-xl border border-teal-300/40 bg-teal-500/5 p-2.5 shadow-inner"
      >
        <div class="text-[10px] font-black uppercase tracking-wide text-teal-100 mb-1.5">Pick a tester</div>
        {#if testers.length === 0 && !sendToQaError}
          <div class="text-[10px] text-slate-300">Loading testers...</div>
        {:else}
          <select
            bind:value={selectedTester}
            class="w-full rounded-md border border-white/15 bg-slate-900/70 px-2 py-1 text-[11px] text-slate-100 focus:outline-none focus:border-teal-300/60"
          >
            {#each testers as t (t.name)}
              <option value={t.name}>{t.name} {t.role === 'admin' ? '(admin)' : ''}</option>
            {/each}
          </select>
        {/if}
        {#if sendToQaError}
          <div class="mt-1.5 line-clamp-3 rounded-md border border-rose-400/40 bg-rose-500/10 px-2 py-1 text-[10px] font-bold text-rose-100">{sendToQaError}</div>
        {/if}
        <div class="mt-2 grid grid-cols-2 gap-1.5">
          <button
            type="button"
            onclick={cancelSendToQa}
            disabled={sendingToQa}
            class="rounded-md border border-white/15 bg-white/5 px-2 py-1 text-[10px] font-black uppercase tracking-wide text-slate-200 hover:bg-white/10 disabled:opacity-50"
          >Cancel</button>
          <button
            type="button"
            onclick={confirmSendToQa}
            disabled={sendingToQa || !selectedTester}
            class="rounded-md border border-teal-300/45 bg-teal-500/25 px-2 py-1 text-[10px] font-black uppercase tracking-wide text-teal-50 hover:bg-teal-500/35 disabled:opacity-60 disabled:cursor-wait"
          >{sendingToQa ? 'Sending...' : 'Send'}</button>
        </div>
      </div>
    {/if}
  {/if}

  {#if task.status === 'qa'}
    {@const qaCtx = (task.context && (task.context as Record<string, unknown>).qa) as { tester?: string; ticket_id?: string } | undefined}
    {#if qaCtx?.tester}
      <div class="mt-2 rounded-lg border border-teal-300/35 bg-teal-500/10 px-2 py-1.5 text-[10px] font-bold text-teal-100">
        In QA with {qaCtx.tester}{#if qaCtx.ticket_id}
          <a
            href={`https://qa.stonelabs.app/dashboard.html?ticket=${qaCtx.ticket_id}`}
            target="_blank"
            rel="noopener"
            onclick={(e) => e.stopPropagation()}
            class="ml-1 underline decoration-dotted hover:text-teal-50"
          >view ticket</a>
        {/if}
      </div>
    {/if}
  {/if}

  <!-- Bottom row: assignee, pr/branch, icons, timer/timestamp -->
  <div class="mt-2 flex items-center gap-1.5 text-[10px] text-slate-400">
    <!-- Assignee indicator -->
    <span
      title="{isAgent ? 'Assigned to Agent' : 'Assigned to Matt'}"
      class="flex items-center justify-center w-5 h-5 shrink-0 rounded-full {isAgent ? 'bg-indigo-500/15 text-indigo-400' : 'bg-emerald-500/15 text-emerald-400'}"
    >
      {#if isAgent}
        <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 0a1 1 0 011 1v1.07A6.002 6.002 0 0114 8v3a2 2 0 01-2 2H4a2 2 0 01-2-2V8a6.002 6.002 0 015-5.93V1a1 1 0 011-1zM6 9a1 1 0 100 2 1 1 0 000-2zm4 0a1 1 0 100 2 1 1 0 000-2z"/>
        </svg>
      {:else}
        <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/>
        </svg>
      {/if}
    </span>

    {#if task.pr_number}
      <span class="rounded bg-violet-500/15 border border-violet-500/30 text-violet-200 px-1.5 py-0.5">PR #{task.pr_number}</span>
    {/if}
    {#if task.branch}
      <span class="rounded bg-white/5 border border-white/10 px-1.5 py-0.5 truncate max-w-[100px]">⎇ {task.branch}</span>
    {:else if task.base_branch}
      <span title="Base branch" class="rounded bg-white/5 border border-white/10 px-1.5 py-0.5 truncate max-w-[100px]">base {task.base_branch}</span>
    {/if}

    <div class="ml-auto flex items-center gap-1.5 shrink-0">
      {#if commentCount > 0}
        <span class="flex items-center gap-0.5 text-slate-500">
          <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
            <path d="M2.678 11.894a1 1 0 01.287.801 10.97 10.97 0 01-.398 2c1.395-.323 2.247-.697 2.634-.893a1 1 0 01.71-.074A8.06 8.06 0 008 14c3.996 0 7-2.807 7-6s-3.004-6-7-6-7 2.808-7 6c0 1.468.617 2.83 1.678 3.894z"/>
          </svg>
          {commentCount}
        </span>
      {/if}

      {#if task.report_url}
        <button
          type="button"
          title="View report"
          onclick={(e) => { e.stopPropagation(); window.open(task.report_url!, '_blank', 'noopener'); }}
          class="flex items-center justify-center w-5 h-5 rounded bg-indigo-500/10 text-indigo-400 hover:bg-indigo-500/20 transition"
        >
          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
        </button>
      {/if}

      {#if hasScreenshots}
        <span title="Has screenshots" class="flex items-center justify-center w-5 h-5 rounded bg-purple-500/10 text-purple-400">
          <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4.502 9a1.5 1.5 0 100-3 1.5 1.5 0 000 3z"/>
            <path d="M14.002 13a2 2 0 01-2 2h-10a2 2 0 01-2-2V5A2 2 0 012 3h2.5l.83-1.36A1 1 0 016.18 1h3.64a1 1 0 01.86.49L11.5 3h2.5a2 2 0 012 2v8z"/>
          </svg>
        </span>
      {/if}

      {#if isWorking && workingElapsed}
        <span class="flex items-center gap-1 text-indigo-400 font-mono font-semibold">
          <span class="w-1.5 h-1.5 rounded-full bg-indigo-400 animate-pulse"></span>
          {workingElapsed}
        </span>
      {:else}
        <span>{relTime(task.updated_at)}</span>
      {/if}
    </div>
  </div>
</div>
