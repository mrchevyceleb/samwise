<script lang="ts">
  import { getGitStore, type GitFileStatus } from '$lib/stores/git.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';

  const git = getGitStore();
  const workspace = getWorkspace();

  let hoveredFile = $state<string | null>(null);
  let stageAllHovered = $state(false);
  let unstageAllHovered = $state(false);

  let stagedFiles = $derived((git.status?.files || []).filter(f => f.staged));
  let unstagedFiles = $derived((git.status?.files || []).filter(f => !f.staged));

  function statusColor(status: string): string {
    switch (status) {
      case 'M': return 'var(--accent-orange)';
      case 'A': return 'var(--accent-green)';
      case 'D': return 'var(--accent-red)';
      case '?': return 'var(--text-muted)';
      case 'R': return 'var(--accent-blue)';
      default: return 'var(--text-secondary)';
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'M': return 'Modified';
      case 'A': return 'Added';
      case 'D': return 'Deleted';
      case '?': return 'Untracked';
      case 'R': return 'Renamed';
      case 'U': return 'Conflicted';
      default: return status;
    }
  }

  function fileName(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1] || path;
  }

  function filePath(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/');
    if (parts.length > 1) {
      return parts.slice(0, -1).join('/') + '/';
    }
    return '';
  }

  async function handleStage(file: GitFileStatus) {
    if (workspace.path) await git.stageFile(workspace.path, file.path);
  }

  async function handleUnstage(file: GitFileStatus) {
    if (workspace.path) await git.unstageFile(workspace.path, file.path);
  }

  async function handleDiscard(file: GitFileStatus) {
    if (workspace.path && file.status !== '?') {
      await git.discardFile(workspace.path, file.path);
    }
  }

  async function handleStageAll() {
    if (workspace.path) await git.stageAll(workspace.path);
  }

  async function handleUnstageAll() {
    if (workspace.path) await git.unstageAll(workspace.path);
  }

  function handleFileClick(file: GitFileStatus) {
    git.selectedFile = file.path;
    if (workspace.path) {
      git.getDiff(workspace.path, file.path, file.staged);
    }
  }
</script>

