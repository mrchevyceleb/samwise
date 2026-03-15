/** Preview store using Svelte 5 runes - silent, magical preview */

async function getInvoke() {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke;
}

async function getListen() {
	const { listen } = await import('@tauri-apps/api/event');
	return listen;
}

export type PreviewTier = 'direct' | 'esbuild' | 'managed' | null;
export type PreviewStatus = 'idle' | 'loading' | 'ready' | 'error';

export interface EnvVar {
	key: string;
	value: string;
}

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
let watcherUnlisten: (() => void) | null = null;
let envVars = $state<EnvVar[]>([]);
let envPanelOpen = $state(false);
let suggestedKeys = $state<string[]>([]);
let missingSecretsOverlay = $state(false);
let envSetupPending = $state(false);
let sessionKey = $state(0);

/** Storage key for env vars, scoped per project */
function envStorageKey(projectPath: string): string {
	return `banana_env_vars_${projectPath.replace(/[\\/]/g, '_')}`;
}

/** Load env vars from localStorage for a project */
function loadEnvVars(projectPath: string): EnvVar[] {
	try {
		const raw = localStorage.getItem(envStorageKey(projectPath));
		if (raw) return JSON.parse(raw) as EnvVar[];
	} catch { /* ignore */ }
	return [];
}

/** Save env vars to localStorage for a project */
function saveEnvVars(projectPath: string, vars: EnvVar[]) {
	try {
		localStorage.setItem(envStorageKey(projectPath), JSON.stringify(vars));
	} catch { /* ignore */ }
}

/** Framework prefix mapping */
const FRAMEWORK_PREFIXES: Record<string, string> = {
	'Next.js': 'NEXT_PUBLIC_',
	'React': 'REACT_APP_',
	'Vite': 'VITE_',
	'Solid': 'VITE_',
	'Preact': 'VITE_',
	'Nuxt': 'NUXT_PUBLIC_',
	'Expo': 'EXPO_PUBLIC_',
	'Astro': 'PUBLIC_',
};

const ALL_PREFIXES = ['NEXT_PUBLIC_', 'REACT_APP_', 'VITE_', 'NUXT_PUBLIC_', 'EXPO_PUBLIC_', 'PUBLIC_'];

/** Check if a key already has a framework prefix */
function hasFrameworkPrefix(key: string): boolean {
	return ALL_PREFIXES.some(p => key.startsWith(p));
}

/**
 * Build a HashMap from the env vars list.
 * Smart prefix handling: if user types "SUPABASE_URL", we auto-set ALL
 * common framework-prefixed versions so it just works regardless of framework:
 *   SUPABASE_URL, NEXT_PUBLIC_SUPABASE_URL, VITE_SUPABASE_URL, REACT_APP_SUPABASE_URL, etc.
 * If the key already has a prefix (e.g. NEXT_PUBLIC_SUPABASE_URL), we only set that exact key.
 */
