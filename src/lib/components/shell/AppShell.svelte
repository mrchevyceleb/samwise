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
	import { getLayout } from '$lib/stores/layout';
	import { getSettingsStore, initSettings } from '$lib/stores/settings';
	import { getTerminals } from '$lib/stores/terminals';
	import { getWorkspace } from '$lib/stores/workspace';

	const layout = getLayout();
	const settingsStore = getSettingsStore();
	const terminals = getTerminals();
	const workspace = getWorkspace();

	let commandPaletteVisible = $state(false);

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
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<div class="app-shell" style="display: flex; flex-direction: column; height: 100vh; width: 100vw; background: var(--bg-primary);">
	<!-- Title Bar -->
	<TitleBar />

	<!-- Main Content Area -->
	<div style="display: flex; flex: 1; overflow: hidden;">
		<!-- Left: Agent Panel -->
		<div style="width: {layout.agentPanelWidth}px; min-width: 280px; max-width: 600px; display: flex; flex-direction: column; overflow: hidden;">
			<AgentPanel />
		</div>

		<ResizeHandle direction="vertical" onResize={(d) => layout.agentPanelWidth = layout.agentPanelWidth + d} />

		<!-- Middle: Preview Panel + Terminal -->
		<div style="flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 300px;">
			<div style="flex: 1; overflow: hidden;">
				<PreviewPanel />
			</div>

			{#if layout.terminalVisible}
				<ResizeHandle direction="horizontal" onResize={(d) => layout.terminalHeight = layout.terminalHeight - d} />
				<div style="height: {layout.terminalHeight}px; min-height: 100px; background: var(--bg-surface); border-top: 1px solid var(--border-default); display: flex; flex-direction: column; overflow: hidden;">
					<TerminalPanel />
				</div>
			{/if}
		</div>

		<ResizeHandle direction="vertical" onResize={(d) => layout.filePanelWidth = layout.filePanelWidth - d} />

		<!-- Right: File Panel -->
		<div style="width: {layout.filePanelWidth}px; min-width: 200px; max-width: 500px; display: flex; flex-direction: column; overflow: hidden;">
			<FilePanel />
		</div>
	</div>

	<!-- Status Bar -->
	<StatusBar />
</div>

<!-- Modals (rendered outside main layout) -->
<SettingsModal />
<CommandPalette visible={commandPaletteVisible} onClose={() => commandPaletteVisible = false} />
