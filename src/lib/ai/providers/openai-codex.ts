import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { aiChatStreamOpenAICodex } from '$lib/utils/tauri';
import type { AIChatSettings, ChatMessage, StreamChunk, ToolCall, ToolDefinition } from '../types';
import { inferModelSettings } from '../model-registry';

interface StreamCallbacks {
  onChunk: (chunk: StreamChunk) => void;
  onDone: () => void;
  onError: (error: string) => void;
}

let requestCounter = 0;

function normalizeModel(model: string): string {
  const raw = (model || '').trim();
  const unprefixed = raw.startsWith('openai/')
    ? raw.slice('openai/'.length)
    : raw;

  if (unprefixed === 'gpt-5.3-codex-medium') {
    return 'gpt-5.3-codex-spark';
  }

  return unprefixed;
}

function normalizeClientVersion(value: string | undefined): string {
  const fallback = '4.0.0';
  const raw = String(value || '').trim();
  if (!raw) return fallback;

  const exact = raw.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (exact) return `${exact[1]}.${exact[2]}.${exact[3]}`;

  const embedded = raw.match(/(\d+)\.(\d+)\.(\d+)/);
  if (embedded) return `${embedded[1]}.${embedded[2]}.${embedded[3]}`;

  return fallback;
}

function toResponsesContent(role: 'user' | 'assistant', content: ChatMessage['content']): Array<Record<string, unknown>> {
  const textType = role === 'assistant' ? 'output_text' : 'input_text';
  if (typeof content === 'string') {
    return content.trim() ? [{ type: textType, text: content.trim() }] : [];
  }
  return [];
}

function toResponsesInput(messages: ChatMessage[]): { instructions: string; input: Array<Record<string, unknown>> } {
  const instructions: string[] = [];
  const input: Array<Record<string, unknown>> = [];

  for (const msg of messages) {
    if (msg.role === 'system') {
      if (msg.content?.trim()) instructions.push(msg.content.trim());
      continue;
    }

    if (msg.role === 'user') {
      const parts = toResponsesContent('user', msg.content);
      if (msg.images?.length) {
        for (const img of msg.images) {
          parts.push({
            type: 'input_image',
            image_url: `data:${img.mediaType};base64,${img.base64}`,
          });
        }
      }
      if (parts.length > 0) {
        input.push({ type: 'message', role: 'user', content: parts });
      }
      continue;
    }

    if (msg.role === 'assistant') {
      const parts = toResponsesContent('assistant', msg.content);
      if (parts.length > 0) {
        input.push({ type: 'message', role: 'assistant', content: parts });
      }
      if (msg.tool_calls?.length) {
        for (const tc of msg.tool_calls) {
          input.push({
            type: 'function_call',
            call_id: tc.id,
            name: tc.function.name,
            arguments: tc.function.arguments || '{}',
          });
        }
      }
      continue;
    }

    if (msg.role === 'tool') {
      input.push({
        type: 'function_call_output',
        call_id: msg.tool_call_id,
        output: msg.content || '',
      });
    }
  }

  return {
    instructions: instructions.join('\n\n').trim(),
    input,
  };
}

function toResponsesTools(tools: ToolDefinition[] | undefined): Array<Record<string, unknown>> {
  if (!tools || tools.length === 0) return [];
  const out: Array<Record<string, unknown>> = [];
  for (const tool of tools) {
    if (tool.type !== 'function' || !tool.function?.name) continue;
    out.push({
      type: 'function',
      name: tool.function.name,
      description: tool.function.description || '',
      parameters: tool.function.parameters || { type: 'object', properties: {} },
    });
  }
  return out;
}

class OpenAICodexStreamProcessor {
  private toolCalls = new Map<string, ToolCall>();
  private toolCallOrder: string[] = [];
  private _inputTokens = 0;
  private _outputTokens = 0;

  get inputTokens(): number { return this._inputTokens; }
  get outputTokens(): number { return this._outputTokens; }

  processLine(data: string): StreamChunk | null {
    let parsed: any;
    try {
      parsed = JSON.parse(data);
    } catch {
      return null;
    }

    const eventType = parsed?.type;
    if (!eventType) return null;

    if (eventType === 'response.output_text.delta') {
      if (typeof parsed.delta === 'string' && parsed.delta) {
        return { type: 'text', content: parsed.delta };
      }
      return null;
    }

    if (eventType === 'response.output_item.added' || eventType === 'response.output_item.done') {
      const item = parsed.item;
      if (!item || item.type !== 'function_call') return null;

      const callId = String(item.call_id || item.id || `call-${this.toolCallOrder.length + 1}`);
      const existing = this.toolCalls.get(callId);
      const rawArgs = item.arguments;
      const args = typeof rawArgs === 'string' ? rawArgs : JSON.stringify(rawArgs || {});

      const next: ToolCall = {
        id: callId,
        type: 'function',
        function: {
          name: item.name || existing?.function.name || 'unknown_tool',
          arguments: args,
        },
      };

      if (!existing) {
        this.toolCallOrder.push(callId);
      }
      this.toolCalls.set(callId, next);
      return null;
    }

    if (eventType === 'response.completed') {
      const usage = parsed.response?.usage;
      if (usage) {
        this._inputTokens = Number(usage.input_tokens || usage.prompt_tokens || 0);
        this._outputTokens = Number(usage.output_tokens || usage.completion_tokens || 0);
      }
      return null;
    }

    if (eventType === 'response.failed') {
      const message = parsed?.response?.error?.message
        || parsed?.error?.message
        || 'OpenAI Codex stream failed';
      return { type: 'error', content: message };
    }

    return null;
  }

