<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/shell/AppShell.svelte';
	import FloatingBananas from '$lib/components/playful/FloatingBananas.svelte';
	import ClickEasterEgg from '$lib/components/playful/ClickEasterEgg.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { updateSetting, getSettings, initSettings } from '$lib/stores/settings.svelte';

	let showManualConfig = $state(false);
	let loaded = $state(false);
	let manualUrl = $state('');
	let manualAnonKey = $state('');
	let configError = $state('');
	let connecting = $state(false);

	onMount(async () => {
		// Load persisted settings first
		await initSettings();
		const saved = getSettings();

		// 1. Check if we have saved credentials in local settings
		if (saved.supabaseUrl && saved.supabaseAnonKey) {
			await safeInvoke('supabase_set_config', {
				url: saved.supabaseUrl,
				anon_key: saved.supabaseAnonKey,
				service_role_key: null,
			});
			loaded = true;
			return;
		}

		// 2. Try Supabase config already in memory (e.g. from a previous session)
		const config = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
		if (config && config.url) {
			loaded = true;
			return;
		}

		// 3. Try Doppler
		const dopplerConfig = await safeInvoke<{ url: string; anon_key: string }>('supabase_load_doppler');
		if (dopplerConfig && dopplerConfig.url) {
			loaded = true;
			return;
		}

		// 4. Nothing worked - show manual entry
		showManualConfig = true;
		loaded = true;
	});

	async function saveManualConfig() {
		if (!manualUrl.trim() || !manualAnonKey.trim()) {
			configError = 'Both fields are required.';
			return;
		}
		configError = '';
		connecting = true;

		// Set in Rust state for immediate use
		await safeInvoke('supabase_set_config', {
			url: manualUrl.trim(),
			anon_key: manualAnonKey.trim(),
			service_role_key: null,
		});

		// Test the connection
		const result = await safeInvoke<string>('supabase_test_connection');
		if (result) {
			// Save to local settings so they persist across restarts
			updateSetting('supabaseUrl', manualUrl.trim());
			updateSetting('supabaseAnonKey', manualAnonKey.trim());
			showManualConfig = false;
			// Reload to re-init with the new config
			window.location.reload();
		} else {
			configError = 'Connection failed. Check URL and key.';
			connecting = false;
		}
	}
</script>

{#if loaded}
	<ClickEasterEgg />
	<FloatingBananas />
	<AppShell />
	{#if showManualConfig}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div style="position: fixed; inset: 0; z-index: 2000; background: rgba(0,0,0,0.75); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center;">
			<div style="width: 480px; max-width: 90vw; background: var(--bg-surface, #161b22); border: 1px solid var(--border-default, #30363d); border-radius: 16px; box-shadow: 0 32px 80px rgba(0,0,0,0.6); overflow: hidden;">
				<div style="padding: 28px 28px 8px; text-align: center;">
					<div style="width: 56px; height: 56px; margin: 0 auto 12px; background: linear-gradient(135deg, #6366f1, #8b5cf6); border-radius: 14px; display: flex; align-items: center; justify-content: center; animation: bob 3s ease-in-out infinite;">
						<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.5">
							<circle cx="12" cy="8" r="5"/><path d="M3 21v-2a7 7 0 0 1 7-7h4a7 7 0 0 1 7 7v2"/><circle cx="9" cy="7" r="1" fill="white"/><circle cx="15" cy="7" r="1" fill="white"/>
						</svg>
					</div>
					<div style="font-size: 20px; font-weight: 700; color: var(--text-primary, #e6edf3);">Connect to Supabase</div>
					<div style="font-size: 13px; color: var(--text-secondary, #8b949e); margin-top: 4px;">Enter your Supabase project credentials to connect.</div>
				</div>

				<div style="padding: 20px 28px 24px; display: flex; flex-direction: column; gap: 14px;">
					<div style="display: flex; flex-direction: column; gap: 6px;">
						<label style="font-size: 12px; color: var(--text-secondary, #8b949e);">Supabase URL</label>
						<input bind:value={manualUrl} placeholder="https://your-project.supabase.co"
							style="padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px; color: var(--text-primary, #e6edf3); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;" />
					</div>
					<div style="display: flex; flex-direction: column; gap: 6px;">
						<label style="font-size: 12px; color: var(--text-secondary, #8b949e);">Anon Key</label>
						<input bind:value={manualAnonKey} type="password" placeholder="eyJhbGci..."
							style="padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px; color: var(--text-primary, #e6edf3); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;" />
					</div>
					{#if configError}
						<div style="font-size: 12px; color: #f85149;">{configError}</div>
					{/if}
					<button
						onclick={saveManualConfig}
						disabled={connecting}
						style="padding: 10px 20px; background: {connecting ? '#4b4d9e' : '#6366f1'}; border: none; border-radius: 8px; color: white; font-size: 13px; font-weight: 700; cursor: {connecting ? 'wait' : 'pointer'}; transition: all 0.15s ease; margin-top: 4px;"
					>
						{connecting ? 'Connecting...' : 'Connect'}
					</button>
					<div style="font-size: 11px; color: var(--text-muted, #6e7681); text-align: center; line-height: 1.5;">
						Find these in your Supabase dashboard under Settings > API.<br/>
						On the master machine, Doppler handles this automatically.
					</div>
				</div>
			</div>
		</div>
	{/if}
{/if}

<style>
	@keyframes bob {
		0%, 100% { transform: translateY(0); }
		50% { transform: translateY(-4px); }
	}
</style>
