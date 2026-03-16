<script lang="ts">
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';

	const worker = getWorkerStore();
	const settingsStore = getSettingsStore();

	let minimizeHovered = $state(false);
	let maximizeHovered = $state(false);
	let closeHovered = $state(false);

	async function minimize() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().minimize();
		} catch { /* browser dev mode */ }
	}

	async function maximize() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().toggleMaximize();
		} catch { /* browser dev mode */ }
	}

	async function close() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().close();
		} catch { /* browser dev mode */ }
	}
</script>

<div class="titlebar" data-tauri-drag-region style="
	display: flex; align-items: center; height: 40px; padding: 0 14px;
	background: linear-gradient(180deg, #141920 0%, #0f1419 100%);
	box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.03);
	gap: 0; position: relative; z-index: 5;
	border-bottom: 1px solid rgba(99, 102, 241, 0.06);
">
	<!-- Left: Logo + Brand -->
	<div style="display: flex; align-items: center; gap: 10px; min-width: 180px;">
		<!-- Robot icon with breathing glow -->
		<div style="
			width: 26px; height: 26px; border-radius: 8px;
			background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(129, 140, 248, 0.1));
			display: flex; align-items: center; justify-content: center;
			animation: robot-breathe 4s ease-in-out infinite;
			box-shadow: 0 0 12px rgba(99, 102, 241, 0.15);
		">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent-indigo)" stroke-width="2" stroke-linecap="round">
				<rect x="4" y="8" width="16" height="12" rx="2"/>
				<line x1="12" y1="4" x2="12" y2="8"/>
				<circle cx="12" cy="3" r="1.5" fill="var(--accent-indigo)"/>
				<circle cx="9" cy="14" r="1.5" fill="var(--accent-indigo)"/>
				<circle cx="15" cy="14" r="1.5" fill="var(--accent-indigo)"/>
			</svg>
		</div>
		<span style="font-family: var(--font-ui); font-weight: 700; font-size: 15px; color: var(--accent-indigo); letter-spacing: -0.3px; text-shadow: 0 0 20px rgba(99, 102, 241, 0.25);">
			Agent One
		</span>
		<span style="font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); background: rgba(99, 102, 241, 0.06); padding: 1px 6px; border-radius: 6px;">
			v0.1
		</span>
	</div>

	<!-- Center: Worker status -->
	<div data-tauri-drag-region style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 12px; color: var(--text-secondary);">
		<div style="display: flex; align-items: center; gap: 5px;">
			<span style="
				width: 7px; height: 7px; border-radius: 50%;
				background: {worker.statusColor};
				box-shadow: 0 0 6px {worker.statusColor}60;
				{worker.status !== 'offline' ? 'animation: pulse-dot 2s ease-in-out infinite;' : ''}
			"></span>
			<span data-tauri-drag-region style="font-size: 11px; color: var(--text-muted);">
				{worker.statusLabel}
			</span>
			{#if worker.currentTask}
				<span data-tauri-drag-region style="font-size: 11px; color: var(--text-muted);">
					- {worker.currentTask.title}
				</span>
			{/if}
		</div>
	</div>

	<!-- Right: Window controls -->
	<div style="display: flex; align-items: center; gap: 2px; min-width: 100px; justify-content: flex-end;">
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {minimizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => minimizeHovered = true}
			onmouseleave={() => minimizeHovered = false}
			onclick={minimize}
			aria-label="Minimize"
		>
			<svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1" rx="0.5"/></svg>
		</button>
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {maximizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => maximizeHovered = true}
			onmouseleave={() => maximizeHovered = false}
			onclick={maximize}
			aria-label="Maximize"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9" rx="1"/></svg>
		</button>
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {closeHovered ? 'var(--accent-red)' : 'transparent'}; color: {closeHovered ? '#fff' : 'var(--text-secondary)'}; border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => closeHovered = true}
			onmouseleave={() => closeHovered = false}
			onclick={close}
			aria-label="Close"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.3"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
		</button>
	</div>
</div>
