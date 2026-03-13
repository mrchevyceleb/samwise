<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getTerminals } from '$lib/stores/terminals.svelte';
  import { getWorkspace } from '$lib/stores/workspace.svelte';
  import { getSettings } from '$lib/stores/settings.svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  interface Props {
    terminalId: string;
  }

  let { terminalId }: Props = $props();

  const termStore = getTerminals();
  const workspace = getWorkspace();
  const settings = getSettings();

  let containerEl: HTMLDivElement;
  let wrapperEl: HTMLDivElement;
  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let unlistenOutput: (() => void) | null = null;
  let unlistenClosed: (() => void) | null = null;
  let resizeObserver: ResizeObserver | null = null;

  function handleResize() {
    if (fitAddon) try { fitAddon.fit(); } catch {}
  }

  onMount(async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { listen } = await import('@tauri-apps/api/event');

    term = new Terminal({
      theme: {
        background: '#0D1117',
        foreground: '#E6EDF3',
        cursor: '#FFD60A',
        cursorAccent: '#0D1117',
        selectionBackground: 'rgba(255, 214, 10, 0.25)',
      },
      fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
      fontSize: settings.terminalFontSize || 14,
      cursorBlink: true,
      cursorStyle: settings.terminalCursorStyle || 'bar',
      scrollback: 5000,
      allowTransparency: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.open(containerEl);

    try {
      const { WebglAddon } = await import('@xterm/addon-webgl');
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => webgl.dispose());
      term.loadAddon(webgl);
    } catch {}

    fitAddon.fit();

    const buf = termStore.getBuffer(terminalId);
    if (buf) term.write(buf);

    term.onData((data: string) => {
      invoke('write_terminal', { id: terminalId, data }).catch(() => {});
    });

    term.onResize(({ rows, cols }) => {
      invoke('resize_terminal', { id: terminalId, rows, cols }).catch(() => {});
    });

    unlistenOutput = await listen<{ id: string; data: string }>('terminal-output', (event) => {
      if (event.payload.id === terminalId && term) {
        termStore.appendBuffer(terminalId, event.payload.data);
        term.write(event.payload.data);
      }
    });

    unlistenClosed = await listen<{ id: string }>('terminal-closed', (event) => {
      if (event.payload.id === terminalId && term) {
        term.writeln('\r\n\x1b[90m[Process exited]\x1b[0m');
      }
    });

    const inst = termStore.instances.find(t => t.id === terminalId);
    const cwd = inst?.cwd || workspace.path || '';
    try {
      await invoke('spawn_terminal', { id: terminalId, cwd, rows: term.rows, cols: term.cols, shell: settings.defaultShell || 'auto' });
    } catch {
      try { await invoke('resize_terminal', { id: terminalId, rows: term.rows, cols: term.cols }); } catch {}
    }

    resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(wrapperEl);
    window.addEventListener('resize', handleResize);
  });

  onDestroy(() => {
    window.removeEventListener('resize', handleResize);
    if (resizeObserver) resizeObserver.disconnect();
    if (unlistenOutput) unlistenOutput();
    if (unlistenClosed) unlistenClosed();
    if (term) { term.dispose(); term = null; fitAddon = null; }
  });
</script>

<div bind:this={wrapperEl} style="display: flex; flex-direction: column; height: 100%; background: var(--bg-primary);">
  <div style="display: flex; align-items: center; gap: 8px; padding: 6px 12px; background: var(--bg-surface); border-bottom: 1px solid var(--border-default);">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--banana-yellow)" stroke-width="2" style="opacity: 0.7;">
      <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
    </svg>
    <span style="font-size: 12px; color: var(--text-muted);">Terminal</span>
  </div>
  <div bind:this={containerEl} style="flex: 1; overflow: hidden; padding: 4px;"></div>
</div>
