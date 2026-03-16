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
	import { safeInvoke } from '$lib/utils/tauri';
	import { subscribeToTable } from '$lib/supabase';
	import type { AeMessage, AeComment } from '$lib/types';

	const layout = getLayout();
	const settingsStore = getSettingsStore();
	const taskStore = getTaskStore();
	const chatStore = getChatStore();
	const commentStore = getCommentStore();

	let chatToggleHovered = $state(false);
	let realtimeChannels: any[] = [];

	let workerUnlisten: (() => void) | null = null;

	onMount(async () => {
		await initSettings();

		// Try to load Supabase config from Doppler
		const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
		if (!config || !config.url) {
			// Doppler not available, try getting existing config
			const existing = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
			if (!existing || !existing.url) {
				console.warn('[app] Supabase not configured. Open Settings to connect.');
				return;
			}
		}

		// Fetch initial data
		await Promise.all([
			taskStore.fetchTasks(),
			chatStore.fetchMessages(),
		]);

		// Start realtime subscriptions
		initRealtime();

		// Listen for worker events from Rust backend
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
			// Not in Tauri environment (browser dev mode)
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

		// Subscribe to task changes
		const taskChannel = subscribeToTable(config.url, config.anon_key, 'ae_tasks', (payload) => {
			const { eventType } = payload;
			if (eventType === 'INSERT' || eventType === 'UPDATE') {
				// Refetch all tasks (simple approach)
				taskStore.fetchTasks();
			} else if (eventType === 'DELETE') {
				taskStore.fetchTasks();
			}
		});
		realtimeChannels.push(taskChannel);

		// Subscribe to new messages
		const msgChannel = subscribeToTable(config.url, config.anon_key, 'ae_messages', (payload) => {
			if (payload.eventType === 'INSERT') {
				chatStore.applyRealtimeMessage(payload.new as AeMessage);
			}
		});
		realtimeChannels.push(msgChannel);

		// Subscribe to new comments
		const commentChannel = subscribeToTable(config.url, config.anon_key, 'ae_comments', (payload) => {
			if (payload.eventType === 'INSERT') {
				commentStore.applyRealtimeComment(payload.new as AeComment);
			}
		});
		realtimeChannels.push(commentChannel);
	}

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
