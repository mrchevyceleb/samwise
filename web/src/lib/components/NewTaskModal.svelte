<script lang="ts">
  let { projects, onClose }: { projects: string[]; onClose: () => void } = $props();

  let title = $state('');
  let description = $state('');
  let project = $state('');
  let priority = $state<'critical' | 'high' | 'medium' | 'low'>('medium');
  let task_type = $state<'code' | 'research'>('code');
  let submitting = $state(false);
  let errorMsg = $state<string | null>(null);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!title.trim() || submitting) return;
    submitting = true;
    errorMsg = null;
    try {
      const res = await fetch('/api/create-task', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          title: title.trim(),
          description: description.trim(),
          project: project || undefined,
          priority,
          task_type
        })
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        errorMsg = typeof body?.error === 'string' ? body.error : `Failed (${res.status})`;
        submitting = false;
        return;
      }
      onClose();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
      submitting = false;
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
  <form
    onsubmit={submit}
    onclick={(e) => e.stopPropagation()}
    class="w-full sm:max-w-lg rounded-t-3xl sm:rounded-3xl bg-slate-900 border border-white/10 shadow-2xl p-4 space-y-3"
  >
    <div class="flex items-center justify-between">
      <h2 class="text-lg font-semibold text-slate-100 flex items-center gap-2">
        <span class="bob">🌱</span> New task for Sam
      </h2>
      <button type="button" onclick={onClose} class="rounded-full h-8 w-8 grid place-items-center bg-white/10 hover:bg-white/20 text-slate-200" aria-label="close">✕</button>
    </div>

    <label class="block text-xs text-slate-400">
      Title
      <input
        type="text"
        bind:value={title}
        required
        maxlength="200"
        placeholder="Fix login redirect on Safari"
        class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
      />
    </label>

    <label class="block text-xs text-slate-400">
      Description
      <textarea
        bind:value={description}
        rows="5"
        placeholder="What should Sam do? Include context, acceptance criteria, links…"
        class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30 resize-y"
      ></textarea>
    </label>

    <div class="grid grid-cols-2 gap-2">
      <label class="block text-xs text-slate-400">
        Project
        <input
          list="project-list"
          bind:value={project}
          placeholder="optional"
          class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
        />
        <datalist id="project-list">
          {#each projects as p}<option value={p}></option>{/each}
        </datalist>
      </label>

      <label class="block text-xs text-slate-400">
        Priority
        <select bind:value={priority} class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-white/30">
          <option value="critical">Critical</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>
      </label>
    </div>

    <fieldset class="flex gap-2 text-xs text-slate-400">
      <legend class="sr-only">Task type</legend>
      {#each [{ v: 'code', l: '💻 Code' }, { v: 'research', l: '🔍 Research' }] as opt}
        <label class="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-center cursor-pointer hover:bg-white/10 {task_type === opt.v ? 'ring-2 ring-emerald-400/60 text-emerald-200' : 'text-slate-300'}">
          <input type="radio" name="task_type" value={opt.v} bind:group={task_type} class="sr-only" />
          {opt.l}
        </label>
      {/each}
    </fieldset>

    {#if errorMsg}
      <div class="rounded-lg border border-rose-500/40 bg-rose-500/10 p-2 text-xs text-rose-100">{errorMsg}</div>
    {/if}

    <div class="flex gap-2 pt-1">
      <button
        type="button"
        onclick={onClose}
        class="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-200 hover:bg-white/10"
      >Cancel</button>
      <button
        type="submit"
        disabled={submitting || !title.trim()}
        class="flex-1 rounded-lg bg-emerald-500/90 hover:bg-emerald-400 disabled:opacity-50 disabled:cursor-not-allowed px-3 py-2 text-sm font-medium text-slate-900 transition"
      >{submitting ? 'Sending…' : '🚀 Queue it'}</button>
    </div>
  </form>
</div>
