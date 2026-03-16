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

<div class="titlebar" data-tauri-drag-region style="display: flex; align-items: center; height: 40px; padding: 0 14px; background: linear-gradient(180deg, #181E28 0%, #12171F 100%); box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.04); gap: 0; position: relative; z-index: 5; border-bottom: 1px solid rgba(255, 214, 10, 0.06);">
	<!-- Left: Logo + Brand -->
	<div style="display: flex; align-items: center; gap: 8px; min-width: 180px;">
		<span class="banana-logo" style="font-size: 22px; animation: bob 3s ease-in-out infinite; filter: drop-shadow(0 0 10px rgba(255, 214, 10, 0.4));">🍌</span>
		<span style="font-family: var(--font-ui); font-weight: 700; font-size: 15px; color: var(--banana-yellow); letter-spacing: -0.3px; text-shadow: 0 0 20px rgba(255, 214, 10, 0.25);">Banana Code</span>
		<span style="font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); background: rgba(255, 214, 10, 0.06); padding: 1px 6px; border-radius: 6px;">v0.1</span>
	</div>

	<!-- Center: Workspace name -->
	<div data-tauri-drag-region style="flex: 1; text-align: center; font-size: 12px; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
		{workspace.name}
	</div>

	<!-- Right: Window controls -->
	<div style="display: flex; align-items: center; gap: 2px; min-width: 100px; justify-content: flex-end;">
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {minimizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => minimizeHovered = true}
			onmouseleave={() => minimizeHovered = false}
			onclick={minimize}
			aria-label="Minimize"
		>
			<svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1" rx="0.5"/></svg>
		</button>
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {maximizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => maximizeHovered = true}
			onmouseleave={() => maximizeHovered = false}
			onclick={maximize}
			aria-label="Maximize"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9" rx="1"/></svg>
		</button>
		<button
			class="win-btn"
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
