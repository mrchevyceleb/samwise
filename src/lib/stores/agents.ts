/** Multi-agent store using Svelte 5 runes */

import { ChatEngine } from '$lib/ai/chat/chat-engine';
import { ChatSession } from '$lib/ai/chat/session';
import type { AIChatSettings, ToolCall as AIToolCall, ToolResult as AIToolResult } from '$lib/ai/types';
import { getSettings, getActiveAIKey, getActiveAIBaseUrl } from '$lib/stores/settings';

export interface ToolCall {
	id: string;
	name: string;
	args: Record<string, unknown>;
	status: 'pending' | 'running' | 'complete' | 'error';
	result?: string;
	startedAt?: number;
	completedAt?: number;
}

export interface ToolResult {
	toolCallId: string;
	content: string;
	isError?: boolean;
}

export interface AgentMessage {
	id: string;
	role: 'user' | 'assistant' | 'system' | 'tool';
	content: string;
	thinking?: string;
	toolCalls?: ToolCall[];
	toolResults?: ToolResult[];
	timestamp: number;
}

export interface Agent {
	id: string;
	name: string;
	status: 'idle' | 'thinking' | 'writing' | 'running_tool' | 'done' | 'error';
	model: string;
	provider: string;
	currentActivity?: string;
	lastError?: string;
	createdAt: number;
}

let agents = $state<Agent[]>([]);
let activeAgentIds = $state<string[]>([]);
let viewMode = $state<'stacked' | 'split'>('stacked');
let agentMessages = $state<Record<string, AgentMessage[]>>({});
let agentLoading = $state<Record<string, boolean>>({});
let focusedAgentId = $state<string | null>(null);

let nextAgentNum = 1;

// Map of agentId -> ChatEngine instance
const chatEngines = new Map<string, ChatEngine>();

function generateId(): string {
	return crypto.randomUUID();
}

/** Build AIChatSettings from the current app settings */
function buildAIChatSettings(): AIChatSettings {
	const s = getSettings();
	return {
		provider: s.aiProvider,
		authMode: s.aiAuthMode,
		apiKey: getActiveAIKey(s),
		model: s.aiModel,
		baseUrl: getActiveAIBaseUrl(s),
		temperature: s.aiTemperature,
		maxTokens: s.aiMaxTokens,
		contextWindow: s.aiMaxContextTokens,
		openAICodexClientVersion: s.aiOpenAICodexClientVersion,
		enableToolUse: s.aiEnableToolUse,
		confirmWrites: s.aiConfirmWrites,
		yoloMode: s.aiYoloMode,
		maxToolIterations: s.aiMaxToolIterations,
		readInstructionsEachMessage: s.aiReadInstructionsEveryMessage,
	};
}

