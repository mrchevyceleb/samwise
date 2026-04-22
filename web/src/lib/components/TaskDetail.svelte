<script lang="ts">
  import type { AeTask } from '$lib/types';
  import { PRIORITY_COLOR, STATUS_LABEL } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';

  let { task, onClose }: { task: AeTask; onClose: () => void } = $props();

  $effect(() => {
    tasksStore.loadCommentsFor(task.id);
  });

  let comments = $derived(tasksStore.comments[task.id] ?? []);
  let before = $derived(task.screenshots_before ?? []);
  let after = $derived(task.screenshots_after ?? []);
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
          <span class="text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 border-white/20 text-slate-300">
            {STATUS_LABEL[task.status] ?? task.status}
          </span>
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

      <section class="grid grid-cols-2 gap-2 text-xs">
        {#if task.pr_url}
          <a href={task.pr_url} target="_blank" rel="noopener" class="rounded-lg border border-violet-500/30 bg-violet-500/15 text-violet-100 px-3 py-2 hover:bg-violet-500/25 transition">
            🔗 PR{task.pr_number ? ` #${task.pr_number}` : ''}
          </a>
        {/if}
        {#if task.preview_url}
          <a href={task.preview_url} target="_blank" rel="noopener" class="rounded-lg border border-sky-500/30 bg-sky-500/15 text-sky-100 px-3 py-2 hover:bg-sky-500/25 transition">
            👀 Preview
          </a>
        {/if}
        {#if task.repo_url}
          <a href={task.repo_url} target="_blank" rel="noopener" class="rounded-lg border border-white/10 bg-white/5 text-slate-200 px-3 py-2 hover:bg-white/10 transition">
            📁 Repo
          </a>
        {/if}
        {#if task.branch}
          <div class="rounded-lg border border-white/10 bg-white/5 text-slate-300 px-3 py-2 truncate">⎇ {task.branch}</div>
        {/if}
      </section>

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
