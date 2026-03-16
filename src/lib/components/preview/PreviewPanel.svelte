<script lang="ts">
	import PreviewToolbar from './PreviewToolbar.svelte';
	import EnvVarsPanel from './EnvVarsPanel.svelte';
	import PreviewPlaceholder from './PreviewPlaceholder.svelte';
	import PreviewLoading from './PreviewLoading.svelte';
	import { getPreviewStore } from '$lib/stores/preview.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	const preview = getPreviewStore();
	const workspace = getWorkspace();

	let showOverlay = $derived(preview.status === 'ready' && preview.missingSecretsOverlay);
	let bananaRotation = $state(0);

	// Gentle banana wobble animation
	$effect(() => {
		if (!showOverlay) return;
		const interval = setInterval(() => {
			bananaRotation = Math.sin(Date.now() / 600) * 12;
		}, 50);
		return () => clearInterval(interval);
	});

	function openSecretsPanel() {
		preview.envPanelOpen = true;
		preview.missingSecretsOverlay = false;
		preview.envSetupPending = true;
	}

	function skipOverlay() {
		preview.missingSecretsOverlay = false;
		preview.envSetupPending = false;
	}

	let containerRef = $state<HTMLDivElement | null>(null);
	let webviewCreated = $state(false);
	let appliedUrl = $state('');
	let appliedSession = $state(-1);
	let closingWebview = $state(false);

	// Watch for workspace changes and auto-open preview
	$effect(() => {
		const path = workspace.path;
		if (path) {
			webviewCreated = false;
			appliedUrl = '';
			appliedSession = -1;
			preview.openProject(path);
		} else {
			preview.stop();
			webviewCreated = false;
			appliedUrl = '';
			appliedSession = -1;
		}
	});

	// Reset webview state when preview goes back to loading (e.g. restart after Doppler sync)
	$effect(() => {
		if (preview.status === 'loading' && webviewCreated) {
			// Close the old webview and wait for it before allowing a new one
			closingWebview = true;
			webviewCreated = false;
			appliedUrl = '';
			appliedSession = -1;
			(async () => {
				try {
					const { invoke } = await import('@tauri-apps/api/core');
					await invoke('close_preview_webview');
				} catch { /* may not exist */ }
				closingWebview = false;
			})();
		}
	});

	// Watch for URL/session becoming available and create or navigate the webview
	// DON'T act if the overlay is showing, env setup is in progress, or a close is pending
	$effect(() => {
		const currentUrl = preview.url;
		const overlayActive = preview.missingSecretsOverlay;
		const setupPending = preview.envSetupPending;
		const session = preview.sessionKey;
		const closing = closingWebview;

		if (!currentUrl || !containerRef || overlayActive || setupPending || closing) return;

		if (!webviewCreated) {
			createWebview(currentUrl, session);
		} else if (currentUrl !== appliedUrl || session !== appliedSession) {
			navigateWebview(currentUrl, session);
		}
	});

	async function createWebview(url: string, session: number) {
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
			appliedUrl = url;
			appliedSession = session;
		} catch (e) {
			console.error('[preview] Failed to create webview:', e);
		}
	}

	async function navigateWebview(url: string, session: number) {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('navigate_preview_webview', { url });
			appliedUrl = url;
			appliedSession = session;
		} catch (e) {
			console.error('[preview] Failed to navigate webview:', e);
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
	<EnvVarsPanel />
	<div
		bind:this={containerRef}
		style="flex: 1; overflow: hidden; position: relative;"
	>
		{#if showOverlay}
			<!-- Friendly "missing secrets" overlay - shown instead of the webview -->
			<div class="secrets-overlay">
				<div class="secrets-card">
					<div class="banana-wobble" style="transform: rotate({bananaRotation}deg);">
						🍌
					</div>
					<h2 class="secrets-title">Almost ready to peel!</h2>
					<p class="secrets-subtitle">Your app needs a few secrets to get started.</p>

					{#if preview.suggestedKeys.length > 0}
						<div class="secrets-keys">
							<p class="keys-label">We found these in your project:</p>
							<ul class="keys-list">
								{#each preview.suggestedKeys as key}
									<li class="key-item">
										<span class="key-bullet">&#x2022;</span>
										<code class="key-name">{key}</code>
									</li>
								{/each}
							</ul>
						</div>
					{/if}

					<button class="secrets-btn" onclick={openSecretsPanel}>
						Open Secrets Panel
					</button>

					<div class="arrow-hint">
						<div class="arrow-up">&#x2191;</div>
					</div>

					<button class="skip-link" onclick={skipOverlay}>
						Skip, I don't need env vars
					</button>
				</div>
			</div>
		{:else if preview.status === 'idle'}
			<PreviewPlaceholder />
		{:else if preview.status === 'loading'}
			<PreviewLoading />
		{:else if preview.status === 'error'}
			<div class="preview-error">
				<div class="error-icon">!</div>
				<p class="error-title">Something went wrong</p>
				<p class="error-detail">{preview.error}</p>
				<button
					class="retry-btn"
					onclick={() => workspace.path && preview.openProject(workspace.path)}
				>
					Retry
				</button>
			</div>
		{:else if preview.status === 'ready' && !webviewCreated}
			<PreviewLoading />
		{/if}
	</div>
</div>

<style>
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

	/* Missing Secrets Overlay */
	.secrets-overlay {
		position: absolute;
		inset: 0;
		z-index: 100;
		background: var(--bg-primary, #1a1a1a);
		display: flex;
		align-items: center;
		justify-content: center;
		animation: fadeIn 0.3s ease;
	}

	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	.secrets-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		padding: 40px 36px;
		max-width: 380px;
		text-align: center;
	}

	.banana-wobble {
		font-size: 48px;
		transition: transform 0.1s ease;
		user-select: none;
	}

	.secrets-title {
		font-size: 18px;
		font-weight: 700;
		color: var(--text-primary, #f0f0f0);
		margin: 0;
	}

	.secrets-subtitle {
		font-size: 13px;
		color: var(--text-muted, #888);
		margin: 0;
		line-height: 1.5;
	}

	.secrets-keys {
		width: 100%;
		background: rgba(255, 255, 255, 0.04);
		border-radius: 8px;
		padding: 12px 16px;
		margin-top: 4px;
	}

	.keys-label {
		font-size: 11px;
		color: var(--text-muted, #888);
		margin: 0 0 8px 0;
		text-align: left;
	}

	.keys-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.key-item {
		display: flex;
		align-items: center;
		gap: 8px;
		text-align: left;
	}

	.key-bullet {
		color: var(--banana-yellow, #ffd60a);
		font-size: 14px;
	}

	.key-name {
		font-size: 12px;
		font-family: 'JetBrains Mono', monospace;
		color: var(--text-primary, #f0f0f0);
		background: rgba(255, 255, 255, 0.06);
		padding: 2px 6px;
		border-radius: 4px;
	}

	.secrets-btn {
		margin-top: 8px;
		padding: 10px 24px;
		background: var(--banana-yellow, #ffd60a);
		color: #1a1a1a;
		border: none;
		border-radius: 8px;
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		transition: transform 0.15s ease, box-shadow 0.15s ease;
	}

	.secrets-btn:hover {
		transform: scale(1.06);
		box-shadow: 0 6px 20px rgba(255, 214, 10, 0.35);
	}

	.secrets-btn:active {
		transform: scale(0.97);
	}

	.arrow-hint {
		margin-top: 4px;
		animation: arrowBounce 1.2s ease-in-out infinite;
	}

	.arrow-up {
		font-size: 20px;
		color: var(--banana-yellow, #ffd60a);
		opacity: 0.7;
	}

	@keyframes arrowBounce {
		0%, 100% { transform: translateY(0); }
		50% { transform: translateY(-6px); }
	}

	.skip-link {
		background: none;
		border: none;
		color: var(--text-muted, #888);
		font-size: 11px;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 4px;
		transition: color 0.15s ease;
	}

	.skip-link:hover {
		color: var(--text-primary, #f0f0f0);
	}

</style>