/** Get or create a ChatEngine for an agent */
function getOrCreateEngine(agentId: string): ChatEngine {
	let engine = chatEngines.get(agentId);
	if (!engine) {
		const session = new ChatSession(`agent-${agentId}`);
		const settings = buildAIChatSettings();

		// We need a reference to the store to update messages, but we can't use
		// `this` from a module function. We'll capture the store functions we need.
		const store = getAgentStore();

		let streamingMsgId: string | null = null;
		let streamedContent = '';
		let streamedThinking = '';

		engine = new ChatEngine(session, settings, {
			onChunk(content: string) {
				streamedContent += content;
				if (streamingMsgId) {
					store.updateMessage(agentId, streamingMsgId, {
						content: streamedContent,
					});
				} else {
					streamingMsgId = generateId();
					store.addMessage(agentId, {
						id: streamingMsgId,
						role: 'assistant',
						content: streamedContent,
						thinking: streamedThinking || undefined,
						timestamp: Date.now(),
					});
				}
				store.setStatus(agentId, 'writing');
				store.setActivity(agentId, 'Writing response...');
			},

			onThinking(content: string) {
				streamedThinking += content;
				if (streamingMsgId) {
					store.updateMessage(agentId, streamingMsgId, {
						thinking: streamedThinking,
					});
				}
				store.setActivity(agentId, 'Thinking...');
			},

			onToolCall(toolCalls: AIToolCall[]) {
				const uiToolCalls: ToolCall[] = toolCalls.map(tc => ({
					id: tc.id,
					name: tc.function.name,
					args: (() => { try { return JSON.parse(tc.function.arguments); } catch { return {}; } })(),
					status: 'running' as const,
					startedAt: Date.now(),
				}));

				// If there was a streaming message with content, finalize it
				if (streamingMsgId && streamedContent) {
					store.updateMessage(agentId, streamingMsgId, {
						content: streamedContent,
						toolCalls: uiToolCalls,
					});
				} else {
					// Create a new message for tool calls
					const msgId = generateId();
					store.addMessage(agentId, {
						id: msgId,
						role: 'assistant',
						content: streamedContent || '',
						toolCalls: uiToolCalls,
						timestamp: Date.now(),
					});
					streamingMsgId = msgId;
				}

				store.setStatus(agentId, 'running_tool');
				store.setActivity(agentId, `Running ${toolCalls.map(tc => tc.function.name).join(', ')}...`);
			},

			onToolResult(results: AIToolResult[]) {
				// Update tool call statuses in the current message
				if (streamingMsgId) {
					const msgs = store.getMessages(agentId);
					const currentMsg = msgs.find(m => m.id === streamingMsgId);
					if (currentMsg?.toolCalls) {
						const updatedToolCalls = currentMsg.toolCalls.map(tc => {
							const result = results.find(r => r.toolCallId === tc.id);
							if (result) {
								return {
									...tc,
									status: result.isError ? 'error' as const : 'complete' as const,
									result: result.content,
									completedAt: Date.now(),
								};
							}
							return tc;
						});
						store.updateMessage(agentId, streamingMsgId, {
							toolCalls: updatedToolCalls,
							toolResults: results.map(r => ({
								toolCallId: r.toolCallId,
								content: r.content,
								isError: r.isError,
							})),
						});
					}
				}

				// Reset for next iteration
				streamingMsgId = null;
				streamedContent = '';
				streamedThinking = '';
				store.setStatus(agentId, 'thinking');
				store.setActivity(agentId, 'Thinking...');
			},

			async onToolConfirmation(_toolCall: AIToolCall): Promise<boolean> {
				// For now, auto-confirm. Can add a UI dialog later.
				return true;
			},

			onDone(_fullContent: string) {
				streamingMsgId = null;
				streamedContent = '';
				streamedThinking = '';
				store.setStatus(agentId, 'idle');
				store.setActivity(agentId, undefined);
				store.setLoading(agentId, false);
			},

			onError(errorMsg: string) {
				if (streamingMsgId) {
					store.updateMessage(agentId, streamingMsgId, {
						content: streamedContent + `\n\n**Error:** ${errorMsg}`,
					});
				} else {
					store.addMessage(agentId, {
						id: generateId(),
						role: 'assistant',
						content: `**Error:** ${errorMsg}`,
						timestamp: Date.now(),
					});
				}
				streamingMsgId = null;
				streamedContent = '';
				streamedThinking = '';
				store.setError(agentId, errorMsg);
				store.setActivity(agentId, undefined);
				store.setLoading(agentId, false);
			},

			onContextUsage(_usage) {
				// Could display context usage in the UI later
			},

			onCompactionStart() {
				store.setActivity(agentId, 'Compacting context...');
			},

			onCompactionEnd() {
				store.setActivity(agentId, undefined);
			},

			onCompaction(_count: number) {
				// Compaction happened
			},
		});

		chatEngines.set(agentId, engine);
	}

	// Always update settings to pick up latest API key, model, etc.
	engine.updateSettings(buildAIChatSettings());
	return engine;
}

