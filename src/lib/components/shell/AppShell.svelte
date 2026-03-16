<script lang="ts">
	import { onMount } from 'svelte';
	import TitleBar from './TitleBar.svelte';
	import StatusBar from './StatusBar.svelte';
	import KanbanBoard from '$lib/components/kanban/KanbanBoard.svelte';
	import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
	import SettingsModal from '$lib/components/settings/SettingsModal.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getSettingsStore, initSettings } from '$lib/stores/settings.svelte';

	const layout = getLayout();
	const settingsStore = getSettingsStore();

	let chatToggleHovered = $state(false);

	onMount(async () => {
		await initSettings();
	});

	async function handleGlobalKeyDown(e: KeyboardEvent) {
		// Ctrl+, -> Settings
		if ((e.ctrlKey || e.metaKey) && e.key === ',') {
			e.preventDefault();
			settingsStore.settingsVisible = !settingsStore.settingsVisible;
		}
		// Ctrl+/ -> Toggle Chat Panel
		if ((e.ctrlKey || e.metaKey) && e.key === '/') {
			e.preventDefault();
			layout.toggleRightPanel();
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<div class="app-shell" style="display: flex; flex-direction: column; height: 100vh; width: 100vw; background: var(--bg-canvas);">
	<!-- Title Bar -->
	<TitleBar />

	<!-- Main Content: Kanban | Chat -->
	<div style="display: flex; flex: 1; overflow: hidden; gap: var(--panel-gap); padding: 0 var(--panel-gap) var(--panel-gap);">
		<!-- Left: Kanban Board (primary, fills available space) -->
		<div style="
			flex: 1; min-width: 400px;
			display: flex; flex-direction: column; overflow: hidden;
			border-radius: var(--panel-radius);
			box-shadow: var(--shadow-panel);
			border: var(--panel-border);
			border-top: 1px solid rgba(99, 102, 241, 0.08);
			background: linear-gradient(180deg, #161b22 0%, #0f1419 100%);
		">
			<KanbanBoard />
		</div>

		<!-- Right: Chat Panel (collapsible sidebar) -->
		{#if layout.rightPanelVisible}
			<div style="
				width: 400px; min-width: 320px; max-width: 500px; flex-shrink: 0;
				display: flex; flex-direction: column; overflow: hidden;
				border-radius: var(--panel-radius);
				box-shadow: var(--shadow-panel);
				border: var(--panel-border);
				border-top: 1px solid rgba(99, 102, 241, 0.08);
				background: linear-gradient(180deg, #161b22 0%, #0d1117 100%);
				animation: sidebar-slide-in 0.2s ease;
			">
				<ChatPanel />
			</div>
		{/if}
	</div>

	<!-- Chat toggle button (floating, bottom-right when chat is hidden) -->
	{#if !layout.rightPanelVisible}
		<button
			title="Open Chat (Ctrl+/)"
			style="
				position: fixed; bottom: 40px; right: 16px; z-index: 100;
				width: 48px; height: 48px; border-radius: 50%;
				display: flex; align-items: center; justify-content: center;
				background: {chatToggleHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
				border: none; cursor: pointer;
				box-shadow: 0 4px 20px rgba(99, 102, 241, 0.4), 0 0 40px rgba(99, 102, 241, 0.15);
				transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
				transform: {chatToggleHovered ? 'scale(1.1) rotate(-5deg)' : 'scale(1)'};
				animation: breathe-glow 3s ease-in-out infinite;
			"
			onclick={() => layout.toggleRightPanel()}
			onmouseenter={() => chatToggleHovered = true}
			onmouseleave={() => chatToggleHovered = false}
		>
			<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
			</svg>
		</button>
	{/if}

	<!-- Status Bar -->
	<StatusBar />
</div>

<!-- Modals -->
<SettingsModal />
