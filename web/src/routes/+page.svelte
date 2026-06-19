<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import { supabase } from '$lib/supabase';
  import { STATUSES, type AeTask, type AeProject, type TaskStatus } from '$lib/types';
  import { isMergeInFlight } from '$lib/utils/review-actions';
  import KanbanColumn from '$lib/components/KanbanColumn.svelte';
  import TaskDetail from '$lib/components/TaskDetail.svelte';
  import NewTaskModal from '$lib/components/NewTaskModal.svelte';
  import ScheduleModal from '$lib/components/ScheduleModal.svelte';

  let selected = $state<AeTask | null>(null);
  let showNew = $state(false);
  let showSchedules = $state(false);
  let query = $state('');
  let projectFilter = $state('');
  let projectRegistry = $state<AeProject[]>([]);
  let buildVersion = $state('');
  let versionTimer: ReturnType<typeof setInterval> | null = null;
  let refreshing = $state(false);
  let collapsedColumns = $state<Record<'done' | 'failed', boolean>>({ done: false, failed: false });
  const COLLAPSED_COLUMNS_KEY = 'samwise-board-collapsed-columns';

  async function refreshAll() {
    refreshing = true;
    try {
      await Promise.all([tasksStore.refresh(), fetchProjectRegistry()]);
      await checkBuildVersion(true);
    } finally {
      refreshing = false;
    }
  }

  function onVisibility() {
    if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
      void refreshAll();
    }
  }

  async function checkBuildVersion(reloadOnChange: boolean) {
    try {
      const res = await fetch(`/_app/version.json?ts=${Date.now()}`, { cache: 'no-store' });
      if (!res.ok) return;
      const data = (await res.json()) as { version?: string };
      if (!data.version) return;
      if (!buildVersion) {
        buildVersion = data.version;
        return;
      }
      if (reloadOnChange && data.version !== buildVersion) {
        window.location.reload();
      }
    } catch (e) {
      console.warn('[version] check failed', e);
    }
  }

  onMount(() => {
    loadCollapsedColumns();
    tasksStore.init();
    fetchProjectRegistry();
    checkBuildVersion(false);
    versionTimer = setInterval(() => checkBuildVersion(true), 60_000);
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', onVisibility);
    }
  });

  onDestroy(() => {
    tasksStore.destroy();
    if (versionTimer) clearInterval(versionTimer);
    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', onVisibility);
    }
  });

  let tasks = $derived(tasksStore.tasks);
  let boardProjects = $derived(
    Array.from(new Set(tasks.map((t) => t.project).filter(Boolean))).sort() as string[]
  );
  let buildLabel = $derived(buildVersion ? buildVersion.slice(-6) : '');
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
      const status = displayStatus(t);
      if (!map.has(status)) map.set(status, []);
      map.get(status)!.push(t);
    }
    return map;
  });

  function displayStatus(task: AeTask): TaskStatus {
    if (isMergeInFlight(task)) return 'qa';
    return task.status === 'testing' ? 'in_progress' : task.status;
  }

  function loadCollapsedColumns() {
    if (typeof localStorage === 'undefined') return;
    try {
      const raw = localStorage.getItem(COLLAPSED_COLUMNS_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Partial<Record<'done' | 'failed', boolean>>;
      collapsedColumns = {
        done: parsed.done === true,
        failed: parsed.failed === true
      };
    } catch {
      collapsedColumns = { done: false, failed: false };
    }
  }

  function persistCollapsedColumns() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(COLLAPSED_COLUMNS_KEY, JSON.stringify(collapsedColumns));
    }
  }

  function isCollapsibleStatus(status: TaskStatus): status is 'done' | 'failed' {
    return status === 'done' || status === 'failed';
  }

  function isColumnCollapsed(status: TaskStatus): boolean {
    return isCollapsibleStatus(status) ? collapsedColumns[status] : false;
  }

  function toggleColumnCollapse(status: 'done' | 'failed') {
    collapsedColumns = { ...collapsedColumns, [status]: !collapsedColumns[status] };
    persistCollapsedColumns();
  }

  let selectedId = $derived(selected?.id);
  $effect(() => {
    if (!selectedId) return;
    const fresh = tasksStore.tasks.find((t) => t.id === selectedId);
    if (fresh && fresh !== selected) selected = fresh;
  });

  async function fetchProjectRegistry() {
    try {
      const { data, error } = await supabase
        .from('ae_projects')
        .select('id,name,repo_url,repo_path,preview_url,client,deploy_method,dev_command,created_at')
        .order('client', { ascending: true })
        .order('name', { ascending: true });
      if (error) throw error;
      projectRegistry = (data ?? []) as AeProject[];
    } catch (e) {
      console.warn('[projects] fetch registry failed', e);
      projectRegistry = [];
    }
  }
