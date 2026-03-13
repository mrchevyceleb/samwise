<script lang="ts">
	import PreviewToolbar from './PreviewToolbar.svelte';
	import PreviewPlaceholder from './PreviewPlaceholder.svelte';
	import { getPreviewStore } from '$lib/stores/preview.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	const preview = getPreviewStore();
	const workspace = getWorkspace();

	let containerRef = $state<HTMLDivElement | null>(null);
	let webviewCreated = $state(false);

	// Watch for workspace changes and auto-open preview
	$effect(() => {
		const path = workspace.path;
		if (path) {
			preview.openProject(path);
		} else {
			preview.stop();
			webviewCreated = false;
		}
	});

	// Watch for URL becoming available and create the webview
	$effect(() => {
		const currentUrl = preview.url;
		if (currentUrl && containerRef && !webviewCreated) {
			createWebview(currentUrl);
		}
	});

	async function createWebview(url: string) {
		if (!containerRef) return;

		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const rect = containerRef.getBoundingClientRect();
			const scaleFactor = window.devicePixelRatio || 1;

			await invoke('create_preview_webview', {
				url,
				bounds: {
					x: rect.left,
					y: rect.top,
					width: rect.width,
					height: rect.height,
					scaleFactor
				}
			});
			webviewCreated = true;
		} catch (e) {
			console.error('[preview] Failed to create webview:', e);
		}
	}

	// Update webview bounds on resize
	$effect(() => {
		if (!containerRef || !webviewCreated) return;

		const observer = new ResizeObserver(async (entries) => {
			const { invoke } = await import('@tauri-apps/api/core');
			for (const entry of entries) {
				const rect = entry.target.getBoundingClientRect();
				const scaleFactor = window.devicePixelRatio || 1;
				try {
					await invoke('set_preview_bounds', {
						bounds: {
							x: rect.left,
							y: rect.top,
							width: rect.width,
							height: rect.height,
							scaleFactor
						}
					});
				} catch {
					// Webview might not exist
				}
			}
		});

		observer.observe(containerRef);
		return () => observer.disconnect();
	});
</script>

<div style="display: flex; flex-direction: column; height: 100%; background: var(--bg-primary);">
	<PreviewToolbar />
	<div
		bind:this={containerRef}
		style="flex: 1; overflow: hidden; position: relative;"
	>
		{#if preview.status === 'idle'}
			<PreviewPlaceholder />
		{:else if preview.status === 'detecting'}
			<div class="preview-loading">
				<div class="spinner"></div>
				<p class="loading-title">Detecting project type...</p>
			</div>
		{:else if preview.status === 'building'}
			<div class="preview-loading">
				<div class="spinner"></div>
				<p class="loading-title">
					{#if preview.tier === 'esbuild'}
						Bundling with esbuild...
					{:else if preview.tier === 'managed'}
						Starting dev server...
					{:else}
						Starting preview...
					{/if}
				</p>
				{#if preview.framework}
					<p class="loading-detail">{preview.framework} project detected</p>
				{/if}
			</div>
		{:else if preview.status === 'error'}
			<div class="preview-error">
				<div class="error-icon">!</div>
				<p class="error-title">Preview failed</p>
				<p class="error-detail">{preview.error}</p>
				<button
					class="retry-btn"
					onclick={() => workspace.path && preview.openProject(workspace.path)}
				>
					Retry
				</button>
			</div>
		{:else if preview.status === 'ready' && !webviewCreated}
			<div class="preview-loading">
				<div class="spinner"></div>
				<p class="loading-title">Loading preview...</p>
			</div>
		{/if}
		<!-- When status is 'ready' and webviewCreated, the native webview is rendered in this container -->
	</div>
</div>

<style>
	.preview-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 12px;
		padding: 32px;
	}

	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--border-default);
		border-top-color: var(--banana-yellow);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	.loading-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.loading-detail {
		font-size: 12px;
		color: var(--text-muted);
	}

	.preview-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 10px;
		padding: 32px;
	}

	.error-icon {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		background: rgba(255, 80, 80, 0.15);
		color: #ff5050;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 20px;
		font-weight: 700;
	}

	.error-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.error-detail {
		font-size: 12px;
		color: var(--text-muted);
		max-width: 320px;
		text-align: center;
		line-height: 1.5;
		word-break: break-word;
	}

	.retry-btn {
		margin-top: 8px;
		padding: 6px 16px;
		background: var(--banana-yellow);
		color: #1a1a1a;
		border: none;
		border-radius: 6px;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: transform 0.12s ease, box-shadow 0.12s ease;
	}

	.retry-btn:hover {
		transform: scale(1.05);
		box-shadow: 0 4px 12px rgba(255, 214, 10, 0.3);
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
