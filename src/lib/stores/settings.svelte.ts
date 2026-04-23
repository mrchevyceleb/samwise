/** Settings store using Svelte 5 runes */

// ---- Types ----

export interface AppSettings {
  defaultTerminalDock: 'bottom' | 'right' | 'tab' | 'left';
  terminalStylePreset: 'metal' | 'minimal' | 'retro' | 'high-contrast';
  defaultShell: 'auto' | 'powershell' | 'bash' | 'cmd';
  terminalFontSize: number;
  terminalCursorStyle: 'block' | 'underline' | 'bar';
  editorFontSize: number;
  tabSize: number;
  wordWrap: boolean;
  autoSaveDelay: number;
  theme: string;
  showHiddenFiles: boolean;
  fileTreeFontSize: number;
  restoreSession: boolean;
  confirmCloseUnsaved: boolean;
  aiProvider: 'openrouter' | 'anthropic' | 'openai' | 'lmstudio';
  aiAuthMode: 'apiKey' | 'oauth';
  aiApiKey: string;
  aiOpenRouterApiKey: string;
  aiAnthropicApiKey: string;
  aiOpenAIApiKey: string;
  aiOpenAIOAuthAccessToken: string;
  aiOpenAIOAuthRefreshToken: string;
  aiOpenAIOAuthExpiresAt: string;
  aiModel: string;
  aiBaseUrl: string;
  aiOpenRouterBaseUrl: string;
  aiAnthropicBaseUrl: string;
  aiOpenAIBaseUrl: string;
  aiOpenAICodexBaseUrl: string;
  aiOpenAICodexClientVersion: string;
  aiLMStudioBaseUrl: string;
  aiTemperature: number;
  aiMaxTokens: number;
  aiMaxContextTokens: number;
  aiReasoningEnabled: boolean;
  aiReasoningMaxBudgetTokens: number;
  aiReasoningExclude: boolean;
  aiEnableToolUse: boolean;
  aiConfirmWrites: boolean;
  aiYoloMode: boolean;
  aiMaxToolIterations: number;
  aiEnabledModels: string[];
  aiChatFontSize: number;
  aiChatFontFamily: string;
  aiChatDock: 'right' | 'bottom' | 'tab';
  aiReadInstructionsEveryMessage: boolean;
  aiEnableAtMentions: boolean;
  aiSlashCommandDirs: string[];
  aiBasePrompt: string;
  aiThinkingMode: 'all' | 'preview' | 'none';
  mcpServers: MCPServerConfig[];
  dopplerToken: string;
  dopplerTokens: Array<{ token: string; orgName: string; orgSlug: string }>;
  dopplerWorkplace: string;
  dopplerProject: string;
  dopplerConfig: string;
  dopplerEnabled: boolean;
  workspaceThemes: Record<string, string>;
  // Agent One settings
  supabaseUrl: string;
  supabaseAnonKey: string;
  agentMachineName: string;
  autoStartWorker: boolean;
  isMaster: boolean;
  masterConfigured: boolean;
  workerRules: string[];
  // Telegram notification preferences
  scanFolders: string[];
  telegramNotificationsEnabled: boolean;
  telegramNotifyTaskStarted: boolean;
  telegramNotifyTaskCompletedCode: boolean;
  telegramNotifyTaskCompletedResearch: boolean;
  telegramNotifyTaskFailed: boolean;
  // Auto-merge gate
  autoMergeEnabled: boolean;
  autoMergeMinScore: number;
  autoMergeMaxDiffLines: number;
  // Codex $samwise-pr-review pass on PRs (only when auto-merge is off)
  autoPrReviewEnabled: boolean;
}

export interface MCPServerConfig {
  id: string;
  name: string;
  enabled: boolean;
  timeoutMs: number;
  transport: 'http' | 'stdio';
  // HTTP fields
  url: string;
  authToken: string;
  headersJson: string;
  // Stdio fields
  command: string;
  args: string[];
  env: Record<string, string>;
}

// ---- Defaults ----

