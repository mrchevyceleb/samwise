<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
  import { fetchWorkplace, fetchProjects } from '$lib/services/doppler';
  import type { DopplerTokenEntry } from '$lib/services/doppler';

  const settings = getSettingsStore();

  let newToken = $state('');
  let addingToken = $state(false);
  let addError = $state('');
  let tokenVisible = $state(false);

  // Migration: if old single dopplerToken exists but no entries in dopplerTokens, migrate it
  let migrated = false;
  $effect(() => {
    if (migrated) return;
    const old = settings.value.dopplerToken;
    const entries = settings.value.dopplerTokens || [];
    if (old && old.trim().length >= 10 && entries.length === 0) {
      migrated = true;
      migrateOldToken(old);
    } else {
      migrated = true;
    }
  });

  async function migrateOldToken(token: string) {
    try {
      const wp = await fetchWorkplace(token);
      const entry: DopplerTokenEntry = {
        token,
        orgName: wp.name || 'Unknown Org',
        orgSlug: wp.id || 'unknown',
      };
      updateSetting('dopplerTokens', [entry]);
    } catch {
      // Migration failed silently, keep old token field as-is
    }
  }

  async function handleAddToken() {
    const token = newToken.trim();
    if (!token || token.length < 10) return;

    // Check for duplicates
    const existing = settings.value.dopplerTokens || [];
    if (existing.some(e => e.token === token)) {
      addError = 'This token is already added.';
      return;
    }

    addingToken = true;
    addError = '';
    try {
      const wp = await fetchWorkplace(token);
      const orgName = wp.name || 'Unknown Org';
      const orgId = wp.id || orgName.toLowerCase().replace(/\s+/g, '-');

      // Verify it can list projects
      await fetchProjects(token);

      const entry: DopplerTokenEntry = { token, orgName, orgSlug: orgId };
      const updated = [...existing, entry];
      updateSetting('dopplerTokens', updated);

      // Also keep dopplerToken in sync with the first/primary token
      if (!settings.value.dopplerToken) {
        updateSetting('dopplerToken', token);
      }

      newToken = '';
    } catch (e) {
      addError = e instanceof Error ? e.message : String(e);
    } finally {
      addingToken = false;
    }
  }

  function handleRemoveToken(index: number) {
    const entries = [...(settings.value.dopplerTokens || [])];
    const removed = entries.splice(index, 1)[0];
    updateSetting('dopplerTokens', entries);

    // If we removed the active token, switch to first remaining
    if (removed && settings.value.dopplerToken === removed.token) {
      updateSetting('dopplerToken', entries.length > 0 ? entries[0].token : '');
    }
  }

  let tokens = $derived(settings.value.dopplerTokens || []);
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Instructions -->
  <div style="padding: 12px 16px; background: color-mix(in srgb, var(--accent-primary) 6%, transparent); border: 1px solid color-mix(in srgb, var(--accent-primary) 15%, transparent); border-radius: 8px;">
    <div style="font-size: 13px; color: var(--accent-primary); font-weight: 600; margin-bottom: 6px;">Connect Doppler</div>
    <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
      Pull secrets from Doppler directly into your preview. Add a token for each organization you work with.
    </div>
    <div style="font-size: 11px; color: var(--text-muted); margin-top: 8px; padding: 8px 10px; background: var(--bg-primary); border-radius: 6px; line-height: 2;">
      <span style="font-family: var(--font-ui);">
        1. Open <span style="color: var(--accent-primary); font-family: var(--font-mono);">dashboard.doppler.com</span><br/>
        2. Click your avatar (bottom-left)<br/>
        3. Select <span style="color: var(--text-primary); font-weight: 600;">Personal Tokens</span> (NOT service tokens)<br/>
        4. Click <span style="color: var(--text-primary); font-weight: 600;">+ Generate</span>, name it anything (e.g. "Banana Code")<br/>
        5. Copy the token (starts with <span style="font-family: var(--font-mono); color: var(--accent-primary);">dp.pt.</span>) and paste below
      </span>
    </div>
    <div style="font-size: 10px; color: var(--text-muted); margin-top: 6px; opacity: 0.7; font-family: var(--font-ui);">
      Each personal token is scoped to one organization. Add one token per org.
    </div>
  </div>

  <!-- Connected Organizations -->
  {#if tokens.length > 0}
    <div style="display: flex; flex-direction: column; gap: 6px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Connected Organizations</span>
      {#each tokens as entry, i}
        <div style="display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: rgba(108, 71, 255, 0.06); border: 1px solid rgba(108, 71, 255, 0.2); border-radius: 8px;">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#6C47FF" stroke-width="1.5"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>
          <div style="flex: 1; min-width: 0;">
            <div style="font-size: 13px; font-weight: 600; color: #6C47FF;">{entry.orgName}</div>
            <div style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">ID: {entry.orgSlug}</div>
          </div>
          <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-green); flex-shrink: 0;" title="Connected"></span>
          <span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">dp.pt.{entry.token.slice(6, 12)}...</span>
          <button
            style="width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: transparent; border: 1px solid transparent; border-radius: 4px; color: var(--text-muted); cursor: pointer; transition: all 0.1s ease; flex-shrink: 0;"
            onclick={() => handleRemoveToken(i)}
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color = '#ff6b6b'; (e.currentTarget as HTMLElement).style.background = 'rgba(255, 107, 107, 0.1)'; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            aria-label="Remove token"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <path d="M5 1h2a1 1 0 0 1 1 1H4a1 1 0 0 1 1-1zM3 2a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2h2.5a.5.5 0 0 1 0 1h-.441l-.443 7.107A2 2 0 0 1 8.622 12H3.378a2 2 0 0 1-1.994-1.893L.941 3H.5a.5.5 0 0 1 0-1H3zm-.944 1l.436 7.003A1 1 0 0 0 3.378 11h5.244a1 1 0 0 0 .997-.947L10.055 3H2.056z"/>
            </svg>
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Add Token -->
  <div style="display: flex; flex-direction: column; gap: 6px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">
      {tokens.length > 0 ? 'Add Another Organization' : 'Add Token'}
    </span>
    <div style="display: flex; gap: 6px; align-items: center;">
      <div style="flex: 1; position: relative; display: flex; align-items: center;">
        <input
          type={tokenVisible ? 'text' : 'password'}
          bind:value={newToken}
          placeholder="dp.pt.xxxxxxxxxxxxxxxxxxxx"
          style="width: 100%; padding: 8px 36px 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none; transition: border-color 0.15s ease;"
          onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent-primary)'}
          onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'}
          onkeydown={(e) => { if (e.key === 'Enter') handleAddToken(); }}
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
      <button
        onclick={handleAddToken}
        disabled={addingToken || newToken.trim().length < 10}
        style="height: 36px; padding: 0 16px; background: {addingToken ? 'var(--bg-elevated)' : 'var(--accent-primary)'}; border: none; border-radius: 6px; color: #0D1117; font-size: 12px; font-weight: 600; font-family: var(--font-ui); cursor: pointer; opacity: {addingToken || newToken.trim().length < 10 ? 0.5 : 1}; transition: all 0.12s ease; white-space: nowrap;"
      >
        {addingToken ? 'Connecting...' : 'Connect'}
      </button>
    </div>
    {#if addError}
      <span style="font-size: 11px; color: var(--accent-red);">{addError}</span>
    {/if}
  </div>

  <!-- Next steps -->
  {#if tokens.length > 0}
    <div style="padding: 12px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <div style="font-size: 12px; color: var(--text-secondary); font-family: var(--font-ui); line-height: 1.6;">
        {tokens.length} organization{tokens.length !== 1 ? 's' : ''} connected. To link secrets to a workspace:
      </div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 6px; font-family: var(--font-ui); line-height: 1.8;">
        1. Open a project in Banana Code<br/>
        2. Click the lock icon in the preview toolbar to open Environment Variables<br/>
        3. Click <span style="color: #6C47FF; font-weight: 600;">Link Doppler Project</span><br/>
        4. {#if tokens.length > 1}Pick the organization, then select{:else}Select{/if} your project and config. Secrets sync automatically.
      </div>
    </div>
  {/if}
</div>
