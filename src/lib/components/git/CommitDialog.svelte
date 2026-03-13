<script lang="ts">
  import { getGitStore } from '$lib/stores/git.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';

  const git = getGitStore();
  const workspace = getWorkspace();

  let message = $state('');
  let committing = $state(false);
  let commitBtnHovered = $state(false);
  let inputFocused = $state(false);

  let stagedCount = $derived((git.status?.files || []).filter(f => f.staged).length);
  let canCommit = $derived(message.trim().length > 0 && stagedCount > 0 && !committing);

  async function handleCommit() {
    if (!canCommit || !workspace.path) return;
    committing = true;
    try {
      await git.commit(workspace.path, message.trim());
      message = '';
    } catch (e) {
      console.error('Commit failed:', e);
    } finally {
      committing = false;
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
      style="width: 100%; padding: 8px 10px; background: var(--bg-primary); border: 1px solid {inputFocused ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-ui); resize: vertical; min-height: 60px; outline: none; transition: border-color 0.15s ease; box-shadow: {inputFocused ? '0 0 0 2px rgba(255, 214, 10, 0.15)' : 'none'};"
    ></textarea>
  </div>
  <div style="display: flex; align-items: center; gap: 8px; margin-top: 6px;">
    <button
      onclick={handleCommit}
      disabled={!canCommit}
      onmouseenter={() => commitBtnHovered = true}
      onmouseleave={() => commitBtnHovered = false}
      style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 6px 12px; border: none; border-radius: 6px; cursor: {canCommit ? 'pointer' : 'not-allowed'}; font-size: 12px; font-weight: 600; font-family: var(--font-ui); transition: all 0.15s ease; background: {canCommit ? (commitBtnHovered ? 'var(--banana-yellow-hover)' : 'var(--banana-yellow)') : 'var(--bg-elevated)'}; color: {canCommit ? '#0D1117' : 'var(--text-muted)'}; transform: {commitBtnHovered && canCommit ? 'translateY(-1px)' : 'translateY(0)'}; box-shadow: {commitBtnHovered && canCommit ? '0 4px 12px rgba(255, 214, 10, 0.3)' : 'none'};"
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