function envVarsToMap(vars: EnvVar[]): Record<string, string> {
	const map: Record<string, string> = {};

	for (const v of vars) {
		const key = v.key.trim();
		if (!key) continue;

		// Always set the exact key
		map[key] = v.value;

		// If no framework prefix, auto-expand to all common prefixes
		if (!hasFrameworkPrefix(key)) {
			for (const prefix of ALL_PREFIXES) {
				map[`${prefix}${key}`] = v.value;
			}
		}
	}
	return map;
}

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
		get envVars() { return envVars; },
		set envVars(v: EnvVar[]) { envVars = v; },
		get envPanelOpen() { return envPanelOpen; },
		set envPanelOpen(v: boolean) { envPanelOpen = v; },
		get suggestedKeys() { return suggestedKeys; },
		get missingSecretsOverlay() { return missingSecretsOverlay; },
		set missingSecretsOverlay(v: boolean) { missingSecretsOverlay = v; },
		get envSetupPending() { return envSetupPending; },
		set envSetupPending(v: boolean) { envSetupPending = v; },
		get sessionKey() { return sessionKey; },

		async openProject(projectDir: string) {
			const invoke = await getInvoke();

			// Try loading from .banana-env file first, then fall back to localStorage
			let loaded: EnvVar[] = [];
			try {
				const fileVars = await invoke<Record<string, string>>('preview_load_env_file', { projectDir });
				const entries = Object.entries(fileVars);
				if (entries.length > 0) {
					loaded = entries.map(([key, value]) => ({ key, value }));
				}
			} catch {
				// .banana-env not available, fall back to localStorage
			}
			if (loaded.length === 0) {
				loaded = loadEnvVars(projectDir);
			}
			envVars = loaded;

			// Scan for suggested keys from .env files
			try {
				const { previewScanEnvKeys } = await import('$lib/utils/tauri');
				suggestedKeys = await previewScanEnvKeys(projectDir);
			} catch {
				suggestedKeys = [];
			}

			try {
				// Silent loading - no infrastructure messages
				status = 'loading';
				error = null;
				tier = null;
				framework = null;
				url = '';
				envSetupPending = false;

				const envMap = envVarsToMap(envVars);
				const detection = await invoke<TierDetection>('preview_open_project', {
					projectDir,
					envVars: envMap
				});

				tier = tierFromDetection(detection.tier);
				framework = detection.framework;

				const previewUrl = await invoke<string | null>('preview_get_url');
				if (previewUrl) {
					// Check for missing secrets BEFORE setting url (which triggers webview creation)
					const hasValues = envVars.some(v => v.key.trim() && v.value.trim());
					console.log('[preview] Overlay check:', { suggestedKeys: suggestedKeys.length, hasValues, envVars: envVars.length });
					if (suggestedKeys.length > 0 && !hasValues) {
						missingSecretsOverlay = true;
						console.log('[preview] Showing missing secrets overlay');
					} else {
						missingSecretsOverlay = false;
					}

					url = previewUrl;
					status = 'ready';
					sessionKey += 1;
				} else {
					status = 'error';
					error = 'Preview could not start';
				}

				await this.listenForChanges();
			} catch (e) {
				status = 'error';
				error = e instanceof Error ? e.message : String(e);
				console.error('[preview] Failed:', e);
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
			missingSecretsOverlay = false;
			envSetupPending = false;

			if (watcherUnlisten) {
				watcherUnlisten();
				watcherUnlisten = null;
			}
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
			watcherUnlisten = await listen<{ paths: string[] }>('preview:file-changed', async () => {
				if (tier === 'esbuild') {
					// Re-bundle with esbuild (sub-second)
					try {
						await invoke('preview_rebuild');
					} catch (e) {
						console.error('[preview] Rebuild failed:', e);
					}
				}

				// Reload the webview
				try {
					await invoke('reload_preview_webview');
				} catch {
					// Webview might not exist yet
				}
			});
		},

		/** Save env vars for a project and optionally restart preview */
		async saveEnvVars(projectDir: string) {
			saveEnvVars(projectDir, envVars);
			// Also persist to .banana-env file
			try {
				const invoke = await getInvoke();
				const map: Record<string, string> = {};
				for (const v of envVars) {
					const key = v.key.trim();
					if (key) map[key] = v.value;
				}
				await invoke('preview_save_env_file', { projectDir, envVars: map });
			} catch (e) {
				console.error('[preview] Failed to save .banana-env:', e);
			}
			// Hide the missing secrets overlay since the user has set vars
			missingSecretsOverlay = false;
		},

		/** Add a new empty env var row */
		addEnvVar() {
			envVars = [...envVars, { key: '', value: '' }];
		},

		/** Remove an env var by index */
		removeEnvVar(index: number) {
			envVars = envVars.filter((_, i) => i !== index);
		},

		/** Update an env var key or value by index */
		updateEnvVar(index: number, field: 'key' | 'value', val: string) {
			envVars = envVars.map((v, i) => i === index ? { ...v, [field]: val } : v);
		},

		/** Add a suggested key if not already present */
		addSuggestedKey(key: string) {
			if (!envVars.some(v => v.key === key)) {
				envVars = [...envVars, { key, value: '' }];
			}
		},

		reset() {
			url = '';
			tier = null;
			status = 'idle';
			error = null;
			framework = null;
			missingSecretsOverlay = false;
		}
	};
}
