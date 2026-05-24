<script lang="ts">
  import type { AeProject, TaskType } from '$lib/types';

  type Uploaded = { url: string; name: string; mime: string };
  type RepoMode = 'project' | 'none' | 'multiple';

  let { projects, onClose }: { projects: AeProject[]; onClose: () => void } = $props();

  let prompt = $state('');
  let repoMode = $state<RepoMode>('project');
  let selectedProjectId = $state('');
  let baseBranch = $state('');
  let task_type = $state<TaskType>('code');
  let qaEnvironment = $state<'staging' | 'production'>('staging');
  let submitting = $state(false);
  let errorMsg = $state<string | null>(null);
  let uploading = $state(0);
  let attachments = $state<Uploaded[]>([]);

  let canSubmit = $derived(
    prompt.trim().length > 0 &&
      uploading === 0 &&
      !submitting &&
      (repoMode !== 'project' || selectedProjectId.length > 0)
  );

  let groupedProjects = $derived.by(() => {
    const groups = new Map<string, AeProject[]>();
    for (const project of projects) {
      const key = project.client || 'Uncategorized';
      const list = groups.get(key) ?? [];
      list.push(project);
      groups.set(key, list);
    }
    return Array.from(groups.entries());
  });

  function setRepoMode(mode: RepoMode) {
    repoMode = mode;
    if (mode !== 'project') {
      selectedProjectId = '';
      baseBranch = '';
    }
  }

  async function uploadFiles(files: FileList | null) {
    if (!files || files.length === 0) return;
    const list = Array.from(files);
    uploading += list.length;
    await Promise.all(
      list.map(async (file) => {
        try {
          const form = new FormData();
          form.append('file', file);
          const res = await fetch('/api/upload-attachment', { method: 'POST', body: form });
          if (!res.ok) {
            const t = await res.text().catch(() => '');
            errorMsg = `Upload failed for ${file.name}: ${t || res.status}`;
            return;
          }
          const body = (await res.json()) as Uploaded;
          attachments = [...attachments, body];
        } catch (e) {
          errorMsg = e instanceof Error ? e.message : String(e);
        } finally {
          uploading -= 1;
        }
      })
    );
  }

  function removeAttachment(url: string) {
    attachments = attachments.filter((a) => a.url !== url);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    submitting = true;
    errorMsg = null;
    try {
      const res = await fetch('/api/create-task', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          prompt: prompt.trim(),
          repo_mode: repoMode,
          project_id: repoMode === 'project' ? selectedProjectId : undefined,
          // qa-verify checks staging/production URLs — base branch is ignored
          // by the worker for that path, so don't send a stale value.
          base_branch:
            repoMode === 'project' && task_type !== 'qa-verify'
              ? baseBranch.trim() || undefined
              : undefined,
          task_type,
          environment: task_type === 'qa-verify' ? qaEnvironment : undefined,
          attachments: attachments.map((a) => ({ url: a.url, name: a.name, mime: a.mime }))
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

  const repoModes: { v: RepoMode; l: string }[] = [
    { v: 'project', l: 'Single repo' },
    { v: 'none', l: 'No repo' },
    { v: 'multiple', l: 'Multiple repos' }
  ];

  const taskModes: { v: TaskType; l: string }[] = [
    { v: 'code', l: 'Coding' },
    { v: 'research', l: 'Research' },
    { v: 'qa-verify', l: 'QA Verify' }
  ];
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
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    onsubmit={submit}
    onclick={(e) => e.stopPropagation()}
    class="w-full sm:max-w-xl rounded-t-2xl sm:rounded-2xl bg-slate-900 border border-white/10 shadow-2xl p-4 space-y-4"
  >
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-lg font-semibold text-slate-100">New task for Sam</h2>
      </div>
      <button type="button" onclick={onClose} class="rounded-full h-8 w-8 grid place-items-center bg-white/10 hover:bg-white/20 text-slate-200" aria-label="close">x</button>
    </div>

    <fieldset class="space-y-2">
      <legend class="text-xs font-semibold text-slate-400">Repo</legend>
      <div class="grid grid-cols-3 gap-2">
        {#each repoModes as opt}
          <button
            type="button"
            onclick={() => setRepoMode(opt.v)}
            class="rounded-lg border px-3 py-2 text-center text-sm font-semibold transition {repoMode === opt.v ? 'border-emerald-400/60 bg-emerald-400/10 text-emerald-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
          >
            {opt.l}
          </button>
        {/each}
      </div>
    </fieldset>

    {#if repoMode === 'project'}
      <label class="block text-xs font-semibold text-slate-400">
        Select repo
        <select
          bind:value={selectedProjectId}
          onchange={() => { baseBranch = ''; }}
          class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-white/30"
        >
          <option value="">{projects.length === 0 ? 'No projects configured' : 'Choose a project...'}</option>
          {#each groupedProjects as [client, clientProjects]}
            <optgroup label={client}>
              {#each clientProjects as p}
                <option value={p.id}>{p.name}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
      </label>

      {#if task_type !== 'qa-verify'}
        <label class="block text-xs font-semibold text-slate-400">
          Base branch (optional)
          <input
            type="text"
            bind:value={baseBranch}
            placeholder="Leave blank for default branch"
            autocomplete="off"
            spellcheck="false"
            class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30"
          />
        </label>
      {/if}
    {/if}

    <label class="block text-xs font-semibold text-slate-400">
      Prompt
      <textarea
        bind:value={prompt}
        rows="7"
        required
        placeholder={repoMode === 'multiple' ? 'Name the repos and describe the work Sam should coordinate across them.' : 'Describe what Sam should build, fix, investigate, or change.'}
        class="mt-1 w-full rounded-lg bg-white/5 border border-white/10 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/30 resize-y leading-relaxed"
      ></textarea>
    </label>

    <fieldset class="space-y-2">
      <legend class="text-xs font-semibold text-slate-400">Mode</legend>
      <div class="grid grid-cols-3 gap-2">
        {#each taskModes as opt}
          <button
            type="button"
            onclick={() => (task_type = opt.v)}
            class="rounded-lg border px-3 py-2 text-center text-sm font-semibold transition {task_type === opt.v ? 'border-sky-400/60 bg-sky-400/10 text-sky-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
          >
            {opt.l}
          </button>
        {/each}
      </div>
    </fieldset>

    {#if task_type === 'qa-verify'}
      <fieldset class="space-y-2">
        <legend class="text-xs font-semibold text-slate-400">QA Environment</legend>
        <div class="grid grid-cols-2 gap-2">
          <button
            type="button"
            onclick={() => (qaEnvironment = 'staging')}
            class="rounded-lg border px-3 py-2 text-center text-sm font-semibold transition {qaEnvironment === 'staging' ? 'border-emerald-400/60 bg-emerald-400/10 text-emerald-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
          >
            Staging
          </button>
          <button
            type="button"
            onclick={() => (qaEnvironment = 'production')}
            class="rounded-lg border px-3 py-2 text-center text-sm font-semibold transition {qaEnvironment === 'production' ? 'border-rose-400/60 bg-rose-400/10 text-rose-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
          >
            Production
          </button>
        </div>
        <p class="text-[11px] text-slate-500">Resolves the selected project's staging or production URL automatically.</p>
      </fieldset>
    {/if}

    <div>
      <div class="block text-xs font-semibold text-slate-400 mb-1">
        Attachments
      </div>
      <label class="flex items-center gap-2 rounded-lg border border-dashed border-white/15 bg-white/5 px-3 py-3 text-sm text-slate-300 hover:bg-white/10 cursor-pointer transition">
        <span class="flex-1">
          {uploading > 0 ? `Uploading ${uploading}...` : 'Tap to attach images or PDFs'}
        </span>
        <input
          type="file"
          multiple
          accept="image/*,application/pdf"
          capture="environment"
          class="sr-only"
          onchange={(e) => { uploadFiles((e.currentTarget as HTMLInputElement).files); (e.currentTarget as HTMLInputElement).value = ''; }}
        />
      </label>

      {#if attachments.length > 0}
        <ul class="mt-2 grid grid-cols-3 gap-2">
          {#each attachments as a (a.url)}
            <li class="relative rounded-lg overflow-hidden border border-white/10 bg-white/5 aspect-square">
              {#if a.mime.startsWith('image/')}
                <img src={a.url} alt={a.name} class="w-full h-full object-cover" loading="lazy" />
              {:else}
                <div class="w-full h-full grid place-items-center text-xs text-slate-300 p-2 text-center">{a.name}</div>
              {/if}
              <button
                type="button"
                onclick={() => removeAttachment(a.url)}
                aria-label="remove"
                class="absolute top-1 right-1 h-6 w-6 grid place-items-center rounded-full bg-black/70 text-slate-100 text-xs hover:bg-rose-500/80"
              >x</button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

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
        disabled={!canSubmit}
        class="flex-1 rounded-lg bg-emerald-500/90 hover:bg-emerald-400 disabled:opacity-50 disabled:cursor-not-allowed px-3 py-2 text-sm font-medium text-slate-900 transition"
      >{submitting ? 'Queuing...' : uploading > 0 ? 'Uploading...' : 'Queue it'}</button>
    </div>
  </form>
</div>
