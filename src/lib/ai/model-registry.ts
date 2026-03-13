/**
 * Built-in model profiles with recommended inference settings.
 * Used as defaults when a model is selected. Users can override per-chat.
 */

export interface ModelProfile {
  id: string;
  name: string;
  provider: 'openrouter' | 'anthropic' | 'openai' | 'lmstudio';
  contextWindow: number;
  maxOutputTokens: number;
  defaultTemperature: number;
  supportsTools: boolean;
  supportsVision: boolean;
  supportsThinking: boolean;
}

const PROFILES: ModelProfile[] = [
  // Anthropic
  {
    id: 'anthropic/claude-opus-4-6',
    name: 'Claude Opus 4.6',
    provider: 'openrouter',
    contextWindow: 200000,
    maxOutputTokens: 32768,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'anthropic/claude-sonnet-4-6',
    name: 'Claude Sonnet 4.6',
    provider: 'openrouter',
    contextWindow: 200000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'claude-opus-4-6',
    name: 'Claude Opus 4.6',
    provider: 'anthropic',
    contextWindow: 200000,
    maxOutputTokens: 32768,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'claude-sonnet-4-6',
    name: 'Claude Sonnet 4.6',
    provider: 'anthropic',
    contextWindow: 200000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'claude-3-5-haiku-latest',
    name: 'Claude 3.5 Haiku',
    provider: 'anthropic',
    contextWindow: 200000,
    maxOutputTokens: 8192,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: false,
  },
  // OpenAI
  {
    id: 'openai/gpt-4.1',
    name: 'GPT-4.1',
    provider: 'openrouter',
    contextWindow: 128000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: false,
  },
  {
    id: 'gpt-4.1',
    name: 'GPT-4.1',
    provider: 'openai',
    contextWindow: 128000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: false,
  },
  {
    id: 'openai/gpt-5.4',
    name: 'GPT 5.4',
    provider: 'openrouter',
    contextWindow: 400000,
    maxOutputTokens: 100000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'gpt-5.4',
    name: 'GPT 5.4',
    provider: 'openai',
    contextWindow: 400000,
    maxOutputTokens: 100000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'openai/o3',
    name: 'O3',
    provider: 'openrouter',
    contextWindow: 200000,
    maxOutputTokens: 100000,
    defaultTemperature: 1.0,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'openai/gpt-5.3-codex',
    name: 'Codex 5.3 High',
    provider: 'openrouter',
    contextWindow: 400000,
    maxOutputTokens: 100000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'gpt-5.3-codex',
    name: 'Codex 5.3 High',
    provider: 'openai',
    contextWindow: 400000,
    maxOutputTokens: 100000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'openai/gpt-5.3-codex-medium',
    name: 'Codex 5.3 Medium',
    provider: 'openrouter',
    contextWindow: 400000,
    maxOutputTokens: 64000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'gpt-5.3-codex-medium',
    name: 'Codex 5.3 Medium',
    provider: 'openai',
    contextWindow: 400000,
    maxOutputTokens: 64000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'openai/gpt-5.3-codex-spark',
    name: 'Codex Spark',
    provider: 'openrouter',
    contextWindow: 400000,
    maxOutputTokens: 64000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  {
    id: 'gpt-5.3-codex-spark',
    name: 'Codex Spark',
    provider: 'openai',
    contextWindow: 400000,
    maxOutputTokens: 64000,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  // Google
  {
    id: 'google/gemini-2.5-pro-preview',
    name: 'Gemini 2.5 Pro',
    provider: 'openrouter',
    contextWindow: 1000000,
    maxOutputTokens: 65536,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: true,
    supportsThinking: true,
  },
  // DeepSeek
  {
    id: 'deepseek/deepseek-r1',
    name: 'DeepSeek R1',
    provider: 'openrouter',
    contextWindow: 128000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.6,
    supportsTools: false,
    supportsVision: false,
    supportsThinking: true,
  },
  {
    id: 'deepseek/deepseek-chat-v3-0324',
    name: 'DeepSeek V3',
    provider: 'openrouter',
    contextWindow: 128000,
    maxOutputTokens: 16384,
    defaultTemperature: 0.7,
    supportsTools: true,
    supportsVision: false,
    supportsThinking: false,
  },
];

const profileMap = new Map<string, ModelProfile>();
for (const p of PROFILES) {
  profileMap.set(p.id, p);
}

/** Look up a built-in model profile by model ID. Returns null for unknown models. */
export function getModelProfile(modelId: string): ModelProfile | null {
  return profileMap.get(modelId) || null;
}

/** Get all built-in profiles. */
export function getAllModelProfiles(): ModelProfile[] {
  return [...PROFILES];
}

/**
 * Infer reasonable defaults for any model ID, using registry if available,
 * otherwise falling back to heuristic inference.
 */
export function inferModelSettings(modelId: string): {
  contextWindow: number;
  maxOutputTokens: number;
  defaultTemperature: number;
  supportsTools: boolean;
} {
  const profile = getModelProfile(modelId);
  if (profile) {
    return {
      contextWindow: profile.contextWindow,
      maxOutputTokens: profile.maxOutputTokens,
      defaultTemperature: profile.defaultTemperature,
      supportsTools: profile.supportsTools,
    };
  }

  // Heuristic fallback
  const id = modelId.toLowerCase();
  let contextWindow = 128000;
  let maxOutputTokens = 16384;
  if (id.includes('gemini')) { contextWindow = 1000000; maxOutputTokens = 65536; }
  else if (id.includes('claude')) { contextWindow = 200000; maxOutputTokens = 16384; }
  else if (id.includes('gpt-5')) contextWindow = 400000;
  else if (id.includes('gpt-4.1')) contextWindow = 128000;
  else if (id.includes('codex')) contextWindow = 400000;
  else if (id.includes('o3') || id.includes('o4')) { contextWindow = 200000; maxOutputTokens = 100000; }

  return {
    contextWindow,
    maxOutputTokens,
    defaultTemperature: 0.7,
    supportsTools: true,
  };
}
