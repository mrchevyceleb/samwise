<script lang="ts">
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';

	const layout = getLayout();
	const taskStore = getTaskStore();
	const worker = getWorkerStore();
	const settingsStore = getSettingsStore();
	const theme = getTheme();

	let leftPanelBtnHovered = $state(false);
	let rightPanelBtnHovered = $state(false);
	let gearHovered = $state(false);
	let themeHovered = $state(false);
</script>

<div class="statusbar" style="
	display: flex; align-items: center; height: 32px; padding: 0 14px;
	background: {theme.c.gradientStatusbar};
	box-shadow: {theme.c.shadowStatusbar};
	font-size: 12px; font-family: var(--font-mono); gap: 12px;
	position: relative; z-index: 5;
">
	<!-- Left section -->
	<div style="display: flex; align-items: center; gap: 10px; flex: 1;">
		<button
			title="Toggle Kanban (Ctrl+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {leftPanelBtnHovered ? theme.c.accentIndigo : layout.leftPanelVisible ? theme.c.textSecondary : theme.c.textMuted}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {leftPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'}; opacity: {layout.leftPanelVisible ? 1 : 0.6};"
			onclick={() => layout.toggleLeftPanel()}
			onmouseenter={() => leftPanelBtnHovered = true}
			onmouseleave={() => leftPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
			</svg>
			Tasks
		</button>

		<button
			title="Toggle Automation (Ctrl+Shift+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {rightPanelBtnHovered ? theme.c.accentIndigo : theme.c.textMuted}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {rightPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => { settingsStore.activeSettingsTab = 'automation'; settingsStore.settingsVisible = true; }}
			onmouseenter={() => rightPanelBtnHovered = true}
			onmouseleave={() => rightPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="3"/><path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
			</svg>
			Automation
		</button>

		<span style="color: {theme.c.textMuted};">|</span>

		<div style="display: flex; align-items: center; gap: 6px; font-size: 10px;">
			<span style="color: {theme.c.textMuted};">
				<span style="color: {theme.c.accentIndigo}; font-weight: 600;">{taskStore.taskCounts.inProgress}</span> in progress
			</span>
			<span style="color: {theme.c.textMuted};">
				<span style="color: {theme.c.accentAmber}; font-weight: 600;">{taskStore.taskCounts.testing}</span> testing
			</span>
			<span style="color: {theme.c.textMuted};">
				<span style="color: {theme.c.accentGreen}; font-weight: 600;">{taskStore.taskCounts.review}</span> review
			</span>
		</div>
	</div>

	<!-- Right section -->
	<div style="display: flex; align-items: center; gap: 10px;">
		<!-- Theme toggle -->
		<button
			title="Toggle theme"
			aria-label="Toggle light/dark mode"
			style="display: flex; align-items: center; background: none; border: none; color: {themeHovered ? theme.c.accentAmber : theme.c.textSecondary}; cursor: pointer; padding: 0; transition: all 0.2s ease; transform: {themeHovered ? 'scale(1.15) rotate(15deg)' : 'scale(1)'};"
			onclick={() => theme.toggle()}
			onmouseenter={() => themeHovered = true}
			onmouseleave={() => themeHovered = false}
		>
			{#if theme.isDark}
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
					<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
				</svg>
			{:else}
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
					<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
				</svg>
			{/if}
		</button>

		<button
			title="Settings (Ctrl+,)"
			aria-label="Settings"
			style="display: flex; align-items: center; background: none; border: none; color: {gearHovered ? theme.c.accentIndigo : theme.c.textSecondary}; cursor: pointer; padding: 0; transition: color 0.12s ease; transform: {gearHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => settingsStore.settingsVisible = !settingsStore.settingsVisible}
			onmouseenter={() => gearHovered = true}
			onmouseleave={() => gearHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"/>
				<path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
			</svg>
		</button>

		<div style="display: flex; align-items: center; gap: 4px;">
			<span style="
				width: 6px; height: 6px; border-radius: 50%;
				background: {worker.statusColor};
				display: inline-block;
				animation: pulse-dot 2s ease-in-out infinite;
				box-shadow: 0 0 6px {worker.statusColor}60;
			"></span>
			<span style="color: {theme.c.textMuted};">{worker.statusLabel}</span>
		</div>
	</div>
</div>
