<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import TitleBar from './TitleBar.svelte';
	import StatusBar from './StatusBar.svelte';
	import KanbanBoard from '$lib/components/kanban/KanbanBoard.svelte';
	import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
	import SettingsModal from '$lib/components/settings/SettingsModal.svelte';
	import MasterPrompt from '$lib/components/settings/MasterPrompt.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getSettingsStore, initSettings, getSettings, updateSetting } from '$lib/stores/settings.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getChatStore } from '$lib/stores/chat.svelte';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { subscribeToTable } from '$lib/supabase';
	import type { AeMessage, AeComment, AeTask } from '$lib/types';

	const layout = getLayout();
	const settingsStore = getSettingsStore();
	const taskStore = getTaskStore();
	const chatStore = getChatStore();
	const commentStore = getCommentStore();
	const worker = getWorkerStore();
	const projectStore = getProjectStore();
	const theme = getTheme();

	let chatToggleHovered = $state(false);
	let showMasterPrompt = $state(false);
	let realtimeChannels: any[] = [];
	let taskRefreshInterval: ReturnType<typeof setInterval> | null = null;

	/**
	 * Realtime coalescing. The WebView runs WITHOUT GPU compositing
	 * (WEBKIT_DISABLE_COMPOSITING_MODE — GPU path black-screens on this NVIDIA
	 * box), so there are no compositor layers and ANY DOM change forces the web
	 * process to software-repaint a large region and ship the buffer over a
	 * socket. When the worker churns the DB it emits many realtime events/sec;
	 * applying each one immediately meant a full repaint per event and pinned
	 * the CPU at 100%. So we BUFFER incoming task/comment events and flush them
	 * in a single batch at most ~1x/sec → at most one repaint per second
	 * regardless of how hard the worker is churning.
	 */
	const FLUSH_INTERVAL = 1000;
	let pendingTaskUpdates = new Map<string, { type: string; row: AeTask }>();
	let pendingComments: AeComment[] = [];
	let flushTimer: ReturnType<typeof setTimeout> | null = null;

	function scheduleFlush() {
		if (flushTimer) return;
		flushTimer = setTimeout(flushRealtime, FLUSH_INTERVAL);
	}

	function flushRealtime() {
		flushTimer = null;
		// Svelte batches these synchronous state writes into a single re-render.
		if (pendingTaskUpdates.size > 0) {
			for (const { type, row } of pendingTaskUpdates.values()) {
				taskStore.applyRealtimeUpdate(type, row);
			}
			pendingTaskUpdates = new Map();
		}
		if (pendingComments.length > 0) {
			for (const c of pendingComments) commentStore.applyRealtimeComment(c);
			pendingComments = [];
		}
	}

	let workerUnlisten: (() => void) | null = null;

	onMount(async () => {
		// Apply theme CSS variables to DOM immediately
		theme.applyNow();

		// Perf-lite: on software-rendered WebViews (the Spark, where GPU
		// compositing is disabled) every animation frame is a full repaint on
		// the WebKit main thread, so always-on `infinite` animations pin the CPU
		// and make clicks lag for seconds. Detect that host and stop the
		// continuous animations (see app.css `.perf-lite`). localStorage
		// 'samwise:perf-lite' = '1' forces it on, '0' forces it off (for testing).
		try {
			const override = localStorage.getItem('samwise:perf-lite');
			const perfLite =
				override === '1'
					? true
					: override === '0'
						? false
						: (await safeInvoke<boolean>('perf_lite_mode')) === true;
			if (perfLite) document.documentElement.classList.add('perf-lite');
		} catch {
			// Non-Tauri / detection failed — leave full animations on.
		}

		await initSettings();

		const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
		if (!config || !config.url) {
			const existing = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
			if (!existing || !existing.url) {
				console.warn('[app] Supabase not configured. Open Settings to connect.');
				return;
			}
		}

		// Master/viewer detection
		const settings = getSettings();
		if (settings.masterConfigured && settings.isMaster) {
			// This is the master machine - auto-start worker
			worker.mode = 'master';
			worker.startWorker();
		} else if (settings.masterConfigured && !settings.isMaster) {
			// Explicitly set to viewer - check if master is alive
			const result = await worker.checkActiveWorker();
			if (result?.active) {
				worker.mode = 'viewer';
				worker.machineName = result.machine_name || 'unknown';
			} else if (result?.error) {
				// Network error - default to viewer, don't offer master (could cause dual-master)
				worker.mode = 'viewer';
				worker.machineName = 'unknown';
			} else {
				// Master seems down, offer to become master
				showMasterPrompt = true;
			}
		} else {
			// First launch - never configured
			const result = await worker.checkActiveWorker();
			if (result?.active) {
				// Another machine is already the master
				worker.mode = 'viewer';
				worker.machineName = result.machine_name || 'unknown';
			} else if (result?.error) {
				// Network error on first launch - default to viewer to be safe
				worker.mode = 'viewer';
				worker.machineName = 'unknown';
			} else {
				// No active worker found - ask if this should be home
				showMasterPrompt = true;
			}
		}

		await Promise.all([
			taskStore.fetchTasks(),
			chatStore.fetchMessages(),
			projectStore.fetchProjects(),
		]);

		initRealtime();
		startTaskRefreshFallback();

		try {
			const { listen } = await import('@tauri-apps/api/event');
			workerUnlisten = await listen('worker-event', (event: any) => {
				const { event_type, message, task_id } = event.payload;
				console.log(`[worker] ${event_type}: ${message}`);
				if (event_type === 'task_completed' || event_type === 'task_failed') {
					taskStore.fetchTasks({ silent: true });
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
		if (taskRefreshInterval) {
			clearInterval(taskRefreshInterval);
			taskRefreshInterval = null;
		}
		if (flushTimer) {
			clearTimeout(flushTimer);
			flushTimer = null;
		}
		workerUnlisten?.();
		chatStore.destroySession();
	});

	async function initRealtime() {
		const config = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
		if (!config || !config.url || !config.anon_key) return;

		const taskChannel = subscribeToTable(config.url, config.anon_key, 'ae_tasks', (payload) => {
			const { eventType } = payload;
			// Buffer the latest state per row id; flushed in one batch ~1x/sec.
			if ((eventType === 'INSERT' || eventType === 'UPDATE') && payload.new) {
				const row = payload.new as AeTask;
				pendingTaskUpdates.set(row.id, { type: eventType, row });
				scheduleFlush();
			} else if (eventType === 'DELETE' && payload.old) {
				const row = payload.old as AeTask;
				pendingTaskUpdates.set(row.id, { type: 'DELETE', row });
				scheduleFlush();
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
				pendingComments.push(payload.new as AeComment);
				scheduleFlush();
			}
		});
		realtimeChannels.push(commentChannel);
	}

	function startTaskRefreshFallback() {
		if (taskRefreshInterval) return;
		// Safety fallback only — realtime is the primary update path.
		taskRefreshInterval = setInterval(() => {
			taskStore.fetchTasks({ silent: true });
		}, 30_000);
	}

	function handleFocus() {
		taskStore.fetchTasks({ silent: true });
	}

	// Watch for reconfigure requests from Settings modal
	$effect(() => {
		if (settingsStore.reconfigureRequested) {
			settingsStore.reconfigureRequested = false;
			showMasterPrompt = true;
		}
	});

	function handleMasterConfirm() {
		updateSetting('masterConfigured', true);
		updateSetting('isMaster', true);
		updateSetting('autoStartWorker', true);
		worker.mode = 'master';
		worker.startWorker();
		showMasterPrompt = false;
	}

	function handleMasterDecline() {
		updateSetting('masterConfigured', true);
		updateSetting('isMaster', false);
		worker.mode = 'viewer';
		showMasterPrompt = false;
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

<svelte:window onkeydown={handleGlobalKeyDown} onfocus={handleFocus} />

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

{#if showMasterPrompt}
	<MasterPrompt onConfirm={handleMasterConfirm} onDecline={handleMasterDecline} />
{/if}

<SettingsModal />
