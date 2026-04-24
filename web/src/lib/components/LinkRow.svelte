<script lang="ts">
  let {
    label,
    url,
    icon,
    colorClass,
    hoverClass,
  }: {
    label: string;
    url: string;
    icon: string;
    colorClass: string;
    hoverClass: string;
  } = $props();

  let justCopied = $state(false);

  async function copyUrl(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(url);
      } else {
        // Fallback for older mobile browsers
        const ta = document.createElement('textarea');
        ta.value = url;
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.select();
        document.execCommand('copy');
        document.body.removeChild(ta);
      }
      justCopied = true;
      setTimeout(() => (justCopied = false), 1400);
    } catch (err) {
      console.warn('[LinkRow] copy failed', err);
    }
  }
</script>

<div class="flex items-stretch gap-2">
  <a
    href={url}
    target="_blank"
    rel="noopener"
    class="flex-1 min-w-0 flex items-center gap-2 rounded-lg border px-3 py-2 transition {colorClass} {hoverClass}"
  >
    <span aria-hidden="true">{icon}</span>
    <span class="font-semibold shrink-0">{label}</span>
    <span class="text-[11px] opacity-70 truncate flex-1 text-left select-all" style="user-select: all; -webkit-user-select: all;">{url}</span>
  </a>
  <button
    type="button"
    onclick={copyUrl}
    aria-label={`Copy ${label} URL`}
    class="rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 active:bg-white/15 text-slate-200 px-3 py-2 text-xs font-semibold shrink-0 transition"
  >
    {justCopied ? '✓ Copied' : 'Copy'}
  </button>
</div>
