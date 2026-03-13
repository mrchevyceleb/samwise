<script lang="ts">
  import { getLayout } from '$lib/stores/layout.svelte';
  import { getSettingsStore } from '$lib/stores/settings.svelte';
  import { getTerminals } from '$lib/stores/terminals.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';

  interface Props {
    visible?: boolean;
    onClose?: () => void;
  }

  let { visible = false, onClose }: Props = $props();

  const layout = getLayout();
  const settingsStore = getSettingsStore();
  const terminals = getTerminals();
  const workspace = getWorkspace();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  interface Command {
    id: string;
    label: string;
    shortcut?: string;
    category: string;
    action: () => void;
  }

  const allCommands: Command[] = [
    {
      id: 'toggle-terminal',
      label: 'Toggle Terminal',
      shortcut: 'Ctrl+`',
      category: 'View',
      action: () => { layout.toggleTerminal(); close(); },
    },
    {
      id: 'new-terminal',
      label: 'New Terminal',
      category: 'Terminal',
      action: () => {
        const cwd = workspace.path || '';
        terminals.add(cwd);
        layout.terminalVisible = true;
        close();
      },
    },
    {
      id: 'settings',
      label: 'Open Settings',
      shortcut: 'Ctrl+,',
      category: 'Preferences',
      action: () => { settingsStore.settingsVisible = true; close(); },
    },
    {
      id: 'ai-settings',
      label: 'AI Model Settings',
      category: 'Preferences',
      action: () => { settingsStore.settingsVisible = true; close(); },
    },
    {
      id: 'toggle-file-panel',
      label: 'Toggle File Explorer',
      category: 'View',
      action: () => { close(); },
    },
    {
      id: 'refresh-preview',
      label: 'Refresh Preview',
      category: 'Preview',
      action: () => { close(); },
    },
    {
      id: 'git-commit',
      label: 'Git: Commit',
      category: 'Git',
      action: () => { close(); },
    },
    {
      id: 'git-push',
      label: 'Git: Push',
      category: 'Git',
      action: () => { close(); },
    },
    {
      id: 'git-pull',
      label: 'Git: Pull',
      category: 'Git',
      action: () => { close(); },
    },
    {
      id: 'git-stash',
      label: 'Git: Stash',
      category: 'Git',
      action: () => { close(); },
    },
  ];

  let filteredCommands = $derived(() => {
    if (!query.trim()) return allCommands;
    const q = query.toLowerCase();
    return allCommands.filter(cmd =>
      cmd.label.toLowerCase().includes(q) ||
      cmd.category.toLowerCase().includes(q)
    );
  });

  function close() {
    query = '';
    selectedIndex = 0;
    onClose?.();
  }

  function handleKeyDown(e: KeyboardEvent) {
    const cmds = filteredCommands();
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, cmds.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (cmds[selectedIndex]) {
        cmds[selectedIndex].action();
      }
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  // Reset selection when query changes
  $effect(() => {
    const _q = query;
    selectedIndex = 0;
  });

  // Focus input on open
  $effect(() => {
    if (visible && inputEl) {
      setTimeout(() => inputEl?.focus(), 50);
    }
  });
</script>

{#if visible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    onclick={handleOverlayClick}
    style="position: fixed; inset: 0; z-index: 1100; background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(2px); display: flex; justify-content: center; padding-top: 15vh;"
  >
    <div style="width: 560px; max-height: 400px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 12px; box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5); overflow: hidden; display: flex; flex-direction: column;">
      <!-- Search input -->
      <div style="display: flex; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-default); gap: 10px;">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2" style="flex-shrink: 0;">
          <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={handleKeyDown}
          placeholder="Type a command..."
          style="flex: 1; background: none; border: none; outline: none; color: var(--text-primary); font-size: 14px; font-family: var(--font-ui);"
        />
        <span style="font-size: 10px; color: var(--text-muted); padding: 2px 6px; background: var(--bg-elevated); border-radius: 4px; font-family: var(--font-mono);">ESC</span>
      </div>

      <!-- Command list -->
      <div style="flex: 1; overflow-y: auto; padding: 4px;">
        {#each filteredCommands() as cmd, i (cmd.id)}
          <button
            onclick={() => cmd.action()}
            onmouseenter={() => selectedIndex = i}
            style="display: flex; align-items: center; width: 100%; padding: 8px 12px; border: none; border-radius: 6px; cursor: pointer; font-size: 13px; font-family: var(--font-ui); gap: 10px; transition: background 0.08s ease; background: {selectedIndex === i ? 'rgba(255, 214, 10, 0.08)' : 'transparent'}; color: {selectedIndex === i ? 'var(--text-primary)' : 'var(--text-secondary)'};"
          >
            <span style="flex: 1; text-align: left;">{cmd.label}</span>
            <span style="font-size: 10px; color: var(--text-muted); background: var(--bg-primary); padding: 2px 6px; border-radius: 3px;">{cmd.category}</span>
            {#if cmd.shortcut}
              <span style="font-size: 10px; color: var(--banana-yellow-dim); font-family: var(--font-mono);">{cmd.shortcut}</span>
            {/if}
          </button>
        {/each}

        {#if filteredCommands().length === 0}
          <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px;">
            No commands found
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