{#if stagedFiles.length > 0}
  <div style="padding: 4px 0;">
    <div style="display: flex; align-items: center; padding: 4px 12px; gap: 6px;">
      <span style="font-size: 11px; font-weight: 600; color: var(--text-secondary); flex: 1; text-transform: uppercase; letter-spacing: 0.05em;">Staged ({stagedFiles.length})</span>
      <button
        style="display: flex; align-items: center; justify-content: center; width: 20px; height: 20px; border: none; border-radius: 3px; cursor: pointer; transition: all 0.12s ease; background: {unstageAllHovered ? 'rgba(248, 81, 73, 0.15)' : 'transparent'}; color: {unstageAllHovered ? 'var(--accent-red)' : 'var(--text-muted)'};"
        onclick={handleUnstageAll}
        onmouseenter={() => unstageAllHovered = true}
        onmouseleave={() => unstageAllHovered = false}
        title="Unstage All"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
    </div>
    {#each stagedFiles as file (file.path)}
      <button
        style="display: flex; align-items: center; width: 100%; padding: 3px 12px 3px 20px; border: none; cursor: pointer; font-size: 12px; font-family: var(--font-ui); gap: 6px; transition: background 0.1s ease; background: {hoveredFile === 's:' + file.path ? 'var(--bg-elevated)' : git.selectedFile === file.path ? 'rgba(255, 214, 10, 0.06)' : 'transparent'}; color: var(--text-primary);"
        onclick={() => handleFileClick(file)}
        onmouseenter={() => hoveredFile = 's:' + file.path}
        onmouseleave={() => hoveredFile = null}
      >
        <span style="width: 16px; height: 16px; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700; color: {statusColor(file.status)}; flex-shrink: 0;">{file.status}</span>
        <span style="color: var(--text-muted); font-size: 11px; flex-shrink: 0;">{filePath(file.path)}</span>
        <span style="color: var(--text-primary); flex: 1; text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{fileName(file.path)}</span>
        {#if hoveredFile === 's:' + file.path}
          <span
            style="display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 3px; flex-shrink: 0; transition: background 0.1s; color: var(--accent-red);"
            onclick={(e) => { e.stopPropagation(); handleUnstage(file); }}
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248, 81, 73, 0.15)'; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            role="button"
            tabindex="0"
            title="Unstage"
            onkeydown={() => {}}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </span>
        {/if}
      </button>
    {/each}
  </div>
{/if}

{#if unstagedFiles.length > 0}
  <div style="padding: 4px 0;">
    <div style="display: flex; align-items: center; padding: 4px 12px; gap: 6px;">
      <span style="font-size: 11px; font-weight: 600; color: var(--text-secondary); flex: 1; text-transform: uppercase; letter-spacing: 0.05em;">Changes ({unstagedFiles.length})</span>
      <button
        style="display: flex; align-items: center; justify-content: center; width: 20px; height: 20px; border: none; border-radius: 3px; cursor: pointer; transition: all 0.12s ease; background: {stageAllHovered ? 'rgba(63, 185, 80, 0.15)' : 'transparent'}; color: {stageAllHovered ? 'var(--accent-green)' : 'var(--text-muted)'};"
        onclick={handleStageAll}
        onmouseenter={() => stageAllHovered = true}
        onmouseleave={() => stageAllHovered = false}
        title="Stage All"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
    </div>
    {#each unstagedFiles as file (file.path)}
      <button
        style="display: flex; align-items: center; width: 100%; padding: 3px 12px 3px 20px; border: none; cursor: pointer; font-size: 12px; font-family: var(--font-ui); gap: 6px; transition: background 0.1s ease; background: {hoveredFile === 'u:' + file.path ? 'var(--bg-elevated)' : git.selectedFile === file.path ? 'rgba(255, 214, 10, 0.06)' : 'transparent'}; color: var(--text-primary);"
        onclick={() => handleFileClick(file)}
        onmouseenter={() => hoveredFile = 'u:' + file.path}
        onmouseleave={() => hoveredFile = null}
      >
        <span style="width: 16px; height: 16px; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700; color: {statusColor(file.status)}; flex-shrink: 0;">{file.status}</span>
        <span style="color: var(--text-muted); font-size: 11px; flex-shrink: 0;">{filePath(file.path)}</span>
        <span style="color: var(--text-primary); flex: 1; text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{fileName(file.path)}</span>
        {#if hoveredFile === 'u:' + file.path}
          <span
            style="display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 3px; flex-shrink: 0; transition: background 0.1s; color: var(--accent-green);"
            onclick={(e) => { e.stopPropagation(); handleStage(file); }}
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(63, 185, 80, 0.15)'; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            role="button"
            tabindex="0"
            title="Stage"
            onkeydown={() => {}}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </span>
          {#if file.status !== '?'}
            <span
              style="display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 3px; flex-shrink: 0; transition: background 0.1s; color: var(--accent-red);"
              onclick={(e) => { e.stopPropagation(); handleDiscard(file); }}
              onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248, 81, 73, 0.15)'; }}
              onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              role="button"
              tabindex="0"
              title="Discard Changes"
              onkeydown={() => {}}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/></svg>
            </span>
          {/if}
        {/if}
      </button>
    {/each}
  </div>
{/if}

{#if stagedFiles.length === 0 && unstagedFiles.length === 0}
  <div style="display: flex; align-items: center; justify-content: center; padding: 32px 24px;">
    <div style="text-align: center;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--accent-green)" stroke-width="1.5" style="margin: 0 auto 8px; opacity: 0.5;">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
      <span style="color: var(--text-muted); font-size: 12px; display: block;">No changes</span>
      <span style="color: var(--text-muted); font-size: 11px; display: block; margin-top: 2px; opacity: 0.7;">Working tree clean</span>
    </div>
  </div>
{/if}
