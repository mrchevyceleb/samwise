<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { getTerminals } from '$lib/stores/terminals.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';
  import { getSettings } from '$lib/stores/settings.svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  const terminals = getTerminals();
  const workspace = getWorkspace();
  const settings = getSettings();

  let xtermMap: Map<string, Terminal> = new Map();
  let fitMap: Map<string, FitAddon> = new Map();
  let spawnedIds: Set<string> = new Set();
  let containerEl: HTMLDivElement;
  let unlistenOutput: (() => void) | null = null;
  let unlistenClosed: (() => void) | null = null;

  async function writeClipboardText(text: string) {
    try { await navigator.clipboard.writeText(text); } catch {}
  }

  async function readClipboardText(): Promise<string> {
    try { return await navigator.clipboard.readText(); } catch { return ''; }
  }

  function handleNewTerminal() {
    const cwd = workspace.path || '';
    terminals.add(cwd);
  }

  async function handleCloseTerminal(id: string) {
    try { const { invoke } = await import('@tauri-apps/api/core'); await invoke('kill_terminal', { id }); } catch {}
    terminals.clearBuffer(id);
    spawnedIds.delete(id);
    const term = xtermMap.get(id);
    if (term) {
      term.dispose();
      xtermMap.delete(id);
      fitMap.delete(id);
    }
    terminals.remove(id);
  }

  async function initTerminal(id: string, el: HTMLDivElement) {
    if (xtermMap.has(id)) return;
    const { invoke } = await import('@tauri-apps/api/core');

    const term = new Terminal({
      theme: {
        background: '#0D1117',
        foreground: '#E6EDF3',
        cursor: '#FFD60A',
        cursorAccent: '#0D1117',
        selectionBackground: 'rgba(255, 214, 10, 0.25)',
        black: '#484F58',
        red: '#F85149',
        green: '#3FB950',
        yellow: '#FFD60A',
        blue: '#58A6FF',
        magenta: '#BC8CFF',
        cyan: '#39D353',
        white: '#E6EDF3',
        brightBlack: '#6E7681',
        brightRed: '#FF7B72',
        brightGreen: '#56D364',
        brightYellow: '#FFE347',
        brightBlue: '#79C0FF',
        brightMagenta: '#D2A8FF',
        brightCyan: '#56D364',
        brightWhite: '#FFFFFF',
      },
      fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
      fontSize: settings.terminalFontSize || 14,
      fontWeight: '400',
      fontWeightBold: '600',
      lineHeight: 1.2,
      cursorBlink: true,
      cursorStyle: settings.terminalCursorStyle || 'bar',
      cursorWidth: 2,
      scrollback: 5000,
      drawBoldTextInBrightColors: true,
      convertEol: false,
      allowTransparency: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    term.open(el);

    try {
      const { WebglAddon } = await import('@xterm/addon-webgl');
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => webgl.dispose());
      term.loadAddon(webgl);
    } catch {}

    fitAddon.fit();

    // Replay buffer
    const buf = terminals.getBuffer(id);
    if (buf) term.write(buf);

    xtermMap.set(id, term);
    fitMap.set(id, fitAddon);

    // Clipboard handlers
    term.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
      const mod = ev.ctrlKey || ev.metaKey;
      const key = ev.key.toLowerCase();
      if (mod && key === 'c' && ev.type === 'keydown') {
        if (term.hasSelection()) {
          writeClipboardText(term.getSelection());
        } else {
          invoke('write_terminal', { id, data: '\u0003' }).catch(() => {});
        }
        return false;
      }
      if (mod && key === 'v' && ev.type === 'keydown') {
        readClipboardText().then(text => {
          if (text) invoke('write_terminal', { id, data: text }).catch(() => {});
        });
        return false;
      }
      return true;
    });

    el.addEventListener('paste', (e) => { e.preventDefault(); e.stopPropagation(); }, true);

    term.onData((data: string) => {
      invoke('write_terminal', { id, data }).catch(() => {});
    });

    term.onResize(({ rows, cols }: { rows: number; cols: number }) => {
      invoke('resize_terminal', { id, rows, cols }).catch(() => {});
    });

    // Spawn PTY if not yet spawned
    if (!spawnedIds.has(id)) {
      const inst = terminals.instances.find(t => t.id === id);
      if (inst) {
        const cwd = inst.cwd || workspace.path || '';
        try {
          await invoke('spawn_terminal', {
            id,
            cwd,
            rows: term.rows,
            cols: term.cols,
            shell: settings.defaultShell || 'auto',
          });
          spawnedIds.add(id);
        } catch (err) {
          const errStr = String(err);
          if (errStr.includes('already exists')) {
            spawnedIds.add(id);
          } else {
            term.writeln(`\x1b[31mFailed to spawn terminal: ${err}\x1b[0m`);
          }
        }
      }
    }
  }

  function fitActive() {
    if (terminals.activeId) {
      const fa = fitMap.get(terminals.activeId);
      if (fa) try { fa.fit(); } catch {}
    }
  }

  // Re-init terminals when instances change
  $effect(() => {
    const insts = terminals.instances;
    tick().then(() => {
      for (const inst of insts) {
        const el = containerEl?.querySelector(`[data-term-id="${inst.id}"]`) as HTMLDivElement | null;
        if (el && !xtermMap.has(inst.id)) {
          initTerminal(inst.id, el);
        }
      }
    });
  });

  // Fit on active tab change
  $effect(() => {
    const _aid = terminals.activeId;
    setTimeout(fitActive, 100);
  });

  onMount(async () => {
    const { listen } = await import('@tauri-apps/api/event');
    window.addEventListener('resize', fitActive);

    unlistenOutput = await listen<{ id: string; data: string }>('terminal-output', (event) => {
      const { id, data } = event.payload;
      const term = xtermMap.get(id);
      if (term) {
        terminals.appendBuffer(id, data);
        term.write(data);
      }
    });

    unlistenClosed = await listen<{ id: string; exit_code: number | null }>('terminal-closed', (event) => {
      const { id } = event.payload;
      const term = xtermMap.get(id);
      if (term) {
        term.writeln('\r\n\x1b[90m[Process exited]\x1b[0m');
      }
    });
  });

  onDestroy(() => {
    window.removeEventListener('resize', fitActive);
    if (unlistenOutput) unlistenOutput();
    if (unlistenClosed) unlistenClosed();
    for (const [, term] of xtermMap.entries()) term.dispose();
    xtermMap.clear();
    fitMap.clear();
  });
