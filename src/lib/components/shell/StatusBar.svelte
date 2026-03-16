<script lang="ts">
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';

	const layout = getLayout();
	const workspace = getWorkspace();
	const settingsStore = getSettingsStore();

	let termBtnHovered = $state(false);
	let leftPanelBtnHovered = $state(false);
	let rightPanelBtnHovered = $state(false);
	let gearHovered = $state(false);
	let aiSettingsHovered = $state(false);
</script>

<div class="statusbar" style="display: flex; align-items: center; height: 28px; padding: 0 14px; background: linear-gradient(0deg, #0E1218 0%, #141920 100%); box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.04); font-size: 11px; font-family: var(--font-mono); gap: 12px; position: relative; z-index: 5;">
	<!-- Left section -->
	<div style="display: flex; align-items: center; gap: 10px; flex: 1;">
		<!-- Git branch -->
		<div style="display: flex; align-items: center; gap: 4px; color: var(--text-secondary);">
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
				<path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.492 2.492 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25z"/>
			</svg>
			<span>main</span>
		</div>

		{#if workspace.isOpen}
			<span style="color: var(--text-muted);">|</span>
			<span style="color: var(--text-muted);">0 files</span>
		{/if}

		<!-- Left panel (Agent) toggle -->
		<button
			title="Toggle Agent Panel (Ctrl+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {leftPanelBtnHovered ? 'var(--banana-yellow)' : layout.leftPanelVisible ? 'var(--text-secondary)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {leftPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'}; opacity: {layout.leftPanelVisible ? 1 : 0.6};"
			onclick={() => layout.toggleLeftPanel()}
			onmouseenter={() => leftPanelBtnHovered = true}
			onmouseleave={() => leftPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
				<path d="M1.5 2A1.5 1.5 0 000 3.5v9A1.5 1.5 0 001.5 14h13a1.5 1.5 0 001.5-1.5v-9A1.5 1.5 0 0014.5 2h-13zM1 3.5a.5.5 0 01.5-.5H6v10H1.5a.5.5 0 01-.5-.5v-9zM7 13V3h7.5a.5.5 0 01.5.5v9a.5.5 0 01-.5.5H7z"/>
			</svg>
			Agent
		</button>

		<!-- Terminal toggle -->
		<button
			title="Toggle Terminal (Ctrl+`)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {termBtnHovered ? 'var(--banana-yellow)' : layout.terminalVisible ? 'var(--text-secondary)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {termBtnHovered ? 'scale(1.05)' : 'scale(1)'}; opacity: {layout.terminalVisible ? 1 : 0.6};"
			onclick={() => layout.toggleTerminal()}
			onmouseenter={() => termBtnHovered = true}
			onmouseleave={() => termBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
				<path d="M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25ZM7 11a.75.75 0 0 1 0 1.5H4a.75.75 0 0 1 0-1.5Zm1.586-4.586a.75.75 0 0 1 0 1.06l-2 2a.75.75 0 1 1-1.06-1.06L6.94 7 5.526 5.586a.75.75 0 0 1 1.06-1.06Z"/>
			</svg>
			Terminal
		</button>

		<!-- Right panel (Files) toggle -->
		<button
			title="Toggle Files Panel (Ctrl+Shift+B)"
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {rightPanelBtnHovered ? 'var(--banana-yellow)' : layout.rightPanelVisible ? 'var(--text-secondary)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {rightPanelBtnHovered ? 'scale(1.05)' : 'scale(1)'}; opacity: {layout.rightPanelVisible ? 1 : 0.6};"
			onclick={() => layout.toggleRightPanel()}
			onmouseenter={() => rightPanelBtnHovered = true}
			onmouseleave={() => rightPanelBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
				<path d="M1.5 2A1.5 1.5 0 000 3.5v9A1.5 1.5 0 001.5 14h13a1.5 1.5 0 001.5-1.5v-9A1.5 1.5 0 0014.5 2h-13zM1 3.5a.5.5 0 01.5-.5H9v10H1.5a.5.5 0 01-.5-.5v-9zM10 13V3h4.5a.5.5 0 01.5.5v9a.5.5 0 01-.5.5H10z"/>
			</svg>
			Files
		</button>
	</div>

	<!-- Right section -->
	<div style="display: flex; align-items: center; gap: 10px;">
		<!-- Gear icon (settings) -->
		<button
			title="Settings (Ctrl+,)"
			aria-label="Settings"
			style="display: flex; align-items: center; background: none; border: none; color: {gearHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; cursor: pointer; padding: 0; transition: color 0.12s ease; transform: {gearHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => settingsStore.settingsVisible = !settingsStore.settingsVisible}
			onmouseenter={() => gearHovered = true}
			onmouseleave={() => gearHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"/>
				<path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
			</svg>
		</button>

		<!-- AI settings (sparkle) - opens settings modal to AI tab -->
		<button
			title="AI & Tools Settings"
			aria-label="AI Settings"
			style="display: flex; align-items: center; background: none; border: none; color: {aiSettingsHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; cursor: pointer; padding: 0; transition: color 0.12s ease; transform: {aiSettingsHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => { settingsStore.activeSettingsTab = 'ai'; settingsStore.settingsVisible = true; }}
			onmouseenter={() => aiSettingsHovered = true}
			onmouseleave={() => aiSettingsHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
				<path d="M12 2L9 12l-7 4 7 4 3 10 3-10 7-4-7-4z"/>
			</svg>
		</button>

		<div style="display: flex; align-items: center; gap: 4px;">
			<span style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-green); display: inline-block; animation: pulse-dot 2s ease-in-out infinite; box-shadow: 0 0 6px rgba(63, 185, 80, 0.4);"></span>
			<span style="color: var(--text-muted);">Ready</span>
		</div>
		<span style="color: var(--banana-yellow); font-weight: 600; font-size: 9px; background: rgba(255, 214, 10, 0.1); padding: 1px 8px; border-radius: 8px; box-shadow: 0 0 8px rgba(255, 214, 10, 0.1);">FREE</span>
	</div>
</div>
