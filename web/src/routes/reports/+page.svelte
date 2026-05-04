<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';
  import type { AeTask } from '$lib/types';

  let query = $state('');
  let projectFilter = $state('');
  let selected = $state<AeTask | null>(null);

  onMount(() => {
    tasksStore.init();
  });

  onDestroy(() => {
    tasksStore.destroy();
  });

  // All research tasks, regardless of status. New runs land in `review` and stay
  // visible. Manually-archived ones in `done` still show here so you can find
  // them later.
  let researchTasks = $derived(
    tasksStore.tasks
      .filter((t) => t.task_type === 'research')
      .slice()
      .sort(
        (a, b) =>
          new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      )
  );

  let projects = $derived(
    Array.from(
      new Set(researchTasks.map((t) => t.project).filter(Boolean) as string[])
    ).sort()
  );

  let filtered = $derived(
    researchTasks.filter((t) => {
      if (projectFilter && t.project !== projectFilter) return false;
      if (query.trim()) {
        const q = query.toLowerCase();
        const hay = [t.title, t.description ?? '', t.project ?? ''].join(' ').toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    })
  );

  function statusBadgeClass(status: string): string {
    switch (status) {
      case 'review':
        return 'border-emerald-500/30 bg-emerald-500/15 text-emerald-200';
      case 'done':
        return 'border-slate-500/30 bg-slate-500/15 text-slate-300';
      case 'in_progress':
        return 'border-indigo-500/30 bg-indigo-500/15 text-indigo-200';
      case 'queued':
        return 'border-slate-500/30 bg-slate-500/10 text-slate-300';
      case 'pending_confirmation':
        return 'border-amber-500/30 bg-amber-500/15 text-amber-200';
      case 'failed':
        return 'border-rose-500/30 bg-rose-500/15 text-rose-200';
      default:
        return 'border-white/10 bg-white/5 text-slate-300';
    }
  }

  function formatRelative(iso: string): string {
    const then = new Date(iso).getTime();
    const now = Date.now();
    const sec = Math.max(1, Math.round((now - then) / 1000));
    if (sec < 60) return `${sec}s ago`;
    const min = Math.round(sec / 60);
    if (min < 60) return `${min}m ago`;
    const hr = Math.round(min / 60);
    if (hr < 48) return `${hr}h ago`;
    const day = Math.round(hr / 24);
    if (day < 30) return `${day}d ago`;
    const mo = Math.round(day / 30);
    return `${mo}mo ago`;
  }
</script>

<svelte:head><title>Reports · Samwise</title></svelte:head>

<main class="min-h-[100dvh] flex flex-col">
  <header class="sticky top-0 z-30 px-3 sm:px-6 py-3 border-b border-white/10 bg-slate-950/70 backdrop-blur">
    <div class="flex items-center gap-3">
      <a href="/" class="flex items-center gap-2 hover:opacity-80 transition-opacity">
        <span class="text-2xl">🌱</span>
        <h1 class="text-lg sm:text-xl font-semibold text-slate-100">Samwise</h1>
      </a>
      <span class="text-slate-600">/</span>
      <h2 class="text-base sm:text-lg font-medium text-slate-200">Reports</h2>
      <div class="ml-auto flex items-center gap-2">
        <input
          type="search"
          bind:value={query}
          placeholder="Search reports…"
          class="w-40 sm:w-64 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
        />
        <select
          bind:value={projectFilter}
          class="rounded-lg bg-white/5 border border-white/10 px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-white/30"
        >
          <option value="">All projects</option>
          {#each projects as p}
            <option value={p}>{p}</option>
          {/each}
        </select>
      </div>
    </div>
  </header>

  {#if tasksStore.loading}
    <div class="flex-1 grid place-items-center text-slate-400">
      <div class="flex items-center gap-2">📄 loading…</div>
    </div>
  {:else if tasksStore.error}
    <div class="flex-1 grid place-items-center p-6">
      <div class="max-w-md rounded-xl border border-rose-500/40 bg-rose-500/10 p-4 text-sm text-rose-100">
        <div class="font-semibold mb-1">Can't reach Supabase</div>
        <div class="text-rose-200/80 text-xs">{tasksStore.error}</div>
      </div>
    </div>
  {:else if researchTasks.length === 0}
    <div class="flex-1 grid place-items-center p-6 text-slate-400">
      <div class="text-center">
        <div class="text-5xl mb-3">📭</div>
        <div class="text-sm">No research tasks yet.</div>
        <div class="text-xs text-slate-500 mt-1">Create one from the board with task type "Research".</div>
      </div>
    </div>
  {:else}
    <div class="flex-1 px-3 sm:px-6 py-4">
      {#if filtered.length === 0}
        <div class="text-sm text-slate-400 text-center py-12">
          No reports match those filters.
        </div>
      {:else}
        <div class="text-xs text-slate-500 mb-3">
          {filtered.length} of {researchTasks.length} report{researchTasks.length === 1 ? '' : 's'}
        </div>
        <ul class="flex flex-col gap-2">
          {#each filtered as task (task.id)}
            <li
              class="group rounded-xl border border-white/10 bg-white/[0.03] hover:bg-white/[0.06] hover:border-white/20 transition-colors"
            >
              <div class="flex items-start gap-3 p-3 sm:p-4">
                <div class="flex-1 min-w-0 cursor-pointer" onclick={() => (selected = task)} onkeydown={(e) => { if (e.key === 'Enter') selected = task; }} role="button" tabindex="0">
                  <div class="flex items-center gap-2 flex-wrap mb-1">
                    <span class="text-[10px] uppercase tracking-wide font-semibold rounded-md border px-1.5 py-0.5 {statusBadgeClass(task.status)}">
                      {task.status.replace('_', ' ')}
                    </span>
                    {#if task.project}
                      <span class="text-[10px] uppercase tracking-wide font-semibold rounded-md border border-violet-500/30 bg-violet-500/10 px-1.5 py-0.5 text-violet-200">
                        @{task.project}
                      </span>
                    {/if}
                    <span class="text-[10px] text-slate-500">{formatRelative(task.created_at)}</span>
                  </div>
                  <div class="text-sm font-medium text-slate-100 leading-snug">{task.title}</div>
                  {#if task.description}
                    <div class="text-xs text-slate-400 mt-1 line-clamp-2">{task.description}</div>
                  {/if}
                </div>
                {#if task.report_url}
                  <a
                    href={task.report_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    class="shrink-0 self-center rounded-lg border border-indigo-500/40 bg-indigo-500/10 hover:bg-indigo-500/20 px-3 py-1.5 text-xs font-semibold text-indigo-100 transition-colors flex items-center gap-1.5"
                    title="Open the rendered report (tailnet only)"
                  >
                    Open
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
                      <polyline points="15 3 21 3 21 9"/>
                      <line x1="10" y1="14" x2="21" y2="3"/>
                    </svg>
                  </a>
                {:else}
                  <span class="shrink-0 self-center text-[11px] text-slate-500 italic">no link</span>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</main>

{#if selected}
  <TaskDetail task={selected} onClose={() => (selected = null)} />
{/if}

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
