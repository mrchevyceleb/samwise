<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import TitleBar from './TitleBar.svelte';
	import StatusBar from './StatusBar.svelte';
	import KanbanBoard from '$lib/components/kanban/KanbanBoard.svelte';
	import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
	import SettingsModal from '$lib/components/settings/SettingsModal.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getSettingsStore, initSettings } from '$lib/stores/settings.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getChatStore } from '$lib/stores/chat.svelte';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { subscribeToTable } from '$lib/supabase';
	import type { AeMessage, AeComment } from '$lib/types';

	const layout = getLayout();
	const settingsStore = getSettingsStore();
	const taskStore = getTaskStore();
	const chatStore = getChatStore();
	const commentStore = getCommentStore();
	const theme = getTheme();

	let chatToggleHovered = $state(false);
	let realtimeChannels: any[] = [];

	let workerUnlisten: (() => void) | null = null;

	onMount(async () => {
		// Apply theme CSS variables to DOM immediately
		theme.applyNow();

		await initSettings();

		const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
		if (!config || !config.url) {
			const existing = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
			if (!existing || !existing.url) {
				console.warn('[app] Supabase not configured. Open Settings to connect.');
				return;
			}
		}

		await Promise.all([
			taskStore.fetchTasks(),
			chatStore.fetchMessages(),
		]);

		initRealtime();

		try {
			const { listen } = await import('@tauri-apps/api/event');
			workerUnlisten = await listen('worker-event', (event: any) => {
				const { event_type, message, task_id } = event.payload;
				console.log(`[worker] ${event_type}: ${message}`);
				if (event_type === 'task_completed' || event_type === 'task_failed') {
					taskStore.fetchTasks();
				}
			});
		} catch {
			// Not in Tauri environment
		}
	});

	onDestroy(() => {
		for (const ch of realtimeChannels) {
			ch.unsubscribe();
		}
		workerUnlisten?.();
	});

	async function initRealtime() {
		const config = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
		if (!config || !config.url || !config.anon_key) return;

		const taskChannel = subscribeToTable(config.url, config.anon_key, 'ae_tasks', (payload) => {
			const { eventType } = payload;
			if (eventType === 'INSERT' || eventType === 'UPDATE' || eventType === 'DELETE') {
				taskStore.fetchTasks();
			}
		});
		realtimeChannels.push(taskChannel);

		const msgChannel = subscribeToTable(config.url, config.anon_key, 'ae_messages', (payload) => {
			if (payload.eventType === 'INSERT') {
				chatStore.applyRealtimeMessage(payload.new as AeMessage);
			}
		});
		realtimeChannels.push(msgChannel);

		const commentChannel = subscribeToTable(config.url, config.anon_key, 'ae_comments', (payload) => {
			if (payload.eventType === 'INSERT') {
				commentStore.applyRealtimeComment(payload.new as AeComment);
			}
		});
		realtimeChannels.push(commentChannel);
	}

	async function handleGlobalKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === ',') {
			e.preventDefault();
			settingsStore.settingsVisible = !settingsStore.settingsVisible;
		}
		if ((e.ctrlKey || e.metaKey) && e.key === '/') {
			e.preventDefault();
			layout.toggleRightPanel();
		}
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'B') {
			e.preventDefault();
			settingsStore.activeSettingsTab = 'automation';
			settingsStore.settingsVisible = !settingsStore.settingsVisible;
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<div class="app-shell" style="display: flex; flex-direction: column; height: 100vh; width: 100vw; background: {theme.c.bgCanvas}; color: {theme.c.textPrimary};">
	<TitleBar />

	<div style="display: flex; flex: 1; overflow: hidden; gap: 6px; padding: 0 6px 6px;">
		<!-- Left: Kanban Board -->
		<div style="
			flex: 1; min-width: 400px;
			display: flex; flex-direction: column; overflow: hidden;
			border-radius: 12px;
			box-shadow: {theme.c.shadowPanel};
			border: {theme.c.panelBorder};
			border-top: {theme.c.panelTopBorder};
			background: {theme.c.gradientPanelMain};
		">
			<KanbanBoard />
		</div>

		<!-- Right: Chat Panel -->
		{#if layout.rightPanelVisible}
			<div style="
				width: 400px; min-width: 320px; max-width: 500px; flex-shrink: 0;
				display: flex; flex-direction: column; overflow: hidden;
				border-radius: 12px;
				box-shadow: {theme.c.shadowPanel};
				border: {theme.c.panelBorder};
				border-top: {theme.c.panelTopBorder};
				background: {theme.c.gradientPanelChat};
				animation: sidebar-slide-in 0.2s ease;
			">
				<ChatPanel />
			</div>
		{/if}
	</div>

	{#if !layout.rightPanelVisible}
		<button
			title="Open Chat (Ctrl+/)"
			style="
				position: fixed; bottom: 40px; right: 16px; z-index: 100;
				width: 48px; height: 48px; border-radius: 50%;
				display: flex; align-items: center; justify-content: center;
				background: {chatToggleHovered ? theme.c.accentHover : theme.c.accentPrimary};
				border: none; cursor: pointer;
				box-shadow: 0 4px 20px {theme.c.accentGlow}, 0 0 40px {theme.c.accentGlow};
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

	<StatusBar />
</div>

<SettingsModal />
