<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/shell/AppShell.svelte';
	import FloatingBananas from '$lib/components/playful/FloatingBananas.svelte';
	import ClickEasterEgg from '$lib/components/playful/ClickEasterEgg.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { getSettings, initSettings } from '$lib/stores/settings.svelte';

	let loaded = $state(false);

	onMount(async () => {
		await initSettings();
		const saved = getSettings();

		// 1. Persisted local settings (legacy path, still respected if present)
		if (saved.supabaseUrl && saved.supabaseAnonKey) {
			await safeInvoke('supabase_set_config', {
				url: saved.supabaseUrl,
				anon_key: saved.supabaseAnonKey,
				service_role_key: null,
			});
			loaded = true;
			return;
		}

		// 2. Rust state (baked-in at compile time via option_env!, or previously loaded)
		const config = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
		if (config && config.url) {
			loaded = true;
			return;
		}

		// 3. Doppler runtime fallback (only fires when the binary wasn't built with secrets)
		const dopplerConfig = await safeInvoke<{ url: string; anon_key: string }>('supabase_load_doppler');
		if (dopplerConfig && dopplerConfig.url) {
			loaded = true;
			return;
		}

		// Nothing worked. Personal tool, one user, no modal. Boot with whatever we've got.
		console.warn('[app] No Supabase config found. Rebuild with secrets (doppler run -- npx tauri build).');
		loaded = true;
	});
</script>

{#if loaded}
	<ClickEasterEgg />
	<FloatingBananas />
	<AppShell />
{/if}
