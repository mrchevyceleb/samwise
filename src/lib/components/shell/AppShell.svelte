<script lang="ts">
	import { onMount } from 'svelte';
	import TitleBar from './TitleBar.svelte';
	import StatusBar from './StatusBar.svelte';
	import ResizeHandle from './ResizeHandle.svelte';
	import AgentPanel from '$lib/components/agents/AgentPanel.svelte';
	import PreviewPanel from '$lib/components/preview/PreviewPanel.svelte';
	import FilePanel from '$lib/components/files/FilePanel.svelte';
	import TerminalPanel from '$lib/components/terminal/TerminalPanel.svelte';
	import SettingsModal from '$lib/components/settings/SettingsModal.svelte';
	import CommandPalette from '$lib/components/modals/CommandPalette.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getSettingsStore, initSettings } from '$lib/stores/settings.svelte';
	import { getTerminals } from '$lib/stores/terminals.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';

	const layout = getLayout();
	const settingsStore = getSettingsStore();
	const terminals = getTerminals();
	const workspace = getWorkspace();

	let commandPaletteVisible = $state(false);
	let leftExpandHovered = $state(false);
	let rightExpandHovered = $state(false);

	onMount(async () => {
		// 1. Load persisted settings from disk
		await initSettings();

		// 2. Restore last workspace if restoreSession is enabled
		const s = settingsStore.value;
		if (s.restoreSession && workspace.lastPath) {
			await workspace.setWorkspace(workspace.lastPath);
		}
	});

	function handleGlobalKeyDown(e: KeyboardEvent) {
		// Ctrl+Shift+P -> Command Palette
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'P') {
			e.preventDefault();
			commandPaletteVisible = !commandPaletteVisible;
		}
		// Ctrl+` -> Toggle Terminal
		if ((e.ctrlKey || e.metaKey) && e.key === '`') {
			e.preventDefault();
			layout.toggleTerminal();
			// Auto-create a terminal if none exist and we're opening
			if (layout.terminalVisible && terminals.instances.length === 0) {
				terminals.add(workspace.path || '');
			}
		}
		// Ctrl+, -> Settings
		if ((e.ctrlKey || e.metaKey) && e.key === ',') {
			e.preventDefault();
			settingsStore.settingsVisible = !settingsStore.settingsVisible;
		}
		// Ctrl+O -> Open Folder
		if ((e.ctrlKey || e.metaKey) && e.key === 'o' && !e.shiftKey) {
			e.preventDefault();
			workspace.openFolder();
		}
		// Ctrl+Shift+N -> New Terminal
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'N') {
			e.preventDefault();
			layout.terminalVisible = true;
			terminals.add(workspace.path || '');
		}
		// Ctrl+B -> Toggle Left Panel (Agent)
		if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'b') {
			e.preventDefault();
			layout.toggleLeftPanel();
		}
		// Ctrl+Shift+B -> Toggle Right Panel (Files)
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'B') {
			e.preventDefault();
			layout.toggleRightPanel();
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<div class="app-shell" style="display: flex; flex-direction: column; height: 100vh; width: 100vw; background: var(--bg-canvas);">
	<!-- Title Bar -->
	<TitleBar />

	<!-- Main Content Area -->
	<div style="display: flex; flex: 1; overflow: hidden; gap: var(--panel-gap); padding: 0 var(--panel-gap) var(--panel-gap);">
		<!-- Left: Agent Panel -->
		{#if layout.leftPanelVisible}
			<div style="width: {layout.agentPanelWidth}px; min-width: 280px; max-width: 600px; display: flex; flex-direction: column; overflow: hidden; border-radius: var(--panel-radius); box-shadow: var(--shadow-panel); border: var(--panel-border); border-top: 1px solid rgba(255, 214, 10, 0.08); background: linear-gradient(180deg, #161C26 0%, #11161E 100%);">
				<AgentPanel />
			</div>

			<ResizeHandle direction="vertical" onResize={(d) => layout.agentPanelWidth = layout.agentPanelWidth + d} />
		{:else}
			<!-- Left panel expand tab -->
			<button
				title="Show Agent Panel (Ctrl+B)"
				style="
					width: 24px; flex-shrink: 0; display: flex; align-items: center; justify-content: center;
					background: {leftExpandHovered ? 'rgba(255, 214, 10, 0.08)' : 'var(--bg-surface)'};
					border: none; border-radius: 0 var(--panel-radius) var(--panel-radius) 0;
					box-shadow: var(--shadow-sm);
					color: {leftExpandHovered ? 'var(--banana-yellow)' : 'var(--text-muted)'};
					cursor: pointer; transition: all 0.15s ease; writing-mode: vertical-rl;
					font-family: var(--font-ui); font-size: 11px; letter-spacing: 0.5px;
				"
				onclick={() => layout.toggleLeftPanel()}
				onmouseenter={() => leftExpandHovered = true}
				onmouseleave={() => leftExpandHovered = false}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" style="margin-bottom: 6px; transform: rotate(0deg);">
					<path d="M6 4l4 4-4 4"/>
				</svg>
				Agent
			</button>
		{/if}

		<!-- Middle: Preview Panel + Terminal -->
		<div style="flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 300px; border-radius: var(--panel-radius); box-shadow: var(--shadow-panel); border: var(--panel-border); border-top: 1px solid rgba(255, 214, 10, 0.08); background: linear-gradient(180deg, #161C26 0%, #0F1318 100%);">
			<div style="flex: 1; overflow: hidden;">
				<PreviewPanel />
			</div>

			{#if layout.terminalVisible}
				<ResizeHandle direction="horizontal" onResize={(d) => layout.terminalHeight = layout.terminalHeight - d} />
				<div style="height: {layout.terminalHeight}px; min-height: 100px; background: var(--bg-surface); display: flex; flex-direction: column; overflow: hidden;">
					<TerminalPanel />
				</div>
			{/if}
		</div>

		<!-- Right: File Panel -->
		{#if layout.rightPanelVisible}
			<ResizeHandle direction="vertical" onResize={(d) => layout.filePanelWidth = layout.filePanelWidth - d} />

			<div style="width: {layout.filePanelWidth}px; min-width: 200px; max-width: 500px; display: flex; flex-direction: column; overflow: hidden; border-radius: var(--panel-radius); box-shadow: var(--shadow-panel); border: var(--panel-border); border-top: 1px solid rgba(255, 214, 10, 0.08); background: linear-gradient(180deg, #161C26 0%, #11161E 100%);">
				<FilePanel />
			</div>
		{:else}
			<!-- Right panel expand tab -->
			<button
				title="Show Files Panel (Ctrl+Shift+B)"
				style="
					width: 24px; flex-shrink: 0; display: flex; align-items: center; justify-content: center;
					background: {rightExpandHovered ? 'rgba(255, 214, 10, 0.08)' : 'var(--bg-surface)'};
					border: none; border-radius: var(--panel-radius) 0 0 var(--panel-radius);
					box-shadow: var(--shadow-sm);
					color: {rightExpandHovered ? 'var(--banana-yellow)' : 'var(--text-muted)'};
					cursor: pointer; transition: all 0.15s ease; writing-mode: vertical-rl;
					font-family: var(--font-ui); font-size: 11px; letter-spacing: 0.5px;
				"
				onclick={() => layout.toggleRightPanel()}
				onmouseenter={() => rightExpandHovered = true}
				onmouseleave={() => rightExpandHovered = false}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" style="margin-bottom: 6px; transform: rotate(180deg);">
					<path d="M6 4l4 4-4 4"/>
				</svg>
				Files
			</button>
		{/if}
	</div>

	<!-- Status Bar -->
	<StatusBar />
</div>

<!-- Modals (rendered outside main layout) -->
<SettingsModal />
<CommandPalette visible={commandPaletteVisible} onClose={() => commandPaletteVisible = false} />
