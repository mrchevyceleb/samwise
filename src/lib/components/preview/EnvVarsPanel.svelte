<script lang="ts">
	import { getPreviewStore } from '$lib/stores/preview.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';
	import { fetchSecrets, fetchProjects, fetchConfigs, type DopplerProject, type DopplerConfig } from '$lib/services/doppler';
	const preview = getPreviewStore();
	const workspace = getWorkspace();
	const settings = getSettingsStore();

	let showValues: Record<number, boolean> = $state({});
	let addHovered = $state(false);
	let applyHovered = $state(false);
	let applying = $state(false);
	let syncHovered = $state(false);
	let syncing = $state(false);
	let syncError = $state('');
	let dopplerLinkOpen = $state(false);
	let dopplerProjects = $state<DopplerProject[]>([]);
	let dopplerConfigs = $state<DopplerConfig[]>([]);
	let dopplerLinking = $state(false);
	let dopplerLinkError = $state('');
	let linkBtnHovered = $state(false);
	let isServiceToken = $state(false);
	let manualProject = $state('');
	let manualConfig = $state('');

	let hasToken = $derived(settings.value.dopplerToken.trim().length >= 10);
	let dopplerConfigured = $derived(
		hasToken && preview.dopplerProject && preview.dopplerConfig
	);
	let dopplerPartial = $derived(
		hasToken && (!preview.dopplerProject || !preview.dopplerConfig)
	);

	async function openDopplerLink() {
		dopplerLinkOpen = true;
		dopplerLinkError = '';
		dopplerLinking = true;
		isServiceToken = false;
		const token = settings.value.dopplerToken;
		console.log('[doppler] openDopplerLink, token prefix:', token?.substring(0, 5));
		try {
			const result = await fetchProjects(token);
			console.log('[doppler] fetchProjects returned:', result?.length, 'projects');
			dopplerProjects = result;
			if (result.length === 0) {
				// Service tokens can't list projects - switch to manual mode
				isServiceToken = true;
			} else if (preview.dopplerProject) {
				dopplerConfigs = await fetchConfigs(token, preview.dopplerProject);
			}
		} catch (e) {
			console.error('[doppler] fetchProjects failed:', e);
			const msg = e instanceof Error ? e.message : String(e);
			// 403/401 likely means service token
			if (msg.includes('403') || msg.includes('401') || msg.includes('Unauthorized')) {
				isServiceToken = true;
			} else {
				dopplerLinkError = msg;
			}
		} finally {
			dopplerLinking = false;
		}
	}

	async function handleDopplerProjectPick(slug: string) {
		preview.dopplerProject = slug;
		preview.dopplerConfig = '';
		dopplerConfigs = [];
		if (!slug) return;
		dopplerLinking = true;
		try {
			dopplerConfigs = await fetchConfigs(settings.value.dopplerToken, slug);
		} catch (e) {
			dopplerLinkError = e instanceof Error ? e.message : String(e);
		} finally {
			dopplerLinking = false;
		}
	}

	function handleDopplerConfigPick(name: string) {
		if (!workspace.path) return;
		preview.saveDopplerLink(workspace.path, preview.dopplerProject, name);
		dopplerLinkOpen = false;
		handleDopplerSync();
	}

	function handleManualLink() {
		const proj = manualProject.trim();
		const conf = manualConfig.trim();
		if (!proj || !conf || !workspace.path) return;
		preview.saveDopplerLink(workspace.path, proj, conf);
		dopplerLinkOpen = false;
		handleDopplerSync();
	}

	// Known public prefixes per framework
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

	// All known prefixes for detection
	const ALL_PREFIXES = ['NEXT_PUBLIC_', 'REACT_APP_', 'VITE_', 'NUXT_PUBLIC_', 'EXPO_PUBLIC_', 'PUBLIC_'];

	/** Check if a key already has a framework prefix */
	function hasFrameworkPrefix(key: string): boolean {
		return ALL_PREFIXES.some(p => key.startsWith(p));
	}

	/** Get the smart display label showing auto-prefix behavior */
	function getPrefixHint(key: string): string | null {
		if (!key.trim() || hasFrameworkPrefix(key)) return null;
		return `Auto-sets NEXT_PUBLIC_, VITE_, REACT_APP_ prefixes`;
	}

	function toggleShowValue(index: number) {
		showValues = { ...showValues, [index]: !showValues[index] };
	}

	function handleAddVar() {
		preview.addEnvVar();
	}

	function handleRemove(index: number) {
		preview.removeEnvVar(index);
		if (workspace.path) preview.saveEnvVars(workspace.path);
	}

	function handleKeyChange(index: number, value: string) {
		preview.updateEnvVar(index, 'key', value);
		if (workspace.path) preview.saveEnvVars(workspace.path);
	}

	function handleValueChange(index: number, value: string) {
		preview.updateEnvVar(index, 'value', value);
		if (workspace.path) preview.saveEnvVars(workspace.path);
	}

	function handleAddSuggested(key: string) {
		preview.addSuggestedKey(key);
		if (workspace.path) preview.saveEnvVars(workspace.path);
	}

	async function handleApplyAndRestart() {
		if (!workspace.path) return;
		applying = true;
		preview.saveEnvVars(workspace.path);
		await preview.openProject(workspace.path);
		applying = false;
	}

	async function handleDopplerSync() {
		const proj = preview.dopplerProject;
		const conf = preview.dopplerConfig;
		const token = settings.value.dopplerToken;
		console.log('[doppler] handleDopplerSync called', { proj, conf, hasToken: !!token, workspacePath: workspace.path });
		if (!workspace.path || !token || !proj || !conf) {
			console.warn('[doppler] sync skipped - missing:', { path: !!workspace.path, token: !!token, proj, conf });
			return;
		}
		syncing = true;
		syncError = '';
		try {
			console.log('[doppler] fetching secrets for', proj, conf);
			const secrets = await fetchSecrets(token, proj, conf);
			console.log('[doppler] got', Object.keys(secrets).length, 'secrets');
			const existing = [...preview.envVars];
			for (const [key, value] of Object.entries(secrets)) {
				const idx = existing.findIndex(v => v.key === key);
				if (idx >= 0) {
					existing[idx] = { key, value };
				} else {
					existing.push({ key, value });
				}
			}
			preview.envVars = existing;
			await preview.saveEnvVars(workspace.path);
			console.log('[doppler] saved env vars, restarting preview');
			await preview.openProject(workspace.path);
			console.log('[doppler] preview restarted');
		} catch (e) {
			console.error('[doppler] sync failed:', e);
			syncError = e instanceof Error ? e.message : String(e);
		} finally {
			syncing = false;
		}
	}

	// Filter suggested keys to only show ones not already added
	let unusedSuggestions = $derived(
		preview.suggestedKeys.filter(k => !preview.envVars.some(v => v.key === k))
	);