export function getAgentStore() {
	return {
		get agents() { return agents; },
		get activeAgentIds() { return activeAgentIds; },
		get viewMode() { return viewMode; },
		set viewMode(v: 'stacked' | 'split') { viewMode = v; },
		get focusedAgentId() { return focusedAgentId; },
		set focusedAgentId(id: string | null) { focusedAgentId = id; },

		get visibleAgents(): Agent[] {
			if (activeAgentIds.length === 0) return agents;
			return agents.filter(a => activeAgentIds.includes(a.id));
		},

		addAgent(name?: string, model?: string, provider?: string): string {
			const id = generateId();
			const s = getSettings();
			const agent: Agent = {
				id,
				name: name || `Agent ${nextAgentNum++}`,
				status: 'idle',
				model: model || s.aiModel || 'anthropic/claude-sonnet-4-6',
				provider: provider || s.aiProvider || 'openrouter',
				createdAt: Date.now()
			};
			agents = [...agents, agent];
			activeAgentIds = [...activeAgentIds, id];
			agentMessages[id] = [];
			agentLoading[id] = false;
			if (!focusedAgentId) {
				focusedAgentId = id;
			}
			return id;
		},

		removeAgent(id: string): void {
			// Clean up the ChatEngine
			const engine = chatEngines.get(id);
			if (engine) {
				engine.abort();
				chatEngines.delete(id);
			}

			agents = agents.filter(a => a.id !== id);
			activeAgentIds = activeAgentIds.filter(aid => aid !== id);
			delete agentMessages[id];
			delete agentLoading[id];
			if (focusedAgentId === id) {
				focusedAgentId = agents.length > 0 ? agents[0].id : null;
			}
		},

		getMessages(agentId: string): AgentMessage[] {
			return agentMessages[agentId] || [];
		},

		addMessage(agentId: string, message: AgentMessage): void {
			if (!agentMessages[agentId]) {
				agentMessages[agentId] = [];
			}
			agentMessages[agentId] = [...agentMessages[agentId], message];
		},

		updateMessage(agentId: string, messageId: string, updates: Partial<AgentMessage>): void {
			const msgs = agentMessages[agentId];
			if (!msgs) return;
			agentMessages[agentId] = msgs.map(m =>
				m.id === messageId ? { ...m, ...updates } : m
			);
		},

		setStatus(agentId: string, status: Agent['status']): void {
			agents = agents.map(a =>
				a.id === agentId ? { ...a, status } : a
			);
		},

		setActivity(agentId: string, activity?: string): void {
			agents = agents.map(a =>
				a.id === agentId ? { ...a, currentActivity: activity } : a
			);
		},

		setError(agentId: string, error?: string): void {
			agents = agents.map(a =>
				a.id === agentId ? { ...a, status: error ? 'error' : 'idle', lastError: error } as Agent : a
			);
		},

		setLoading(agentId: string, loading: boolean): void {
			agentLoading[agentId] = loading;
		},

		isLoading(agentId: string): boolean {
			return agentLoading[agentId] || false;
		},

		getAgent(agentId: string): Agent | undefined {
			return agents.find(a => a.id === agentId);
		},

		clearMessages(agentId: string): void {
			agentMessages[agentId] = [];
			// Also reset the ChatEngine session
			const engine = chatEngines.get(agentId);
			if (engine) {
				engine.getSession().clear();
			}
		},

		/** Abort a running agent */
		abortAgent(agentId: string): void {
			const engine = chatEngines.get(agentId);
			if (engine) {
				engine.abort();
			}
			this.setStatus(agentId, 'idle');
			this.setActivity(agentId, undefined);
			this.setLoading(agentId, false);
		},

		/** Send a user message to an agent via the real ChatEngine */
		async sendMessage(agentId: string, content: string): Promise<void> {
			// Add user message to the UI
			const userMsg: AgentMessage = {
				id: generateId(),
				role: 'user',
				content,
				timestamp: Date.now()
			};
			this.addMessage(agentId, userMsg);
			this.setStatus(agentId, 'thinking');
			this.setLoading(agentId, true);
			this.setActivity(agentId, 'Thinking...');

			// Check for API key
			const s = getSettings();
			const apiKey = getActiveAIKey(s);
			if (!apiKey) {
				this.addMessage(agentId, {
					id: generateId(),
					role: 'assistant',
					content: '**No API key configured.** Go to Settings (Ctrl+,) and add your API key to start chatting.',
					timestamp: Date.now(),
				});
				this.setStatus(agentId, 'error');
				this.setError(agentId, 'No API key');
				this.setLoading(agentId, false);
				this.setActivity(agentId, undefined);
				return;
			}

			// Get or create the ChatEngine and send the message
			const engine = getOrCreateEngine(agentId);
			try {
				await engine.sendMessage(content);
			} catch (err) {
				// Error handling is done in the onError callback, but catch any unhandled ones
				console.error('[agent] sendMessage error:', err);
			}
		}
	};
}
