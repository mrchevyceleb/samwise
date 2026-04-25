<script lang="ts">
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';

	const worker = getWorkerStore();
	const taskStore = getTaskStore();
	const theme = getTheme();

	let currentTaskTitle = $derived(() => {
		if (worker.currentTask) return worker.currentTask.title;
		if (worker.workerId) {
			const task = taskStore.getTask(worker.workerId);
			return task?.title ?? null;
		}
		return null;
	});

	let minimizeHovered = $state(false);
	let maximizeHovered = $state(false);
	let closeHovered = $state(false);
	let themeHovered = $state(false);

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

	let showCloseDialog = $state(false);

	async function close() {
		showCloseDialog = true;
	}

	async function confirmClose() {
		showCloseDialog = false;
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().hide();
		} catch { /* browser dev mode */ }
	}

	function cancelClose() {
		showCloseDialog = false;
	}
</script>

<div class="titlebar" data-tauri-drag-region style="
	display: flex; align-items: center; height: 40px; padding: 0 14px;
	background: {theme.c.gradientTitlebar};
	box-shadow: {theme.c.shadowTitlebar};
	gap: 0; position: relative; z-index: 5;
	border-bottom: 1px solid {theme.c.borderGlow};
">
	<!-- Left: Logo + Brand -->
	<div style="display: flex; align-items: center; gap: 10px; min-width: 180px;">
		<div style="
			width: 26px; height: 26px; border-radius: 8px;
			background: linear-gradient(135deg, {theme.c.accentGlow}, rgba(129, 140, 248, 0.1));
			display: flex; align-items: center; justify-content: center;
			animation: robot-breathe 4s ease-in-out infinite;
			box-shadow: 0 0 12px {theme.c.accentGlow};
		">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="{theme.c.accentIndigo}" stroke-width="2" stroke-linecap="round">
				<rect x="4" y="8" width="16" height="12" rx="2"/>
				<line x1="12" y1="4" x2="12" y2="8"/>
				<circle cx="12" cy="3" r="1.5" fill="{theme.c.accentIndigo}"/>
				<circle cx="9" cy="14" r="1.5" fill="{theme.c.accentIndigo}"/>
				<circle cx="15" cy="14" r="1.5" fill="{theme.c.accentIndigo}"/>
			</svg>
		</div>
		<span style="font-family: var(--font-ui); font-weight: 700; font-size: 15px; color: {theme.c.accentIndigo}; letter-spacing: -0.3px; text-shadow: 0 0 20px {theme.c.accentGlow};">
			SamWise
		</span>
		<span style="font-size: 9px; color: {theme.c.textMuted}; font-family: var(--font-mono); background: {theme.c.accentGlow}; padding: 1px 6px; border-radius: 6px;">
			v0.1
		</span>
	</div>

	<!-- Center: Worker status -->
	<div data-tauri-drag-region style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 12px; color: {theme.c.textSecondary};">
		<div style="display: flex; align-items: center; gap: 5px;">
			<span style="
				width: 7px; height: 7px; border-radius: 50%;
				background: {worker.statusColor};
				box-shadow: 0 0 6px {worker.statusColor}60;
				{worker.status !== 'offline' ? 'animation: pulse-dot 2s ease-in-out infinite;' : ''}
			"></span>
			<span data-tauri-drag-region style="font-size: 11px; color: {theme.c.textMuted};">
				{worker.statusLabel}
			</span>
			{#if currentTaskTitle()}
				<span data-tauri-drag-region style="font-size: 11px; color: {theme.c.textMuted};">
					- {currentTaskTitle()}
				</span>
			{/if}
		</div>
	</div>

	<!-- Right: Window controls -->
	<div style="display: flex; align-items: center; gap: 2px; min-width: 100px; justify-content: flex-end;">
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {themeHovered ? theme.c.bgElevated : 'transparent'}; color: {theme.c.textSecondary}; border-radius: 6px; cursor: pointer; transition: all 0.15s ease; margin-right: 4px;"
			onmouseenter={() => themeHovered = true}
			onmouseleave={() => themeHovered = false}
			onclick={() => theme.toggle()}
			aria-label={theme.isDark ? 'Switch to light theme' : 'Switch to dark theme'}
			title={theme.isDark ? 'Switch to light theme' : 'Switch to dark theme'}
		>
			{#if theme.isDark}
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<circle cx="12" cy="12" r="4"/>
					<path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/>
				</svg>
			{:else}
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
				</svg>
			{/if}
		</button>
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {minimizeHovered ? theme.c.bgElevated : 'transparent'}; color: {theme.c.textSecondary}; border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => minimizeHovered = true}
			onmouseleave={() => minimizeHovered = false}
			onclick={minimize}
			aria-label="Minimize"
		>
			<svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1" rx="0.5"/></svg>
		</button>
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {maximizeHovered ? theme.c.bgElevated : 'transparent'}; color: {theme.c.textSecondary}; border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => maximizeHovered = true}
			onmouseleave={() => maximizeHovered = false}
			onclick={maximize}
			aria-label="Maximize"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9" rx="1"/></svg>
		</button>
		<button
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {closeHovered ? theme.c.accentRed : 'transparent'}; color: {closeHovered ? '#fff' : theme.c.textSecondary}; border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => closeHovered = true}
			onmouseleave={() => closeHovered = false}
			onclick={close}
			aria-label="Close"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.3"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
		</button>
	</div>
