<script lang="ts">
  import type { AeTask, TaskStatus } from '$lib/types';
  import { STATUS_LABEL } from '$lib/types';
  import KanbanCard from './KanbanCard.svelte';

  let { status, tasks, onOpen }: { status: TaskStatus; tasks: AeTask[]; onOpen: (t: AeTask) => void } = $props();

  const statusDot: Record<TaskStatus, string> = {
    queued: 'bg-slate-400',
    in_progress: 'bg-sky-400',
    testing: 'bg-amber-400',
    review: 'bg-violet-400',
    fixes_needed: 'bg-orange-400',
    approved: 'bg-emerald-400',
    done: 'bg-emerald-500',
    failed: 'bg-rose-500',
    pending_confirmation: 'bg-yellow-400'
  };
</script>

<section class="w-[85vw] sm:w-80 md:w-72 shrink-0 rounded-2xl bg-white/5 border border-white/10 backdrop-blur flex flex-col max-h-[calc(100dvh-7rem)]">
  <header class="flex items-center justify-between px-3 py-2 border-b border-white/10 sticky top-0 bg-white/5 backdrop-blur rounded-t-2xl">
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bob {statusDot[status]}"></span>
      <h2 class="text-sm font-semibold text-slate-100">{STATUS_LABEL[status]}</h2>
    </div>
    <span class="text-xs text-slate-400">{tasks.length}</span>
  </header>

  <div class="flex-1 overflow-y-auto p-2 space-y-2">
    {#each tasks as t (t.id)}
      <KanbanCard task={t} {onOpen} />
    {:else}
      <div class="text-center text-xs text-slate-500 py-6">nothing here</div>
    {/each}
  </div>
</section>
