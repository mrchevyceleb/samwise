<script lang="ts">
  type Uploaded = { url: string; name: string; mime: string };
  let { projects, onClose }: { projects: string[]; onClose: () => void } = $props();

  let title = $state('');
  let description = $state('');
  let project = $state('');
  let priority = $state<'critical' | 'high' | 'medium' | 'low'>('medium');
  let task_type = $state<'code' | 'research'>('code');
  let submitting = $state(false);
  let errorMsg = $state<string | null>(null);
  let uploading = $state(0);
  let attachments = $state<Uploaded[]>([]);

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
    if (!title.trim() || submitting || uploading > 0) return;
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
          task_type,
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

    <div>
      <label class="block text-xs text-slate-400 mb-1">
        Attachments (images, PDFs — helps Sam see the bug)
      </label>
      <label class="flex items-center gap-2 rounded-lg border border-dashed border-white/15 bg-white/5 px-3 py-3 text-sm text-slate-300 hover:bg-white/10 cursor-pointer transition">
        <span class="text-lg">📎</span>
        <span class="flex-1">
          {uploading > 0 ? `Uploading ${uploading}…` : 'Tap to attach files'}
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
              >✕</button>
            </li>
          {/each}
        </ul>
      {/if}
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
        disabled={submitting || uploading > 0 || !title.trim()}
        class="flex-1 rounded-lg bg-emerald-500/90 hover:bg-emerald-400 disabled:opacity-50 disabled:cursor-not-allowed px-3 py-2 text-sm font-medium text-slate-900 transition"
      >{submitting ? 'Sending…' : uploading > 0 ? 'Uploading…' : '🚀 Queue it'}</button>
    </div>
  </form>
</div>