export const DEFAULT_SETTINGS: AppSettings = {
  defaultTerminalDock: 'bottom',
  terminalStylePreset: 'metal',
  defaultShell: 'auto',
  terminalFontSize: 14,
  terminalCursorStyle: 'bar',
  editorFontSize: 14,
  tabSize: 2,
  wordWrap: false,
  autoSaveDelay: 2000,
  theme: 'catppuccin-mocha',
  showHiddenFiles: false,
  fileTreeFontSize: 14,
  restoreSession: true,
  confirmCloseUnsaved: true,
  aiProvider: 'openrouter',
  aiAuthMode: 'apiKey',
  aiApiKey: '',
  aiOpenRouterApiKey: '',
  aiAnthropicApiKey: '',
  aiOpenAIApiKey: '',
  aiOpenAIOAuthAccessToken: '',
  aiOpenAIOAuthRefreshToken: '',
  aiOpenAIOAuthExpiresAt: '',
  aiModel: 'anthropic/claude-sonnet-4-6',
  aiBaseUrl: 'https://openrouter.ai/api/v1',
  aiOpenRouterBaseUrl: 'https://openrouter.ai/api/v1',
  aiAnthropicBaseUrl: 'https://api.anthropic.com/v1',
  aiOpenAIBaseUrl: 'https://api.openai.com/v1',
  aiOpenAICodexBaseUrl: 'https://chatgpt.com/backend-api/codex',
  aiOpenAICodexClientVersion: '4.0.0',
  aiLMStudioBaseUrl: 'http://localhost:1234/v1',
  aiTemperature: 0.7,
  aiMaxTokens: 16384,
  aiMaxContextTokens: 128000,
  aiReasoningEnabled: false,
  aiReasoningMaxBudgetTokens: 20000,
  aiReasoningExclude: false,
  aiEnableToolUse: true,
  aiConfirmWrites: true,
  aiYoloMode: false,
  aiMaxToolIterations: 75,
  aiChatFontSize: 15,
  aiChatFontFamily: 'system',
  aiChatDock: 'bottom',
  aiReadInstructionsEveryMessage: false,
  aiEnableAtMentions: true,
  aiSlashCommandDirs: [],
  aiBasePrompt: 'default',
  aiThinkingMode: 'preview',
  aiEnabledModels: [
    'anthropic/claude-sonnet-4-6',
    'anthropic/claude-opus-4-6',
    'openai/gpt-5.4',
    'openai/gpt-5.3-codex-medium',
    'openai/gpt-5.3-codex',
    'openai/gpt-5.3-codex-spark',
    'google/gemini-2.5-pro-preview',
  ],
  mcpServers: [],
  dopplerToken: '',
  dopplerTokens: [],
  dopplerWorkplace: '',
  dopplerProject: '',
  dopplerConfig: '',
  dopplerEnabled: false,
  workspaceThemes: {},
  supabaseUrl: '',
  supabaseAnonKey: '',
  agentMachineName: '',
  autoStartWorker: false,
  isMaster: false,
  masterConfigured: false,
  workerRules: [],
  scanFolders: [],
  telegramNotificationsEnabled: true,
  telegramNotifyTaskStarted: true,
  telegramNotifyTaskCompletedCode: true,
  telegramNotifyTaskCompletedResearch: true,
  telegramNotifyTaskFailed: true,
  autoMergeEnabled: false,
  autoMergeMinScore: 8,
  autoMergeMaxDiffLines: 400,
  autoPrReviewEnabled: true,
};

// ---- Svelte 5 Runes State ----

let currentSettings = $state<AppSettings>({ ...DEFAULT_SETTINGS });
let settingsVisible = $state(false);
let activeSettingsTab = $state<string>('connection');
let settingsLoaded = $state(false);
let reconfigureRequested = $state(false);

// Debounce timer for auto-saving
let saveTimer: ReturnType<typeof setTimeout> | null = null;

/** Persist current settings to disk via Tauri */
async function persistSettings(): Promise<void> {
  try {
    const { saveSettings } = await import('$lib/utils/tauri');
    await saveSettings(JSON.stringify(currentSettings));
  } catch (e) {
    console.warn('[settings] Failed to save:', e);
  }
}

/** Schedule a debounced save (300ms) */
function scheduleSave(): void {
  if (!settingsLoaded) return; // Don't save during initial load
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    persistSettings();
    saveTimer = null;
  }, 300);
}

// ---- Accessors ----

export function getSettings(): AppSettings {
  return currentSettings;
}

export function setSettings(s: AppSettings): void {
  currentSettings = { ...s };
  scheduleSave();
}

export function getSettingsStore() {
  return {
    get value() { return currentSettings; },
    set value(s: AppSettings) { currentSettings = { ...s }; scheduleSave(); },

    get settingsVisible() { return settingsVisible; },
    set settingsVisible(v: boolean) { settingsVisible = v; },

    get activeSettingsTab() { return activeSettingsTab; },
    set activeSettingsTab(v: string) { activeSettingsTab = v; },

    get loaded() { return settingsLoaded; },

    get reconfigureRequested() { return reconfigureRequested; },
    set reconfigureRequested(v: boolean) { reconfigureRequested = v; },
  };
}

