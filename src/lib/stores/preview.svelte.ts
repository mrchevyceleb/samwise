/** Preview store using Svelte 5 runes */

async function getInvoke() {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke;
}

async function getListen() {
	const { listen } = await import('@tauri-apps/api/event');
	return listen;
}

export type PreviewTier = 'direct' | 'esbuild' | 'managed' | null;
export type PreviewStatus = 'idle' | 'detecting' | 'installing' | 'building' | 'ready' | 'error';

interface TierDetection {
	tier: 'direct_serve' | 'esbuild_bundle' | 'managed_process';
	framework: string | null;
	entry_point: string | null;
	dev_command: string | null;
	reason: string;
}

let url = $state('');
let tier = $state<PreviewTier>(null);
let status = $state<PreviewStatus>('idle');
let error = $state<string | null>(null);
let framework = $state<string | null>(null);
let reason = $state<string>('');
let statusMessage = $state<string>('');
let watcherUnlisten: (() => void) | null = null;
let statusUnlisten: (() => void) | null = null;

function tierFromDetection(t: TierDetection['tier']): PreviewTier {
	switch (t) {
		case 'direct_serve': return 'direct';
		case 'esbuild_bundle': return 'esbuild';
		case 'managed_process': return 'managed';
		default: return null;
	}
}

export function getPreviewStore() {
	return {
		get url() { return url; },
		set url(v: string) { url = v; },
		get tier() { return tier; },
		get status() { return status; },
		get error() { return error; },
		get framework() { return framework; },
		get reason() { return reason; },
		get statusMessage() { return statusMessage; },

		async openProject(projectDir: string) {
			const invoke = await getInvoke();
			const listen = await getListen();

			try {
				status = 'detecting';
				error = null;
				tier = null;
				framework = null;
				url = '';
				statusMessage = 'Analyzing project...';

				// Listen for status updates from the backend
				if (statusUnlisten) {
					statusUnlisten();
				}
				statusUnlisten = await listen<{ phase: string; message: string; framework?: string }>('preview:status', (event) => {
					const { phase, message } = event.payload;
					statusMessage = message;
					console.log('[preview] Status:', phase, message);
					if (phase === 'installing') {
						status = 'installing';
					}
				});

				const detection = await invoke<TierDetection>('preview_open_project', {
					projectDir
				});

				// By the time preview_open_project resolves, the server is already running.
				// Go straight to getting the URL.
				tier = tierFromDetection(detection.tier);
				framework = detection.framework;
				reason = detection.reason;

				// Get the URL from the backend
				const previewUrl = await invoke<string | null>('preview_get_url');
				if (previewUrl) {
					url = previewUrl;
					status = 'ready';
					statusMessage = '';
					// Clean up status listener - no longer needed once ready
					if (statusUnlisten) {
						statusUnlisten();
						statusUnlisten = null;
					}
				} else {
					status = 'error';
					error = 'Preview server started but no URL available';
				}

				// Listen for file changes
				await this.listenForChanges();
			} catch (e) {
				status = 'error';
				const errMsg = e instanceof Error ? e.message : String(e);
				error = errMsg;
				statusMessage = '';
				console.error('[preview] Failed to open project:', e);
			}
		},

		async stop() {
			try {
				const invoke = await getInvoke();
				await invoke('preview_stop');
			} catch {
				// Preview may not be active
			}
			url = '';
			tier = null;
			status = 'idle';
			error = null;
			framework = null;
			reason = '';

			if (watcherUnlisten) {
				watcherUnlisten();
				watcherUnlisten = null;
			}
			if (statusUnlisten) {
				statusUnlisten();
				statusUnlisten = null;
			}
			statusMessage = '';
		},

		async refresh() {
			if (!url) return;
			const invoke = await getInvoke();
			try {
				await invoke('reload_preview_webview');
			} catch (e) {
				console.error('[preview] Failed to refresh:', e);
			}
		},

		async listenForChanges() {
			const invoke = await getInvoke();
			const listen = await getListen();
			if (watcherUnlisten) {
				watcherUnlisten();
			}
			watcherUnlisten = await listen<{ paths: string[] }>('preview:file-changed', async (event) => {
				console.log('[preview] Files changed:', event.payload.paths.length, 'files');

				if (tier === 'esbuild') {
					// Rebuild the bundle
					try {
						status = 'building';
						await invoke('preview_rebuild');
						status = 'ready';
					} catch (e) {
						console.error('[preview] Rebuild failed:', e);
					}
				}

				// Reload the webview for all tiers
				try {
					await invoke('reload_preview_webview');
				} catch {
					// Webview might not exist yet
				}
			});
		},

		reset() {
			url = '';
			tier = null;
			status = 'idle';
			error = null;
			framework = null;
			reason = '';
		}
	};
}
