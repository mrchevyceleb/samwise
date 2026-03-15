/** Models store using Svelte 5 runes - manages dynamic model fetching */

import { aiFetchModels } from '$lib/utils/tauri';
import { getSettingsStore, updateSetting, getActiveAIKey } from '$lib/stores/settings.svelte';

// ---- Types ----

export interface FetchedModel {
  id: string;
  name: string;
  contextLength: number;
  pricing: { prompt: number; completion: number } | null;
}

// ---- Svelte 5 Runes State ----

let fetchedModels = $state<FetchedModel[]>([]);
let isLoading = $state(false);
let error = $state<string | null>(null);
let searchQuery = $state('');

// ---- Functions ----

export async function fetchAvailableModels(): Promise<void> {
  const settings = getSettingsStore();
  const s = settings.value;
  const provider = s.aiProvider;

  // Anthropic uses a static list, skip fetching
  if (provider === 'anthropic') {
    error = 'Anthropic does not support dynamic model listing. Use a static list.';
    return;
  }

  let baseUrl: string;
  if (provider === 'openrouter') {
    baseUrl = 'https://openrouter.ai/api/v1';
  } else if (provider === 'openai') {
    baseUrl = 'https://api.openai.com/v1';
  } else if (provider === 'lmstudio') {
    baseUrl = (s.aiLMStudioBaseUrl || 'http://localhost:1234/v1').trim();
  } else {
    error = `Unknown provider: ${provider}`;
    return;
  }

  const apiKey = getActiveAIKey(s);

  isLoading = true;
  error = null;

  try {
    const raw = await aiFetchModels(baseUrl, apiKey);
    const parsed = JSON.parse(raw) as {
      data: Array<{
        id: string;
        name?: string;
        context_length?: number;
        pricing?: { prompt: string | number; completion: string | number };
      }>;
    };

    fetchedModels = (parsed.data || []).map((m) => ({
      id: m.id,
      name: m.name || m.id,
      contextLength: m.context_length || 0,
      pricing: m.pricing
        ? {
            prompt: typeof m.pricing.prompt === 'string' ? parseFloat(m.pricing.prompt) : m.pricing.prompt,
            completion: typeof m.pricing.completion === 'string' ? parseFloat(m.pricing.completion) : m.pricing.completion,
          }
        : null,
    }));
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    fetchedModels = [];
  } finally {
    isLoading = false;
  }
}

export function toggleModelEnabled(modelId: string): void {
  const settings = getSettingsStore();
  const current = settings.value.aiEnabledModels || [];
  const idx = current.indexOf(modelId);

  if (idx >= 0) {
    const newList = current.filter((id) => id !== modelId);
    updateSetting('aiEnabledModels', newList);
  } else {
    updateSetting('aiEnabledModels', [...current, modelId]);
  }
}

export function isModelEnabled(modelId: string): boolean {
  const settings = getSettingsStore();
  return (settings.value.aiEnabledModels || []).includes(modelId);
}

export function setSearchQuery(query: string): void {
  searchQuery = query;
}

// ---- Store Getter ----

export function getModelsStore() {
  return {
    get fetchedModels() { return fetchedModels; },
    get isLoading() { return isLoading; },
    get error() { return error; },
    get searchQuery() { return searchQuery; },

    get filteredModels() {
      if (!searchQuery.trim()) return fetchedModels;
      const q = searchQuery.toLowerCase();
      return fetchedModels.filter(
        (m) => m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q),
      );
    },

    fetchAvailableModels,
    toggleModelEnabled,
    isModelEnabled,
    setSearchQuery,
  };
}
