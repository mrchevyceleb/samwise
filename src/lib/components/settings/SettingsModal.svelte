<script lang="ts">
  import { getSettingsStore, updateSetting, type AppSettings } from '$lib/stores/settings.svelte';
  import AISettingsPage from './AISettingsPage.svelte';
  import DopplerTab from './DopplerTab.svelte';

  const settingsStore = getSettingsStore();

  type Tab = 'general' | 'ai' | 'doppler' | 'about';
  let closeBtnHovered = $state(false);
  let hoveredTab = $state<string | null>(null);

  // Use store-level tab state so external buttons (e.g. StatusBar) can control which tab opens
  let activeTab = $derived(settingsStore.activeSettingsTab as Tab);
  function setActiveTab(tab: Tab) { settingsStore.activeSettingsTab = tab; }

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'general', label: 'General', icon: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm0 0' },
    { id: 'ai', label: 'AI & Tools', icon: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z' },
    { id: 'doppler', label: 'Doppler', icon: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z' },
    { id: 'about', label: 'About', icon: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z' },
  ];

  function close() {
    settingsStore.settingsVisible = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if settingsStore.settingsVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    onclick={handleOverlayClick}
    style="position: fixed; inset: 0; z-index: 1000; background: rgba(0, 0, 0, 0.6); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center;"
  >
    <div style="width: 90vw; max-width: 900px; height: 80vh; max-height: 700px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 12px; box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5); display: flex; overflow: hidden;">
      <!-- Sidebar -->
      <div style="width: 200px; background: var(--bg-primary); border-right: 1px solid var(--border-default); padding: 16px 0; display: flex; flex-direction: column;">
        <div style="padding: 0 16px 16px; display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 18px;">🍌</span>
          <span style="font-size: 14px; font-weight: 700; color: var(--banana-yellow);">Settings</span>
        </div>
        <div style="flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 0 8px;">
          {#each tabs as tab (tab.id)}
            <button
              onclick={() => setActiveTab(tab.id)}
              onmouseenter={() => hoveredTab = tab.id}
              onmouseleave={() => hoveredTab = null}
              style="display: flex; align-items: center; gap: 10px; padding: 8px 12px; border: none; border-radius: 6px; cursor: pointer; font-size: 13px; font-family: var(--font-ui); transition: all 0.12s ease; text-align: left; background: {activeTab === tab.id ? 'rgba(255, 214, 10, 0.1)' : hoveredTab === tab.id ? 'var(--bg-elevated)' : 'transparent'}; color: {activeTab === tab.id ? 'var(--banana-yellow)' : 'var(--text-secondary)'}; border-left: {activeTab === tab.id ? '2px solid var(--banana-yellow)' : '2px solid transparent'};"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d={tab.icon}/>
              </svg>
              {tab.label}
            </button>
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div style="flex: 1; display: flex; flex-direction: column; overflow: hidden;">
        <!-- Header -->
        <div style="display: flex; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border-default);">
          <span style="font-size: 16px; font-weight: 600; color: var(--text-primary); flex: 1;">
            {tabs.find(t => t.id === activeTab)?.label || 'Settings'}
          </span>
          <button
            onclick={close}
            onmouseenter={() => closeBtnHovered = true}
            onmouseleave={() => closeBtnHovered = false}
            aria-label="Close settings"
            style="display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; border: none; border-radius: 6px; cursor: pointer; transition: all 0.15s ease; background: {closeBtnHovered ? 'rgba(248, 81, 73, 0.15)' : 'transparent'}; color: {closeBtnHovered ? 'var(--accent-red)' : 'var(--text-muted)'}; transform: {closeBtnHovered ? 'rotate(90deg)' : 'rotate(0)'};"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <!-- Tab content -->
        <div style="flex: 1; overflow-y: auto; padding: 20px;">
          {#if activeTab === 'general'}
            <!-- General Settings -->
            <div style="display: flex; flex-direction: column; gap: 20px;">
              <!-- Appearance -->
              <div style="display: flex; flex-direction: column; gap: 12px;">
                <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Appearance</div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Theme</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Color theme for the IDE</div>
                  </div>
                  <select
                    value={settingsStore.value.theme}
                    onchange={(e) => updateSetting('theme', (e.target as HTMLSelectElement).value)}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="catppuccin-mocha">Catppuccin Mocha</option>
                    <option value="banana-dark">Banana Dark</option>
                    <option value="dracula">Dracula</option>
                  </select>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Editor Font Size</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Font size for the code editor</div>
                  </div>
                  <input
                    type="number" min="10" max="24"
                    value={settingsStore.value.editorFontSize}
                    onchange={(e) => updateSetting('editorFontSize', parseInt((e.target as HTMLInputElement).value) || 14)}
                    style="width: 64px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
                  />
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Tab Size</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Number of spaces per tab</div>
                  </div>
                  <select
                    value={String(settingsStore.value.tabSize)}
                    onchange={(e) => updateSetting('tabSize', parseInt((e.target as HTMLSelectElement).value))}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="2">2 spaces</option>
                    <option value="4">4 spaces</option>
                  </select>
                </div>
              </div>

              <!-- Terminal -->
              <div style="display: flex; flex-direction: column; gap: 12px;">
                <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Terminal</div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Default Shell</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Shell to use for new terminals</div>
                  </div>
                  <select
                    value={settingsStore.value.defaultShell}
                    onchange={(e) => updateSetting('defaultShell', (e.target as HTMLSelectElement).value as any)}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="auto">Auto</option>
                    <option value="powershell">PowerShell</option>
                    <option value="bash">Bash</option>
                    <option value="cmd">CMD</option>
                  </select>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Terminal Font Size</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Font size for terminal text</div>
                  </div>
                  <input
                    type="number" min="10" max="24"
                    value={settingsStore.value.terminalFontSize}
                    onchange={(e) => updateSetting('terminalFontSize', parseInt((e.target as HTMLInputElement).value) || 14)}
                    style="width: 64px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
                  />
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Cursor Style</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Terminal cursor appearance</div>
                  </div>
                  <select
                    value={settingsStore.value.terminalCursorStyle}
                    onchange={(e) => updateSetting('terminalCursorStyle', (e.target as HTMLSelectElement).value as any)}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="bar">Bar</option>
                    <option value="block">Block</option>
                    <option value="underline">Underline</option>
                  </select>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Style Preset</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Terminal visual theme</div>
                  </div>
                  <select
                    value={settingsStore.value.terminalStylePreset}
                    onchange={(e) => updateSetting('terminalStylePreset', (e.target as HTMLSelectElement).value as any)}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="metal">Metal</option>
                    <option value="minimal">Minimal</option>
                    <option value="retro">Retro</option>
                    <option value="high-contrast">High Contrast</option>
                  </select>
                </div>
              </div>

              <!-- Files -->
              <div style="display: flex; flex-direction: column; gap: 12px;">
                <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Files</div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Show Hidden Files</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Show dotfiles in the file tree</div>
                  </div>
                  <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
                    <input type="checkbox" checked={settingsStore.value.showHiddenFiles} onchange={(e) => updateSetting('showHiddenFiles', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
                    <span style="position: absolute; inset: 0; background: {settingsStore.value.showHiddenFiles ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
                      <span style="position: absolute; top: 2px; left: {settingsStore.value.showHiddenFiles ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
                    </span>
                  </label>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">File Tree Font Size</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Font size in the file explorer</div>
                  </div>
                  <input
                    type="number" min="10" max="24"
                    value={settingsStore.value.fileTreeFontSize}
                    onchange={(e) => updateSetting('fileTreeFontSize', parseInt((e.target as HTMLInputElement).value) || 14)}
                    style="width: 64px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
                  />
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Word Wrap</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Wrap long lines in the editor</div>
                  </div>
                  <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
                    <input type="checkbox" checked={settingsStore.value.wordWrap} onchange={(e) => updateSetting('wordWrap', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
                    <span style="position: absolute; inset: 0; background: {settingsStore.value.wordWrap ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
                      <span style="position: absolute; top: 2px; left: {settingsStore.value.wordWrap ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
                    </span>
                  </label>
                </div>
              </div>

              <!-- Behavior -->
              <div style="display: flex; flex-direction: column; gap: 12px;">
                <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Behavior</div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Restore Session</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Reopen last workspace on startup</div>
                  </div>
                  <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
                    <input type="checkbox" checked={settingsStore.value.restoreSession} onchange={(e) => updateSetting('restoreSession', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
                    <span style="position: absolute; inset: 0; background: {settingsStore.value.restoreSession ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
                      <span style="position: absolute; top: 2px; left: {settingsStore.value.restoreSession ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
                    </span>
                  </label>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Confirm Close Unsaved</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Prompt before closing unsaved files</div>
                  </div>
                  <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
                    <input type="checkbox" checked={settingsStore.value.confirmCloseUnsaved} onchange={(e) => updateSetting('confirmCloseUnsaved', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
                    <span style="position: absolute; inset: 0; background: {settingsStore.value.confirmCloseUnsaved ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
                      <span style="position: absolute; top: 2px; left: {settingsStore.value.confirmCloseUnsaved ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
                    </span>
                  </label>
                </div>
                <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
                  <div style="flex: 1;">
                    <div style="font-size: 13px; color: var(--text-primary);">Auto-Save Delay</div>
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Milliseconds before auto-saving (0 = off)</div>
                  </div>
                  <select
                    value={String(settingsStore.value.autoSaveDelay)}
                    onchange={(e) => updateSetting('autoSaveDelay', parseInt((e.target as HTMLSelectElement).value))}
                    style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
                  >
                    <option value="0">Off</option>
                    <option value="1000">1 second</option>
                    <option value="2000">2 seconds</option>
                    <option value="5000">5 seconds</option>
                  </select>
                </div>
              </div>
            </div>

          {:else if activeTab === 'ai'}
            <AISettingsPage />

          {:else if activeTab === 'doppler'}
            <DopplerTab />

          {:else if activeTab === 'about'}
            <div style="display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 24px;">
              <div style="font-size: 48px; animation: bob 3s ease-in-out infinite;">🍌</div>
              <div style="text-align: center;">
                <div style="font-size: 24px; font-weight: 700; color: var(--banana-yellow);">Banana Code</div>
                <div style="font-size: 13px; color: var(--text-secondary); margin-top: 4px;">Vibe coding for real developers</div>
                <div style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">Version 0.1.0</div>
              </div>
              <div style="display: flex; gap: 12px; margin-top: 8px;">
                <a href="https://bananacode.ai" target="_blank" rel="noopener" style="padding: 8px 16px; background: var(--banana-yellow); color: #0D1117; border-radius: 6px; font-size: 12px; font-weight: 600; text-decoration: none; transition: all 0.15s ease;">
                  bananacode.ai
                </a>
              </div>
              <div style="color: var(--text-muted); font-size: 11px; margin-top: 16px;">Built with Tauri 2 + SvelteKit 5 + Rust</div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
