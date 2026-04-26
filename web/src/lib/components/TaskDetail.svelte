<script lang="ts">
  import type { AeTask, TaskStatus } from '$lib/types';
  import { PRIORITY_COLOR, STATUS_LABEL, STATUSES } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import {
    extractReviewActionPanel,
    getUiStamp,
    getMergeDeployState,
    isReviewActionStatus,
    isMergeDeployBusy,
    mergeDeployButtonLabel,
    nextManualInProgressStampContext,
    requestMergeDeployContext
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
  let canMergeDeploy = $derived(!!task.pr_url && (task.status === 'approved' || mergeDeployState.status === 'failed'));

  function verdictColor(verdict: string | undefined) {
    if (verdict === 'merge') return '#34d399';
    if (verdict === 'fix' || verdict === 'blocked') return '#fb923c';
    if (verdict === 'errored') return '#fb7185';
    return '#60a5fa';
  }

  function openPr() {
    if (task.pr_url) window.open(task.pr_url, '_blank', 'noopener');
  }

  async function toggleManualInProgressStamp() {
    await tasksStore.updateTask(task.id, { context: nextManualInProgressStampContext(task) });
  }

  async function requestMergeDeploy() {
    if (!canMergeDeploy || isMergeDeployBusy(mergeDeployState)) return;
    await tasksStore.updateTask(task.id, { context: requestMergeDeployContext(task) });
  }

  async function markDone() {
    await tasksStore.setStatus(task.id, 'done');
    onClose();
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
            value={task.status}
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
          <span class="text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 {PRIORITY_COLOR[task.priority]}">
            {task.priority}
          </span>
          {#if task.project}
            <span class="text-xs text-slate-400">📦 {task.project}</span>
          {/if}
        </div>
        <h2 class="mt-1 text-lg font-semibold text-slate-100">{task.title}</h2>
      </div>
      <button
        type="button"
        onclick={onClose}
        class="shrink-0 rounded-full h-8 w-8 grid place-items-center bg-white/10 hover:bg-white/20 text-slate-200"
        aria-label="close"
      >✕</button>
    </header>

    <div class="px-4 py-4 space-y-4">
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
                onclick={canMergeDeploy ? requestMergeDeploy : markDone}
                disabled={isMergeDeployBusy(mergeDeployState)}
                class="rounded-lg border px-3 py-2 text-xs font-black {canMergeDeploy ? 'border-cyan-300/35 bg-cyan-400/10 text-cyan-100 hover:bg-cyan-400/15' : 'border-emerald-300/30 bg-emerald-400/10 text-emerald-100 hover:bg-emerald-400/15'} {isMergeDeployBusy(mergeDeployState) ? 'opacity-70 cursor-wait' : ''}"
              >
                {canMergeDeploy ? mergeDeployButtonLabel(mergeDeployState) : 'Mark Done'}
              </button>
            {/if}
          </div>
        </section>
      {/if}

      {#if task.description}
        <section>
          <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-1">Description</h3>
          <p class="text-sm text-slate-200 whitespace-pre-wrap">{task.description}</p>
        </section>
      {/if}

      {#if task.failure_reason}
        <section class="rounded-lg border border-rose-500/40 bg-rose-500/10 p-3">
          <h3 class="text-xs uppercase tracking-wide text-rose-300 mb-1">Failure</h3>
          <p class="text-sm text-rose-100 whitespace-pre-wrap">{task.failure_reason}</p>
        </section>
      {/if}

      {#if task.subtasks && task.subtasks.length > 0}
        <section>
          <h3 class="text-xs uppercase tracking-wide text-slate-400 mb-2">Subtasks</h3>
          <ul class="space-y-1">
            {#each task.subtasks as s}
              <li class="flex items-start gap-2 text-sm">
                <span class="mt-0.5">{s.done ? '✅' : '⬜'}</span>
                <span class={s.done ? 'line-through text-slate-500' : 'text-slate-200'}>{s.title}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <section class="flex flex-col gap-2 text-xs">
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
        {#if comments.length === 0}
          <p class="text-xs text-slate-500">No comments yet.</p>
        {:else}
          <ul class="space-y-2">
            {#each comments as c}
              <li class="rounded-lg border border-white/10 bg-white/5 p-2">
                <div class="text-[10px] uppercase tracking-wide text-slate-400">{c.author} · {new Date(c.created_at).toLocaleString()}</div>
                <p class="text-sm text-slate-200 whitespace-pre-wrap">{c.content}</p>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <footer class="text-[10px] text-slate-500 pt-2">
        created {new Date(task.created_at).toLocaleString()} · updated {new Date(task.updated_at).toLocaleString()}
      </footer>
    </div>
  </div>
</div>
