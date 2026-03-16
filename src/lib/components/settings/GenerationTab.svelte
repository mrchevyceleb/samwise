<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
  const settingsStore = getSettingsStore();
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Generation Parameters -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Generation Parameters</div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Temperature</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Controls randomness in AI responses (0 = deterministic, 2 = creative)</div>
    </div>
    <div style="display: flex; align-items: center; gap: 12px;">
      <input type="range" min="0" max="2" step="0.1" value={settingsStore.value.aiTemperature} oninput={(e) => updateSetting('aiTemperature', parseFloat((e.target as HTMLInputElement).value))} style="width: 120px; accent-color: var(--accent-primary); cursor: pointer;" />
      <span style="font-size: 12px; color: var(--accent-primary); font-family: var(--font-mono); min-width: 2rem; text-align: center;">{settingsStore.value.aiTemperature.toFixed(1)}</span>
    </div>
  </div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Max Output Tokens</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Maximum number of tokens the AI can generate per response</div>
    </div>
    <select
      value={settingsStore.value.aiMaxTokens}
      onchange={(e) => updateSetting('aiMaxTokens', parseInt((e.target as HTMLSelectElement).value))}
      style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
    >
      <option value={4096}>4,096</option>
      <option value={8192}>8,192</option>
      <option value={16384}>16,384</option>
      <option value={32768}>32,768</option>
      <option value={65536}>65,536</option>
      <option value={128000}>128,000</option>
    </select>
  </div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Max Context Tokens</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Maximum context window size sent to the model</div>
    </div>
    <input
      type="number" min="1024" max="1000000"
      value={settingsStore.value.aiMaxContextTokens}
      onchange={(e) => updateSetting('aiMaxContextTokens', parseInt((e.target as HTMLInputElement).value) || 128000)}
      style="width: 90px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
    />
  </div>

  <!-- Reasoning -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Reasoning</div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Enable Reasoning</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Allow the model to use extended thinking before responding</div>
    </div>
    <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
      <input type="checkbox" checked={settingsStore.value.aiReasoningEnabled} onchange={(e) => updateSetting('aiReasoningEnabled', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
      <span style="position: absolute; inset: 0; background: {settingsStore.value.aiReasoningEnabled ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
        <span style="position: absolute; top: 2px; left: {settingsStore.value.aiReasoningEnabled ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
      </span>
    </label>
  </div>

  {#if settingsStore.value.aiReasoningEnabled}
    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">Reasoning Budget</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Maximum tokens allocated for the model's internal reasoning</div>
      </div>
      <input
        type="number" min="1000" max="100000"
        value={settingsStore.value.aiReasoningMaxBudgetTokens}
        onchange={(e) => updateSetting('aiReasoningMaxBudgetTokens', parseInt((e.target as HTMLInputElement).value) || 20000)}
        style="width: 80px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
      />
    </div>
  {/if}

  {#if settingsStore.value.aiReasoningEnabled}
    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">Exclude Reasoning from Context</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Strip reasoning tokens from conversation history to save context</div>
      </div>
      <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
        <input type="checkbox" checked={settingsStore.value.aiReasoningExclude} onchange={(e) => updateSetting('aiReasoningExclude', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
        <span style="position: absolute; inset: 0; background: {settingsStore.value.aiReasoningExclude ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
          <span style="position: absolute; top: 2px; left: {settingsStore.value.aiReasoningExclude ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
        </span>
      </label>
    </div>
  {/if}

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Thinking Display</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">How to display the model's reasoning in chat</div>
    </div>
    <select
      value={settingsStore.value.aiThinkingMode}
      onchange={(e) => updateSetting('aiThinkingMode', (e.target as HTMLSelectElement).value as any)}
      style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
    >
      <option value="all">All</option>
      <option value="preview">Preview</option>
      <option value="none">None</option>
    </select>
  </div>

  <!-- Chat Display -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Chat Display</div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Chat Font Size</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Font size for messages in the AI chat panel</div>
    </div>
    <input
      type="number" min="10" max="24"
      value={settingsStore.value.aiChatFontSize}
      onchange={(e) => updateSetting('aiChatFontSize', parseInt((e.target as HTMLInputElement).value) || 15)}
      style="width: 64px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
    />
  </div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Chat Font Family</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Font used for chat messages</div>
    </div>
    <select
      value={settingsStore.value.aiChatFontFamily}
      onchange={(e) => updateSetting('aiChatFontFamily', (e.target as HTMLSelectElement).value as any)}
      style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
    >
      <option value="system">System</option>
      <option value="inter">Inter</option>
      <option value="jetbrains">JetBrains Mono</option>
      <option value="cascadia">Cascadia Code</option>
      <option value="fira">Fira Code</option>
    </select>
  </div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Chat Dock Position</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Where the AI chat panel is docked in the layout</div>
    </div>
    <select
      value={settingsStore.value.aiChatDock}
      onchange={(e) => updateSetting('aiChatDock', (e.target as HTMLSelectElement).value as any)}
      style="padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; cursor: pointer;"
    >
      <option value="right">Right</option>
      <option value="bottom">Bottom</option>
      <option value="tab">Tab</option>
    </select>
  </div>

  <!-- Behavior -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Behavior</div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Read Instructions Every Message</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Reload AGENTS.md/CLAUDE.md before each message</div>
    </div>
    <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
      <input type="checkbox" checked={settingsStore.value.aiReadInstructionsEveryMessage} onchange={(e) => updateSetting('aiReadInstructionsEveryMessage', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
      <span style="position: absolute; inset: 0; background: {settingsStore.value.aiReadInstructionsEveryMessage ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
        <span style="position: absolute; top: 2px; left: {settingsStore.value.aiReadInstructionsEveryMessage ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
      </span>
    </label>
  </div>

  <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
    <div style="flex: 1;">
      <div style="font-size: 13px; color: var(--text-primary);">Enable @Mentions</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Use @ to reference files and folders in chat</div>
    </div>
    <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
      <input type="checkbox" checked={settingsStore.value.aiEnableAtMentions} onchange={(e) => updateSetting('aiEnableAtMentions', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
      <span style="position: absolute; inset: 0; background: {settingsStore.value.aiEnableAtMentions ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
        <span style="position: absolute; top: 2px; left: {settingsStore.value.aiEnableAtMentions ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
      </span>
    </label>
  </div>
</div>
