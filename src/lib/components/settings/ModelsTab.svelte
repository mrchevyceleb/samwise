<script lang="ts">
  import { onMount } from 'svelte';
  import { getSettingsStore, updateSetting, type AppSettings } from '$lib/stores/settings.svelte';
  import { getModelsStore } from '$lib/stores/models.svelte';
  import {
    startOpenAIDeviceOAuth,
    completeOpenAIDeviceOAuth,
    getPendingOpenAIDeviceOAuth,
    cancelOpenAIDeviceOAuth,
    type OpenAIDeviceAuthState,
  } from '$lib/utils/oauth';

  const settingsStore = getSettingsStore();
  const modelsStore = getModelsStore();

  let showApiKey = $state(false);
  let keyToggleHovered = $state(false);

  // OpenAI OAuth state
  let openAIDeviceBusy = $state(false);
  let openAIDeviceStatus = $state('');
  let openAIDeviceState = $state<OpenAIDeviceAuthState | null>(null);

  let providerKey = $derived(
    settingsStore.value.aiProvider === 'anthropic' ? 'aiAnthropicApiKey' as const :
    settingsStore.value.aiProvider === 'openai' ? 'aiOpenAIApiKey' as const :
    settingsStore.value.aiProvider === 'lmstudio' ? null :
    'aiOpenRouterApiKey' as const
  );

  let currentKey = $derived(
    providerKey ? (settingsStore.value[providerKey] as string) : ''
  );

  let baseUrlKey = $derived(
    settingsStore.value.aiProvider === 'anthropic' ? 'aiAnthropicBaseUrl' as const :
    settingsStore.value.aiProvider === 'openai' ? 'aiOpenAIBaseUrl' as const :
    settingsStore.value.aiProvider === 'lmstudio' ? 'aiLMStudioBaseUrl' as const :
    'aiOpenRouterBaseUrl' as const
  );

  let currentBaseUrl = $derived(
    settingsStore.value[baseUrlKey] as string
  );

  function setKey(value: string) {
    if (providerKey) {
      updateSetting(providerKey as keyof AppSettings, value);
    }
  }

  function providerLabel(p: string): string {
    switch (p) {
      case 'anthropic': return 'Anthropic';
      case 'openrouter': return 'OpenRouter';
      case 'openai': return 'OpenAI';
      case 'lmstudio': return 'LM Studio';
      default: return p;
    }
  }

  // ── OpenAI OAuth handlers ──
  async function handleOpenAIOAuthStart() {
    if (openAIDeviceBusy) return;
    openAIDeviceBusy = true;
    openAIDeviceStatus = '';
    try {
      openAIDeviceState = await startOpenAIDeviceOAuth();
      openAIDeviceStatus = 'Browser opened. Complete authorization, then click Complete OAuth.';
    } catch (e) {
      openAIDeviceStatus = `OAuth start failed: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      openAIDeviceBusy = false;
    }
  }

  async function handleOpenAIOAuthComplete() {
    if (openAIDeviceBusy) return;
    openAIDeviceBusy = true;
    openAIDeviceStatus = 'Waiting for authorization...';
    try {
      openAIDeviceStatus = await completeOpenAIDeviceOAuth();
      openAIDeviceState = null;
    } catch (e) {
      openAIDeviceStatus = `OAuth completion failed: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      openAIDeviceBusy = false;
    }
  }

  function handleOpenAIOAuthCancel() {
    cancelOpenAIDeviceOAuth();
    openAIDeviceState = null;
    openAIDeviceStatus = 'OpenAI OAuth flow canceled.';
  }

  async function copyToClipboard(text: string) {
    try { await navigator.clipboard.writeText(text); } catch { /* ignore */ }
  }

  function removeModel(modelId: string) {
    modelsStore.toggleModelEnabled(modelId);
  }

  onMount(() => {
    if (!openAIDeviceState) {
      openAIDeviceState = getPendingOpenAIDeviceOAuth();
    }
  });
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Provider Selection -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Provider</div>
    <div style="display: flex; gap: 8px; flex-wrap: wrap;">
      {#each ['openrouter', 'anthropic', 'openai', 'lmstudio'] as provider}
        <button
          onclick={() => updateSetting('aiProvider', provider as AppSettings['aiProvider'])}
          style="padding: 8px 16px; border: 1px solid {settingsStore.value.aiProvider === provider ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 8px; cursor: pointer; font-size: 12px; font-family: var(--font-ui); transition: all 0.15s ease; background: {settingsStore.value.aiProvider === provider ? 'rgba(255, 214, 10, 0.1)' : 'var(--bg-primary)'}; color: {settingsStore.value.aiProvider === provider ? 'var(--banana-yellow)' : 'var(--text-secondary)'};"
          onmouseenter={(e) => { if (settingsStore.value.aiProvider !== provider) { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--banana-yellow-dim)'; t.style.background = 'var(--bg-elevated)'; }}}
          onmouseleave={(e) => { if (settingsStore.value.aiProvider !== provider) { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.background = 'var(--bg-primary)'; }}}
        >
          {providerLabel(provider)}
        </button>
      {/each}
    </div>
  </div>

  <!-- API Key -->
  {#if providerKey}
    <div style="display: flex; flex-direction: column; gap: 8px;">
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">API Key</div>
      <div style="display: flex; gap: 8px;">
        <input
          type={showApiKey ? 'text' : 'password'}
          value={currentKey}
          oninput={(e) => setKey((e.target as HTMLInputElement).value)}
          placeholder="Enter your API key..."
          style="flex: 1; padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
        />
        <button
          onclick={() => showApiKey = !showApiKey}
          onmouseenter={() => keyToggleHovered = true}
          onmouseleave={() => keyToggleHovered = false}
          style="padding: 8px 12px; border: 1px solid var(--border-default); border-radius: 6px; cursor: pointer; font-size: 11px; transition: all 0.15s ease; background: {keyToggleHovered ? 'var(--bg-elevated)' : 'var(--bg-primary)'}; color: var(--text-secondary);"
        >
          {showApiKey ? 'Hide' : 'Show'}
        </button>
      </div>
      <span style="font-size: 11px; color: var(--text-muted);">
        {#if settingsStore.value.aiProvider === 'openrouter'}
          Get your key at openrouter.ai/keys
        {:else if settingsStore.value.aiProvider === 'anthropic'}
          Get your key at console.anthropic.com
        {:else if settingsStore.value.aiProvider === 'openai'}
          Get your key at platform.openai.com, or use OAuth below
        {/if}
      </span>
    </div>
  {/if}

  <!-- Base URL -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Base URL</div>
    <input
      type="text"
      value={currentBaseUrl}
      oninput={(e) => updateSetting(baseUrlKey, (e.target as HTMLInputElement).value)}
      placeholder={settingsStore.value.aiProvider === 'lmstudio' ? 'http://localhost:1234/v1' : 'Default URL (leave blank for default)'}
      style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
    />
  </div>

  <!-- OpenAI OAuth Device Flow -->
  {#if settingsStore.value.aiProvider === 'openai'}
    <div style="display: flex; flex-direction: column; gap: 8px;">
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">OpenAI OAuth (Codex)</div>
      <div style="display: flex; align-items: center; gap: 8px;">
        <button
          onclick={handleOpenAIOAuthStart}
          disabled={openAIDeviceBusy}
          style="padding: 6px 12px; border: 1px solid var(--border-default); border-radius: 6px; cursor: {openAIDeviceBusy ? 'not-allowed' : 'pointer'}; font-size: 11px; font-family: var(--font-ui); background: var(--bg-primary); color: var(--text-secondary); opacity: {openAIDeviceBusy ? '0.5' : '1'}; transition: all 0.15s ease;"
        >
          {openAIDeviceBusy ? 'Working...' : 'Step 1: Start Device Login'}
        </button>
        <button
          onclick={handleOpenAIOAuthComplete}
          disabled={openAIDeviceBusy || !openAIDeviceState}
          style="padding: 6px 12px; border-radius: 6px; font-size: 11px; font-family: var(--font-ui); font-weight: 700; transition: all 0.15s ease; background: {openAIDeviceBusy || !openAIDeviceState ? 'transparent' : 'rgba(255, 214, 10, 0.15)'}; border: 1px solid {openAIDeviceBusy || !openAIDeviceState ? 'var(--border-default)' : 'var(--banana-yellow-dim)'}; color: {openAIDeviceBusy || !openAIDeviceState ? 'var(--text-muted)' : 'var(--banana-yellow)'}; cursor: {openAIDeviceBusy || !openAIDeviceState ? 'not-allowed' : 'pointer'}; opacity: {openAIDeviceBusy || !openAIDeviceState ? '0.4' : '1'};"
        >
          Step 2: Complete OAuth
        </button>
        <button
          onclick={handleOpenAIOAuthCancel}
          disabled={openAIDeviceBusy || !openAIDeviceState}
          style="padding: 6px 12px; border: 1px solid var(--border-default); border-radius: 6px; cursor: {openAIDeviceBusy || !openAIDeviceState ? 'not-allowed' : 'pointer'}; font-size: 11px; font-family: var(--font-ui); background: var(--bg-primary); color: var(--text-secondary); opacity: {openAIDeviceBusy || !openAIDeviceState ? '0.5' : '1'}; transition: all 0.15s ease;"
        >
          Cancel
        </button>
      </div>

      {#if openAIDeviceState}
        <div style="font-size: 11px; color: var(--text-muted); line-height: 1.6; margin-top: 4px;">
          <div style="display: flex; align-items: center; gap: 8px;">
            <span>Code: <span style="font-family: var(--font-mono); color: var(--text-primary);">{openAIDeviceState.userCode}</span></span>
            <button
              onclick={() => copyToClipboard(openAIDeviceState?.userCode || '')}
              style="padding: 2px 8px; border: 1px solid var(--border-default); border-radius: 4px; background: var(--bg-primary); color: var(--text-secondary); cursor: pointer; font-size: 10px; font-family: var(--font-ui); transition: all 0.1s ease;"
            >
              Copy code
            </button>
          </div>
          <div>
            Verify at
            <a href={openAIDeviceState.verificationUrl} target="_blank" rel="noreferrer" style="color: var(--banana-yellow); text-decoration: underline;">{openAIDeviceState.verificationUrl}</a>
          </div>
          <div style="color: var(--banana-yellow); font-weight: 700;">After approving in browser, click Step 2: Complete OAuth.</div>
        </div>
      {/if}

      {#if openAIDeviceStatus}
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">{openAIDeviceStatus}</div>
      {/if}

      {#if settingsStore.value.aiOpenAIOAuthAccessToken}
        <div style="font-size: 11px; color: rgb(34, 197, 94); margin-top: 2px;">
          OAuth connected{#if settingsStore.value.aiOpenAIOAuthExpiresAt} (expires {settingsStore.value.aiOpenAIOAuthExpiresAt}){/if}
        </div>
        <div style="font-size: 11px; color: var(--text-muted);">
          OAuth token is active and takes precedence over API key.
        </div>
      {/if}
    </div>
  {/if}

  <!-- Default Model -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Default Model</div>
    <input
      type="text"
      value={settingsStore.value.aiModel}
      oninput={(e) => updateSetting('aiModel', (e.target as HTMLInputElement).value)}
      placeholder="e.g. anthropic/claude-sonnet-4-6"
      style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
    />
  </div>

  <!-- Enabled Models -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default); flex: 1;">Enabled Models</div>
      <button
        onclick={() => modelsStore.fetchAvailableModels()}
        disabled={modelsStore.isLoading}
        style="padding: 4px 10px; border: 1px solid var(--border-default); border-radius: 6px; cursor: {modelsStore.isLoading ? 'not-allowed' : 'pointer'}; font-size: 11px; font-family: var(--font-ui); background: var(--bg-primary); color: var(--text-secondary); opacity: {modelsStore.isLoading ? '0.5' : '1'}; transition: all 0.15s ease;"
        onmouseenter={(e) => { if (!modelsStore.isLoading) { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--banana-yellow-dim)'; t.style.color = 'var(--banana-yellow)'; }}}
        onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-secondary)'; }}
      >
        {modelsStore.isLoading ? 'Fetching...' : 'Fetch Models'}
      </button>
    </div>

    <!-- Current enabled models as chips -->
    {#if settingsStore.value.aiEnabledModels.length > 0}
      <div style="display: flex; flex-wrap: wrap; gap: 6px;">
        {#each settingsStore.value.aiEnabledModels as modelId}
          <span style="display: flex; align-items: center; gap: 4px; padding: 3px 8px; background: rgba(255, 214, 10, 0.08); border: 1px solid rgba(255, 214, 10, 0.2); border-radius: 6px; font-size: 11px; color: var(--text-primary); font-family: var(--font-mono);">
            {modelId}
            <button
              onclick={() => removeModel(modelId)}
              style="display: flex; align-items: center; justify-content: center; width: 14px; height: 14px; border: none; background: transparent; color: var(--text-muted); cursor: pointer; padding: 0; font-size: 14px; line-height: 1; transition: color 0.1s ease;"
              onmouseenter={(e) => (e.currentTarget as HTMLElement).style.color = 'rgb(248, 113, 113)'}
              onmouseleave={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'}
            >
              x
            </button>
          </span>
        {/each}
      </div>
    {:else}
      <div style="font-size: 11px; color: var(--text-muted);">No models enabled. Click "Fetch Models" to browse available models.</div>
    {/if}

    <!-- Error -->
    {#if modelsStore.error}
      <div style="font-size: 11px; color: rgb(248, 113, 113);">{modelsStore.error}</div>
    {/if}

    <!-- Fetched models search and list -->
    {#if modelsStore.fetchedModels.length > 0}
      <input
        type="text"
        value={modelsStore.searchQuery}
        oninput={(e) => modelsStore.setSearchQuery((e.target as HTMLInputElement).value)}
        placeholder="Search models..."
        style="padding: 7px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none;"
      />
      <div style="max-height: 200px; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; border: 1px solid var(--border-default); border-radius: 6px; padding: 4px;">
        {#each modelsStore.filteredModels.slice(0, 50) as model (model.id)}
          <button
            onclick={() => modelsStore.toggleModelEnabled(model.id)}
            style="display: flex; align-items: center; gap: 8px; padding: 6px 8px; border: none; border-radius: 4px; cursor: pointer; font-size: 11px; font-family: var(--font-mono); text-align: left; transition: background 0.1s ease; background: {modelsStore.isModelEnabled(model.id) ? 'rgba(255, 214, 10, 0.08)' : 'transparent'}; color: var(--text-primary);"
            onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = modelsStore.isModelEnabled(model.id) ? 'rgba(255, 214, 10, 0.12)' : 'var(--bg-elevated)'}
            onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = modelsStore.isModelEnabled(model.id) ? 'rgba(255, 214, 10, 0.08)' : 'transparent'}
          >
            <span style="width: 14px; height: 14px; border: 1px solid {modelsStore.isModelEnabled(model.id) ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 3px; display: flex; align-items: center; justify-content: center; background: {modelsStore.isModelEnabled(model.id) ? 'var(--banana-yellow)' : 'transparent'}; flex-shrink: 0; transition: all 0.15s ease;">
              {#if modelsStore.isModelEnabled(model.id)}
                <svg width="10" height="10" viewBox="0 0 16 16" fill="#0D1117">
                  <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/>
                </svg>
              {/if}
            </span>
            <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{model.name || model.id}</span>
            {#if model.contextLength}
              <span style="font-size: 10px; color: var(--text-muted); flex-shrink: 0;">{(model.contextLength / 1000).toFixed(0)}k ctx</span>
            {/if}
          </button>
        {/each}
        {#if modelsStore.filteredModels.length > 50}
          <div style="padding: 6px 8px; font-size: 11px; color: var(--text-muted); text-align: center;">
            ...and {modelsStore.filteredModels.length - 50} more. Refine your search.
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
