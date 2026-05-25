<script lang="ts">
  import type { AeTask, TaskStatus } from '$lib/types';
  import { STATUS_LABEL } from '$lib/types';
  import { tasksStore } from '$lib/stores/tasks.svelte';
  import KanbanCard from './KanbanCard.svelte';

  let {
    status,
    tasks,
    onOpen,
    collapsed = false,
    onToggleCollapse
  }: {
    status: TaskStatus;
    tasks: AeTask[];
    onOpen: (t: AeTask) => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
  } = $props();

  let dragHover = $state(false);

  const statusDot: Record<TaskStatus, string> = {
    queued: 'bg-slate-400',
    in_progress: 'bg-sky-400',
    testing: 'bg-amber-400',
    review: 'bg-violet-400',
    fixes_needed: 'bg-orange-400',
    approved: 'bg-emerald-400',
    qa: 'bg-teal-400',
    done: 'bg-emerald-500',
    failed: 'bg-rose-500',
    pending_confirmation: 'bg-yellow-400'
  };

  function handleDragOver(e: DragEvent) {
    // preventDefault is what marks this element a valid drop target.
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragHover = true;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragHover = false;
    const taskId =
      e.dataTransfer?.getData('application/x-samwise-task') ||
      e.dataTransfer?.getData('text/plain') ||
      '';
    if (!taskId) return;
    tasksStore.setStatus(taskId, status);
  }

  let collapseLabel = $derived(`${collapsed ? 'Expand' : 'Collapse'} ${STATUS_LABEL[status]}`);
</script>

<section
  class="{collapsed ? 'w-12' : 'w-[85vw] sm:w-80 md:w-72'} shrink-0 rounded-2xl bg-white/5 border backdrop-blur flex flex-col max-h-[calc(100dvh-7rem)] transition-all {dragHover
    ? 'border-emerald-400/60 bg-emerald-400/5'
    : 'border-white/10'}"
  ondragover={handleDragOver}
  ondragleave={() => (dragHover = false)}
  ondrop={handleDrop}
  role="list"
  aria-label={STATUS_LABEL[status]}
>
  <header class="{collapsed ? 'h-full border-b-0 rounded-2xl' : 'border-b rounded-t-2xl'} border-white/10 sticky top-0 bg-white/5 backdrop-blur">
    <button
      type="button"
      disabled={!onToggleCollapse}
      onclick={() => onToggleCollapse?.()}
      aria-label={onToggleCollapse ? collapseLabel : STATUS_LABEL[status]}
      title={onToggleCollapse ? collapseLabel : undefined}
      class="{collapsed ? 'h-full min-h-44 flex-col justify-start px-2 py-3' : 'px-3 py-2 justify-between'} flex w-full items-center gap-2 text-left transition-colors {onToggleCollapse ? 'cursor-pointer hover:bg-white/5' : 'cursor-default'} disabled:cursor-default"
    >
      <span class="flex items-center gap-2 {collapsed ? 'flex-col' : ''}">
        <span class="h-2 w-2 shrink-0 rounded-full bob {statusDot[status]}"></span>
        <h2 class="{collapsed ? '[writing-mode:vertical-rl] text-xs' : 'text-sm'} font-semibold text-slate-100">
          {STATUS_LABEL[status]}
        </h2>
      </span>
      <span class="{collapsed ? 'mt-2' : ''} text-xs text-slate-400">{tasks.length}</span>
      {#if onToggleCollapse && !collapsed}
        <span class="text-slate-500" aria-hidden="true">‹</span>
      {/if}
    </button>
  </header>

  {#if !collapsed}
    <div class="flex-1 overflow-y-auto p-2 space-y-2">
      {#each tasks as t (t.id)}
        <KanbanCard task={t} {onOpen} />
      {:else}
        <div class="text-center text-xs text-slate-500 py-6">nothing here</div>
      {/each}
    </div>
  {/if}
</section>
