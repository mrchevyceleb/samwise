<script lang="ts">
  import { getGitStore } from '$lib/stores/git.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';
  import GitStatus from './GitStatus.svelte';
  import CommitDialog from './CommitDialog.svelte';
  import BranchSelector from './BranchSelector.svelte';
  import CommitHistory from './CommitHistory.svelte';

  const git = getGitStore();
  const workspace = getWorkspace();

  let refreshHovered = $state(false);
  let pullHovered = $state(false);
  let pushHovered = $state(false);
  let stashHovered = $state(false);

  $effect(() => {
    if (workspace.path) {
      git.refresh(workspace.path);
    }
  });

  function handleRefresh() {
    if (workspace.path) git.refresh(workspace.path);
  }

  async function handlePush() {
    if (!workspace.path) return;
    try { await git.push(workspace.path); } catch (e) { console.error('Push failed:', e); }
  }

  async function handlePull() {
    if (!workspace.path) return;
    try { await git.pull(workspace.path); } catch (e) { console.error('Pull failed:', e); }
  }

  async function handleStash() {
    if (!workspace.path) return;
    try { await git.stash(workspace.path); } catch (e) { console.error('Stash failed:', e); }
  }
</script>

<div style="display: flex; flex-direction: column; height: 100%; background: var(--bg-surface); overflow: hidden;">
  <!-- Header -->
  <div style="display: flex; align-items: center; padding: 8px 12px; border-bottom: 1px solid var(--border-default); gap: 8px;">
    <svg width="14" height="14" viewBox="0 0 16 16" fill="var(--accent-orange)" style="flex-shrink: 0;">
      <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.492 2.492 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25z"/>
    </svg>
    <span style="font-size: 12px; font-weight: 600; color: var(--text-primary); flex: 1;">Source Control</span>

    <!-- Action buttons -->
    <button
      style="display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; background: {refreshHovered ? 'var(--bg-elevated)' : 'transparent'}; color: {refreshHovered ? 'var(--accent-primary)' : 'var(--text-muted)'}; transform: {refreshHovered ? 'rotate(180deg)' : 'rotate(0deg)'};"
      onclick={handleRefresh}
      onmouseenter={() => refreshHovered = true}
      onmouseleave={() => refreshHovered = false}
      title="Refresh"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
      </svg>
    </button>
    <button
      style="display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; background: {pullHovered ? 'var(--bg-elevated)' : 'transparent'}; color: {pullHovered ? 'var(--accent-blue)' : 'var(--text-muted)'};"
      onclick={handlePull}
      onmouseenter={() => pullHovered = true}
      onmouseleave={() => pullHovered = false}
      title="Pull"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19"/><polyline points="19 12 12 19 5 12"/>
      </svg>
    </button>
    <button
      style="display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; background: {pushHovered ? 'var(--bg-elevated)' : 'transparent'}; color: {pushHovered ? 'var(--accent-green)' : 'var(--text-muted)'};"
      onclick={handlePush}
      onmouseenter={() => pushHovered = true}
      onmouseleave={() => pushHovered = false}
      title="Push"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/>
      </svg>
    </button>
    <button
      style="display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; background: {stashHovered ? 'var(--bg-elevated)' : 'transparent'}; color: {stashHovered ? 'var(--accent-purple)' : 'var(--text-muted)'};"
      onclick={handleStash}
      onmouseenter={() => stashHovered = true}
      onmouseleave={() => stashHovered = false}
      title="Stash"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/>
      </svg>
    </button>
  </div>

  <!-- Branch selector -->
  <BranchSelector />

  {#if !workspace.isOpen}
    <div style="flex: 1; display: flex; align-items: center; justify-content: center; padding: 24px;">
      <span style="color: var(--text-muted); font-size: 12px; text-align: center;">Open a project folder to use Git</span>
    </div>
  {:else if git.loading}
    <div style="flex: 1; display: flex; align-items: center; justify-content: center;">
      <span style="color: var(--text-muted); font-size: 12px; animation: breathe-glow 2s ease-in-out infinite;">Loading...</span>
    </div>
  {:else if git.error}
    <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 24px; gap: 8px;">
      <span style="color: var(--accent-red); font-size: 12px;">Not a git repository</span>
      <span style="color: var(--text-muted); font-size: 11px; text-align: center;">Initialize with git init or open a repo</span>
    </div>
  {:else}
    <!-- Commit dialog -->
    <CommitDialog />

    <!-- Changed files + commit history -->
    <div style="flex: 1; overflow-y: auto;">
      <GitStatus />
      <CommitHistory />
    </div>
  {/if}
</div>
