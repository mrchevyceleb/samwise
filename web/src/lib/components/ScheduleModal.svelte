<script lang="ts">
  import { onMount } from 'svelte';
  import { cronsStore } from '$lib/stores/crons.svelte';
  import type { AeCron } from '$lib/types';

  let { onClose }: { onClose: () => void } = $props();

  const presets = [
    { value: '*/15 * * * *', label: 'Every 15 min' },
    { value: '*/30 * * * *', label: 'Every 30 min' },
    { value: '0 * * * *', label: 'Every hour' },
    { value: '0 14 * * *', label: 'Daily at 9am ET' },
    { value: '0 14 * * 1-5', label: 'Weekdays at 9am ET' },
    { value: '0 14 * * 1', label: 'Every Monday 9am ET' },
    { value: '0 17 * * *', label: 'Daily at noon ET' },
    { value: '0 5 * * *', label: 'Daily at midnight ET' },
    { value: '0 */6 * * *', label: 'Every 6 hours' }
  ];

  let showForm = $state(false);
  let editing = $state<AeCron | null>(null);
  let prompt = $state('');
  let scheduleMode = $state<'preset' | 'custom'>('preset');
  let presetSchedule = $state('0 14 * * *');
  let customSchedule = $state('');
  let targetMode = $state<'project' | 'parent'>('project');
  let project = $state('');
  let repoParent = $state('');
  let enabled = $state(true);
  let adminKey = $state('');
  let localError = $state<string | null>(null);
  let deletingId = $state<string | null>(null);

  onMount(() => {
    void cronsStore.checkAdminSession();
  });

  let schedule = $derived(scheduleMode === 'preset' ? presetSchedule : customSchedule.trim());
  let targetReady = $derived(targetMode === 'project' ? !!project : !!repoParent.trim());
  let canSave = $derived(!!prompt.trim() && !!schedule && targetReady && !cronsStore.saving);
  let errorText = $derived(localError || cronsStore.error);

  function resetForm() {
    editing = null;
    prompt = '';
    scheduleMode = 'preset';
    presetSchedule = '0 14 * * *';
    customSchedule = '';
    targetMode = 'project';
    project = '';
    repoParent = '';
    enabled = true;
    localError = null;
  }

  function deriveName(text: string) {
    const firstLine = text.split('\n').map((line) => line.trim()).find(Boolean) || 'Scheduled job';
    return firstLine.length > 80 ? `${firstLine.slice(0, 77)}...` : firstLine;
  }

  function startCreate() {
    resetForm();
    showForm = true;
  }

  function startEdit(cron: AeCron) {
    const template = cron.task_template ?? {};
    editing = cron;
    prompt =
      (typeof template.description === 'string' && template.description) ||
      (typeof template.title === 'string' && template.title) ||
      cron.name;
    project = typeof template.project === 'string' ? template.project : '';
    repoParent = typeof template.repo_parent === 'string' ? template.repo_parent : '';
    targetMode = repoParent ? 'parent' : 'project';
    enabled = cron.enabled;

    const preset = presets.find((item) => item.value === cron.schedule);
    if (preset) {
      scheduleMode = 'preset';
      presetSchedule = preset.value;
      customSchedule = '';
    } else {
      scheduleMode = 'custom';
      customSchedule = cron.schedule;
    }

    localError = null;
    showForm = true;
  }

  async function saveCron(e: SubmitEvent) {
    e.preventDefault();
    if (!canSave) return;

    localError = null;
    const promptText = prompt.trim();
    const name = deriveName(promptText);
    const taskTemplate: Record<string, unknown> = {
      title: name,
      description: promptText,
      priority: 'medium',
      task_type: 'code'
    };

    if (targetMode === 'project') {
      taskTemplate.project = project;
    } else {
      taskTemplate.repo_parent = repoParent.trim();
    }

    const payload = {
      name,
      schedule,
      task_template: taskTemplate,
      enabled
    };

    const saved = editing
      ? await cronsStore.updateCron(editing.id, payload)
      : await cronsStore.createCron(payload);

    if (saved) {
      showForm = false;
      resetForm();
    }
  }

  async function toggleCron(cron: AeCron) {
    await cronsStore.toggleCron(cron.id);
  }

  async function deleteCron(cron: AeCron) {
    if (!confirm(`Delete "${cron.name}"?`)) return;
    deletingId = cron.id;
    const deleted = await cronsStore.deleteCron(cron.id);
    if (deleted && editing?.id === cron.id) {
      showForm = false;
      resetForm();
    }
    deletingId = null;
  }

  async function unlockAdmin(e: SubmitEvent) {
    e.preventDefault();
    localError = null;
    const unlocked = await cronsStore.unlockAdmin(adminKey);
    if (unlocked) adminKey = '';
  }

  function closeFromBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function humanSchedule(value: string) {
    return presets.find((item) => item.value === value)?.label ?? value;
  }

  function formatDate(value: string | null) {
    if (!value) return 'never';
    return new Date(value).toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    });
  }

  function targetLabel(cron: AeCron) {
    const template = cron.task_template ?? {};
    if (typeof template.repo_parent === 'string' && template.repo_parent) {
      const tail = template.repo_parent.split(/[\\/]/).filter(Boolean).pop() || template.repo_parent;
      return `${tail}/*`;
    }
    if (typeof template.project === 'string' && template.project) return template.project;
    if (typeof template.repo_url === 'string' && template.repo_url) return template.repo_url;
    if (typeof template.repo_path === 'string' && template.repo_path) {
      return template.repo_path.split(/[\\/]/).filter(Boolean).pop() || template.repo_path;
    }
    return 'No target';
  }
