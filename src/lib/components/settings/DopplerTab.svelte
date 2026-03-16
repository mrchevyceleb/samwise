<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
  import { fetchProjects, type DopplerProject } from '$lib/services/doppler';

  const settings = getSettingsStore();

  let projects = $state<DopplerProject[]>([]);
  let loadingProjects = $state(false);
  let error = $state('');
  let tokenVisible = $state(false);
  let fetchDebounce: ReturnType<typeof setTimeout> | null = null;

  // Load projects on mount if token exists
  let initialized = false;
  $effect(() => {
    const token = settings.value.dopplerToken;
    if (!initialized && token && token.trim().length >= 10) {
      initialized = true;
      loadProjects(token);
    }
  });

  function handleTokenInput(value: string) {
    updateSetting('dopplerToken', value);
    if (fetchDebounce) clearTimeout(fetchDebounce);
    if (!value || value.trim().length < 10) {
      projects = [];
      error = '';
      return;
    }
    fetchDebounce = setTimeout(() => loadProjects(value), 500);
  }

  async function loadProjects(token: string) {
    loadingProjects = true;
    error = '';
    try {
      projects = await fetchProjects(token);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      projects = [];
    } finally {
      loadingProjects = false;
    }
  }
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Instructions -->
  <div style="padding: 12px 16px; background: rgba(255, 214, 10, 0.06); border: 1px solid rgba(255, 214, 10, 0.15); border-radius: 8px;">
    <div style="font-size: 13px; color: var(--banana-yellow); font-weight: 600; margin-bottom: 6px;">Connect Doppler</div>
    <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
      Pull secrets from Doppler directly into your preview. No more .env files.
    </div>
    <div style="font-size: 11px; color: var(--text-muted); margin-top: 8px; padding: 8px 10px; background: var(--bg-primary); border-radius: 6px; line-height: 2;">
      <span style="font-family: var(--font-ui);">
        1. Open <span style="color: var(--banana-yellow); font-family: var(--font-mono);">dashboard.doppler.com</span><br/>
        2. Click your avatar (bottom-left)<br/>
        3. Select <span style="color: var(--text-primary); font-weight: 600;">Personal Tokens</span> (NOT service tokens)<br/>
        4. Click <span style="color: var(--text-primary); font-weight: 600;">+ Generate</span>, name it anything (e.g. "Banana Code")<br/>
        5. Copy the token (starts with <span style="font-family: var(--font-mono); color: var(--banana-yellow);">dp.pt.</span>) and paste below
      </span>
    </div>
    <div style="font-size: 10px; color: var(--text-muted); margin-top: 6px; opacity: 0.7; font-family: var(--font-ui);">
      Personal tokens let Banana Code list your projects. Service tokens (dp.st.) won't work here.
    </div>
  </div>

  <!-- Token Input -->
  <div style="display: flex; flex-direction: column; gap: 6px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Personal Token</span>
      {#if projects.length > 0}
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-green); display: inline-block;" title="Connected - {projects.length} projects found"></span>
        <span style="font-size: 11px; color: var(--accent-green);">Connected ({projects.length} projects)</span>
      {:else if error}
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-red); display: inline-block;" title={error}></span>
      {/if}
    </div>
    <div style="position: relative; display: flex; align-items: center;">
      <input
        type={tokenVisible ? 'text' : 'password'}
        value={settings.value.dopplerToken}
        oninput={(e) => handleTokenInput((e.currentTarget as HTMLInputElement).value)}
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

  <!-- Next steps -->
  {#if projects.length > 0}
    <div style="padding: 12px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <div style="font-size: 12px; color: var(--text-secondary); font-family: var(--font-ui); line-height: 1.6;">
        Token is working. To link a Doppler project to your workspace:
      </div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 6px; font-family: var(--font-ui); line-height: 1.8;">
        1. Open a project in Banana Code<br/>
        2. Click the lock icon in the preview toolbar to open Environment Variables<br/>
        3. Click <span style="color: #6C47FF; font-weight: 600;">Link Doppler Project</span><br/>
        4. Select your project and config. Secrets sync automatically.
      </div>
      <div style="font-size: 10px; color: var(--text-muted); margin-top: 6px; opacity: 0.7; font-family: var(--font-ui);">
        Each workspace remembers its own Doppler project link.
      </div>
    </div>
  {/if}
</div>
