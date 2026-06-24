<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
  import { safeInvoke } from '$lib/utils/tauri';
  import SchedulingTab from './SchedulingTab.svelte';
  import ProjectsTab from './ProjectsTab.svelte';
  import RulesTab from './RulesTab.svelte';
  import NotificationsTab from './NotificationsTab.svelte';
  import AutoMergeTab from './AutoMergeTab.svelte';

  const settingsStore = getSettingsStore();

  type Tab = 'connection' | 'worker' | 'llm-proxy' | 'projects' | 'rules' | 'notifications' | 'automerge' | 'automation' | 'about';
  let closeBtnHovered = $state(false);
  let hoveredTab = $state<string | null>(null);

  let activeTab = $derived(settingsStore.activeSettingsTab as Tab);
  function setActiveTab(tab: Tab) { settingsStore.activeSettingsTab = tab; }

  // Connection state
  let supabaseUrl = $state('');
  let anonKey = $state('');
  let serviceRoleKey = $state('');
  let connectionStatus = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
  let connectionMessage = $state('');
  let loadingDoppler = $state(false);

  // Load config on mount
  $effect(() => {
    if (settingsStore.settingsVisible) {
      loadConfig();
    }
  });

  // LLM Proxy state
  let proxyStatus = $state<'idle' | 'checking' | 'healthy' | 'error'>('idle');
  let proxyMessage = $state('');

  async function checkProxy() {
    const s = settingsStore.value;
    if (!s.llmProxyBaseUrl?.trim()) {
      proxyStatus = 'error';
      proxyMessage = 'No proxy URL configured';
      return;
    }
    proxyStatus = 'checking';
    proxyMessage = 'Checking proxy...';
    try {
      const result = await safeInvoke<string>('check_llm_proxy', { proxyUrl: s.llmProxyBaseUrl });
      proxyStatus = 'healthy';
      proxyMessage = result || 'Proxy healthy';
    } catch (e: any) {
      proxyStatus = 'error';
      proxyMessage = e?.toString?.() || 'Proxy unreachable';
    }
  }

  async function loadConfig() {
    const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_get_config');
    if (config) {
      supabaseUrl = config.url;
      anonKey = config.anon_key;
      serviceRoleKey = config.service_role_key || '';
    }
  }

  async function saveConfig() {
    await safeInvoke('supabase_set_config', {
      url: supabaseUrl,
      anon_key: anonKey,
      service_role_key: serviceRoleKey || null,
    });
  }

  async function testConnection() {
    await saveConfig();
    connectionStatus = 'testing';
    connectionMessage = '';
    const result = await safeInvoke<string>('supabase_test_connection');
    if (result) {
      connectionStatus = 'success';
      connectionMessage = result;
    } else {
      connectionStatus = 'error';
      connectionMessage = 'Connection failed';
    }
  }

  async function loadFromDoppler() {
    loadingDoppler = true;
    const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
    if (config) {
      supabaseUrl = config.url;
      anonKey = config.anon_key;
      serviceRoleKey = config.service_role_key || '';
      connectionStatus = 'success';
      connectionMessage = 'Loaded from Doppler';
    } else {
      connectionStatus = 'error';
      connectionMessage = 'Doppler load failed';
    }
    loadingDoppler = false;
  }

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'connection', label: 'Connection', icon: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z' },
    { id: 'worker', label: 'Worker', icon: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm0 0' },
    { id: 'llm-proxy', label: 'LLM Proxy', icon: 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z' },
    { id: 'projects', label: 'Projects', icon: 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z' },
    { id: 'rules', label: 'Rules', icon: 'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4' },
    { id: 'notifications', label: 'Notifications', icon: 'M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0' },
    { id: 'automerge', label: 'Auto-Merge', icon: 'M6 3v12a3 3 0 003 3 3 3 0 003-3V3M6 21h.01M18 9a3 3 0 11-6 0 3 3 0 016 0z' },
    { id: 'automation', label: 'Scheduling', icon: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z' },
    { id: 'about', label: 'About', icon: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z' },
  ];

  function close() { settingsStore.settingsVisible = false; }
  function handleKeyDown(e: KeyboardEvent) { if (e.key === 'Escape') close(); }
  function handleOverlayClick(e: MouseEvent) { if (e.target === e.currentTarget) close(); }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if settingsStore.settingsVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div onclick={handleOverlayClick} style="position: fixed; inset: 0; z-index: 1000; background: rgba(0,0,0,0.6); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center;">
    <div style="width: 90vw; max-width: 900px; height: 80vh; max-height: 700px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 12px; box-shadow: 0 24px 64px rgba(0,0,0,0.5); display: flex; overflow: hidden;">
      <!-- Sidebar -->
      <div style="width: 200px; background: var(--bg-primary); border-right: 1px solid var(--border-default); padding: 16px 0; display: flex; flex-direction: column;">
        <div style="padding: 0 16px 16px; display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 16px; width: 24px; height: 24px; background: var(--accent-indigo); border-radius: 6px; display: flex; align-items: center; justify-content: center;">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v4m0 12v4m-7.07-2.93l2.83-2.83m8.48-8.48l2.83-2.83M2 12h4m12 0h4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83"/></svg>
          </span>
          <span style="font-size: 14px; font-weight: 700; color: var(--text-primary);">Settings</span>
        </div>
        <div style="flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 0 8px;">
          {#each tabs as tab (tab.id)}
            <button
              onclick={() => setActiveTab(tab.id)}
              onmouseenter={() => hoveredTab = tab.id}
              onmouseleave={() => hoveredTab = null}
              style="display: flex; align-items: center; gap: 10px; padding: 8px 12px; border: none; border-radius: 6px; cursor: pointer; font-size: 13px; font-family: var(--font-ui); transition: all 0.12s ease; text-align: left;
                background: {activeTab === tab.id ? 'rgba(99,102,241,0.1)' : hoveredTab === tab.id ? 'var(--bg-elevated)' : 'transparent'};
                color: {activeTab === tab.id ? 'var(--accent-indigo)' : 'var(--text-secondary)'};
                border-left: {activeTab === tab.id ? '2px solid var(--accent-indigo)' : '2px solid transparent'};"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d={tab.icon}/></svg>
              {tab.label}
            </button>
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div style="flex: 1; display: flex; flex-direction: column; overflow: hidden;">
        <div style="display: flex; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border-default);">
          <span style="font-size: 16px; font-weight: 600; color: var(--text-primary); flex: 1;">
            {tabs.find(t => t.id === activeTab)?.label || 'Settings'}
          </span>
          <button
            onclick={close}
            onmouseenter={() => closeBtnHovered = true}
            onmouseleave={() => closeBtnHovered = false}
            aria-label="Close settings"
            style="display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; border: none; border-radius: 6px; cursor: pointer; transition: all 0.15s ease;
              background: {closeBtnHovered ? 'rgba(248,81,73,0.15)' : 'transparent'};
              color: {closeBtnHovered ? '#f85149' : 'var(--text-muted)'};
              transform: {closeBtnHovered ? 'rotate(90deg)' : 'rotate(0)'};"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>

        <div style="flex: 1; overflow-y: auto; padding: 20px;">
          {#if activeTab === 'connection'}
            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Supabase Connection</div>

              <div style="display: flex; flex-direction: column; gap: 8px;">
                <label style="font-size: 12px; color: var(--text-secondary);">Supabase URL</label>
                <input bind:value={supabaseUrl} onblur={saveConfig} placeholder="https://your-project.supabase.co"
                  style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;" />
              </div>

              <div style="display: flex; flex-direction: column; gap: 8px;">
                <label style="font-size: 12px; color: var(--text-secondary);">Anon Key</label>
                <input bind:value={anonKey} onblur={saveConfig} type="password" placeholder="eyJhbGci..."
                  style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;" />
              </div>

              <div style="display: flex; flex-direction: column; gap: 8px;">
                <label style="font-size: 12px; color: var(--text-secondary);">Service Role Key (optional)</label>
                <input bind:value={serviceRoleKey} onblur={saveConfig} type="password" placeholder="eyJhbGci..."
                  style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;" />
              </div>

              <div style="display: flex; gap: 12px; align-items: center;">
                <button onclick={testConnection}
                  style="padding: 8px 16px; background: #6366f1; color: white; border: none; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s ease;">
                  {connectionStatus === 'testing' ? 'Testing...' : 'Test Connection'}
                </button>
                <button onclick={loadFromDoppler}
                  style="padding: 8px 16px; background: var(--bg-primary); color: var(--text-secondary); border: 1px solid var(--border-default); border-radius: 6px; font-size: 13px; cursor: pointer; transition: all 0.15s ease;">
                  {loadingDoppler ? 'Loading...' : 'Load from Doppler'}
                </button>
                {#if connectionStatus === 'success'}
                  <span style="font-size: 12px; color: #3fb950;">{connectionMessage}</span>
                {:else if connectionStatus === 'error'}
                  <span style="font-size: 12px; color: #f85149;">{connectionMessage}</span>
                {/if}
              </div>
            </div>

          {:else if activeTab === 'worker'}
            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Worker Settings</div>
              <div style="font-size: 13px; color: var(--text-secondary);">
                The worker runs on this machine and picks up tasks from the Kanban board. Start/stop it from the status bar.
              </div>
              <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                <div style="flex: 1;">
                  <div style="font-size: 13px; color: var(--text-primary);">Machine Role</div>
                  <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">
                    {settingsStore.value.isMaster ? 'This is Sam\'s home machine (master)' : 'This machine is in viewer mode'}
                  </div>
                </div>
                <button
                  onclick={() => {
                    // Show the master/viewer prompt immediately
                    settingsStore.reconfigureRequested = true;
                    close();
                  }}
                  style="padding: 6px 12px; background: var(--bg-primary); color: var(--text-secondary); border: 1px solid var(--border-default); border-radius: 6px; font-size: 12px; cursor: pointer; transition: all 0.15s ease;"
                >
                  Reconfigure
                </button>
              </div>
            </div>

          {:else if activeTab === 'llm-proxy'}
            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">LLM Proxy</div>
              <div style="font-size: 13px; color: var(--text-secondary);">
                Route Claude Code through a LiteLLM proxy to use Fireworks GLM instead of Anthropic directly. Same agent loop, 15-20x cheaper.
              </div>

              <!-- Enable toggle -->
              <div style="display: flex; align-items: center; gap: 16px; padding: 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
                <label style="display: flex; align-items: center; gap: 10px; cursor: pointer; flex: 1;">
                  <input
                    type="checkbox"
                    checked={settingsStore.value.llmProxyEnabled}
                    onchange={(e: any) => { updateSetting('llmProxyEnabled', e.target.checked); }}
                    style="width: 16px; height: 16px; accent-color: var(--accent-indigo);"
                  />
                  <div>
                    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Enable LLM Proxy</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Route Claude Code through LiteLLM to Fireworks GLM</div>
                  </div>
                </label>
              </div>

              {#if settingsStore.value.llmProxyEnabled}
                <!-- Backend selector -->
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <div style="font-size: 12px; font-weight: 600; color: var(--text-primary);">Backend Model</div>
                  <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                    {#each ([
                      { id: 'fireworks-glm-5.1', label: 'Fireworks GLM 5.1', cost: '$0.98/$3.08 per 1M' },
                      { id: 'fireworks-glm-5.2', label: 'Fireworks GLM 5.2', cost: '$1.40/$4.40 per 1M (soon)' },
                      { id: 'openrouter-glm-5.2', label: 'OpenRouter GLM 5.2', cost: '$1.40/$4.40 per 1M' },
                      { id: 'custom', label: 'Custom', cost: '' },
                    ] as const) as backend}
                      <button
                        onclick={() => updateSetting('llmProxyBackend', backend.id)}
                        style="padding: 8px 12px; border-radius: 6px; font-size: 12px; cursor: pointer; transition: all 0.15s ease; text-align: left;
                          background: {settingsStore.value.llmProxyBackend === backend.id ? 'rgba(99,102,241,0.15)' : 'var(--bg-primary)'};
                          border: 1px solid {settingsStore.value.llmProxyBackend === backend.id ? 'var(--accent-indigo)' : 'var(--border-default)'};
                          color: {settingsStore.value.llmProxyBackend === backend.id ? 'var(--accent-indigo)' : 'var(--text-secondary)'};"
                      >
                        <div style="font-weight: 600;">{backend.label}</div>
                        {#if backend.cost}
                          <div style="font-size: 10px; opacity: 0.7; margin-top: 2px;">{backend.cost}</div>
                        {/if}
                      </button>
                    {/each}
                  </div>
                </div>

                <!-- Proxy URL -->
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <div style="font-size: 12px; font-weight: 600; color: var(--text-primary);">Proxy Base URL</div>
                  <div style="font-size: 11px; color: var(--text-muted);">The LiteLLM proxy address (usually localhost:9876)</div>
                  <input
                    value={settingsStore.value.llmProxyBaseUrl}
                    oninput={(e: any) => updateSetting('llmProxyBaseUrl', e.target.value)}
                    placeholder="http://127.0.0.1:9876"
                    style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-mono); outline: none;"
                  />
                </div>

                <!-- Proxy API Key -->
                <div style="display: flex; flex-direction: column; gap: 6px;">
                  <div style="font-size: 12px; font-weight: 600; color: var(--text-primary);">Proxy API Key</div>
                  <div style="font-size: 11px; color: var(--text-muted);">Set in LiteLLM config if you enabled master key. Usually left empty.</div>
                  <input
                    type="password"
                    value={settingsStore.value.llmProxyApiKey}
                    oninput={(e: any) => updateSetting('llmProxyApiKey', e.target.value)}
                    placeholder="Leave empty if LiteLLM has no auth"
                    style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-mono); outline: none;"
                  />
                </div>

                <!-- Health check -->
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <button
                    onclick={checkProxy}
                    style="padding: 8px 16px; background: var(--accent-indigo); border: none; border-radius: 6px; color: white; font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s ease; align-self: flex-start;"
                  >
                    Check Proxy Health
                  </button>
                  {#if proxyStatus !== 'idle'}
                    <div style="display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px;">
                      <span style="width: 8px; height: 8px; border-radius: 50%; background: {proxyStatus === 'healthy' ? '#3fb950' : proxyStatus === 'error' ? '#f85149' : '#6366f1'}; {proxyStatus === 'checking' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"></span>
                      <span style="font-size: 12px; color: var(--text-secondary);">{proxyMessage}</span>
                    </div>
                  {/if}
                </div>

                <!-- Cost comparison -->
                <div style="padding: 12px; background: rgba(63,185,80,0.05); border: 1px solid rgba(63,185,80,0.2); border-radius: 8px;">
                  <div style="font-size: 12px; font-weight: 600; color: #3fb950; margin-bottom: 8px;">Cost Savings</div>
                  <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 8px; font-size: 11px;">
                    <div style="color: var(--text-muted);">Model</div>
                    <div style="color: var(--text-muted);">Input/1M</div>
                    <div style="color: var(--text-muted);">Output/1M</div>
                    <div style="color: var(--text-secondary);">Claude Opus 4</div>
                    <div style="color: #f85149;">$15.00</div>
                    <div style="color: #f85149;">$75.00</div>
                    <div style="color: var(--text-primary); font-weight: 600;">GLM 5.1</div>
                    <div style="color: #3fb950;">$0.98</div>
                    <div style="color: #3fb950;">$3.08</div>
                    <div style="color: var(--text-primary); font-weight: 600;">GLM 5.2</div>
                    <div style="color: #3fb950;">$1.40</div>
                    <div style="color: #3fb950;">$4.40</div>
                  </div>
                </div>

                <!-- Vision Adapter -->
                <div style="padding: 12px; background: rgba(99,102,241,0.05); border: 1px solid rgba(99,102,241,0.2); border-radius: 8px;">
                  <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 8px;">
                    <span style="font-size: 16px;">👁️</span>
                    <div style="font-size: 12px; font-weight: 600; color: var(--accent-indigo);">Vision Adapter</div>
                  </div>
                  <div style="font-size: 11px; color: var(--text-muted); margin-bottom: 8px;">
                    GLM can't see images. When enabled, image attachments are described by a local vision model before being passed to the coding model. Same pattern as Pi's Vision Adapter.
                  </div>
                  <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                      type="checkbox"
                      checked={settingsStore.value.llmVisionAdapterEnabled}
                      onchange={(e: any) => { updateSetting('llmVisionAdapterEnabled', e.target.checked); }}
                      style="width: 14px; height: 14px; accent-color: var(--accent-indigo);"
                    />
                    <span style="font-size: 12px; color: var(--text-secondary);">Enable vision adapter for image attachments</span>
                  </label>
                  {#if settingsStore.value.llmVisionAdapterEnabled}
                    <div style="display: flex; gap: 8px; margin-top: 8px;">
                      <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
                        <div style="font-size: 10px; color: var(--text-muted);">Vision Model URL</div>
                        <input
                          value={settingsStore.value.llmVisionAdapterUrl}
                          oninput={(e: any) => updateSetting('llmVisionAdapterUrl', e.target.value)}
                          placeholder="http://localhost:1234/v1"
                          style="padding: 6px 8px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 4px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;"
                        />
                      </div>
                      <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
                        <div style="font-size: 10px; color: var(--text-muted);">Vision Model</div>
                        <input
                          value={settingsStore.value.llmVisionAdapterModel}
                          oninput={(e: any) => updateSetting('llmVisionAdapterModel', e.target.value)}
                          placeholder="qwen3-vl-4b-instruct"
                          style="padding: 6px 8px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 4px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;"
                        />
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>

          {:else if activeTab === 'projects'}
            <ProjectsTab />

          {:else if activeTab === 'rules'}
            <RulesTab />

          {:else if activeTab === 'notifications'}
            <NotificationsTab />

          {:else if activeTab === 'automerge'}
            <AutoMergeTab />

          {:else if activeTab === 'automation'}
            <SchedulingTab />

          {:else if activeTab === 'about'}
            <div style="display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 24px;">
              <div style="width: 72px; height: 72px; border-radius: 50%; overflow: hidden; border: 2px solid rgba(99,102,241,0.3); box-shadow: 0 4px 16px rgba(99,102,241,0.2); animation: bob 3s ease-in-out infinite;">
                <img src="/samwise-avatar.png" alt="SamWise" style="width: 100%; height: 100%; object-fit: cover;" />
              </div>
              <div style="text-align: center;">
                <div style="font-size: 24px; font-weight: 700; color: var(--text-primary);">SamWise</div>
                <div style="font-size: 13px; color: var(--text-secondary); margin-top: 4px;">Your AI Employee. Always on. Always ON IT.</div>
                <div style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">Version 0.1.0</div>
              </div>
              <div style="color: var(--text-muted); font-size: 11px; margin-top: 16px;">Built with Tauri 2 + SvelteKit 5 + Rust</div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
