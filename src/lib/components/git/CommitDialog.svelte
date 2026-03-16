<script lang="ts">
  import { getGitStore } from '$lib/stores/git.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';

  const git = getGitStore();
  const workspace = getWorkspace();

  let message = $state('');
  let committing = $state(false);
  let commitBtnHovered = $state(false);
  let inputFocused = $state(false);
  let generating = $state(false);
  let aiBtnHovered = $state(false);
  let aiError = $state<string | null>(null);
  let commitError = $state<string | null>(null);

  let stagedCount = $derived((git.status?.files || []).filter(f => f.staged).length);
  let canCommit = $derived(message.trim().length > 0 && stagedCount > 0 && !committing);
  let canGenerate = $derived(stagedCount > 0 && !generating && !committing);

  async function handleCommit() {
    if (!canCommit || !workspace.path) return;
    committing = true;
    commitError = null;
    try {
      await git.commit(workspace.path, message.trim());
      message = '';
    } catch (e: any) {
      commitError = e.message || String(e);
      setTimeout(() => commitError = null, 6000);
    } finally {
      committing = false;
    }
  }

  async function handleGenerateMessage() {
    if (!canGenerate || !workspace.path) return;
    generating = true;
    aiError = null;
    try {
      message = await git.generateCommitMessage(workspace.path);
    } catch (e: any) {
      aiError = e.message || String(e);
      setTimeout(() => aiError = null, 5000);
    } finally {
      generating = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && canCommit) {
      e.preventDefault();
      handleCommit();
    }
  }
</script>

<div style="padding: 8px 12px; border-bottom: 1px solid var(--border-default);">
  <div style="position: relative;">
    <textarea
      bind:value={message}
      onkeydown={handleKeyDown}
      onfocus={() => inputFocused = true}
      onblur={() => inputFocused = false}
      placeholder="Commit message..."
      rows="3"
      style="width: 100%; padding: 8px 10px; background: var(--bg-primary); border: 1px solid {inputFocused ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-ui); resize: vertical; min-height: 60px; outline: none; transition: border-color 0.15s ease; box-shadow: {inputFocused ? '0 0 0 2px color-mix(in srgb, var(--accent-primary) 15%, transparent)' : 'none'};"
    ></textarea>

    <!-- AI generate button (top-right of textarea) -->
    <button
      onclick={handleGenerateMessage}
      disabled={!canGenerate}
      onmouseenter={() => aiBtnHovered = true}
      onmouseleave={() => aiBtnHovered = false}
      title={stagedCount === 0 ? 'Stage files first' : 'Generate commit message with AI'}
      style="position: absolute; top: 6px; right: 6px; display: flex; align-items: center; justify-content: center; width: 26px; height: 26px; border: none; border-radius: 5px; cursor: {canGenerate ? 'pointer' : 'not-allowed'}; transition: all 0.2s ease; background: {generating ? 'var(--accent-purple)' : aiBtnHovered && canGenerate ? 'rgba(203, 166, 247, 0.2)' : 'rgba(203, 166, 247, 0.08)'}; color: {canGenerate ? (aiBtnHovered ? 'var(--accent-purple)' : 'rgba(203, 166, 247, 0.7)') : 'var(--text-muted)'}; transform: {aiBtnHovered && canGenerate ? 'scale(1.1)' : 'scale(1)'}; opacity: {canGenerate ? 1 : 0.4};"
    >
      {#if generating}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" style="animation: spin 1s linear infinite;">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
        </svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 0L13.5 8.5L22 10L13.5 11.5L12 20L10.5 11.5L2 10L10.5 8.5L12 0Z"/>
          <path d="M20 14L20.75 17.25L24 18L20.75 18.75L20 22L19.25 18.75L16 18L19.25 17.25L20 14Z" opacity="0.6"/>
        </svg>
      {/if}
    </button>
  </div>

  {#if aiError || commitError}
    <div style="font-size: 10px; color: var(--accent-red); margin-top: 4px; padding: 4px 6px; background: rgba(243, 139, 168, 0.1); border-radius: 4px;">
      {aiError || commitError}
    </div>
  {/if}

  <div style="display: flex; align-items: center; gap: 8px; margin-top: 6px;">
    <button
      onclick={handleCommit}
      disabled={!canCommit}
      onmouseenter={() => commitBtnHovered = true}
      onmouseleave={() => commitBtnHovered = false}
      style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 6px 12px; border: none; border-radius: 6px; cursor: {canCommit ? 'pointer' : 'not-allowed'}; font-size: 12px; font-weight: 600; font-family: var(--font-ui); transition: all 0.15s ease; background: {canCommit ? (commitBtnHovered ? 'var(--accent-hover)' : 'var(--accent-primary)') : 'var(--bg-elevated)'}; color: {canCommit ? '#0D1117' : 'var(--text-muted)'}; transform: {commitBtnHovered && canCommit ? 'translateY(-1px)' : 'translateY(0)'}; box-shadow: {commitBtnHovered && canCommit ? '0 4px 12px color-mix(in srgb, var(--accent-primary) 30%, transparent)' : 'none'};"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
      {committing ? 'Committing...' : `Commit (${stagedCount})`}
    </button>
  </div>
  <div style="font-size: 10px; color: var(--text-muted); margin-top: 4px; text-align: center;">
    Ctrl+Enter to commit
  </div>
</div>

<style>
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
