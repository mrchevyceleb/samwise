<script lang="ts">
  import { getGitStore, type GitCommitInfo } from '$lib/stores/git.svelte';

  const git = getGitStore();

  let expanded = $state(true);
  let hoveredHash = $state<string | null>(null);
  let copiedHash = $state<string | null>(null);

  let commits = $derived(git.log || []);

  function timeAgo(timestamp: number): string {
    const seconds = Math.floor(Date.now() / 1000 - timestamp);
    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    if (seconds < 604800) return `${Math.floor(seconds / 86400)}d ago`;
    if (seconds < 2592000) return `${Math.floor(seconds / 604800)}w ago`;
    return new Date(timestamp * 1000).toLocaleDateString();
  }

  function getInitials(name: string): string {
    return name.split(' ').map(w => w[0] || '').join('').toUpperCase().slice(0, 2);
  }

  function getAvatarColor(name: string): string {
    const colors = [
      '#f38ba8', '#fab387', '#f9e2af', '#a6e3a1',
      '#89dceb', '#74c7ec', '#89b4fa', '#cba6f7',
      '#f5c2e7', '#94e2d5', '#b4befe', '#eba0ac',
    ];
    let hash = 0;
    for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
    return colors[Math.abs(hash) % colors.length];
  }

  async function copyHash(hash: string) {
    try {
      await navigator.clipboard.writeText(hash);
      copiedHash = hash;
      setTimeout(() => copiedHash = null, 1500);
    } catch (e) {
      console.warn('Failed to copy hash:', e);
    }
  }
</script>

<div style="border-top: 1px solid var(--border-default);">
  <!-- Header -->
  <button
    onclick={() => expanded = !expanded}
    style="display: flex; align-items: center; gap: 6px; width: 100%; padding: 8px 12px; border: none; background: transparent; cursor: pointer; color: var(--text-secondary); font-size: 11px; font-weight: 600; font-family: var(--font-ui); text-transform: uppercase; letter-spacing: 0.5px;"
  >
    <svg
      width="10" height="10" viewBox="0 0 24 24" fill="currentColor"
      style="transition: transform 0.15s ease; transform: {expanded ? 'rotate(90deg)' : 'rotate(0deg)'};"
    >
      <path d="M8 5l8 7-8 7z"/>
    </svg>
    Commits
    <span style="font-weight: 400; color: var(--text-muted); font-size: 10px;">({commits.length})</span>
  </button>

  {#if expanded}
    <div style="max-height: 400px; overflow-y: auto;">
      {#if commits.length === 0}
        <div style="padding: 16px 12px; text-align: center;">
          <span style="color: var(--text-muted); font-size: 11px;">No commits yet</span>
        </div>
      {:else}
        {#each commits as commit, i (commit.hash)}
          {@const isHovered = hoveredHash === commit.hash}
          <div
            onmouseenter={() => hoveredHash = commit.hash}
            onmouseleave={() => hoveredHash = null}
            style="display: flex; gap: 10px; padding: 6px 12px; cursor: default; transition: background 0.1s ease; background: {isHovered ? 'var(--bg-elevated)' : 'transparent'}; position: relative;"
          >
            <!-- Timeline line -->
            {#if i < commits.length - 1}
              <div style="position: absolute; left: 22px; top: 28px; bottom: -6px; width: 1px; background: var(--border-default);"></div>
            {/if}

            <!-- Avatar -->
            <div
              style="width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 8px; font-weight: 700; color: #0d1117; flex-shrink: 0; background: {getAvatarColor(commit.author)}; z-index: 1;"
              title={commit.author}
            >
              {getInitials(commit.author)}
            </div>

            <!-- Content -->
            <div style="flex: 1; min-width: 0;">
              <div style="font-size: 12px; color: var(--text-primary); line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title={commit.message}>
                {commit.message}
              </div>
              <div style="display: flex; align-items: center; gap: 6px; margin-top: 2px;">
                <button
                  onclick={() => copyHash(commit.hash)}
                  style="font-size: 10px; font-family: var(--font-mono, monospace); color: {copiedHash === commit.hash ? 'var(--accent-green)' : 'var(--accent-blue)'}; background: none; border: none; cursor: pointer; padding: 0; transition: color 0.15s ease;"
                  title="Copy full hash"
                >
                  {copiedHash === commit.hash ? 'copied!' : commit.short_hash}
                </button>
                <span style="font-size: 10px; color: var(--text-muted);">{commit.author.split(' ')[0]}</span>
                <span style="font-size: 10px; color: var(--text-muted);">{timeAgo(commit.timestamp)}</span>
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>
