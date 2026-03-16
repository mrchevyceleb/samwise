<script lang="ts">
  import { getGitStore } from '$lib/stores/git.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';

  const git = getGitStore();
  const workspace = getWorkspace();

  let open = $state(false);
  let hovered = $state(false);
  let hoveredBranch = $state<string | null>(null);
  let newBranchMode = $state(false);
  let newBranchName = $state('');

  let currentBranch = $derived(git.status?.branch || 'main');
  let localBranches = $derived(git.branches.filter(b => !b.is_remote));

  function toggle() {
    open = !open;
    newBranchMode = false;
    newBranchName = '';
  }

  async function handleCheckout(name: string) {
    if (!workspace.path) return;
    try {
      await git.checkout(workspace.path, name);
    } catch (e) {
      console.error('Checkout failed:', e);
    }
    open = false;
  }

  async function handleCreateBranch() {
    if (!workspace.path || !newBranchName.trim()) return;
    try {
      await git.createBranch(workspace.path, newBranchName.trim());
    } catch (e) {
      console.error('Create branch failed:', e);
    }
    newBranchMode = false;
    newBranchName = '';
    open = false;
  }
</script>

<div style="position: relative; padding: 4px 12px;">
  <button
    onclick={toggle}
    onmouseenter={() => hovered = true}
    onmouseleave={() => hovered = false}
    style="display: flex; align-items: center; gap: 6px; width: 100%; padding: 5px 10px; border: 1px solid {hovered ? 'var(--accent-dim)' : 'var(--border-default)'}; border-radius: 6px; cursor: pointer; font-size: 12px; font-family: var(--font-mono); transition: all 0.15s ease; background: {hovered ? 'var(--bg-elevated)' : 'var(--bg-primary)'}; color: var(--text-primary);"
  >
    <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="color: var(--accent-orange); flex-shrink: 0;">
      <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.492 2.492 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25z"/>
    </svg>
    <span style="flex: 1; text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{currentBranch}</span>
    {#if git.status}
      {#if git.status.ahead > 0}
        <span style="font-size: 10px; color: var(--accent-green);">{git.status.ahead}↑</span>
      {/if}
      {#if git.status.behind > 0}
        <span style="font-size: 10px; color: var(--accent-blue);">{git.status.behind}↓</span>
      {/if}
    {/if}
    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="transform: {open ? 'rotate(180deg)' : 'rotate(0)'}; transition: transform 0.15s ease;">
      <polyline points="6 9 12 15 18 9"/>
    </svg>
  </button>

  {#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      style="position: fixed; inset: 0; z-index: 99;"
      onclick={toggle}
      onkeydown={() => {}}
    ></div>
    <div style="position: absolute; top: 100%; left: 12px; right: 12px; z-index: 100; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 8px; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4); overflow: hidden; margin-top: 4px; max-height: 240px; overflow-y: auto;">
      {#each localBranches as branch (branch.name)}
        <button
          style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 6px 12px; border: none; cursor: pointer; font-size: 12px; font-family: var(--font-mono); transition: background 0.1s ease; background: {hoveredBranch === branch.name ? 'var(--bg-surface)' : 'transparent'}; color: {branch.name === currentBranch ? 'var(--accent-primary)' : 'var(--text-primary)'};"
          onclick={() => handleCheckout(branch.name)}
          onmouseenter={() => hoveredBranch = branch.name}
          onmouseleave={() => hoveredBranch = null}
        >
          {#if branch.name === currentBranch}
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
          {:else}
            <span style="width: 12px;"></span>
          {/if}
          <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{branch.name}</span>
        </button>
      {/each}

      <div style="border-top: 1px solid var(--border-default); padding: 4px;">
        {#if newBranchMode}
          <div style="display: flex; gap: 4px; padding: 4px;">
            <input
              bind:value={newBranchName}
              placeholder="branch-name"
              onkeydown={(e) => { if (e.key === 'Enter') handleCreateBranch(); if (e.key === 'Escape') { newBranchMode = false; } }}
              style="flex: 1; padding: 4px 8px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 4px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
            />
            <button
              onclick={handleCreateBranch}
              style="padding: 4px 8px; background: var(--accent-primary); border: none; border-radius: 4px; color: #0D1117; font-size: 11px; font-weight: 600; cursor: pointer;"
            >Create</button>
          </div>
        {:else}
          <button
            style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 6px 12px; border: none; cursor: pointer; font-size: 12px; color: var(--accent-primary); background: transparent; font-family: var(--font-ui); transition: background 0.1s ease;"
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-surface)'; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            onclick={() => newBranchMode = true}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            New Branch
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