</script>

{#if preview.envPanelOpen}
	<div style="
		border-bottom: 1px solid var(--border-default);
		background: var(--bg-surface);
		padding: 10px 12px;
		animation: slideDown 0.15s ease-out;
		max-height: 320px;
		overflow-y: auto;
	">
		<!-- Header -->
		<div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
			<div style="display: flex; align-items: center; gap: 6px;">
				<svg width="14" height="14" viewBox="0 0 16 16" fill="var(--banana-yellow)">
					<path d="M8 1a2 2 0 0 1 2 2v4H6V3a2 2 0 0 1 2-2zm3 6V3a3 3 0 0 0-6 0v4a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2zM5 9h6a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z"/>
				</svg>
				<span style="font-family: var(--font-ui); font-size: 11px; font-weight: 600; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px;">
					Environment Variables
				</span>
				<span style="font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); padding: 1px 5px; background: var(--bg-elevated); border-radius: 4px;">
					{preview.envVars.filter(v => v.key.trim()).length}
				</span>
			</div>
			<button
				style="width: 22px; height: 22px; display: flex; align-items: center; justify-content: center; background: transparent; border: none; color: var(--text-muted); cursor: pointer; border-radius: 4px; transition: all 0.1s ease;"
				onclick={() => preview.envPanelOpen = false}
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; }}
				aria-label="Close env vars panel"
			>
				<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
					<path d="M3.05 3.05a.5.5 0 0 1 .707 0L6 5.293l2.243-2.243a.5.5 0 0 1 .707.707L6.707 6l2.243 2.243a.5.5 0 0 1-.707.707L6 6.707 3.757 8.95a.5.5 0 0 1-.707-.707L5.293 6 3.05 3.757a.5.5 0 0 1 0-.707z"/>
				</svg>
			</button>
		</div>

		<!-- Suggested keys from .env files -->
		{#if unusedSuggestions.length > 0}
			<div style="display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 8px;">
				<span style="font-family: var(--font-ui); font-size: 10px; color: var(--text-muted); padding-top: 2px;">Detected:</span>
				{#each unusedSuggestions as key}
					<button
						style="padding: 2px 8px; font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary); background: var(--bg-elevated); border: 1px dashed var(--border-default); border-radius: 4px; cursor: pointer; transition: all 0.1s ease;"
						onclick={() => handleAddSuggested(key)}
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'; (e.currentTarget as HTMLElement).style.color = 'var(--banana-yellow)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; (e.currentTarget as HTMLElement).style.color = 'var(--text-secondary)'; }}
					>
						+ {key}
					</button>
				{/each}
			</div>
		{/if}

		<!-- Env var rows -->
		<div style="display: flex; flex-direction: column; gap: 4px;">
			{#each preview.envVars as envVar, index}
				<div style="display: flex; flex-direction: column; gap: 2px;">
				<div style="display: flex; align-items: center; gap: 4px;">
					<!-- Key input -->
					<input
						type="text"
						placeholder="SUPABASE_URL"
						value={envVar.key}
						oninput={(e) => handleKeyChange(index, (e.currentTarget as HTMLInputElement).value)}
						style="flex: 0 0 160px; height: 28px; padding: 0 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 5px; color: var(--text-primary); font-family: var(--font-mono); font-size: 11px; outline: none; transition: border-color 0.1s ease;"
						onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'}
						onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
					/>
					<span style="color: var(--text-muted); font-family: var(--font-mono); font-size: 12px;">=</span>
					<!-- Value input -->
					<div style="flex: 1; position: relative; display: flex; align-items: center;">
						<input
							type={showValues[index] ? 'text' : 'password'}
							placeholder="value"
							value={envVar.value}
							oninput={(e) => handleValueChange(index, (e.currentTarget as HTMLInputElement).value)}
							style="width: 100%; height: 28px; padding: 0 30px 0 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 5px; color: var(--text-primary); font-family: var(--font-mono); font-size: 11px; outline: none; transition: border-color 0.1s ease;"
							onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'}
							onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
						/>
						<!-- Show/hide toggle -->
						<button
							style="position: absolute; right: 4px; width: 22px; height: 22px; display: flex; align-items: center; justify-content: center; background: transparent; border: none; color: var(--text-muted); cursor: pointer; border-radius: 3px; transition: color 0.1s ease;"
							onclick={() => toggleShowValue(index)}
							onmouseenter={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'}
							onmouseleave={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'}
							aria-label={showValues[index] ? 'Hide value' : 'Show value'}
						>
							{#if showValues[index]}
								<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
									<path d="M8 3C4.5 3 1.7 5.1 0.5 8c1.2 2.9 4 5 7.5 5s6.3-2.1 7.5-5c-1.2-2.9-4-5-7.5-5zm0 8.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7zm0-5.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/>
								</svg>
							{:else}
								<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
									<path d="M13.359 11.238C15.06 9.72 16 8 16 8s-3-5.5-8-5.5a7.028 7.028 0 0 0-2.79.588l.77.771A5.944 5.944 0 0 1 8 3.5c2.12 0 3.879 1.168 5.168 2.457A13.134 13.134 0 0 1 14.828 8c-.058.087-.122.183-.195.288-.335.48-.83 1.12-1.465 1.755-.165.165-.337.328-.517.486l.708.709z"/>
									<path d="M11.297 9.176a3.5 3.5 0 0 0-4.474-4.474l.823.823a2.5 2.5 0 0 1 2.829 2.829l.822.822zm-2.943 1.299l.822.822a3.5 3.5 0 0 1-4.474-4.474l.823.823a2.5 2.5 0 0 0 2.829 2.829z"/>
									<path d="M3.35 5.47c-.18.16-.353.322-.518.487A13.134 13.134 0 0 0 1.172 8l.195.288c.335.48.83 1.12 1.465 1.755C4.121 11.332 5.881 12.5 8 12.5c.716 0 1.39-.133 2.02-.36l.77.772A7.029 7.029 0 0 1 8 13.5C3 13.5 0 8 0 8s.939-1.721 2.641-3.238l.708.709z"/>
									<path d="M13.646 14.354l-12-12 .708-.708 12 12-.708.708z"/>
								</svg>
							{/if}
						</button>
					</div>
					<!-- Delete button -->
					<button
						style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; background: transparent; border: 1px solid transparent; border-radius: 5px; color: var(--text-muted); cursor: pointer; transition: all 0.1s ease; flex-shrink: 0;"
						onclick={() => handleRemove(index)}
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color = '#ff6b6b'; (e.currentTarget as HTMLElement).style.background = 'rgba(255, 107, 107, 0.1)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
						aria-label="Remove variable"
					>
						<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
							<path d="M5 1h2a1 1 0 0 1 1 1H4a1 1 0 0 1 1-1zM3 2a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2h2.5a.5.5 0 0 1 0 1h-.441l-.443 7.107A2 2 0 0 1 8.622 12H3.378a2 2 0 0 1-1.994-1.893L.941 3H.5a.5.5 0 0 1 0-1H3zm-.944 1l.436 7.003A1 1 0 0 0 3.378 11h5.244a1 1 0 0 0 .997-.947L10.055 3H2.056z"/>
						</svg>
					</button>
				</div>
				{#if getPrefixHint(envVar.key)}
					<div style="padding-left: 4px; font-family: var(--font-mono); font-size: 9px; color: rgba(255, 214, 10, 0.6);">
						{getPrefixHint(envVar.key)}
					</div>
				{/if}
				</div>
			{/each}
		</div>

		<!-- Action buttons -->
		<div style="display: flex; align-items: center; gap: 6px; margin-top: 8px;">
			<button
				style="display: flex; align-items: center; gap: 4px; height: 26px; padding: 0 10px; background: {addHovered ? 'rgba(255, 214, 10, 0.15)' : 'var(--bg-elevated)'}; border: 1px solid {addHovered ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 5px; color: {addHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; cursor: pointer; font-family: var(--font-ui); font-size: 11px; font-weight: 500; transition: all 0.12s ease;"
				onmouseenter={() => addHovered = true}
				onmouseleave={() => addHovered = false}
				onclick={handleAddVar}
			>
				<svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
					<path d="M5 0a.5.5 0 0 1 .5.5V4.5h4a.5.5 0 0 1 0 1h-4v4a.5.5 0 0 1-1 0v-4h-4a.5.5 0 0 1 0-1h4V.5A.5.5 0 0 1 5 0z"/>
				</svg>
				Add Variable
			</button>

			{#if dopplerConfigured}
				<button
					style="display: flex; align-items: center; gap: 4px; height: 26px; padding: 0 10px; background: {syncHovered ? 'rgba(108, 71, 255, 0.2)' : 'var(--bg-elevated)'}; border: 1px solid {syncHovered ? '#6C47FF' : 'var(--border-default)'}; border-radius: 5px; color: {syncHovered ? '#6C47FF' : 'var(--text-secondary)'}; cursor: pointer; font-family: var(--font-ui); font-size: 11px; font-weight: 500; transition: all 0.12s ease; opacity: {syncing ? 0.7 : 1};"
					onmouseenter={() => syncHovered = true}
					onmouseleave={() => syncHovered = false}
					onclick={handleDopplerSync}
					disabled={syncing}
				>
					{#if syncing}
						Syncing...
					{:else}
						<svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
							<path d="M5 1.5a3.5 3.5 0 1 0 2.84 1.46.5.5 0 0 1 .82-.57A4.5 4.5 0 1 1 5 .5v1z"/>
							<path d="M5 0a.5.5 0 0 1 .354.146l1.5 1.5a.5.5 0 0 1-.708.708L5 1.207 3.854 2.354a.5.5 0 1 1-.708-.708l1.5-1.5A.5.5 0 0 1 5 0z"/>
						</svg>
						Sync Doppler
					{/if}
				</button>
			{/if}

			{#if preview.envVars.some(v => v.key.trim())}
				<button
					style="display: flex; align-items: center; gap: 4px; height: 26px; padding: 0 12px; background: {applyHovered ? 'var(--banana-yellow)' : 'rgba(255, 214, 10, 0.2)'}; border: 1px solid var(--banana-yellow); border-radius: 5px; color: {applyHovered ? '#0D1117' : 'var(--banana-yellow)'}; cursor: pointer; font-family: var(--font-ui); font-size: 11px; font-weight: 600; transition: all 0.12s ease; opacity: {applying ? 0.7 : 1};"
					onmouseenter={() => applyHovered = true}
					onmouseleave={() => applyHovered = false}
					onclick={handleApplyAndRestart}
					disabled={applying}
				>
					{#if applying}
						Applying...
					{:else}
						<svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
							<path d="M5 1.5a3.5 3.5 0 1 0 2.84 1.46.5.5 0 0 1 .82-.57A4.5 4.5 0 1 1 5 .5v1z"/>
							<path d="M5 0a.5.5 0 0 1 .354.146l1.5 1.5a.5.5 0 0 1-.708.708L5 1.207 3.854 2.354a.5.5 0 1 1-.708-.708l1.5-1.5A.5.5 0 0 1 5 0z"/>
						</svg>
						Apply & Restart
					{/if}
				</button>
			{/if}
		</div>

		{#if syncError}
			<div style="font-family: var(--font-ui); font-size: 11px; color: var(--accent-red); margin-top: 4px;">
				Doppler sync failed: {syncError}
			</div>
		{/if}

		<!-- Doppler section -->
		{#if !hasToken}
			<div style="margin-top: 8px; padding: 8px 10px; background: rgba(108, 71, 255, 0.06); border: 1px dashed rgba(108, 71, 255, 0.2); border-radius: 6px; display: flex; align-items: center; gap: 8px;">
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#6C47FF" stroke-width="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
				<span style="font-family: var(--font-ui); font-size: 11px; color: var(--text-muted);">
					Use <span style="color: #6C47FF; font-weight: 600;">Doppler</span> to auto-fill secrets.
					<button
						style="background: none; border: none; color: #6C47FF; cursor: pointer; font-size: 11px; font-family: var(--font-ui); text-decoration: underline; padding: 0;"
						onclick={() => { settings.settingsVisible = true; settings.activeSettingsTab = 'doppler'; }}
					>Add token in Settings</button>
				</span>
			</div>
		{:else if dopplerPartial && !dopplerLinkOpen}
			<div style="margin-top: 8px;">
				<button
					style="display: flex; align-items: center; gap: 6px; width: 100%; height: 30px; padding: 0 10px; background: {linkBtnHovered ? 'rgba(108, 71, 255, 0.12)' : 'rgba(108, 71, 255, 0.06)'}; border: 1px solid {linkBtnHovered ? '#6C47FF' : 'rgba(108, 71, 255, 0.2)'}; border-radius: 6px; color: {linkBtnHovered ? '#6C47FF' : 'var(--text-secondary)'}; cursor: pointer; font-family: var(--font-ui); font-size: 11px; font-weight: 500; transition: all 0.12s ease;"
					onmouseenter={() => linkBtnHovered = true}
					onmouseleave={() => linkBtnHovered = false}
					onclick={openDopplerLink}
				>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
					Link Doppler Project
				</button>
			</div>
		{/if}

		<!-- Inline Doppler linker -->
		{#if dopplerLinkOpen && !dopplerConfigured}
			<div style="margin-top: 8px; padding: 8px 10px; background: rgba(108, 71, 255, 0.06); border: 1px solid rgba(108, 71, 255, 0.15); border-radius: 6px; display: flex; flex-direction: column; gap: 8px;">
				<div style="display: flex; align-items: center; justify-content: space-between;">
					<div style="display: flex; align-items: center; gap: 6px;">
						<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#6C47FF" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
						<span style="font-family: var(--font-ui); font-size: 11px; font-weight: 600; color: #6C47FF;">Link Doppler</span>
					</div>
					<button
						style="width: 18px; height: 18px; display: flex; align-items: center; justify-content: center; background: transparent; border: none; color: var(--text-muted); cursor: pointer; border-radius: 3px;"
						onclick={() => dopplerLinkOpen = false}
						aria-label="Close Doppler linker"
					>
						<svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor"><path d="M3.05 3.05a.5.5 0 0 1 .707 0L6 5.293l2.243-2.243a.5.5 0 0 1 .707.707L6.707 6l2.243 2.243a.5.5 0 0 1-.707.707L6 6.707 3.757 8.95a.5.5 0 0 1-.707-.707L5.293 6 3.05 3.757a.5.5 0 0 1 0-.707z"/></svg>
					</button>
				</div>

				{#if dopplerLinking}
					<div style="padding: 6px 0; font-size: 12px; color: var(--banana-yellow); font-family: var(--font-ui);">Loading projects from Doppler...</div>
				{:else if dopplerLinkError}
					<div style="padding: 6px 8px; font-size: 12px; color: #ff6b6b; background: rgba(255, 107, 107, 0.1); border-radius: 4px; font-family: var(--font-ui);">{dopplerLinkError}</div>
				{:else if isServiceToken || (!dopplerLinking && dopplerProjects.length === 0)}
					<div style="padding: 8px 10px; font-size: 11px; color: var(--text-secondary); background: rgba(255, 107, 107, 0.06); border: 1px solid rgba(255, 107, 107, 0.15); border-radius: 5px; font-family: var(--font-ui); line-height: 1.6;">
						No projects found. You likely have a <span style="font-weight: 600;">service token</span> (dp.st.) instead of a <span style="font-weight: 600;">personal token</span> (dp.pt.).
						<button
							style="display: block; margin-top: 6px; background: none; border: none; color: #6C47FF; cursor: pointer; font-size: 11px; font-family: var(--font-ui); text-decoration: underline; padding: 0;"
							onclick={() => { settings.settingsVisible = true; settings.activeSettingsTab = 'doppler'; }}
						>Open Settings to fix your token</button>
					</div>
					<div style="padding: 6px 0; font-size: 10px; color: var(--text-muted); font-family: var(--font-ui);">
						Or enter project details manually:
					</div>
					<div style="display: flex; gap: 4px; align-items: center;">
						<input
							type="text"
							placeholder="project-slug"
							bind:value={manualProject}
							style="flex: 1; height: 28px; padding: 0 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 5px; color: var(--text-primary); font-family: var(--font-mono); font-size: 11px; outline: none;"
							onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'}
							onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
						/>
						<input
							type="text"
							placeholder="dev"
							bind:value={manualConfig}
							style="width: 80px; height: 28px; padding: 0 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 5px; color: var(--text-primary); font-family: var(--font-mono); font-size: 11px; outline: none;"
							onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'}
							onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
						/>
						<button
							onclick={handleManualLink}
							disabled={!manualProject.trim() || !manualConfig.trim()}
							style="height: 28px; padding: 0 12px; background: var(--banana-yellow); border: none; border-radius: 5px; color: #0D1117; font-size: 11px; font-weight: 600; font-family: var(--font-ui); cursor: pointer; opacity: {!manualProject.trim() || !manualConfig.trim() ? 0.5 : 1};"
						>
							Sync
						</button>
					</div>
				{/if}

				{#if dopplerProjects.length > 0}
					<select
						value={preview.dopplerProject}
						onchange={(e) => handleDopplerProjectPick((e.target as HTMLSelectElement).value)}
						style="height: 28px; padding: 0 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 5px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none; cursor: pointer;"
					>
						<option value="">Select project...</option>
						{#each dopplerProjects as p}
							<option value={p.slug}>{p.name}</option>
						{/each}
					</select>
				{/if}

				{#if dopplerConfigs.length > 0}
					<div style="font-size: 10px; color: var(--text-muted); font-family: var(--font-ui);">Pick a config to sync secrets and restart preview:</div>
					<div style="display: flex; gap: 4px; flex-wrap: wrap;">
						{#each dopplerConfigs as c}
							<button
								onclick={() => handleDopplerConfigPick(c.name)}
								style="padding: 6px 14px; border: 1px solid {preview.dopplerConfig === c.name ? '#6C47FF' : 'var(--border-default)'}; border-radius: 5px; cursor: pointer; font-size: 11px; font-weight: 500; font-family: var(--font-ui); transition: all 0.12s ease; background: {preview.dopplerConfig === c.name ? 'rgba(108, 71, 255, 0.15)' : 'var(--bg-elevated)'}; color: {preview.dopplerConfig === c.name ? '#6C47FF' : 'var(--text-secondary)'};"
								onmouseenter={(e) => (e.currentTarget as HTMLElement).style.borderColor = '#6C47FF'}
								onmouseleave={(e) => { if (preview.dopplerConfig !== c.name) (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
							>
								{c.name}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{/if}

		<!-- Empty state hint -->
		{#if preview.envVars.length === 0 && !dopplerLinkOpen}
			<div style="text-align: center; padding: 12px 0 4px; font-family: var(--font-ui); font-size: 11px; color: var(--text-muted);">
				Add your API keys and secrets here.<br/>
				<span style="font-size: 10px; opacity: 0.7;">Just type the key name (e.g. SUPABASE_URL). Prefixes are added automatically.</span>
			</div>
		{/if}
	</div>
{/if}

<style>
	@keyframes slideDown {
		from { opacity: 0; max-height: 0; padding-top: 0; padding-bottom: 0; }
		to { opacity: 1; max-height: 320px; }
	}
</style>
