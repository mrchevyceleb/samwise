import { getWorkspace } from '$lib/stores/workspace';
import { readFileText } from '$lib/utils/tauri';
import { streamOpenAICompatibleCompletion } from '../providers/openrouter';
import { streamOpenAICodexCompletion } from '../providers/openai-codex';
import { streamAnthropicCompletion } from '../providers/anthropic';
import { inferModelSettings } from '../model-registry';
import { executeTool } from '../tools/tool-executor';
import { getAllToolDefinitions, WRITE_TOOLS } from '../tools/tool-definitions';
import { getPromptProfile } from '../prompts/base-prompts';
import { ChatSession } from './session';
import type {
  ChatMessage,
  ToolCall,
  ToolResult,
  StreamChunk,
  AIChatSettings,
  ChatEngineCallbacks,
  ContextUsage,
} from '../types';

/** File names to look for as workspace instructions (checked in order). */
const INSTRUCTION_FILES = [
  'AGENTS.MD', 'AGENTS.md', 'agents.md',
  'CLAUDE.md', 'CLAUDE.MD', 'claude.md',
];
const DEFAULT_CONTEXT_WINDOW = 128000;
const AUTO_COMPACT_THRESHOLD_PERCENT = 75;

export class ChatEngine {
  private session: ChatSession;
  private settings: AIChatSettings;
  private callbacks: ChatEngineCallbacks;
  private abortController: AbortController | null = null;
  private isRunning = false;
  private workspaceInstructions: string | null = null;
  private instructionsWorkspacePath: string | null = null;
  private workspacePromptProfile: string | null = null;
  promptProfile: string | null = null;

  constructor(session: ChatSession, settings: AIChatSettings, callbacks: ChatEngineCallbacks) {
    this.session = session;
    this.settings = settings;
    this.callbacks = callbacks;
  }

  get running(): boolean {
    return this.isRunning;
  }

  get currentProvider(): string {
    return this.settings.provider;
  }

  abort(): void {
    this.abortController?.abort();
    this.compactAbort?.abort();
    this.compactAbort = null;
    this.isRunning = false;
    this.cleanupOrphanedToolCalls();
  }

