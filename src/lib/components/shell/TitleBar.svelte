<script lang="ts">
	import { getWorkspace } from '$lib/stores/workspace.svelte';

	const workspace = getWorkspace();

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

<div class="titlebar" data-tauri-drag-region style="display: flex; align-items: center; height: 38px; padding: 0 10px; background: var(--bg-surface); border-bottom: 1px solid var(--border-default); gap: 0;">
	<!-- Left: Logo + Brand -->
	<div style="display: flex; align-items: center; gap: 8px; min-width: 180px;">
		<span class="banana-logo" style="font-size: 20px; animation: bob 3s ease-in-out infinite;">🍌</span>
		<span style="font-family: var(--font-ui); font-weight: 700; font-size: 14px; color: var(--banana-yellow); letter-spacing: -0.3px;">Banana Code</span>
		<span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">v0.1</span>
	</div>

	<!-- Center: Workspace name -->
	<div data-tauri-drag-region style="flex: 1; text-align: center; font-size: 12px; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
		{workspace.name}
	</div>

	<!-- Right: Window controls -->
	<div style="display: flex; align-items: center; gap: 2px; min-width: 100px; justify-content: flex-end;">
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {minimizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 4px; cursor: pointer; font-size: 16px; transition: all 0.12s ease;"
			onmouseenter={() => minimizeHovered = true}
			onmouseleave={() => minimizeHovered = false}
			onclick={minimize}
			aria-label="Minimize"
		>
			<svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1" rx="0.5"/></svg>
		</button>
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {maximizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 4px; cursor: pointer; font-size: 16px; transition: all 0.12s ease;"
			onmouseenter={() => maximizeHovered = true}
			onmouseleave={() => maximizeHovered = false}
			onclick={maximize}
			aria-label="Maximize"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9" rx="1"/></svg>
		</button>
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {closeHovered ? 'var(--accent-red)' : 'transparent'}; color: {closeHovered ? '#fff' : 'var(--text-secondary)'}; border-radius: 4px; cursor: pointer; font-size: 16px; transition: all 0.12s ease;"
			onmouseenter={() => closeHovered = true}
			onmouseleave={() => closeHovered = false}
			onclick={close}
			aria-label="Close"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.3"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
		</button>
	</div>
</div>
