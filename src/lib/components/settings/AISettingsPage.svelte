<script lang="ts">
  import { getSettingsStore, updateSetting, type AppSettings } from '$lib/stores/settings';

  const settingsStore = getSettingsStore();

  let showApiKey = $state(false);
  let keyToggleHovered = $state(false);

  let providerKey = $derived(
    settingsStore.value.aiProvider === 'anthropic' ? 'aiAnthropicApiKey' :
    settingsStore.value.aiProvider === 'openai' ? 'aiOpenAIApiKey' :
    settingsStore.value.aiProvider === 'lmstudio' ? null :
    'aiOpenRouterApiKey'
  );

  let currentKey = $derived(
    providerKey ? settingsStore.value[providerKey] as string : ''
  );

  function setKey(value: string) {
    if (providerKey) {
      updateSetting(providerKey, value);
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
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Provider Selection -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">Provider</span>
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
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">API Key</span>
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
          Get your key at platform.openai.com
        {/if}
      </span>
    </div>
  {/if}

  <!-- Model -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">Model</span>
    <input
      type="text"
      value={settingsStore.value.aiModel}
      oninput={(e) => updateSetting('aiModel', (e.target as HTMLInputElement).value)}
      placeholder="e.g. anthropic/claude-sonnet-4-6"
      style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
    />
  </div>

  <!-- Temperature -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); flex: 1;">Temperature</span>
      <span style="font-size: 12px; color: var(--banana-yellow); font-family: var(--font-mono);">{settingsStore.value.aiTemperature.toFixed(1)}</span>
    </div>
    <input
      type="range"
      min="0"
      max="2"
      step="0.1"
      value={settingsStore.value.aiTemperature}
      oninput={(e) => updateSetting('aiTemperature', parseFloat((e.target as HTMLInputElement).value))}
      style="width: 100%; accent-color: var(--banana-yellow); cursor: pointer;"
    />
  </div>

  <!-- Max Tokens -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); flex: 1;">Max Tokens</span>
      <span style="font-size: 12px; color: var(--banana-yellow); font-family: var(--font-mono);">{settingsStore.value.aiMaxTokens.toLocaleString()}</span>
    </div>
    <input
      type="range"
      min="1024"
      max="128000"
      step="1024"
      value={settingsStore.value.aiMaxTokens}
      oninput={(e) => updateSetting('aiMaxTokens', parseInt((e.target as HTMLInputElement).value))}
      style="width: 100%; accent-color: var(--banana-yellow); cursor: pointer;"
    />
  </div>

  <!-- LM Studio base URL -->
  {#if settingsStore.value.aiProvider === 'lmstudio'}
    <div style="display: flex; flex-direction: column; gap: 8px;">
      <span style="font-size: 13px; font-weight: 600; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">Base URL</span>
      <input
        type="text"
        value={settingsStore.value.aiLMStudioBaseUrl}
        oninput={(e) => updateSetting('aiLMStudioBaseUrl', (e.target as HTMLInputElement).value)}
        placeholder="http://localhost:1234/v1"
        style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
      />
    </div>
  {/if}
</div>