  getToolCalls(): ToolCall[] {
    const out = this.toolCallOrder
      .map((id) => this.toolCalls.get(id))
      .filter((call): call is ToolCall => Boolean(call));
    this.toolCalls.clear();
    this.toolCallOrder = [];
    return out;
  }
}

/** Strip orphaned tool_call/result pairs from messages before Responses API conversion. */
function sanitizeCodexMessages(messages: ChatMessage[]): ChatMessage[] {
  const completeIds = new Set<string>();

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.role !== 'assistant' || !msg.tool_calls?.length) continue;

    const expected = new Set(msg.tool_calls.map((tc) => tc.id));
    let j = i + 1;
    while (j < messages.length && messages[j].role === 'tool') {
      if (messages[j].tool_call_id && expected.has(messages[j].tool_call_id!)) {
        completeIds.add(messages[j].tool_call_id!);
      }
      j++;
    }
  }

  const result: ChatMessage[] = [];
  for (const msg of messages) {
    if (msg.role === 'tool') {
      if (msg.tool_call_id && completeIds.has(msg.tool_call_id)) result.push(msg);
      continue;
    }
    if (msg.role === 'assistant' && msg.tool_calls?.length) {
      const kept = msg.tool_calls.filter((tc) => completeIds.has(tc.id));
      if (kept.length > 0) {
        result.push({ ...msg, tool_calls: kept });
      } else if (msg.content) {
        result.push({ ...msg, tool_calls: undefined });
      }
      continue;
    }
    result.push(msg);
  }
  return result;
}

export async function streamOpenAICodexCompletion(
  messages: ChatMessage[],
  settings: AIChatSettings,
  tools: ToolDefinition[] | undefined,
  callbacks: StreamCallbacks,
): Promise<void> {
  const requestId = `codex-${Date.now()}-${++requestCounter}`;
  const processor = new OpenAICodexStreamProcessor();

  const translated = toResponsesInput(sanitizeCodexMessages(messages));
  const contextLimit = settings.contextWindow || 128000;
  const { maxOutputTokens: registryMax } = inferModelSettings(settings.model);
  const safeMaxOutputTokens = Math.min(settings.maxTokens, registryMax, Math.floor(contextLimit * 0.75));

  const body: Record<string, unknown> = {
    model: normalizeModel(settings.model),
    input: translated.input,
    stream: true,
    store: false,
    parallel_tool_calls: true,
    max_output_tokens: safeMaxOutputTokens,
  };

  if (translated.instructions) {
    body.instructions = translated.instructions;
  }

  if (settings.enableToolUse) {
    const formattedTools = toResponsesTools(tools);
    if (formattedTools.length > 0) {
      body.tools = formattedTools;
    }
  }

  const bodyJson = JSON.stringify(body);
  const clientVersion = normalizeClientVersion(settings.openAICodexClientVersion);
  const cleanupFns: UnlistenFn[] = [];

  try {
    cleanupFns.push(
      await listen<{ request_id: string; data: string }>('ai-stream-chunk', (event) => {
        if (event.payload.request_id !== requestId) return;
        const chunk = processor.processLine(event.payload.data);
        if (chunk) {
          callbacks.onChunk(chunk);
        }
      }),
    );

    const doneUnlisten = listen<{ request_id: string }>('ai-stream-done', (event) => {
      if (event.payload.request_id !== requestId) return;

      const toolCalls = processor.getToolCalls();
      for (let i = 0; i < toolCalls.length; i++) {
        callbacks.onChunk({
          type: 'tool_call',
          toolCall: toolCalls[i],
          finishReason: i === 0 ? 'tool_calls' : undefined,
        });
      }

      callbacks.onChunk({
        type: 'done',
        finishReason: toolCalls.length > 0 ? 'tool_calls' : 'stop',
        inputTokens: processor.inputTokens || undefined,
        outputTokens: processor.outputTokens || undefined,
      });

      callbacks.onDone();
      doneResolve();
    });

    const errorUnlisten = listen<{ request_id: string; error: string }>('ai-stream-error', (event) => {
      if (event.payload.request_id !== requestId) return;
      callbacks.onError(event.payload.error);
      doneReject(new Error(event.payload.error));
    });

    let doneResolve!: () => void;
    let doneReject!: (err: Error) => void;
    const donePromise = new Promise<void>((resolve, reject) => {
      doneResolve = resolve;
      doneReject = reject;
    });

    cleanupFns.push(await doneUnlisten);
    cleanupFns.push(await errorUnlisten);

    donePromise.catch(() => {});
    await aiChatStreamOpenAICodex(requestId, settings.baseUrl, settings.apiKey, bodyJson, clientVersion);
    await donePromise;
  } finally {
    for (const fn of cleanupFns) fn();
  }
}
