<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
  import { fetchProjects, fetchConfigs, type DopplerProject, type DopplerConfig } from '$lib/services/doppler';
  import { untrack } from 'svelte';

  const settings = getSettingsStore();

  let projects = $state<DopplerProject[]>([]);
  let configs = $state<DopplerConfig[]>([]);
  let loadingProjects = $state(false);
  let loadingConfigs = $state(false);
  let error = $state('');
  let tokenVisible = $state(false);
  let fetchDebounce: ReturnType<typeof setTimeout> | null = null;

  // Auto-fetch projects when token changes (only tracks token)
  $effect(() => {
    const token = settings.value.dopplerToken;
    if (fetchDebounce) clearTimeout(fetchDebounce);
    if (!token || token.trim().length < 10) {
      projects = [];
      configs = [];
      error = '';
      untrack(() => {
        updateSetting('dopplerEnabled', false);
      });
      return;
    }
    fetchDebounce = setTimeout(async () => {
      loadingProjects = true;
      error = '';
      try {
        projects = await fetchProjects(token);
        // If previously selected project no longer exists, clear it
        const currentProject = untrack(() => settings.value.dopplerProject);
        if (currentProject && !projects.some(p => p.slug === currentProject)) {
          updateSetting('dopplerProject', '');
          updateSetting('dopplerConfig', '');
          configs = [];
        }
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        projects = [];
      } finally {
        loadingProjects = false;
      }
    }, 500);
  });

  // Auto-fetch configs when project changes (only tracks project)
  $effect(() => {
    const project = settings.value.dopplerProject;
    if (!project) {
      configs = [];
      return;
    }
    const token = untrack(() => settings.value.dopplerToken);
    if (!token) {
      configs = [];
      return;
    }
    loadingConfigs = true;
    configs = [];
    fetchConfigs(token, project)
      .then(c => {
        configs = c;
        // If previously selected config no longer exists, clear it
        const currentConfig = untrack(() => settings.value.dopplerConfig);
        if (currentConfig && !c.some(cfg => cfg.name === currentConfig)) {
          updateSetting('dopplerConfig', '');
        }
      })
      .catch(e => {
        error = e instanceof Error ? e.message : String(e);
        configs = [];
      })
      .finally(() => { loadingConfigs = false; });
  });

  function handleProjectChange(slug: string) {
    updateSetting('dopplerProject', slug);
    updateSetting('dopplerConfig', '');
  }

  function handleConfigChange(name: string) {
    updateSetting('dopplerConfig', name);
  }

  function handleTokenChange(value: string) {
    updateSetting('dopplerToken', value);
  }

  let isConfigured = $derived(
    settings.value.dopplerToken && settings.value.dopplerProject && settings.value.dopplerConfig
  );
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Instructions -->
  <div style="padding: 12px 16px; background: rgba(255, 214, 10, 0.06); border: 1px solid rgba(255, 214, 10, 0.15); border-radius: 8px;">
    <div style="font-size: 13px; color: var(--banana-yellow); font-weight: 600; margin-bottom: 6px;">Connect Doppler</div>
    <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
      Pull secrets from Doppler directly into your preview. No more .env files.
    </div>
    <div style="font-size: 11px; color: var(--text-muted); margin-top: 8px; padding: 8px 10px; background: var(--bg-primary); border-radius: 6px; font-family: var(--font-mono); line-height: 1.8;">
      1. Go to <span style="color: var(--banana-yellow);">dashboard.doppler.com</span><br/>
      2. Click your avatar (bottom-left) > <span style="color: var(--text-primary);">Access Tokens</span><br/>
      3. Click <span style="color: var(--text-primary);">Generate Personal Token</span><br/>
      4. Paste the token below
    </div>
  </div>

  <!-- Token Input -->
  <div style="display: flex; flex-direction: column; gap: 6px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Personal Token</span>
      {#if projects.length > 0}
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-green); display: inline-block;" title="Connected"></span>
      {:else if error}
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-red); display: inline-block;" title={error}></span>
      {/if}
    </div>
    <div style="position: relative; display: flex; align-items: center;">
      <input
        type={tokenVisible ? 'text' : 'password'}
        value={settings.value.dopplerToken}
        oninput={(e) => handleTokenChange((e.currentTarget as HTMLInputElement).value)}
        placeholder="dp.pt.xxxxxxxxxxxxxxxxxxxx"
        style="width: 100%; padding: 8px 36px 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none; transition: border-color 0.15s ease;"
        onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow)'}
        onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
      />
      <button
        style="position: absolute; right: 6px; width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: transparent; border: none; color: var(--text-muted); cursor: pointer; border-radius: 4px; transition: color 0.1s ease;"
        onclick={() => tokenVisible = !tokenVisible}
        onmouseenter={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'}
        onmouseleave={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'}
        aria-label={tokenVisible ? 'Hide token' : 'Show token'}
      >
        {#if tokenVisible}
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 3C4.5 3 1.7 5.1 0.5 8c1.2 2.9 4 5 7.5 5s6.3-2.1 7.5-5c-1.2-2.9-4-5-7.5-5zm0 8.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7zm0-5.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/>
          </svg>
        {:else}
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M13.359 11.238C15.06 9.72 16 8 16 8s-3-5.5-8-5.5a7.028 7.028 0 0 0-2.79.588l.77.771A5.944 5.944 0 0 1 8 3.5c2.12 0 3.879 1.168 5.168 2.457A13.134 13.134 0 0 1 14.828 8c-.058.087-.122.183-.195.288-.335.48-.83 1.12-1.465 1.755-.165.165-.337.328-.517.486l.708.709z"/>
            <path d="M11.297 9.176a3.5 3.5 0 0 0-4.474-4.474l.823.823a2.5 2.5 0 0 1 2.829 2.829l.822.822zm-2.943 1.299l.822.822a3.5 3.5 0 0 1-4.474-4.474l.823.823a2.5 2.5 0 0 0 2.829 2.829z"/>
            <path d="M3.35 5.47c-.18.16-.353.322-.518.487A13.134 13.134 0 0 0 1.172 8l.195.288c.335.48.83 1.12 1.465 1.755C4.121 11.332 5.881 12.5 8 12.5c.716 0 1.39-.133 2.02-.36l.77.772A7.029 7.029 0 0 1 8 13.5C3 13.5 0 8 0 8s.939-1.721 2.641-3.238l.708.709z"/>
            <path d="M13.646 14.354l-12-12 .708-.708 12 12-.708.708z"/>
          </svg>
        {/if}
      </button>
    </div>
    {#if loadingProjects}
      <span style="font-size: 11px; color: var(--text-muted);">Connecting to Doppler...</span>
    {/if}
    {#if error}
      <span style="font-size: 11px; color: var(--accent-red);">{error}</span>
    {/if}
  </div>

  <!-- Project Selector -->
  {#if projects.length > 0}
    <div style="display: flex; flex-direction: column; gap: 6px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Project</span>
      <select
        value={settings.value.dopplerProject}
        onchange={(e) => handleProjectChange((e.target as HTMLSelectElement).value)}
        style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none; cursor: pointer;"
      >
        <option value="">Select a project...</option>
        {#each projects as project}
          <option value={project.slug}>{project.name}</option>
        {/each}
      </select>
    </div>
  {/if}

  <!-- Config Selector -->
  {#if configs.length > 0}
    <div style="display: flex; flex-direction: column; gap: 6px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Config</span>
      <div style="display: flex; gap: 8px; flex-wrap: wrap;">
        {#each configs as config}
          <button
            onclick={() => handleConfigChange(config.name)}
            style="padding: 6px 16px; border: 1px solid {settings.value.dopplerConfig === config.name ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 6px; cursor: pointer; font-size: 12px; font-family: var(--font-ui); transition: all 0.15s ease; background: {settings.value.dopplerConfig === config.name ? 'rgba(255, 214, 10, 0.1)' : 'var(--bg-primary)'}; color: {settings.value.dopplerConfig === config.name ? 'var(--banana-yellow)' : 'var(--text-secondary)'};"
            onmouseenter={(e) => { if (settings.value.dopplerConfig !== config.name) (e.currentTarget as HTMLElement).style.borderColor = 'var(--banana-yellow-dim)'; }}
            onmouseleave={(e) => { if (settings.value.dopplerConfig !== config.name) (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
          >
            {config.name}
          </button>
        {/each}
      </div>
      {#if loadingConfigs}
        <span style="font-size: 11px; color: var(--text-muted);">Loading configs...</span>
      {/if}
    </div>
  {/if}

  <!-- Enable Toggle -->
  {#if isConfigured}
    <div style="display: flex; align-items: center; gap: 16px; padding: 12px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary); font-weight: 500;">Enable Doppler Sync</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">
          Pull secrets from <span style="color: var(--banana-yellow); font-family: var(--font-mono);">{settings.value.dopplerProject}/{settings.value.dopplerConfig}</span> into your preview
        </div>
      </div>
      <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
        <input
          type="checkbox"
          checked={settings.value.dopplerEnabled}
          onchange={(e) => updateSetting('dopplerEnabled', (e.target as HTMLInputElement).checked)}
          style="opacity: 0; width: 0; height: 0;"
        />
        <span style="position: absolute; inset: 0; background: {settings.value.dopplerEnabled ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
          <span style="position: absolute; top: 2px; left: {settings.value.dopplerEnabled ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
        </span>
      </label>
    </div>
  {/if}
</div>
