<script lang="ts">
  import { safeInvoke } from '$lib/utils/tauri';

  let loading = $state(false);
  let status = $state<'idle' | 'success' | 'error'>('idle');
  let statusMessage = $state('');
  let configUrl = $state('');

  async function checkConfig() {
    const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_get_config');
    if (config && config.url) {
      configUrl = config.url;
      status = 'success';
      statusMessage = 'Supabase configured';
    } else {
      configUrl = '';
      status = 'idle';
      statusMessage = 'Not configured';
    }
  }

  async function loadFromDoppler() {
    loading = true;
    status = 'idle';
    statusMessage = '';
    const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
    if (config && config.url) {
      configUrl = config.url;
      status = 'success';
      statusMessage = 'Loaded from Doppler successfully';
    } else {
      status = 'error';
      statusMessage = 'Failed to load from Doppler. Is Doppler CLI installed and configured?';
    }
    loading = false;
  }

  // Check config on mount
  $effect(() => {
    checkConfig();
  });
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <div style="padding: 12px 16px; background: color-mix(in srgb, var(--accent-primary) 6%, transparent); border: 1px solid color-mix(in srgb, var(--accent-primary) 15%, transparent); border-radius: 8px;">
    <div style="font-size: 13px; color: var(--accent-primary); font-weight: 600; margin-bottom: 6px;">Doppler Integration</div>
    <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
      Load Supabase credentials directly from Doppler. Requires the Doppler CLI to be installed and authenticated on this machine.
    </div>
  </div>

  <!-- Current Status -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Supabase Config Status</span>
    <div style="display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <span style="
        width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0;
        background: {status === 'success' ? 'var(--accent-green, #3fb950)' : status === 'error' ? '#f85149' : 'var(--text-muted)'};
      "></span>
      <div style="flex: 1; min-width: 0;">
        {#if configUrl}
          <div style="font-size: 12px; color: var(--text-primary); font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{configUrl}</div>
        {:else}
          <div style="font-size: 12px; color: var(--text-muted);">No Supabase URL configured</div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Load from Doppler Button -->
  <div style="display: flex; gap: 12px; align-items: center;">
    <button
      onclick={loadFromDoppler}
      disabled={loading}
      style="
        padding: 8px 16px; border-radius: 6px; font-size: 13px; font-weight: 600;
        font-family: var(--font-ui); cursor: {loading ? 'wait' : 'pointer'};
        background: {loading ? 'var(--bg-elevated)' : '#6C47FF'};
        color: white; border: none;
        opacity: {loading ? 0.6 : 1};
        transition: all 0.15s ease;
      "
    >
      {loading ? 'Loading...' : 'Load from Doppler'}
    </button>
    {#if statusMessage}
      <span style="font-size: 12px; color: {status === 'success' ? 'var(--accent-green, #3fb950)' : status === 'error' ? '#f85149' : 'var(--text-muted)'};">
        {statusMessage}
      </span>
    {/if}
  </div>
</div>