</script>

<div
  class="fixed inset-0 z-40 flex items-end justify-center bg-black/60 backdrop-blur-sm sm:items-center"
  data-no-egg
  onclick={closeFromBackdrop}
  onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <section
    class="max-h-[94dvh] w-full overflow-hidden rounded-t-3xl border border-white/10 bg-slate-900 shadow-2xl sm:max-w-4xl sm:rounded-3xl"
    role="document"
  >
    <header class="flex items-start gap-3 border-b border-white/10 bg-slate-900/95 px-4 py-3 backdrop-blur">
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2 text-[10px] font-black uppercase tracking-wide text-emerald-300">
          <span class="bob inline-block">⏱</span>
          Recurring Work
        </div>
        <h2 class="mt-1 text-lg font-semibold text-slate-100">Cron Jobs</h2>
      </div>
      {#if cronsStore.adminUnlocked}
        <button
          type="button"
          onclick={startCreate}
          class="rounded-lg border border-emerald-400/30 bg-emerald-400/10 px-3 py-2 text-xs font-black text-emerald-100 transition hover:-translate-y-0.5 hover:bg-emerald-400/20"
        >
          + New
        </button>
      {/if}
      <button
        type="button"
        onclick={onClose}
        class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-white/10 text-slate-200 transition hover:rotate-6 hover:bg-white/20"
        aria-label="close"
      >✕</button>
    </header>

    {#if cronsStore.checkingAdmin && !cronsStore.adminUnlocked}
      <div class="grid min-h-80 place-items-center p-6 text-sm text-slate-400">
        Unlocking...
      </div>
    {:else if !cronsStore.adminUnlocked}
      <form class="mx-auto max-w-md space-y-4 p-5" onsubmit={unlockAdmin}>
        <div class="rounded-2xl border border-white/10 bg-white/[0.04] p-4">
          <h3 class="text-sm font-black text-slate-100">Admin Unlock</h3>
          <p class="mt-2 text-xs leading-relaxed text-slate-400">
            Cron management can create recurring worker tasks, so it needs the Samwise admin key.
          </p>
          <input
            bind:value={adminKey}
            type="password"
            autocomplete="current-password"
            placeholder="Admin key"
            class="mt-4 w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-white/30 focus:outline-none"
          />
          {#if errorText}
            <div class="mt-3 rounded-xl border border-rose-500/40 bg-rose-500/10 p-3 text-xs font-semibold text-rose-100">
              {errorText}
            </div>
          {/if}
          <button
            type="submit"
            disabled={!adminKey.trim() || cronsStore.checkingAdmin}
            class="mt-4 w-full rounded-xl bg-emerald-500/90 px-3 py-2 text-sm font-black text-slate-950 transition hover:-translate-y-0.5 hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {cronsStore.checkingAdmin ? 'Checking...' : 'Unlock Schedules'}
          </button>
        </div>
      </form>
    {:else}
      <div class="grid max-h-[calc(94dvh-74px)] gap-0 overflow-y-auto lg:grid-cols-[minmax(0,1fr)_360px]">
      <div class="space-y-3 p-4">
        {#if errorText}
          <div class="rounded-xl border border-rose-500/40 bg-rose-500/10 p-3 text-xs font-semibold text-rose-100">
            {errorText}
          </div>
        {/if}

        {#if cronsStore.loadingCrons}
          <div class="grid min-h-52 place-items-center rounded-2xl border border-white/10 bg-white/5 text-sm text-slate-400">
            Loading schedules...
          </div>
        {:else if cronsStore.crons.length === 0}
          <div class="grid min-h-52 place-items-center rounded-2xl border border-dashed border-white/15 bg-white/5 p-6 text-center">
            <div>
              <div class="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl bg-emerald-400/10 text-2xl bob">⏱</div>
              <div class="text-sm font-semibold text-slate-100">No cron jobs yet</div>
              <button
                type="button"
                onclick={startCreate}
                class="mt-4 rounded-lg bg-emerald-500/90 px-4 py-2 text-sm font-black text-slate-950 transition hover:-translate-y-0.5 hover:bg-emerald-400"
              >
                Create one
              </button>
            </div>
          </div>
        {:else}
          {#each cronsStore.crons as cron (cron.id)}
            <article class="rounded-2xl border border-white/10 bg-white/[0.04] p-3 transition hover:-translate-y-0.5 hover:border-white/20 hover:bg-white/[0.07]">
              <div class="flex items-start gap-3">
                <button
                  type="button"
                  onclick={() => toggleCron(cron)}
                  class="mt-1 h-5 w-9 rounded-full border transition {cron.enabled ? 'border-emerald-300/50 bg-emerald-400/25' : 'border-white/15 bg-white/10'}"
                  aria-label={cron.enabled ? 'Disable cron' : 'Enable cron'}
                  title={cron.enabled ? 'Disable cron' : 'Enable cron'}
                >
                  <span class="block h-4 w-4 rounded-full bg-white shadow transition {cron.enabled ? 'translate-x-4' : 'translate-x-0.5'}"></span>
                </button>
                <div class="min-w-0 flex-1">
                  <h3 class="break-words text-sm font-semibold text-slate-100">{cron.name}</h3>
                  <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-slate-400">
                    <span class="rounded-md border border-sky-300/20 bg-sky-400/10 px-2 py-0.5 font-mono text-sky-100">
                      {humanSchedule(cron.schedule)}
                    </span>
                    <span class="rounded-md border border-indigo-300/20 bg-indigo-400/10 px-2 py-0.5 font-mono text-indigo-100">
                      {targetLabel(cron)}
                    </span>
                    <span>Next: {formatDate(cron.next_run)}</span>
                    <span>Last: {formatDate(cron.last_run)}</span>
                  </div>
                </div>
              </div>
              <div class="mt-3 flex justify-end gap-2">
                <button
                  type="button"
                  onclick={() => startEdit(cron)}
                  class="rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-semibold text-slate-200 transition hover:scale-105 hover:bg-white/10"
                >
                  Edit
                </button>
                <button
                  type="button"
                  onclick={() => deleteCron(cron)}
                  disabled={deletingId === cron.id}
                  class="rounded-lg border border-rose-300/25 bg-rose-500/10 px-3 py-1.5 text-xs font-semibold text-rose-100 transition hover:scale-105 hover:bg-rose-500/20 disabled:cursor-wait disabled:opacity-60"
                >
                  {deletingId === cron.id ? 'Deleting...' : 'Delete'}
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>

      <aside class="border-t border-white/10 bg-black/15 p-4 lg:border-l lg:border-t-0">
        {#if showForm}
          <form class="space-y-3" onsubmit={saveCron}>
            <div>
              <h3 class="text-sm font-black text-slate-100">{editing ? 'Edit Schedule' : 'New Schedule'}</h3>
              <p class="mt-1 text-xs leading-relaxed text-slate-400">
                Presets are Eastern Time. Custom cron expressions use UTC.
              </p>
            </div>

            <label class="block text-xs font-semibold text-slate-400">
              Prompt
              <textarea
                bind:value={prompt}
                rows="5"
                placeholder="What should Sam do on this schedule?"
                class="mt-1 w-full resize-y rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm leading-relaxed text-slate-100 placeholder:text-slate-500 focus:border-white/30 focus:outline-none"
              ></textarea>
            </label>

            <div class="space-y-2">
              <div class="text-xs font-semibold text-slate-400">Repo Target</div>
              <div class="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onclick={() => targetMode = 'project'}
                  class="rounded-lg border px-3 py-2 text-xs font-black transition hover:scale-[1.02] {targetMode === 'project' ? 'border-emerald-300/40 bg-emerald-400/15 text-emerald-100' : 'border-white/10 bg-white/5 text-slate-300'}"
                >
                  Project
                </button>
                <button
                  type="button"
                  onclick={() => targetMode = 'parent'}
                  class="rounded-lg border px-3 py-2 text-xs font-black transition hover:scale-[1.02] {targetMode === 'parent' ? 'border-emerald-300/40 bg-emerald-400/15 text-emerald-100' : 'border-white/10 bg-white/5 text-slate-300'}"
                >
                  Folder
                </button>
              </div>
              {#if targetMode === 'project'}
                <select
                  bind:value={project}
                  disabled={cronsStore.loadingProjects}
                  class="w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-100 focus:border-white/30 focus:outline-none disabled:opacity-60"
                >
                  <option value="">Select a project...</option>
                  {#each cronsStore.projects as item}
                    <option value={item.name}>{item.client ? `${item.client} · ${item.name}` : item.name}</option>
                  {/each}
                </select>
              {:else}
                <input
                  bind:value={repoParent}
                  placeholder="/Users/mjohnst/samwise/Personal-Apps"
                  class="w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 font-mono text-xs text-slate-100 placeholder:text-slate-500 focus:border-white/30 focus:outline-none"
                />
                <p class="text-[11px] leading-relaxed text-slate-500">
                  The worker creates one task per direct git repo inside this folder.
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <div class="text-xs font-semibold text-slate-400">Schedule</div>
              <div class="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onclick={() => scheduleMode = 'preset'}
                  class="rounded-lg border px-3 py-2 text-xs font-black transition hover:scale-[1.02] {scheduleMode === 'preset' ? 'border-sky-300/40 bg-sky-400/15 text-sky-100' : 'border-white/10 bg-white/5 text-slate-300'}"
                >
                  Presets
                </button>
                <button
                  type="button"
                  onclick={() => scheduleMode = 'custom'}
                  class="rounded-lg border px-3 py-2 text-xs font-black transition hover:scale-[1.02] {scheduleMode === 'custom' ? 'border-sky-300/40 bg-sky-400/15 text-sky-100' : 'border-white/10 bg-white/5 text-slate-300'}"
                >
                  Custom
                </button>
              </div>
              {#if scheduleMode === 'preset'}
                <div class="grid grid-cols-1 gap-1.5 sm:grid-cols-2 lg:grid-cols-1">
                  {#each presets as preset}
                    <button
                      type="button"
                      onclick={() => presetSchedule = preset.value}
                      class="rounded-lg border px-3 py-2 text-left text-xs font-semibold transition hover:translate-x-1 {presetSchedule === preset.value ? 'border-sky-300/40 bg-sky-400/15 text-sky-100' : 'border-white/10 bg-white/5 text-slate-300'}"
                    >
                      {preset.label}
                    </button>
                  {/each}
                </div>
              {:else}
                <input
                  bind:value={customSchedule}
                  placeholder="*/5 * * * *"
                  class="w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 font-mono text-xs text-slate-100 placeholder:text-slate-500 focus:border-white/30 focus:outline-none"
                />
                <p class="text-[11px] leading-relaxed text-slate-500">
                  Format: minute hour day month weekday.
                </p>
              {/if}
            </div>

            <label class="flex items-center justify-between rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-slate-200">
              <span>Enabled</span>
              <input type="checkbox" bind:checked={enabled} class="h-4 w-4 accent-emerald-400" />
            </label>

            <div class="flex gap-2 pt-1">
              <button
                type="button"
                onclick={() => { showForm = false; resetForm(); }}
                class="flex-1 rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm font-semibold text-slate-200 transition hover:bg-white/10"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!canSave}
                class="flex-1 rounded-xl bg-emerald-500/90 px-3 py-2 text-sm font-black text-slate-950 transition hover:-translate-y-0.5 hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {cronsStore.saving ? 'Saving...' : editing ? 'Update' : 'Create'}
              </button>
            </div>
          </form>
        {:else}
          <div class="grid min-h-64 place-items-center rounded-2xl border border-white/10 bg-white/5 p-5 text-center">
            <div>
              <div class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-2xl bg-sky-400/10 text-xl bob">⌁</div>
              <div class="text-sm font-semibold text-slate-100">Pick a schedule or make a new one</div>
              <p class="mt-2 text-xs leading-relaxed text-slate-400">
                Cron jobs create queued Sam tasks automatically while the worker is online.
              </p>
            </div>
          </div>
        {/if}
      </aside>
      </div>
    {/if}
  </section>
</div>