</script>

<svelte:head><title>Samwise Board</title></svelte:head>

<main class="min-h-[100dvh] flex flex-col">
  <header class="sticky top-0 z-30 px-3 sm:px-6 py-3 border-b border-white/10 bg-slate-950/70 backdrop-blur">
    <div class="flex flex-wrap items-center gap-2 sm:gap-3">
      <div class="flex items-center gap-2">
        <span class="text-2xl bob">🌱</span>
        <h1 class="text-lg sm:text-xl font-semibold text-slate-100">Samwise</h1>
      </div>
      <div class="flex items-center gap-1.5 text-xs {tasksStore.connected ? 'text-emerald-300' : 'text-slate-400'}">
        <span class="h-1.5 w-1.5 rounded-full {tasksStore.connected ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'}"></span>
        {tasksStore.connected ? 'live' : 'connecting'}
      </div>
      {#if buildLabel}
        <div class="hidden sm:block text-[10px] uppercase tracking-wide text-slate-500">build {buildLabel}</div>
      {/if}
      <div class="ml-auto flex items-center gap-2">
        <button
          type="button"
          onclick={refreshAll}
          disabled={refreshing}
          class="hidden sm:inline-flex items-center rounded-lg border border-emerald-500/40 bg-emerald-500/10 px-3 py-1.5 text-xs font-semibold text-emerald-100 transition hover:-translate-y-0.5 hover:bg-emerald-500/20 disabled:opacity-60 disabled:cursor-not-allowed"
          title="Refresh tasks and projects"
          aria-label="Refresh"
        >
          <span class="inline-block {refreshing ? 'animate-spin' : ''}">🔄</span>
          <span class="ml-1 hidden sm:inline">{refreshing ? 'Refreshing…' : 'Refresh'}</span>
        </button>
        <a
          href="/reports"
          class="rounded-lg border border-indigo-500/40 bg-indigo-500/10 px-3 py-1.5 text-xs font-semibold text-indigo-100 hover:bg-indigo-500/20 transition-colors"
          title="Browse research reports"
        >
          📄 Reports
        </a>
        <button
          type="button"
          onclick={() => (showSchedules = true)}
          class="rounded-lg border border-sky-500/40 bg-sky-500/10 px-3 py-1.5 text-xs font-semibold text-sky-100 transition hover:-translate-y-0.5 hover:bg-sky-500/20"
          title="Manage cron jobs"
        >
          ⏱ Schedules
        </button>
      </div>
      <div class="basis-full sm:hidden" aria-hidden="true"></div>
      <input
        type="search"
        bind:value={query}
        placeholder="Search…"
        class="flex-1 min-w-0 sm:flex-none sm:w-56 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
      />
      <select
        bind:value={projectFilter}
        class="shrink-0 rounded-lg bg-white/5 border border-white/10 px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-white/30"
      >
        <option value="">All</option>
        {#each boardProjects as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
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
            collapsed={isColumnCollapsed(status)}
            onToggleCollapse={isCollapsibleStatus(status) ? () => toggleColumnCollapse(status) : undefined}
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

{#if showNew}
  <NewTaskModal projects={projectRegistry} onClose={() => (showNew = false)} />
{/if}

{#if showSchedules}
  <ScheduleModal onClose={() => (showSchedules = false)} />
{/if}

<button
  type="button"
  onclick={() => (showNew = true)}
  aria-label="New task"
  class="fixed bottom-5 right-5 z-30 h-14 w-14 rounded-full bg-emerald-500 hover:bg-emerald-400 shadow-xl shadow-emerald-500/30 text-slate-900 text-3xl font-bold grid place-items-center hover:scale-105 active:scale-95 transition bob"
>+</button>

<button
  type="button"
  onclick={refreshAll}
  disabled={refreshing}
  aria-label="Refresh tasks"
  class="sm:hidden fixed bottom-5 left-5 z-30 h-12 rounded-full border border-emerald-300/50 bg-slate-950/90 px-4 text-sm font-semibold text-emerald-100 shadow-xl shadow-black/30 backdrop-blur flex items-center gap-2 active:scale-95 transition disabled:opacity-60 disabled:cursor-not-allowed"
>
  <span class="inline-block text-base {refreshing ? 'animate-spin' : ''}">🔄</span>
  <span>{refreshing ? 'Refreshing...' : 'Refresh'}</span>
</button>