</div>

{#if showCloseDialog}
<!-- Close confirmation overlay -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	style="
		position: fixed; inset: 0; z-index: 9999;
		background: rgba(0, 0, 0, 0.6); backdrop-filter: blur(4px);
		display: flex; align-items: center; justify-content: center;
	"
	onclick={cancelClose}
	onkeydown={(e) => { if (e.key === 'Escape') cancelClose(); }}
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		style="
			background: {theme.c.bgCard}; border: 1px solid {theme.c.borderGlow};
			border-radius: 16px; padding: 28px 32px; max-width: 400px; width: 90%;
			box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 30px {theme.c.accentGlow};
			text-align: center;
		"
		onclick={(e) => e.stopPropagation()}
	>
		<div style="font-size: 28px; margin-bottom: 12px;">
			<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="{theme.c.accentAmber}" stroke-width="2" stroke-linecap="round" style="display: inline-block;">
				<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
				<line x1="12" y1="9" x2="12" y2="13"/>
				<line x1="12" y1="17" x2="12.01" y2="17"/>
			</svg>
		</div>
		<h3 style="margin: 0 0 8px; font-size: 17px; font-weight: 700; color: {theme.c.textPrimary}; font-family: var(--font-ui);">
			Minimize to Tray?
		</h3>
		<p style="margin: 0 0 20px; font-size: 13px; color: {theme.c.textSecondary}; line-height: 1.5;">
			SamWise will keep running in the system tray. Right-click the tray icon to quit completely.
		</p>
		<div style="display: flex; gap: 10px; justify-content: center;">
			<button
				onclick={cancelClose}
				style="
					padding: 8px 20px; border-radius: 10px; border: 1px solid {theme.c.borderSubtle};
					background: {theme.c.bgElevated}; color: {theme.c.textSecondary};
					font-size: 13px; font-weight: 600; cursor: pointer;
					font-family: var(--font-ui); transition: all 0.15s ease;
				"
			>
				Cancel
			</button>
			<button
				onclick={confirmClose}
				style="
					padding: 8px 20px; border-radius: 10px; border: none;
					background: linear-gradient(135deg, {theme.c.accentAmber}, {theme.c.accentRed});
					color: #fff; font-size: 13px; font-weight: 600; cursor: pointer;
					font-family: var(--font-ui); transition: all 0.15s ease;
					box-shadow: 0 4px 15px rgba(245, 158, 11, 0.3);
				"
			>
				Minimize to Tray
			</button>
		</div>
	</div>
</div>
{/if}
