<script lang="ts">
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';

	const layout = getLayout();
	const taskStore = getTaskStore();
	const worker = getWorkerStore();
	const settingsStore = getSettingsStore();

	let leftPanelBtnHovered = $state(false);
	let rightPanelBtnHovered = $state(false);
	let gearHovered = $state(false);
</script>

<div class="statusbar" style="
	display: flex; align-items: center; height: 28px; padding: 0 14px;
	background: linear-gradient(0deg, #0d1117 0%, #0f1419 100%);
	box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.03);
	font-size: 11px; font-family: var(--font-mono); gap: 12px;
	position: relative; z-index: 5;
">
	<!-- Left section -->
	<div style="display: flex; align-items: center; gap: 10px; flex: 1;">
		<!-- Kanban panel toggle -->
		<button
			title="Toggle Kanban (Ctrl+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {leftPanelBtnHovered ? 'var(--accent-indigo)' : layout.leftPanelVisible ? 'var(--text-secondary)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {leftPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'}; opacity: {layout.leftPanelVisible ? 1 : 0.6};"
			onclick={() => layout.toggleLeftPanel()}
			onmouseenter={() => leftPanelBtnHovered = true}
			onmouseleave={() => leftPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
			</svg>
			Tasks
		</button>

		<!-- Automation panel toggle (opens Settings > Automation tab) -->
		<button
			title="Toggle Automation (Ctrl+Shift+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {rightPanelBtnHovered ? 'var(--accent-indigo)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {rightPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => { settingsStore.activeSettingsTab = 'automation'; settingsStore.settingsVisible = true; }}
			onmouseenter={() => rightPanelBtnHovered = true}
			onmouseleave={() => rightPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="3"/><path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
			</svg>
			Automation
		</button>

		<span style="color: var(--text-muted);">|</span>

		<!-- Task counts -->
		<div style="display: flex; align-items: center; gap: 6px; font-size: 10px;">
			<span style="color: var(--text-muted);">
				<span style="color: var(--accent-indigo); font-weight: 600;">{taskStore.taskCounts.inProgress}</span> in progress
			</span>
			<span style="color: var(--text-muted);">
				<span style="color: var(--accent-amber); font-weight: 600;">{taskStore.taskCounts.testing}</span> testing
			</span>
			<span style="color: var(--text-muted);">
				<span style="color: var(--accent-green); font-weight: 600;">{taskStore.taskCounts.review}</span> review
			</span>
		</div>
	</div>

	<!-- Right section -->
	<div style="display: flex; align-items: center; gap: 10px;">
		<!-- Gear icon (settings) -->
		<button
			title="Settings (Ctrl+,)"
			aria-label="Settings"
			style="display: flex; align-items: center; background: none; border: none; color: {gearHovered ? 'var(--accent-indigo)' : 'var(--text-secondary)'}; cursor: pointer; padding: 0; transition: color 0.12s ease; transform: {gearHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => settingsStore.settingsVisible = !settingsStore.settingsVisible}
			onmouseenter={() => gearHovered = true}
			onmouseleave={() => gearHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"/>
				<path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
			</svg>
		</button>

		<!-- Worker status -->
		<div style="display: flex; align-items: center; gap: 4px;">
			<span style="
				width: 6px; height: 6px; border-radius: 50%;
				background: {worker.statusColor};
				display: inline-block;
				animation: pulse-dot 2s ease-in-out infinite;
				box-shadow: 0 0 6px {worker.statusColor}60;
			"></span>
			<span style="color: var(--text-muted);">{worker.statusLabel}</span>
		</div>
	</div>
</div>