</script>

<div class="term-panel" bind:this={containerEl} style="display: flex; flex-direction: column; height: 100%; background: var(--bg-primary);">
  <!-- Tab bar -->
  <div style="display: flex; align-items: center; height: 36px; padding: 0 8px; border-bottom: 1px solid var(--border-default); background: var(--bg-surface); gap: 2px;">
    {#each terminals.instances as inst (inst.id)}
      <button
        style="display: flex; align-items: center; gap: 6px; padding: 4px 12px; font-size: 12px; font-family: var(--font-mono); border: none; border-radius: 4px 4px 0 0; cursor: pointer; transition: all 0.15s ease; white-space: nowrap; background: {terminals.activeId === inst.id ? 'var(--bg-elevated)' : 'transparent'}; color: {terminals.activeId === inst.id ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; border-bottom: {terminals.activeId === inst.id ? '2px solid var(--banana-yellow)' : '2px solid transparent'};"
        onclick={() => terminals.activeId = inst.id}
        onmouseenter={(e) => { if (terminals.activeId !== inst.id) { const t = e.currentTarget as HTMLElement; t.style.background = 'var(--bg-elevated)'; t.style.color = 'var(--text-primary)'; }}}
        onmouseleave={(e) => { if (terminals.activeId !== inst.id) { const t = e.currentTarget as HTMLElement; t.style.background = 'transparent'; t.style.color = 'var(--text-secondary)'; }}}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="4 17 10 11 4 5"/>
          <line x1="12" y1="19" x2="20" y2="19"/>
        </svg>
        {inst.title}
        <span
          style="display: flex; align-items: center; justify-content: center; width: 16px; height: 16px; border-radius: 3px; cursor: pointer; opacity: 0.5; transition: all 0.12s ease;"
          onclick={(e) => { e.stopPropagation(); handleCloseTerminal(inst.id); }}
          onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.opacity = '1'; t.style.background = 'rgba(248, 81, 73, 0.2)'; t.style.color = 'var(--accent-red)'; }}
          onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.opacity = '0.5'; t.style.background = 'transparent'; t.style.color = 'inherit'; }}
          role="button"
          tabindex="0"
          onkeydown={() => {}}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.3">
            <line x1="2" y1="2" x2="8" y2="8"/><line x1="8" y1="2" x2="2" y2="8"/>
          </svg>
        </span>
      </button>
    {/each}

    <button
      style="display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; border: none; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; background: transparent; color: var(--text-muted);"
      onclick={handleNewTerminal}
      onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--banana-yellow)'; t.style.background = 'var(--bg-elevated)'; t.style.transform = 'scale(1.15) rotate(90deg)'; }}
      onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; t.style.background = 'transparent'; t.style.transform = 'scale(1) rotate(0deg)'; }}
      title="New Terminal"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
      </svg>
    </button>
  </div>

  <!-- Terminal instances -->
  <div style="flex: 1; position: relative; overflow: hidden;">
    {#each terminals.instances as inst (inst.id)}
      <div
        style="position: absolute; inset: 0; padding: 4px; {terminals.activeId !== inst.id ? 'display: none;' : ''}"
        data-term-id={inst.id}
      ></div>
    {/each}

    {#if terminals.instances.length === 0}
      <div style="display: flex; align-items: center; justify-content: center; height: 100%;">
        <button
          style="display: flex; align-items: center; gap: 8px; color: var(--text-muted); font-size: 13px; font-family: var(--font-ui); padding: 12px 24px; border-radius: 8px; border: 1px dashed var(--border-default); background: transparent; cursor: pointer; transition: all 0.2s ease;"
          onclick={handleNewTerminal}
          onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--banana-yellow)'; t.style.borderColor = 'var(--banana-yellow)'; t.style.background = 'rgba(255, 214, 10, 0.05)'; t.style.transform = 'scale(1.03)'; }}
          onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; t.style.borderColor = 'var(--border-default)'; t.style.background = 'transparent'; t.style.transform = 'scale(1)'; }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
          New Terminal
        </button>
      </div>
    {/if}
  </div>
</div>
