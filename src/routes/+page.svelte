<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/shell/AppShell.svelte';
	import FloatingBananas from '$lib/components/playful/FloatingBananas.svelte';
	import ClickEasterEgg from '$lib/components/playful/ClickEasterEgg.svelte';
	import SetupWizard from '$lib/components/settings/SetupWizard.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { updateSetting } from '$lib/stores/settings.svelte';

	let showSetup = $state(false);
	let loaded = $state(false);

	onMount(async () => {
		// Try to auto-load config from Doppler on startup
		const config = await safeInvoke<{ url: string; anon_key: string }>('supabase_get_config');
		if (!config || !config.url) {
			// Config empty - try loading from Doppler automatically
			const dopplerConfig = await safeInvoke<{ url: string; anon_key: string }>('supabase_load_doppler');
			if (!dopplerConfig || !dopplerConfig.url) {
				// Doppler failed too - show setup wizard as last resort
				showSetup = true;
			}
		}
		loaded = true;
	});

	function handleSetupComplete(machineName: string) {
		if (machineName) {
			updateSetting('agentMachineName', machineName);
		}
		showSetup = false;
	}
</script>

{#if loaded}
	<ClickEasterEgg />
	<FloatingBananas />
	<AppShell />
	{#if showSetup}
		<SetupWizard onComplete={handleSetupComplete} />
	{/if}
{/if}
