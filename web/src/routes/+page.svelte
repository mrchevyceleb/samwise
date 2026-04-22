<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import { STATUSES, type AeTask } from '$lib/types';
  import KanbanColumn from '$lib/components/KanbanColumn.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';

  let selected = $state<AeTask | null>(null);
  let query = $state('');
  let projectFilter = $state('');

  onMount(() => { tasksStore.init(); });
  onDestroy(() => { tasksStore.destroy(); });

  let tasks = $derived(tasksStore.tasks);
  let projects = $derived(
    Array.from(new Set(tasks.map((t) => t.project).filter(Boolean))).sort() as string[]
  );
  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return tasks.filter((t) => {
      if (projectFilter && t.project !== projectFilter) return false;
      if (!q) return true;
      return (
        t.title.toLowerCase().includes(q) ||
        (t.description ?? '').toLowerCase().includes(q) ||
        (t.project ?? '').toLowerCase().includes(q)
      );
    });
  });
  let byStatus = $derived.by(() => {
    const map = new Map<string, AeTask[]>();
    for (const s of STATUSES) map.set(s, []);
    for (const t of filtered) {
      if (!map.has(t.status)) map.set(t.status, []);
      map.get(t.status)!.push(t);
    }
    return map;
  });

  let selectedId = $derived(selected?.id);
  $effect(() => {
    if (!selectedId) return;
    const fresh = tasksStore.tasks.find((t) => t.id === selectedId);
    if (fresh && fresh !== selected) selected = fresh;
  });
</script>

<svelte:head><title>Samwise Board</title></svelte:head>

<main class="min-h-[100dvh] flex flex-col">
  <header class="sticky top-0 z-30 px-3 sm:px-6 py-3 border-b border-white/10 bg-slate-950/70 backdrop-blur">
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-2">
        <span class="text-2xl bob">🌱</span>
        <h1 class="text-lg sm:text-xl font-semibold text-slate-100">Samwise</h1>
      </div>
      <div class="flex items-center gap-1.5 text-xs {tasksStore.connected ? 'text-emerald-300' : 'text-slate-400'}">
        <span class="h-1.5 w-1.5 rounded-full {tasksStore.connected ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'}"></span>
        {tasksStore.connected ? 'live' : 'connecting'}
      </div>
      <div class="ml-auto flex items-center gap-2">
        <input
          type="search"
          bind:value={query}
          placeholder="Search…"
          class="w-32 sm:w-56 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
        />
        <select
          bind:value={projectFilter}
          class="rounded-lg bg-white/5 border border-white/10 px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-white/30"
        >
          <option value="">All</option>
          {#each projects as p}
            <option value={p}>{p}</option>
          {/each}
        </select>
      </div>
    </div>
  </header>

  {#if tasksStore.loading}
    <div class="flex-1 grid place-items-center text-slate-400">
      <div class="flex items-center gap-2"><span class="bob">🌱</span> loading…</div>
    </div>
  {:else if tasksStore.error}
    <div class="flex-1 grid place-items-center p-6">
      <div class="max-w-md rounded-xl border border-rose-500/40 bg-rose-500/10 p-4 text-sm text-rose-100">
        <div class="font-semibold mb-1">Can't reach Supabase</div>
        <div class="text-rose-200/80 text-xs">{tasksStore.error}</div>
      </div>
    </div>
  {:else}
    <div class="flex-1 overflow-x-auto overflow-y-hidden">
      <div class="flex gap-3 px-3 sm:px-6 py-4 h-full">
        {#each STATUSES as status}
          <KanbanColumn
            {status}
            tasks={byStatus.get(status) ?? []}
            onOpen={(t) => (selected = t)}
          />
        {/each}
      </div>
    </div>
  {/if}
</main>

{#if selected}
  <TaskDetail task={selected} onClose={() => (selected = null)} />
{/if}