  /**
   * After aborting, the session may have an assistant message with tool_calls
   * but no corresponding tool_result messages. Add synthetic "cancelled" tool_results.
   */
  private cleanupOrphanedToolCalls(): void {
    const messages = this.session.getMessages();
    if (messages.length === 0) return;

    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.role === 'assistant' && msg.tool_calls?.length) {
        const toolCallIds = new Set(msg.tool_calls.map(tc => tc.id));
        for (let j = i + 1; j < messages.length; j++) {
          if (messages[j].role === 'tool' && messages[j].tool_call_id) {
            toolCallIds.delete(messages[j].tool_call_id!);
          }
        }

        for (const orphanId of toolCallIds) {
          const tc = msg.tool_calls.find(t => t.id === orphanId);
          this.session.addMessage({
            role: 'tool',
            content: 'Operation cancelled by user.',
            tool_call_id: orphanId,
            name: tc?.function?.name || 'unknown',
          });
        }
        break;
      }
      if (msg.role === 'user') break;
    }
  }

  updateSettings(settings: AIChatSettings): void {
    const oldModel = this.settings.model;
    const oldContextWindow = this.getContextWindow();
    this.settings = settings;
    const newContextWindow = this.getContextWindow();

    if (settings.model !== oldModel) {
      this._lastApiInputTokens = 0;
      this._lastApiMessageCount = 0;
      this._lastCacheCreation = 0;
      this._lastCacheRead = 0;
      this._lastApiOutputTokens = 0;
    }

    if (newContextWindow < oldContextWindow && this.session.getMessages().length > 0) {
      const usage = this.getContextUsage();
      if (usage.usedPercent > 90) {
        this.compactSession(true, 2).catch(() => {});
      }
    }
  }

  getSession(): ChatSession {
    return this.session;
  }

  getContextUsage(): ContextUsage {
    return this.computeContextUsage(this.buildMessages());
  }

  private compactAbort: AbortController | null = null;

  async compactNow(): Promise<{ usage: ContextUsage; compacted: boolean }> {
    this.compactAbort?.abort();
    this.compactAbort = new AbortController();
    const compacted = await this.compactSession(true, 0, this.compactAbort.signal);
    this.compactAbort = null;
    return {
      usage: this.getContextUsage(),
      compacted,
    };
  }

  /** Try to load AGENTS.MD / CLAUDE.md from the workspace root. */
  private async loadWorkspaceInstructions(): Promise<void> {
    const workspace = getWorkspace();
    const wsPath = workspace.path;
    this.workspaceInstructions = null;
    this.instructionsWorkspacePath = wsPath;
    if (!wsPath) return;

    const sep = wsPath.includes('\\') ? '\\' : '/';

    for (const name of INSTRUCTION_FILES) {
      try {
        const content = await readFileText(`${wsPath}${sep}${name}`);
        if (content && content.trim()) {
          this.workspaceInstructions = content.trim();
          this.instructionsWorkspacePath = wsPath;
          return;
        }
      } catch {
        // File doesn't exist, try next
      }
    }
    this.instructionsWorkspacePath = wsPath;
  }

  setPromptProfile(profileId: string): void {
    this.promptProfile = profileId;
  }

  async sendMessage(
    userContent: string,
    options?: {
      mentions?: string[];
      steer?: string;
      images?: Array<{ mediaType: string; base64: string }>;
      slashCommandName?: string;
      slashCommandPrompt?: string;
      slashCommandArgs?: string;
    },
  ): Promise<void> {
    if (this.isRunning) return;
    this.isRunning = true;
    const myController = new AbortController();
    this.abortController = myController;

    try {
      const workspace = getWorkspace();
      const wsPath = workspace.path;
      if (
        this.settings.readInstructionsEachMessage !== false ||
        this.instructionsWorkspacePath !== wsPath
      ) {
        await this.loadWorkspaceInstructions();
      }

      this.emitContextUsage();

      // Add user message
      let enrichedUserContent = userContent;
      if (options?.slashCommandPrompt) {
        const slashName = options.slashCommandName || 'command';
        const requestText = (options.slashCommandArgs || '').trim() || userContent;
        enrichedUserContent =
          `Slash command /${slashName}:\n${options.slashCommandPrompt}\n\n` +
          `User request:\n${requestText}`;
      }

      if (options?.mentions && options.mentions.length > 0) {
        const mentionLines = options.mentions.map((m) => `- ${m}`).join('\n');
        enrichedUserContent = `Tagged paths:\n${mentionLines}\n\nUser request:\n${enrichedUserContent}`;
      }
      const userMessage: ChatMessage = { role: 'user', content: enrichedUserContent };
      if (options?.images?.length) {
        userMessage.images = options.images.map(img => ({
          mediaType: img.mediaType as 'image/png' | 'image/jpeg' | 'image/webp' | 'image/gif',
          base64: img.base64,
        }));
      }
      this.session.addMessage(userMessage);
      await this.compactSession(false, 1);
      this.emitContextUsage();

      // Run agentic loop
      let iterations = 0;
      const maxIterations = this.settings.maxToolIterations;

      while (iterations < maxIterations) {
        if (myController.signal.aborted) break;
        iterations++;

        const messages = this.buildMessages();

        const tools = this.settings.enableToolUse ? getAllToolDefinitions() : undefined;
        let fullContent = '';
        let toolCalls: ToolCall[] = [];
        let finishReason = 'stop';
        let lastCacheCreation = 0;
        let lastCacheRead = 0;

        await new Promise<void>((resolve, reject) => {
          const streamFn = this.getStreamFn();

          let inThinkTag = false;
          let textBuffer = '';

          const flushTextBuffer = () => {
            if (!textBuffer) return;
            let remaining = textBuffer;
            textBuffer = '';

            while (remaining.length > 0) {
              if (inThinkTag) {
                const closeIdx = remaining.indexOf('</think>');
                if (closeIdx === -1) {
                  this.callbacks.onThinking?.(remaining);
                  remaining = '';
                } else {
                  const thinkContent = remaining.slice(0, closeIdx);
                  if (thinkContent) this.callbacks.onThinking?.(thinkContent);
                  remaining = remaining.slice(closeIdx + '</think>'.length);
                  inThinkTag = false;
                }
              } else {
                const openIdx = remaining.indexOf('<think>');
                if (openIdx === -1) {
                  fullContent += remaining;
                  this.callbacks.onChunk(remaining);
                  remaining = '';
                } else {
                  const before = remaining.slice(0, openIdx);
                  if (before) {
                    fullContent += before;
                    this.callbacks.onChunk(before);
                  }
                  remaining = remaining.slice(openIdx + '<think>'.length);
                  inThinkTag = true;
                }
              }
            }
          };

          streamFn(
            messages,
            this.settings,
            tools,
            {
              onChunk: (chunk: StreamChunk) => {
                if (myController.signal.aborted) return;

                switch (chunk.type) {
                  case 'text':
                    textBuffer += chunk.content || '';
                    flushTextBuffer();
                    break;
                  case 'thinking':
                    this.callbacks.onThinking?.(chunk.content || '');
                    break;
                  case 'tool_call':
                    if (chunk.toolCall) {
                      toolCalls.push(chunk.toolCall);
                    }
                    if (chunk.finishReason) {
                      finishReason = chunk.finishReason;
                    }
                    break;
                  case 'done':
                    if (chunk.finishReason) {
                      finishReason = chunk.finishReason;
                    }
                    if (chunk.cacheCreationTokens) lastCacheCreation = chunk.cacheCreationTokens;
                    if (chunk.cacheReadTokens) lastCacheRead = chunk.cacheReadTokens;
                    if (chunk.inputTokens) {
                      this._lastApiInputTokens = chunk.inputTokens;
                      this._lastApiMessageCount = messages.length;
                    }
                    if (chunk.outputTokens) this._lastApiOutputTokens = chunk.outputTokens;
                    break;
                  case 'error':
                    reject(new Error(chunk.content || 'Stream error'));
                    break;
                }
              },
              onDone: () => resolve(),
              onError: (error: string) => reject(new Error(error)),
            }
          ).catch(reject);
        });

        if (lastCacheCreation) this._lastCacheCreation = lastCacheCreation;
        if (lastCacheRead) this._lastCacheRead = lastCacheRead;

        if (myController.signal.aborted) break;

        // Handle response
        if (finishReason === 'tool_calls' && toolCalls.length > 0) {
          toolCalls = toolCalls
            .filter((tc) => tc?.type === 'function' && !!tc.function?.name)
            .map((tc) => ({
              ...tc,
              function: {
                name: tc.function.name,
                arguments: (tc.function.arguments || '{}').trim() || '{}',
              },
            }));

          if (toolCalls.length === 0) {
            this.session.addMessage({
              role: 'assistant',
              content: fullContent || 'I could not produce a valid tool call. Please try again.',
            });
            this.callbacks.onDone(fullContent);
            this.emitContextUsage();
            break;
          }

          this.session.addMessage({
            role: 'assistant',
            content: fullContent || null,
            tool_calls: toolCalls,
          });

          this.callbacks.onToolCall(toolCalls);
          this.emitContextUsage();

          const results: ToolResult[] = [];
          for (const tc of toolCalls) {
            if (myController.signal.aborted) break;

            if (!this.settings.yoloMode && this.settings.confirmWrites && (WRITE_TOOLS.has(tc.function.name) || tc.function.name.startsWith('mcp__'))) {
              const confirmed = await this.callbacks.onToolConfirmation(tc);
              if (!confirmed) {
                results.push({
                  toolCallId: tc.id,
                  toolName: tc.function.name,
                  content: 'User denied this operation.',
                  isError: true,
                });
                continue;
              }
            }

            const result = await executeTool(tc);
            results.push(result);
          }

          if (myController.signal.aborted) break;

          for (const result of results) {
            this.session.addMessage({
              role: 'tool',
              content: result.content,
              tool_call_id: result.toolCallId,
              name: result.toolName,
            });
          }

          this.callbacks.onToolResult(results);
          this.emitContextUsage();

          fullContent = '';
          toolCalls = [];
          continue;
        }

        // No tool calls - we're done
        this.session.addMessage({
          role: 'assistant',
          content: fullContent,
        });
        this.callbacks.onDone(fullContent);
        this.emitContextUsage();
        break;
      }

      if (iterations >= maxIterations) {
        this.callbacks.onError(`Reached maximum tool iterations (${maxIterations})`);
      }

      await this.session.save();
    } catch (error) {
      if (!myController.signal.aborted) {
        this.callbacks.onError(
          error instanceof Error ? error.message : String(error)
        );
      }
    } finally {
      if (this.abortController === myController) {
        this.isRunning = false;
        this.abortController = null;
      }
    }
  }

  private getContextWindow(): number {
    return Math.max(this.settings.contextWindow || DEFAULT_CONTEXT_WINDOW, 4096);
  }

  private estimateTokens(text: string): number {
    if (!text) return 0;
    return Math.ceil(text.length / 4);
  }

  private estimateMessageTokens(message: ChatMessage): number {
    let tokens = 6;
    if (message.content) {
      tokens += this.estimateTokens(message.content);
    }
    if (message.images?.length) {
      tokens += message.images.length * 1600;
    }
    if (message.tool_calls?.length) {
      for (const call of message.tool_calls) {
        tokens += 12;
        tokens += this.estimateTokens(call.function.name || '');
        tokens += this.estimateTokens(call.function.arguments || '');
      }
    }
    if (message.tool_call_id) {
      tokens += this.estimateTokens(message.tool_call_id);
    }
    if (message.name) {
      tokens += this.estimateTokens(message.name);
    }
    return tokens;
  }

  private _lastCacheCreation = 0;
  private _lastCacheRead = 0;
  private _lastApiInputTokens = 0;
  private _lastApiOutputTokens = 0;
  private _lastApiMessageCount = 0;

  private computeContextUsage(messages: ChatMessage[]): ContextUsage {
    const { maxOutputTokens } = inferModelSettings(this.settings.model);
    const fullContextWindow = this.getContextWindow();
    const reservedOutputTokens = maxOutputTokens;
    const effectiveBudget = Math.max(1, fullContextWindow - reservedOutputTokens);

    const estimate = messages.reduce((sum, m) => sum + this.estimateMessageTokens(m), 0);

    let usedTokens: number;
    let isEstimated: boolean;

    if (this._lastApiInputTokens > 0) {
      const msgCountSinceApi = messages.length - this._lastApiMessageCount;
      if (msgCountSinceApi <= 0) {
        usedTokens = this._lastApiInputTokens;
      } else {
        const newMsgs = messages.slice(-msgCountSinceApi);
        const newMsgTokens = newMsgs.reduce((sum, m) => sum + this.estimateMessageTokens(m), 0);
        usedTokens = this._lastApiInputTokens + newMsgTokens;
      }
      isEstimated = msgCountSinceApi > 0;
    } else {
      const toolOverhead = this.estimateToolDefinitionTokens();
      usedTokens = estimate + toolOverhead;
      isEstimated = true;
    }

    const remainingTokens = Math.max(0, effectiveBudget - usedTokens);
    const usedPercent = Math.min(100, (usedTokens / effectiveBudget) * 100);

    return {
      usedTokens,
      maxTokens: effectiveBudget,
      remainingTokens,
      usedPercent,
      cacheCreationTokens: this._lastCacheCreation || undefined,
      cacheReadTokens: this._lastCacheRead || undefined,
      apiInputTokens: this._lastApiInputTokens || undefined,
      apiOutputTokens: this._lastApiOutputTokens || undefined,
      reservedOutputTokens,
      isEstimated,
    };
  }

  private estimateToolDefinitionTokens(): number {
    if (!this.settings.enableToolUse) return 0;
    const tools = getAllToolDefinitions();
    return tools.length * 200;
  }

  private emitContextUsage(): void {
    this.callbacks.onContextUsage?.(this.getContextUsage());
  }

  private getStreamFn() {
    if (this.settings.provider === 'anthropic') return streamAnthropicCompletion;
    if (this.settings.provider === 'openai' && this.settings.authMode === 'oauth') return streamOpenAICodexCompletion;
    return streamOpenAICompatibleCompletion;
  }

  private buildLocalCompactionSummary(messages: ChatMessage[]): string {
    const lines: string[] = [];

    for (const message of messages) {
      const role = message.role.toUpperCase();
      const content = (message.content || '').trim().replace(/\s+/g, ' ');
      if (content) {
        lines.push(`${role}: ${content.slice(0, 220)}`);
      }

      if (message.tool_calls?.length) {
        for (const call of message.tool_calls) {
          const args = (call.function.arguments || '').replace(/\s+/g, ' ').slice(0, 120);
          lines.push(`TOOL: ${call.function.name}(${args})`);
        }
      }
    }

    const joined = lines.join('\n').slice(0, 50000);
    if (!joined) {
      return 'Older context was compacted to save room for newer messages.';
    }
    return joined;
  }

  private async buildLLMCompactionSummary(messages: ChatMessage[], signal?: AbortSignal): Promise<string> {
    const localSummary = this.buildLocalCompactionSummary(messages);

    const summarizeMessages: ChatMessage[] = [
      {
        role: 'system',
        content: `You are a conversation summarizer. Your job is to produce a concise but comprehensive summary of a conversation between a user and an AI assistant. The summary will replace the original messages in the conversation context, so it must preserve:

- All key decisions, conclusions, and agreements
- Important facts, code snippets, file paths, and technical details discussed
- The current state of any ongoing tasks or problems
- Any user preferences or constraints mentioned
- Tool calls made and their outcomes (what was read, written, executed)

Write the summary in a structured format. Be thorough but concise. Do NOT include preamble like "Here is a summary". Just write the summary directly.`,
      },
      {
        role: 'user',
        content: `Summarize this conversation history:\n\n${localSummary}`,
      },
    ];

    let summary = '';

    const streamFn = this.getStreamFn();

    await new Promise<void>((resolve, reject) => {
      if (signal?.aborted) {
        reject(new Error('Compaction aborted'));
        return;
      }

      const onAbort = () => reject(new Error('Compaction aborted'));
      signal?.addEventListener('abort', onAbort, { once: true });

      streamFn(
        summarizeMessages,
        { ...this.settings, maxTokens: 2048, enableToolUse: false },
        undefined,
        {
          onChunk: (chunk: StreamChunk) => {
            if (signal?.aborted) return;
            if (chunk.type === 'text' && chunk.content) {
              summary += chunk.content;
            }
          },
          onDone: () => {
            signal?.removeEventListener('abort', onAbort);
            resolve();
          },
          onError: (error: string) => {
            signal?.removeEventListener('abort', onAbort);
            reject(new Error(error));
          },
        },
      ).catch((err) => {
        signal?.removeEventListener('abort', onAbort);
        reject(err);
      });
    });

    if (!summary.trim()) {
      throw new Error('LLM returned empty summary');
    }

    return summary.trim();
  }

  private async compactSession(force: boolean, preserveLatestCount: number, signal?: AbortSignal): Promise<boolean> {
    const usage = this.getContextUsage();
    if (!force && usage.usedPercent < AUTO_COMPACT_THRESHOLD_PERCENT) {
      return false;
    }

    const messages = this.session.getMessages();
    if (messages.length <= 1) {
      return false;
    }

    const safePreserve = Math.max(0, Math.min(preserveLatestCount, messages.length - 1));
    let splitIndex = messages.length - safePreserve;
    while (splitIndex > 0 && splitIndex < messages.length && messages[splitIndex].role === 'tool') {
      splitIndex--;
    }
    const head = messages.slice(0, splitIndex);
    const tail = messages.slice(splitIndex);

    if (head.length === 0) {
      return false;
    }

    let summary: string;
    if (force) {
      this.callbacks.onCompactionStart?.();
      try {
        summary = await this.buildLLMCompactionSummary(head, signal);
      } catch (err) {
        if (signal?.aborted) {
          this.callbacks.onCompactionEnd?.();
          return false;
        }
        summary = this.buildLocalCompactionSummary(head);
      }
      this.callbacks.onCompactionEnd?.();
    } else {
      summary = this.buildLocalCompactionSummary(head);
    }

    const compacted: ChatMessage[] = [
      {
        role: 'assistant',
        content: `[Conversation Memory]\n${summary}`,
      },
      ...tail,
    ];

    this.session.replaceMessages(compacted);
    this._lastApiInputTokens = 0;
    this._lastApiMessageCount = 0;
    this._lastCacheCreation = 0;
    this._lastCacheRead = 0;
    this._lastApiOutputTokens = 0;
    await this.session.save();
    this.callbacks.onCompaction?.(head.length);
    return true;
  }

  private buildMessages(): ChatMessage[] {
    const workspace = getWorkspace();
    const wsPath = workspace.path || 'No workspace open';

    const profile = getPromptProfile(this.promptProfile || this.workspacePromptProfile || 'default');
    const now = new Date();
    const dateStr = now.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
    let systemContent = `${profile.systemPrompt}\n\nCurrent date: ${dateStr}\nCurrent workspace: ${wsPath}`;

    if (this.workspaceInstructions) {
      systemContent += `\n\n## Workspace Instructions\nThe following instructions were loaded from the workspace root. Follow them when working in this project:\n\n${this.workspaceInstructions}`;
    }

    systemContent += `\n\n## Grounding Rules\n- For repository-specific claims, inspect workspace files first when possible.\n- If evidence is missing, say what is unknown and what to check next.\n- Do not invent files, commands, outputs, APIs, or model capabilities.\n- Keep tool outputs authoritative over assumptions.`;

    const systemMessage: ChatMessage = {
      role: 'system',
      content: systemContent,
    };

    return [systemMessage, ...this.sanitizeMessages(this.session.getMessages())];
  }

  /**
   * Sanitize message history to enforce strict tool_call/tool_result pairing.
   */
  private sanitizeMessages(messages: ChatMessage[]): ChatMessage[] {
    const completeToolCallIds = new Set<string>();
    const incompleteAssistantIndices = new Set<number>();

    for (let i = 0; i < messages.length; i++) {
      const msg = messages[i];
      if (msg.role !== 'assistant' || !msg.tool_calls?.length) continue;

      const expectedIds = new Set(msg.tool_calls.map((tc) => tc.id));
      const foundIds = new Set<string>();

      let j = i + 1;
      while (j < messages.length && messages[j].role === 'tool') {
        if (messages[j].tool_call_id && expectedIds.has(messages[j].tool_call_id!)) {
          foundIds.add(messages[j].tool_call_id!);
        }
        j++;
      }

      if (foundIds.size === expectedIds.size) {
        for (const id of foundIds) completeToolCallIds.add(id);
      } else if (foundIds.size === 0) {
        incompleteAssistantIndices.add(i);
      } else {
        incompleteAssistantIndices.add(i);
        for (const id of foundIds) completeToolCallIds.add(id);
      }
    }

    const result: ChatMessage[] = [];
    for (let i = 0; i < messages.length; i++) {
      const msg = messages[i];

      if (msg.role === 'tool' && msg.tool_call_id) {
        if (!completeToolCallIds.has(msg.tool_call_id)) continue;
        result.push(msg);
        continue;
      }

      if (msg.role === 'assistant' && msg.tool_calls?.length && incompleteAssistantIndices.has(i)) {
        const keptCalls = msg.tool_calls.filter((tc) => completeToolCallIds.has(tc.id));
        if (keptCalls.length === 0) {
          if (msg.content) {
            result.push({ ...msg, tool_calls: undefined });
          }
          continue;
        }
        result.push({ ...msg, tool_calls: keptCalls });
        continue;
      }

      result.push(msg);
    }

    return result;
  }
}
