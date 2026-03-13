/** Preview store using Svelte 5 runes */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type PreviewTier = 'direct' | 'esbuild' | 'managed' | null;
export type PreviewStatus = 'idle' | 'detecting' | 'building' | 'ready' | 'error';

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
let watcherUnlisten: (() => void) | null = null;

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

		async openProject(projectDir: string) {
			try {
				status = 'detecting';
				error = null;
				tier = null;
				framework = null;
				url = '';

				const detection = await invoke<TierDetection>('preview_open_project', {
					projectDir
				});

				tier = tierFromDetection(detection.tier);
				framework = detection.framework;
				reason = detection.reason;
				status = 'building';

				// Get the URL from the backend
				const previewUrl = await invoke<string | null>('preview_get_url');
				if (previewUrl) {
					url = previewUrl;
					status = 'ready';
				} else {
					status = 'error';
					error = 'Preview server started but no URL available';
				}

				// Listen for file changes
				await this.listenForChanges();
			} catch (e) {
				status = 'error';
				error = e instanceof Error ? e.message : String(e);
				console.error('[preview] Failed to open project:', e);
			}
		},

		async stop() {
			try {
				await invoke('preview_stop');
			} catch (e) {
				console.error('[preview] Failed to stop:', e);
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
		},

		async refresh() {
			if (!url) return;
			try {
				await invoke('reload_preview_webview');
			} catch (e) {
				console.error('[preview] Failed to refresh:', e);
			}
		},

		async listenForChanges() {
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
