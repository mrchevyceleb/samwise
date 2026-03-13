<script lang="ts">
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';

	const layout = getLayout();
	const workspace = getWorkspace();

	let termBtnHovered = $state(false);
</script>

<div class="statusbar" style="display: flex; align-items: center; height: 24px; padding: 0 12px; background: var(--bg-surface); border-top: 1px solid var(--border-default); font-size: 11px; font-family: var(--font-mono); gap: 12px;">
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

		<!-- Terminal toggle -->
		<button
			style="display: flex; align-items: center; gap: 4px; background: none; border: none; color: {termBtnHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; cursor: pointer; font-family: var(--font-mono); font-size: 11px; transition: color 0.12s ease; transform: {termBtnHovered ? 'scale(1.05)' : 'scale(1)'};"
			onclick={() => layout.toggleTerminal()}
			onmouseenter={() => termBtnHovered = true}
			onmouseleave={() => termBtnHovered = false}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
				<path d="M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25ZM7 11a.75.75 0 0 1 0 1.5H4a.75.75 0 0 1 0-1.5Zm1.586-4.586a.75.75 0 0 1 0 1.06l-2 2a.75.75 0 1 1-1.06-1.06L6.94 7 5.526 5.586a.75.75 0 0 1 1.06-1.06Z"/>
			</svg>
			Terminal
		</button>
	</div>

	<!-- Right section -->
	<div style="display: flex; align-items: center; gap: 10px;">
		<div style="display: flex; align-items: center; gap: 4px;">
			<span style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-green); display: inline-block; animation: pulse-dot 2s ease-in-out infinite;"></span>
			<span style="color: var(--text-muted);">Ready</span>
		</div>
		<span style="color: var(--banana-yellow-dim); font-weight: 600;">FREE</span>
	</div>
</div>
