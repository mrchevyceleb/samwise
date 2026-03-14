<script lang="ts">
	import { getPreviewStore } from '$lib/stores/preview.svelte';
	const preview = getPreviewStore();

	let refreshHovered = $state(false);
	let viewportHovered = $state(false);
	let envHovered = $state(false);
	let selectedViewport = $state('Desktop');
	const viewports = ['Desktop', 'Tablet', 'Mobile'];
	let dropdownOpen = $state(false);

	let envCount = $derived(preview.envVars.filter(v => v.key.trim()).length);

	function displayUrl(): string {
		if (preview.url) {
			return preview.url.replace('http://', '');
		}
		if (preview.status === 'loading') return 'Loading...';
		if (preview.status === 'error') return 'Error';
		return 'No preview';
	}

	async function handleRefresh() {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('reload_preview_webview');
		} catch {
			// Webview might not exist
		}
	}
</script>

<div style="display: flex; align-items: center; height: 36px; padding: 0 10px; border-bottom: 1px solid var(--border-default); background: var(--bg-surface); gap: 6px;">
	<!-- Live indicator when ready -->
	{#if preview.status === 'ready'}
		<div style="display: flex; align-items: center; gap: 4px; height: 20px; padding: 0 6px; background: rgba(72, 199, 142, 0.12); border-radius: 4px; font-size: 10px; font-weight: 600; color: #48c78e; white-space: nowrap; letter-spacing: 0.3px; text-transform: uppercase;">
			<div style="width: 6px; height: 6px; border-radius: 50%; background: #48c78e; animation: glow 2s ease-in-out infinite;"></div>
			Live
		</div>
	{/if}

	<!-- URL bar -->
	<div style="flex: 1; display: flex; align-items: center; height: 26px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 6px; padding: 0 8px; gap: 6px;">
		<svg width="12" height="12" viewBox="0 0 16 16" fill="var(--text-muted)">
			<path d="M8 0C3.58 0 0 3.58 0 8s3.58 8 8 8 8-3.58 8-8-3.58-8-8-8zm6.5 8a6.47 6.47 0 0 1-.87 3.25l-.15-.22c-.22-.34-.52-.63-.85-.85l-.15-.1a5.03 5.03 0 0 0 .92-2.08h1.1zm-1.1-1h-1.1a5.03 5.03 0 0 0-.92-2.08l.15-.1c.33-.22.63-.51.85-.85l.15-.22A6.47 6.47 0 0 1 14.5 7z"/>
		</svg>
		<input
			type="text"
			value={displayUrl()}
			readonly
			style="flex: 1; background: none; border: none; outline: none; color: var(--text-secondary); font-family: var(--font-mono); font-size: 11px; cursor: default;"
		/>
	</div>

	<!-- Refresh button -->
	<button
		style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; background: {refreshHovered ? 'var(--bg-elevated)' : 'transparent'}; border: none; border-radius: 6px; cursor: pointer; color: {refreshHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; transition: all 0.12s ease; transform: {refreshHovered ? 'rotate(45deg) scale(1.1)' : 'rotate(0deg) scale(1)'};"
		onmouseenter={() => refreshHovered = true}
		onmouseleave={() => refreshHovered = false}
		onclick={handleRefresh}
		aria-label="Refresh"
	>
		<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
			<path d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2v1z"/>
			<path d="M8 0a.5.5 0 0 1 .5.5v3a.5.5 0 0 1-1 0v-3A.5.5 0 0 1 8 0z"/>
			<path d="M8 0a.5.5 0 0 1 .354.146l2 2a.5.5 0 0 1-.708.708L8 1.207 6.354 2.854a.5.5 0 1 1-.708-.708l2-2A.5.5 0 0 1 8 0z"/>
		</svg>
	</button>

	<!-- Env vars button -->
	<button
		style="display: flex; align-items: center; gap: 3px; height: 28px; padding: 0 8px; background: {envHovered || preview.envPanelOpen ? 'var(--bg-elevated)' : 'transparent'}; border: 1px solid {preview.envPanelOpen ? 'var(--banana-yellow)' : 'transparent'}; border-radius: 6px; cursor: pointer; color: {envHovered || preview.envPanelOpen ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; transition: all 0.12s ease; position: relative;"
		onmouseenter={() => envHovered = true}
		onmouseleave={() => envHovered = false}
		onclick={() => preview.envPanelOpen = !preview.envPanelOpen}
		aria-label="Environment variables"
	>
		<svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
			<path d="M8 1a2 2 0 0 1 2 2v4H6V3a2 2 0 0 1 2-2zm3 6V3a3 3 0 0 0-6 0v4a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2zM5 9h6a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z"/>
		</svg>
		{#if envCount > 0}
			<span style="font-family: var(--font-mono); font-size: 9px; min-width: 14px; height: 14px; display: flex; align-items: center; justify-content: center; background: var(--banana-yellow); color: #0D1117; border-radius: 7px; font-weight: 700;">{envCount}</span>
		{/if}
	</button>

	<!-- Viewport selector -->
	<div style="position: relative;">
		<button
			style="display: flex; align-items: center; gap: 4px; height: 26px; padding: 0 8px; background: {viewportHovered ? 'var(--bg-elevated)' : 'transparent'}; border: 1px solid {viewportHovered ? 'var(--border-bright)' : 'var(--border-default)'}; border-radius: 6px; color: {viewportHovered ? 'var(--text-primary)' : 'var(--text-secondary)'}; cursor: pointer; font-family: var(--font-ui); font-size: 11px; transition: all 0.12s ease;"
			onmouseenter={() => viewportHovered = true}
			onmouseleave={() => viewportHovered = false}
			onclick={() => dropdownOpen = !dropdownOpen}
		>
			{selectedViewport}
			<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" style="transform: {dropdownOpen ? 'rotate(180deg)' : 'rotate(0)'}; transition: transform 0.15s ease;">
				<path d="M1 2.5l3 3 3-3"/>
			</svg>
		</button>

		{#if dropdownOpen}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				style="position: absolute; top: 30px; right: 0; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 8px; overflow: hidden; z-index: 100; min-width: 100px; box-shadow: 0 8px 24px rgba(0,0,0,0.4);"
				onmouseleave={() => dropdownOpen = false}
			>
				{#each viewports as vp}
					<button
						style="display: block; width: 100%; text-align: left; padding: 6px 12px; background: {vp === selectedViewport ? 'rgba(255, 214, 10, 0.1)' : 'transparent'}; border: none; color: {vp === selectedViewport ? 'var(--banana-yellow)' : 'var(--text-primary)'}; cursor: pointer; font-family: var(--font-ui); font-size: 12px; transition: background 0.1s ease;"
						onclick={() => { selectedViewport = vp; dropdownOpen = false; }}
						onmouseenter={(e) => { if (vp !== selectedViewport) (e.currentTarget as HTMLElement).style.background = 'var(--bg-surface)'; }}
						onmouseleave={(e) => { if (vp !== selectedViewport) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
					>
						{vp}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	@keyframes glow {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
</style>
