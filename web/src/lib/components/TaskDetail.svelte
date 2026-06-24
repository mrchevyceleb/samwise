<script lang="ts">
  import type { AeTask, TaskStatus, TaskPriority, Subtask } from '$lib/types';
  import { PRIORITY_COLOR, STATUS_LABEL, STATUSES } from '$lib/types';
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
  import LinkRow from './LinkRow.svelte';

  let { task, onClose }: { task: AeTask; onClose: () => void } = $props();

  $effect(() => {
    tasksStore.loadCommentsFor(task.id);
  });

  let comments = $derived(tasksStore.comments[task.id] ?? []);
  let before = $derived(task.screenshots_before ?? []);
  let after = $derived(task.screenshots_after ?? []);
  let attachments = $derived(task.attachments ?? []);
  let reviewPanel = $derived(extractReviewActionPanel(task, comments));
  let uiStamp = $derived(getUiStamp(task));
  let mergeDeployState = $derived(getMergeDeployState(task));
  let mergeConflictFixState = $derived(getMergeConflictFixState(task));
  let reviewMergeState = $derived(getReviewMergeState(task));
  let displayStatus = $derived(task.status === 'testing' ? 'in_progress' : task.status);
  let mergeDeployRequestError = $state<string | null>(null);
  let mergeConflictFixRequestError = $state<string | null>(null);
  let reviewMergeRequestError = $state<string | null>(null);
  let stopping = $state(false);
  let restarting = $state(false);
  let confirmDelete = $state(false);
  let deleting = $state(false);
  let isStoppable = $derived(task.status === 'in_progress' || task.status === 'testing');
  let visualQA = $derived(task.visual_qa_result);
  let visualQaVerdict = $derived((visualQA?.verdict || (visualQA?.pass ? 'PASS' : 'FAIL')).toUpperCase());
  let canMergeDeploy = $derived(!!task.pr_url && (task.status === 'approved' || mergeDeployState.status === 'failed' || reviewMergeState.status === 'failed'));
  let canRequestMergeConflictFix = $derived(
    !!task.pr_url &&
    mergeDeployState.status === 'failed' &&
    isMergeConflictError(mergeDeployState.error) &&
    !isMergeConflictFixBusy(mergeConflictFixState)
  );
  // Re-queue is for stuck non-terminal statuses. `in_progress`/`testing`
  // would need the live worker killed first, so leave those out; the
  // wedge sweep handles them server-side after 60/45 min.
  let isRequeueable = $derived(
    task.status === 'failed' ||
    task.status === 'fixes_needed' ||
    task.status === 'pending_confirmation' ||
    task.status === 'review' ||
    task.status === 'approved'
  );
  let requeueing = $state(false);
  let closingPr = $state(false);
  let closePrError = $state<string | null>(null);
  let copiedPr = $state(false);
  let canClosePr = $derived(!!task.pr_url && task.status !== 'done');

  // --- Inline editing: title + description ---
  let editingTitle = $state(false);
  let editTitle = $state('');
  let editingDesc = $state(false);
  let editDesc = $state('');

  // --- Comment composer ---
  let commentInput = $state('');
  let postingComment = $state(false);
  let commentsEl = $state<HTMLUListElement | null>(null);

  // --- Interactive subtasks ---
  let newSubtaskTitle = $state('');
  let editingSubtaskId = $state<string | null>(null);
  let editSubtaskText = $state('');
  let subtaskDragId = $state<string | null>(null);
  let dropTargetId = $state<string | null>(null);
  let dropPosition = $state<'above' | 'below'>('below');

  let subtasks = $derived<Subtask[]>((task.subtasks || []).slice().sort((a, b) => a.order - b.order));
  let subtasksDone = $derived(subtasks.filter((s) => s.done).length);

  const priorities: TaskPriority[] = ['critical', 'high', 'medium', 'low'];

  /** Focus + select-all action for inline edit inputs. */
  function autofocus(node: HTMLElement) {
    node.focus();
    if (node instanceof HTMLInputElement || node instanceof HTMLTextAreaElement) node.select();
  }

  /** Escape HTML then render markdown-ish markup (code/bold/links/mentions).
   *  HTML is escaped first for XSS safety, then code spans are stashed so the
   *  bold/URL/mention regexes never run inside them. Mirrors the desktop app. */
  function renderCommentHtml(content: string): string {
    let safe = content
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');

    // Stash code blocks + inline code so other regexes skip them.
    const codeSlots: string[] = [];
    const stash = (html: string): string => {
      codeSlots.push(html);
      return `\x00CODE${codeSlots.length - 1}\x00`;
    };

    // Code blocks (triple backticks) — keep internal newlines via pre-wrap.
    safe = safe.replace(/```([\s\S]*?)```/g, (_m, inner: string) =>
      stash(`<code class="block bg-black/40 rounded-md px-2.5 py-2 font-mono text-[11px] my-1 whitespace-pre-wrap overflow-x-auto break-words">${inner}</code>`)
    );
    // Inline code (single backticks)
    safe = safe.replace(/`([^`]+)`/g, (_m, inner: string) =>
      stash(`<code class="bg-black/30 rounded px-1 py-0.5 font-mono text-[11px]">${inner}</code>`)
    );
    // Bold (**text**)
    safe = safe.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    // Auto-link URLs
    safe = safe.replace(
      /(https?:\/\/[^\s<]+)/g,
      '<a href="$1" target="_blank" rel="noopener noreferrer" class="text-indigo-300 underline hover:text-indigo-200">$1</a>'
    );
    // @mentions
    safe = safe.replace(/(^|\s)@(\w+)/g, '$1<span class="text-indigo-300 font-semibold">@$2</span>');
    // Newlines to <br>
    safe = safe.replace(/\n/g, '<br>');
    // Restore code
    safe = safe.replace(/\x00CODE(\d+)\x00/g, (_m, idx: string) => codeSlots[parseInt(idx, 10)]);
    return safe;
  }

  function verdictColor(verdict: string | undefined) {
    if (verdict === 'merge') return '#34d399';
    if (verdict === 'fix' || verdict === 'blocked') return '#fb923c';
    if (verdict === 'errored') return '#fb7185';
    return '#60a5fa';
  }

  function visualQaColor(verdict: string) {
    if (verdict === 'PASS') return '#34d399';
    if (verdict === 'SKIP') return '#fbbf24';
    return '#fb7185';
  }

  function openPr() {
    if (task.pr_url) window.open(task.pr_url, '_blank', 'noopener');
  }

  // --- Inline title / description editing ---
  function startEditTitle() {
    editingTitle = true;
    editTitle = task.title;
  }
  async function saveTitle() {
    const trimmed = editTitle.trim();
    if (trimmed && trimmed !== task.title) {
      await tasksStore.updateTask(task.id, { title: trimmed });
    }
    editingTitle = false;
  }
  function startEditDesc() {
    editingDesc = true;
    editDesc = task.description || '';
  }
  async function saveDescription() {
    if (editDesc !== (task.description || '')) {
      await tasksStore.updateTask(task.id, { description: editDesc || null });
    }
    editingDesc = false;
  }

  function changePriority(p: TaskPriority) {
    void tasksStore.updateTask(task.id, { priority: p });
  }

  // --- Comments ---
  async function handlePostComment() {
    const content = commentInput.trim();
    if (!content || postingComment) return;
    postingComment = true;
    try {
      const ok = await tasksStore.postComment(task.id, 'matt', content);
      if (ok) {
        commentInput = '';
        // The realtime channel re-inserts the row; scroll to bottom after it lands.
        requestAnimationFrame(() => {
          if (commentsEl) commentsEl.scrollTop = commentsEl.scrollHeight;
        });
      }
    } finally {
      postingComment = false;
    }
  }
  function handleCommentKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void handlePostComment();
    }
  }

  // --- Subtasks ---
  async function persistSubtasks(updated: Subtask[]) {
    await tasksStore.updateTask(task.id, { subtasks: updated });
  }
  function toggleSubtask(id: string) {
    void persistSubtasks(subtasks.map((s) => (s.id === id ? { ...s, done: !s.done } : s)));
  }
  function deleteSubtask(id: string) {
    void persistSubtasks(subtasks.filter((s) => s.id !== id));
  }
  function addSubtask() {
    const title = newSubtaskTitle.trim();
    if (!title) return;
    void persistSubtasks([...subtasks, { id: crypto.randomUUID(), title, done: false, order: subtasks.length }]);
    newSubtaskTitle = '';
  }
  function handleAddSubtaskKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      addSubtask();
    }
  }
  function startEditSubtask(s: Subtask) {
    editingSubtaskId = s.id;
    editSubtaskText = s.title;
  }
  function saveEditSubtask() {
    if (editingSubtaskId && editSubtaskText.trim()) {
      void persistSubtasks(subtasks.map((s) => (s.id === editingSubtaskId ? { ...s, title: editSubtaskText.trim() } : s)));
    }
    editingSubtaskId = null;
    editSubtaskText = '';
  }
  function handleEditSubtaskKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); saveEditSubtask(); }
    if (e.key === 'Escape') { editingSubtaskId = null; editSubtaskText = ''; }
  }
  // Drag-to-reorder via data-subtask-id + mousemove, matching the desktop app.
  function onSubtaskDragStart(e: MouseEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();
    subtaskDragId = id;
    const onMove = (ev: MouseEvent) => {
      const els = document.querySelectorAll('[data-subtask-id]');
      let closestId: string | null = null;
      let closestPos: 'above' | 'below' = 'below';
      let minDist = Infinity;
      for (const el of els) {
        const rect = (el as HTMLElement).getBoundingClientRect();
        const mid = rect.top + rect.height / 2;
        const dist = Math.abs(ev.clientY - mid);
        if (dist < minDist) {
          minDist = dist;
          closestId = (el as HTMLElement).dataset.subtaskId!;
          closestPos = ev.clientY < mid ? 'above' : 'below';
        }
      }
      if (closestId && closestId !== subtaskDragId) {
        dropTargetId = closestId;
        dropPosition = closestPos;
      } else {
        dropTargetId = null;
      }
    };
    const onUp = () => {
      if (subtaskDragId && dropTargetId) {
        reorderSubtasks(subtaskDragId, dropTargetId, dropPosition);
      }
      subtaskDragId = null;
      dropTargetId = null;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }
  function reorderSubtasks(fromId: string, toId: string, pos: 'above' | 'below') {
    const arr = [...subtasks];
    const fromIdx = arr.findIndex((s) => s.id === fromId);
    if (fromIdx < 0) return;
    const item = arr.splice(fromIdx, 1)[0];
    let toIdx = arr.findIndex((s) => s.id === toId);
    if (pos === 'below') toIdx += 1;
    arr.splice(toIdx, 0, item);
    void persistSubtasks(arr.map((s, i) => ({ ...s, order: i })));
  }

  // --- Review panel / merge actions ---
  async function toggleManualInProgressStamp() {
    await tasksStore.updateTask(task.id, { context: nextManualInProgressStampContext(task) });
  }
  async function requestReviewMerge() {
    if (!canMergeDeploy || isReviewMergeBusy(reviewMergeState, mergeDeployState)) return;
    reviewMergeRequestError = null;
    const ok = await tasksStore.updateTask(task.id, { context: requestReviewMergeContext(task) });
    if (!ok) {
      reviewMergeRequestError = tasksStore.error || 'Could not queue Review & Merge.';
    }
  }
  async function handleStop() {
    if (stopping) return;
    stopping = true;
    try {
      await tasksStore.stopTask(task.id);
    } finally {
      stopping = false;
    }
  }
  async function handleRestart() {
    if (restarting) return;
    restarting = true;
    try {
      await tasksStore.restartTask(task.id);
    } finally {
      restarting = false;
    }
  }
  async function handleDelete() {
    if (!confirmDelete) {
      confirmDelete = true;
      setTimeout(() => { confirmDelete = false; }, 3000);
      return;
    }
    deleting = true;
    try {
      await tasksStore.deleteTask(task.id);
      onClose();
    } finally {
      deleting = false;
    }
  }
  async function requestMergeConflictFix() {
    if (!canRequestMergeConflictFix) return;
    mergeConflictFixRequestError = null;
    const ok = await tasksStore.updateTask(task.id, { context: requestMergeConflictFixContext(task) });
    if (!ok) {
      mergeConflictFixRequestError = tasksStore.error || 'Could not queue Sam conflict recovery.';
    }
  }
  async function markDone() {
    await tasksStore.setStatus(task.id, 'done');
    onClose();
  }
  async function closePrAndDone() {
    if (!canClosePr || closingPr) return;
    closingPr = true;
    closePrError = null;
    const result = await tasksStore.closePr(task.id);
    if (!result.ok) {
      closePrError = result.error || 'Could not close the PR.';
      closingPr = false;
      return;
    }
    // PR closed (or already closed) — now mark the task done. setStatus also
    // fires the origin-ticket closeout for Operly/Banana/etc. cards.
    await tasksStore.setStatus(task.id, 'done');
    onClose();
  }
  async function copyPrLink() {
    if (!task.pr_url) return;
    try {
      await navigator.clipboard.writeText(task.pr_url);
      copiedPr = true;
      setTimeout(() => { copiedPr = false; }, 2000);
    } catch (e) {
      console.warn('[task-detail] copy PR link failed:', e);
    }
  }
  async function handleRequeue() {
    if (requeueing) return;
    requeueing = true;
    try {
      await tasksStore.requeueTask(task.id);
    } finally {
      requeueing = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-40 flex items-end sm:items-center justify-center bg-black/60 backdrop-blur-sm"
  data-no-egg
  onclick={onClose}
  onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div
    class="w-full sm:max-w-2xl max-h-[92dvh] overflow-y-auto rounded-t-3xl sm:rounded-3xl bg-slate-900 border border-white/10 shadow-2xl"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="document"
  >
    <header class="sticky top-0 z-10 px-4 py-3 bg-slate-900/90 backdrop-blur border-b border-white/10 flex items-start gap-3">
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <select
            value={displayStatus}
            onchange={(e) => {
              const next = (e.currentTarget as HTMLSelectElement).value as TaskStatus;
              if (next !== task.status) tasksStore.setStatus(task.id, next);
            }}
            class="text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 border-white/20 bg-slate-900 text-slate-200 focus:outline-none focus:border-white/40"
            aria-label="Move task"
          >
            {#each STATUSES as s}
              <option value={s}>{STATUS_LABEL[s]}</option>
            {/each}
          </select>
          <select
            value={task.priority}
            onchange={(e) => changePriority((e.currentTarget as HTMLSelectElement).value as TaskPriority)}
            class="text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 cursor-pointer focus:outline-none focus:border-white/40 {PRIORITY_COLOR[task.priority]}"
            aria-label="Set priority"
            title="Change priority"
          >
            {#each priorities as p}
              <option value={p}>{p}</option>
            {/each}
          </select>
          {#if task.project}
            <span class="text-xs text-slate-400">📦 {task.project}</span>
          {/if}
        </div>
        {#if editingTitle}
          <input
            type="text"
            bind:value={editTitle}
            use:autofocus
            class="mt-1 w-full rounded-lg border border-indigo-400/40 bg-slate-950 px-3 py-1.5 text-lg font-semibold text-slate-100 focus:outline-none focus:border-indigo-400/70"
            onblur={saveTitle}
            onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveTitle(); } if (e.key === 'Escape') editingTitle = false; }}
          />
        {:else}
          <h2 class="mt-1 text-lg font-semibold text-slate-100">
            <button
              type="button"
              class="block w-full text-left cursor-text rounded-md -mx-1 px-1 py-0 hover:bg-white/5 transition-colors"
              title="Click to edit"
              onclick={startEditTitle}
            >{task.title}</button>
          </h2>
        {/if}
      </div>
      <button
        type="button"
        onclick={onClose}
        class="shrink-0 rounded-full h-8 w-8 grid place-items-center bg-white/10 hover:bg-white/20 text-slate-200"
        aria-label="close"
      >✕</button>
    </header>

    <div class="px-4 py-4 space-y-4">
      {#if task.commit_message && task.commit_message.trim()}
        <pre class="max-h-[420px] overflow-y-auto whitespace-pre-wrap break-words rounded-xl border-l-4 border-indigo-400/55 bg-indigo-500/5 px-3 py-3 font-mono text-xs leading-relaxed text-slate-200">{task.commit_message}</pre>
      {/if}

      {#if reviewPanel && isReviewActionStatus(task.status)}
        <section
          class="rounded-2xl border p-3 shadow-inner"
          style="border-color: {verdictColor(reviewPanel.verdict)}66; background: linear-gradient(135deg, {verdictColor(reviewPanel.verdict)}22, rgba(14, 165, 233, 0.08));"
        >
          <div class="flex items-start gap-3">
            <div class="min-w-0 flex-1">
              <div class="text-[10px] font-black uppercase tracking-wide" style="color: {verdictColor(reviewPanel.verdict)};">
                {reviewPanel.label}
              </div>
              <p class="mt-1 text-sm font-semibold leading-snug text-slate-100">{reviewPanel.why}</p>
            </div>
            {#if uiStamp}
              <span class="shrink-0 rounded-full border border-orange-400/40 bg-orange-400/15 px-2 py-0.5 text-[10px] font-black uppercase tracking-wide text-orange-200">
                Manual In Progress
              </span>
            {/if}
          </div>

          <div class="mt-3 rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-xs font-bold leading-snug {reviewPanel.hasDeploymentCallout ? 'text-amber-200' : 'text-slate-400'}">
            Deployment: {reviewPanel.deployment}
          </div>

          <div class="mt-3 flex flex-wrap gap-2">
            {#if task.pr_url}
              <button
                type="button"
                onclick={openPr}
                class="rounded-lg border border-emerald-300/30 bg-emerald-400/10 px-3 py-2 text-xs font-black text-emerald-100 hover:bg-emerald-400/15"
              >
                Open PR
              </button>
            {/if}
            {#if task.status !== 'done'}
              <button
                type="button"
                onclick={toggleManualInProgressStamp}
                class="rounded-lg border px-3 py-2 text-xs font-black text-orange-100 {uiStamp ? 'border-orange-300/50 bg-orange-400/20' : 'border-orange-300/25 bg-orange-400/10 hover:bg-orange-400/15'}"
              >
                {uiStamp ? 'Clear In Progress Stamp' : 'Stamp In Progress'}
              </button>
              <button
                type="button"
                onclick={canMergeDeploy ? requestReviewMerge : markDone}
                disabled={isReviewMergeBusy(reviewMergeState, mergeDeployState)}
                class="rounded-lg border px-3 py-2 text-xs font-black {canMergeDeploy ? 'border-cyan-300/35 bg-cyan-400/10 text-cyan-100 hover:bg-cyan-400/15' : 'border-emerald-300/30 bg-emerald-400/10 text-emerald-100 hover:bg-emerald-400/15'} {isReviewMergeBusy(reviewMergeState, mergeDeployState) ? 'opacity-70 cursor-wait' : ''}"
              >
                {canMergeDeploy ? reviewMergeButtonLabel(reviewMergeState, mergeDeployState) : 'Mark Done'}
              </button>
              {#if canRequestMergeConflictFix || isMergeConflictFixBusy(mergeConflictFixState) || mergeConflictFixRequestError || mergeConflictFixState.error}
                <button
                  type="button"
                  onclick={requestMergeConflictFix}
                  disabled={isMergeConflictFixBusy(mergeConflictFixState)}
                  class="rounded-lg border border-amber-300/40 bg-gradient-to-r from-orange-400/20 to-teal-400/10 px-3 py-2 text-xs font-black text-orange-100 {isMergeConflictFixBusy(mergeConflictFixState) ? 'opacity-70 cursor-wait' : 'hover:bg-orange-400/20'}"
                >
                  {mergeConflictFixButtonLabel(mergeConflictFixState)}
                </button>
              {/if}
            {/if}
            {#if isRequeueable}
              <button
                type="button"
                onclick={handleRequeue}
                disabled={requeueing}
                class="rounded-lg border border-indigo-300/35 bg-indigo-400/10 px-3 py-2 text-xs font-black text-indigo-100 hover:bg-indigo-400/15 {requeueing ? 'opacity-70 cursor-wait' : ''}"
              >
                {requeueing ? 'Re-queuing...' : 'Re-queue Task'}
              </button>
            {/if}
          </div>
          {#if reviewMergeRequestError || reviewMergeState.error}
            <div class="mt-3 whitespace-pre-wrap rounded-xl border border-rose-400/35 bg-rose-500/10 px-3 py-2 text-xs font-bold leading-snug text-rose-100">
              Review &amp; Merge: {reviewMergeRequestError || reviewMergeState.error}
            </div>
          {/if}
          {#if mergeConflictFixRequestError || mergeConflictFixState.error}
            <div class="mt-3 whitespace-pre-wrap rounded-xl border border-amber-400/35 bg-orange-500/10 px-3 py-2 text-xs font-bold leading-snug text-orange-100">
              Sam conflict recovery failed: {mergeConflictFixRequestError || mergeConflictFixState.error}
            </div>
          {/if}
        </section>
      {/if}

      {#if visualQA}
        <section
          class="rounded-lg border p-3"
          style="border-color: {visualQaColor(visualQaVerdict)}55; background: {visualQaColor(visualQaVerdict)}14;"
        >
          <h3 class="text-xs uppercase tracking-wide mb-1" style="color: {visualQaColor(visualQaVerdict)};">Visual QA</h3>
          <div class="flex items-start gap-2">
            <span class="shrink-0 rounded-md border px-2 py-0.5 text-[10px] font-black uppercase tracking-wide" style="border-color: {visualQaColor(visualQaVerdict)}55; color: {visualQaColor(visualQaVerdict)};">
              {visualQaVerdict}
            </span>
            <p class="text-sm text-slate-200 whitespace-pre-wrap">{visualQA.explanation}</p>
          </div>
        </section>
      {/if}

      <section>
        <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-1">Description</h3>
        {#if editingDesc}
          <textarea
            bind:value={editDesc}
            use:autofocus
            rows={4}
            class="w-full rounded-xl border border-indigo-400/40 bg-slate-950 px-3 py-2 text-sm text-slate-200 resize-y focus:outline-none focus:border-indigo-400/70"
            onblur={saveDescription}
            onkeydown={(e) => { if (e.key === 'Escape') { e.preventDefault(); editingDesc = false; editDesc = task.description || ''; } }}
          ></textarea>
          <p class="mt-1 text-[10px] text-slate-500">Click outside to save · Escape to cancel</p>
        {:else}
          <button
            type="button"
            class="min-h-[44px] w-full text-left rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm whitespace-pre-wrap cursor-text hover:bg-white/10 transition-colors {task.description ? 'text-slate-200' : 'text-slate-500 italic'}"
            title="Click to edit"
            onclick={startEditDesc}
          >{task.description || 'Add a description...'}</button>
        {/if}
      </section>

      {#if task.failure_reason}
        <section class="rounded-lg border border-rose-500/40 bg-rose-500/10 p-3">
          <h3 class="text-xs uppercase tracking-wide text-rose-300 mb-1">Failure</h3>
          <p class="text-sm text-rose-100 whitespace-pre-wrap">{task.failure_reason}</p>
        </section>
      {/if}

      <section>
        <div class="flex items-center gap-2 mb-2">
          <h3 class="text-xs uppercase tracking-wide text-slate-400">Subtasks</h3>
          {#if subtasks.length > 0}
            <span class="text-xs font-bold text-slate-400">{subtasksDone}/{subtasks.length}</span>
            {#if subtasks.length > 0}
              <div class="flex-1 h-1 rounded-full bg-white/10 overflow-hidden">
                <div class="h-full bg-indigo-400/70 transition-all" style="width:{Math.round((subtasksDone / subtasks.length) * 100)}%"></div>
              </div>
            {/if}
          {/if}
        </div>
        {#if subtasks.length > 0}
          <ul class="space-y-0.5">
            {#each subtasks as st (st.id)}
              <li
                data-subtask-id={st.id}
                class="group flex items-center gap-2 rounded-lg px-1.5 py-1 text-sm transition-colors
                  {subtaskDragId === st.id ? 'opacity-40' : ''}
                  {dropTargetId === st.id && dropPosition === 'above' ? 'border-t-2 border-indigo-400/70' : ''}
                  {dropTargetId === st.id && dropPosition === 'below' ? 'border-b-2 border-indigo-400/70' : ''}
                  hover:bg-white/5"
              >
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  class="cursor-grab active:cursor-grabbing text-slate-600 hover:text-slate-300 opacity-0 group-hover:opacity-100 transition-opacity select-none"
                  title="Drag to reorder"
                  onmousedown={(e) => onSubtaskDragStart(e, st.id)}
                >⠿</span>
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <button
                  type="button"
                  role="checkbox"
                  aria-checked={st.done}
                  onclick={() => toggleSubtask(st.id)}
                  class="h-[18px] w-[18px] shrink-0 rounded border flex items-center justify-center transition-colors {st.done ? 'border-indigo-400 bg-indigo-500/80' : 'border-white/30 hover:border-white/50'}"
                >
                  {#if st.done}
                    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2 5.5l2 2 4-4" /></svg>
                  {/if}
                </button>
                {#if editingSubtaskId === st.id}
                  <input
                    type="text"
                    bind:value={editSubtaskText}
                    use:autofocus
                    class="flex-1 rounded-md border border-indigo-400/40 bg-slate-950 px-2 py-1 text-sm text-slate-200 focus:outline-none focus:border-indigo-400/70"
                    onblur={saveEditSubtask}
                    onkeydown={handleEditSubtaskKeydown}
                  />
                {:else}
                  <span
                    class="flex-1 cursor-text {st.done ? 'line-through text-slate-500' : 'text-slate-200'}"
                    role="button"
                    tabindex="0"
                    onclick={() => startEditSubtask(st)}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); startEditSubtask(st); } }}
                  >{st.title}</span>
                {/if}
                <button
                  type="button"
                  onclick={() => deleteSubtask(st.id)}
                  class="shrink-0 text-slate-600 hover:text-rose-400 opacity-0 group-hover:opacity-100 transition-opacity"
                  aria-label="Delete subtask"
                >✕</button>
              </li>
            {/each}
          </ul>
        {/if}
        <div class="mt-2">
          <input
            type="text"
            bind:value={newSubtaskTitle}
            placeholder="Add a subtask..."
            class="w-full rounded-lg border border-white/10 bg-slate-950 px-2.5 py-1.5 text-sm text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-indigo-400/50"
            onkeydown={handleAddSubtaskKeydown}
          />
        </div>
      </section>

      <section class="flex flex-col gap-2 text-xs">
        {#if task.report_url}
          <LinkRow
            label="Report"
            url={task.report_url}
            icon="📄"
            colorClass="border-indigo-500/40 bg-indigo-500/15 text-indigo-100"
            hoverClass="hover:bg-indigo-500/25"
          />
        {/if}
        {#if task.pr_url}
          <LinkRow
            label={`PR${task.pr_number ? ` #${task.pr_number}` : ''}`}
            url={task.pr_url}
            icon="🔗"
            colorClass="border-violet-500/30 bg-violet-500/15 text-violet-100"
            hoverClass="hover:bg-violet-500/25"
          />
        {/if}
        {#if task.preview_url}
          <LinkRow
            label="Preview"
            url={task.preview_url}
            icon="👀"
            colorClass="border-sky-500/30 bg-sky-500/15 text-sky-100"
            hoverClass="hover:bg-sky-500/25"
          />
        {/if}
        {#if task.repo_url}
          <LinkRow
            label="Repo"
            url={task.repo_url}
            icon="📁"
            colorClass="border-white/10 bg-white/5 text-slate-200"
            hoverClass="hover:bg-white/10"
          />
        {/if}
        {#if task.branch}
          <div class="rounded-lg border border-white/10 bg-white/5 text-slate-300 px-3 py-2 truncate">⎇ {task.branch}</div>
        {:else if task.base_branch}
          <div class="rounded-lg border border-white/10 bg-white/5 text-slate-300 px-3 py-2 truncate">base {task.base_branch}</div>
        {/if}
      </section>

      {#if attachments.length > 0}
        <section>
          <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-2">Attachments</h3>
          <ul class="grid grid-cols-3 gap-2">
            {#each attachments as a (a.url)}
              <li class="rounded-lg overflow-hidden border border-white/10 bg-white/5 aspect-square">
                <a href={a.url} target="_blank" rel="noopener" class="block w-full h-full">
                  {#if a.mime.startsWith('image/')}
                    <img src={a.url} alt={a.name} class="w-full h-full object-cover" loading="lazy" />
                  {:else}
                    <div class="w-full h-full grid place-items-center text-xs text-slate-300 p-2 text-center">{a.name}</div>
                  {/if}
                </a>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if before.length > 0 || after.length > 0}
        <section>
          <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-2">Screenshots</h3>
          <div class="grid grid-cols-2 gap-2">
            {#each before as url}
              <figure class="rounded-lg overflow-hidden border border-white/10">
                <img src={url} alt="before" class="w-full h-auto" loading="lazy" />
                <figcaption class="text-[10px] text-slate-400 px-2 py-1 bg-white/5">before</figcaption>
              </figure>
            {/each}
            {#each after as url}
              <figure class="rounded-lg overflow-hidden border border-white/10">
                <img src={url} alt="after" class="w-full h-auto" loading="lazy" />
                <figcaption class="text-[10px] text-emerald-300 px-2 py-1 bg-white/5">after</figcaption>
              </figure>
            {/each}
          </div>
        </section>
      {/if}

      <section>
        <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-2">Activity</h3>
        {#if comments.length > 0}
          <ul bind:this={commentsEl} class="space-y-2 max-h-[40vh] overflow-y-auto pr-1">
            {#each comments as c (c.id)}
              <li class="rounded-lg border border-white/10 bg-white/5 p-2">
                <div class="text-[10px] uppercase tracking-wide text-slate-400">{c.author} · {new Date(c.created_at).toLocaleString()}</div>
                <p class="mt-0.5 text-sm leading-snug text-slate-200">{@html renderCommentHtml(c.content)}</p>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-xs text-slate-500 mb-2">No comments yet.</p>
        {/if}
        <div class="mt-2 flex items-end gap-2">
          <textarea
            bind:value={commentInput}
            placeholder="Add a comment... (use @agent or @matt for mentions)"
            rows={2}
            class="flex-1 rounded-lg border border-white/10 bg-slate-950 px-3 py-2 text-sm text-slate-200 placeholder:text-slate-500 resize-none focus:outline-none focus:border-indigo-400/50 disabled:opacity-60"
            onkeydown={handleCommentKeydown}
            disabled={postingComment}
          ></textarea>
          <button
            type="button"
            onclick={handlePostComment}
            disabled={!commentInput.trim() || postingComment}
            class="shrink-0 rounded-lg border border-emerald-300/30 bg-emerald-400/10 px-4 py-2 text-xs font-black text-emerald-100 hover:bg-emerald-400/15 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {postingComment ? '...' : 'Post'}
          </button>
        </div>
        <p class="mt-1 text-[10px] text-slate-500">Enter to post · Shift+Enter for new line</p>
      </section>

      <section class="flex flex-wrap gap-2 pt-2 border-t border-white/10">
        {#if task.pr_url}
          <button
            type="button"
            onclick={copyPrLink}
            class="rounded-lg border border-indigo-300/30 bg-indigo-400/10 px-3 py-2 text-xs font-black text-indigo-100 hover:bg-indigo-400/15"
            title="Copy PR link to clipboard"
          >
            {copiedPr ? '✓ Copied!' : '⧉ Copy PR link'}
          </button>
        {/if}
        {#if canClosePr}
          <button
            type="button"
            onclick={closePrAndDone}
            disabled={closingPr}
            class="rounded-lg border border-rose-400/40 bg-rose-500/10 px-3 py-2 text-xs font-black text-rose-100 hover:bg-rose-500/15 {closingPr ? 'opacity-70 cursor-wait' : ''}"
            title="Close this PR without merging and mark the task Done"
          >
            {closingPr ? 'Closing...' : '✕ Close PR & Mark Done'}
          </button>
        {/if}
        {#if closePrError}
          <div class="w-full whitespace-pre-wrap rounded-lg border border-rose-400/35 bg-rose-500/10 px-3 py-2 text-xs font-bold leading-snug text-rose-100">
            {closePrError}
          </div>
        {/if}
        {#if task.status === 'failed'}
          <button
            type="button"
            onclick={handleRestart}
            disabled={restarting}
            class="rounded-lg border border-emerald-400/40 bg-emerald-500/10 px-3 py-2 text-xs font-black text-emerald-100 hover:bg-emerald-500/15 {restarting ? 'opacity-70 cursor-wait' : ''}"
          >
            {restarting ? 'Restarting...' : '↻ Restart Task'}
          </button>
        {/if}
        {#if isStoppable}
          <button
            type="button"
            onclick={handleStop}
            disabled={stopping}
            class="rounded-lg border border-amber-400/40 bg-amber-500/10 px-3 py-2 text-xs font-black text-amber-100 hover:bg-amber-500/15 {stopping ? 'opacity-70 cursor-wait' : ''}"
          >
            {stopping ? 'Stopping...' : 'Stop Task'}
          </button>
        {/if}
        <button
          type="button"
          onclick={handleDelete}
          disabled={deleting}
          class="rounded-lg border px-3 py-2 text-xs font-black transition {confirmDelete ? 'border-rose-400/60 bg-rose-500/20 text-rose-100' : 'border-rose-400/20 bg-rose-500/5 text-rose-300 hover:bg-rose-500/10'} {deleting ? 'opacity-70 cursor-wait' : ''}"
        >
          {deleting ? 'Deleting...' : confirmDelete ? 'Click again to confirm' : 'Delete Task'}
        </button>
      </section>

      <footer class="text-[10px] text-slate-500 pt-2">
        created {new Date(task.created_at).toLocaleString()} · updated {new Date(task.updated_at).toLocaleString()}
      </footer>
    </div>
  </div>
</div>
