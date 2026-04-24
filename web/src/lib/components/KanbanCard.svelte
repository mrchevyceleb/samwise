<script lang="ts">
  import type { AeTask } from '$lib/types';
  import { PRIORITY_COLOR } from '$lib/types';
  let { task, onOpen }: { task: AeTask; onOpen: (t: AeTask) => void } = $props();

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
</script>

<button
  type="button"
  draggable="true"
  ondragstart={(e) => {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('application/x-samwise-task', task.id);
    // Plain text fallback for browsers that ignore custom MIME types.
    e.dataTransfer.setData('text/plain', task.id);
  }}
  onclick={() => onOpen(task)}
  class="group w-full text-left rounded-xl border border-white/10 bg-white/5 p-3 backdrop-blur hover:bg-white/10 hover:scale-[1.01] hover:-translate-y-0.5 active:scale-[0.99] transition-all shadow-sm cursor-grab active:cursor-grabbing"
>
  <div class="flex items-start justify-between gap-2">
    <div class="text-sm font-medium text-slate-100 line-clamp-2">{task.title}</div>
    <span class="shrink-0 text-[10px] uppercase tracking-wide rounded-md border px-1.5 py-0.5 {PRIORITY_COLOR[task.priority]}">
      {task.priority}
    </span>
  </div>

  {#if task.project}
    <div class="mt-2 text-xs text-slate-400">📦 {task.project}</div>
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

  <div class="mt-2 flex flex-wrap items-center gap-1.5 text-[10px] text-slate-400">
    {#if task.pr_number}
      <span class="rounded bg-violet-500/15 border border-violet-500/30 text-violet-200 px-1.5 py-0.5">PR #{task.pr_number}</span>
    {/if}
    {#if task.branch}
      <span class="rounded bg-white/5 border border-white/10 px-1.5 py-0.5 truncate max-w-[140px]">⎇ {task.branch}</span>
    {/if}
    <span class="ml-auto">{relTime(task.updated_at)}</span>
  </div>
</button>