// ---- Helpers ----

/** Update a single setting by key */
export function updateSetting<K extends keyof AppSettings>(
  key: K,
  value: AppSettings[K],
) {
  currentSettings = { ...currentSettings, [key]: value };
  scheduleSave();
}

/** Load settings from disk on app startup. Merges with defaults for any missing keys. */
export async function initSettings(): Promise<void> {
  try {
    const { loadSettings } = await import('$lib/utils/tauri');
    const json = await loadSettings();
    if (json) {
      const loaded = JSON.parse(json) as Partial<AppSettings>;
      // Merge with defaults so new keys are always present
      currentSettings = { ...DEFAULT_SETTINGS, ...loaded };
    }
  } catch (e) {
    console.warn('[settings] Failed to load (using defaults):', e);
  } finally {
    settingsLoaded = true;
  }
}

export function getActiveAIKey(s: AppSettings): string {
  if (s.aiProvider === 'anthropic') return (s.aiAnthropicApiKey || '').trim();
  if (s.aiProvider === 'openai') {
    const oauthToken = (s.aiOpenAIOAuthAccessToken || '').trim();
    if (oauthToken) return oauthToken;
    return (s.aiOpenAIApiKey || '').trim();
  }
  if (s.aiProvider === 'lmstudio') return 'lm-studio';
  return (s.aiOpenRouterApiKey || s.aiApiKey || '').trim();
}

/** Human-readable provider name from provider ID */
export function getProviderDisplayName(provider: string): string {
  switch (provider) {
    case 'anthropic': return 'Anthropic';
    case 'openrouter': return 'OpenRouter';
    case 'openai': return 'OpenAI';
    case 'lmstudio': return 'LM Studio';
    default: return provider;
  }
}

/**
 * Infer which provider a model ID belongs to based on its format.
 */
export function inferProviderForModel(
  modelId: string,
  activeProvider?: AppSettings['aiProvider'],
): string {
  const id = (modelId || '').toLowerCase();
  if (activeProvider === 'lmstudio') return 'LM Studio';

  if (id.startsWith('anthropic/')) return activeProvider === 'openrouter' ? 'Anthropic via OpenRouter' : 'Anthropic';
  if (id.startsWith('openai/')) return activeProvider === 'openrouter' ? 'OpenAI via OpenRouter' : 'OpenAI';
  if (id.startsWith('google/')) return 'Google via OpenRouter';
  if (id.startsWith('mistralai/')) return 'Mistral via OpenRouter';
  if (id.startsWith('deepseek/')) return 'DeepSeek via OpenRouter';

  if (id.startsWith('claude-')) return 'Anthropic';
  if (id.startsWith('gpt-') || id.startsWith('o3') || id.startsWith('o4') || id.startsWith('codex-')) return 'OpenAI';

  if (activeProvider) return getProviderDisplayName(activeProvider);
  return 'Unknown';
}

/** Infer routing provider from model ID. */
export function inferRoutingProviderForModel(
  modelId: string,
  fallback: AppSettings['aiProvider'] = 'openrouter',
): AppSettings['aiProvider'] {
  const id = (modelId || '').toLowerCase();
  if (!id) return fallback;

  if (id.startsWith('google/') || id.startsWith('mistralai/') || id.startsWith('deepseek/') || id.startsWith('moonshotai/') || id.startsWith('z-ai/') || id.startsWith('minimax/')) {
    return 'openrouter';
  }
  if (id.startsWith('anthropic/') || id.startsWith('claude-')) return 'anthropic';
  if (id.startsWith('openai/') || id.startsWith('gpt-') || id.startsWith('o3') || id.startsWith('o4') || id.startsWith('codex-')) return 'openai';
  return fallback;
}

export function getActiveAIBaseUrl(s: AppSettings): string {
  if (s.aiProvider === 'anthropic') {
    return (s.aiAnthropicBaseUrl || 'https://api.anthropic.com/v1').trim();
  }
  if (s.aiProvider === 'openai') {
    const oauthToken = (s.aiOpenAIOAuthAccessToken || '').trim();
    if (oauthToken) {
      return (s.aiOpenAICodexBaseUrl || 'https://chatgpt.com/backend-api/codex').trim();
    }
    return (s.aiOpenAIBaseUrl || 'https://api.openai.com/v1').trim();
  }
  if (s.aiProvider === 'lmstudio') {
    return (s.aiLMStudioBaseUrl || 'http://localhost:1234/v1').trim();
  }
  return (s.aiOpenRouterBaseUrl || s.aiBaseUrl || 'https://openrouter.ai/api/v1').trim();
}
