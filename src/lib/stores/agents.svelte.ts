/** Multi-agent store using Svelte 5 runes */

export interface ConversationRef {
	id: string;
	type: 'agent' | 'claude-code';
	title: string;
	status: string;
	lastMessageAt: number;
	lastActivity: string;
	archived: boolean;
}

import { ChatEngine } from '$lib/ai/chat/chat-engine';
import { ChatSession } from '$lib/ai/chat/session';
import type { AIChatSettings, ToolCall as AIToolCall, ToolResult as AIToolResult } from '$lib/ai/types';
import { getSettings, getActiveAIKey, getActiveAIBaseUrl } from '$lib/stores/settings.svelte';

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
	title: string;
	status: 'idle' | 'thinking' | 'writing' | 'running_tool' | 'done' | 'error';
	model: string;
	provider: string;
	currentActivity?: string;
	lastError?: string;
	lastMessageAt: number;
	lastActivity: string;
	archived: boolean;
	createdAt: number;
}

let agents = $state<Agent[]>([]);
let agentMessages = $state<Record<string, AgentMessage[]>>({});
let agentLoading = $state<Record<string, boolean>>({});
let focusedAgentId = $state<string | null>(null);
let searchQuery = $state('');

let nextAgentNum = 1;

// Map of agentId -> ChatEngine instance
const chatEngines = new Map<string, ChatEngine>();

function generateId(): string {
	return crypto.randomUUID();
}

/** Auto-generate title from first user message */
function autoTitle(content: string): string {
	const cleaned = content.replace(/\s+/g, ' ').trim();
	if (cleaned.length <= 40) return cleaned;
	const truncated = cleaned.slice(0, 40);
	const lastSpace = truncated.lastIndexOf(' ');
	return (lastSpace > 20 ? truncated.slice(0, lastSpace) : truncated) + '...';
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

				if (streamingMsgId && streamedContent) {
					store.updateMessage(agentId, streamingMsgId, {
						content: streamedContent,
						toolCalls: uiToolCalls,
					});
				} else {
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
				const toolNames = toolCalls.map(tc => tc.function.name).join(', ');
				store.setActivity(agentId, `Running ${toolNames}...`);
				// Update lastActivity for sidebar display
				store.updateAgentMeta(agentId, { lastActivity: toolNames });
			},

			onToolResult(results: AIToolResult[]) {
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

				streamingMsgId = null;
				streamedContent = '';
				streamedThinking = '';
				store.setStatus(agentId, 'thinking');
				store.setActivity(agentId, 'Thinking...');
			},

			async onToolConfirmation(_toolCall: AIToolCall): Promise<boolean> {
				return true;
			},

			onDone(_fullContent: string) {
				streamingMsgId = null;
				streamedContent = '';
				streamedThinking = '';
				store.setStatus(agentId, 'idle');
				store.setActivity(agentId, undefined);
				store.setLoading(agentId, false);
				// Update lastActivity with a preview of the response
				const preview = _fullContent.replace(/\s+/g, ' ').trim().slice(0, 60);
				if (preview) {
					store.updateAgentMeta(agentId, { lastActivity: preview + (preview.length >= 60 ? '...' : '') });
				}
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

	engine.updateSettings(buildAIChatSettings());
	return engine;
}

export function getAgentStore() {
	return {
		get agents() { return agents; },
		get focusedAgentId() { return focusedAgentId; },
		set focusedAgentId(id: string | null) { focusedAgentId = id; },
		get searchQuery() { return searchQuery; },
		set searchQuery(q: string) { searchQuery = q; },

		get activeConversations(): Agent[] {
			return agents
				.filter(a => !a.archived)
				.sort((a, b) => b.lastMessageAt - a.lastMessageAt);
		},

		get archivedConversations(): Agent[] {
			return agents
				.filter(a => a.archived)
				.sort((a, b) => b.lastMessageAt - a.lastMessageAt);
		},

		get filteredActiveConversations(): Agent[] {
			const q = searchQuery.toLowerCase().trim();
			const active = agents.filter(a => !a.archived).sort((a, b) => b.lastMessageAt - a.lastMessageAt);
			if (!q) return active;
			return active.filter(a => a.title.toLowerCase().includes(q) || a.lastActivity.toLowerCase().includes(q));
		},

		get filteredArchivedConversations(): Agent[] {
			const q = searchQuery.toLowerCase().trim();
			const archived = agents.filter(a => a.archived).sort((a, b) => b.lastMessageAt - a.lastMessageAt);
			if (!q) return archived;
			return archived.filter(a => a.title.toLowerCase().includes(q) || a.lastActivity.toLowerCase().includes(q));
		},

		get focusedAgent(): Agent | undefined {
			return focusedAgentId ? agents.find(a => a.id === focusedAgentId) : undefined;
		},

		addAgent(name?: string, model?: string, provider?: string): string {
			const id = generateId();
			const s = getSettings();
			const agent: Agent = {
				id,
				name: name || `Agent ${nextAgentNum++}`,
				title: 'New Chat',
				status: 'idle',
				model: model || s.aiModel || 'anthropic/claude-sonnet-4-6',
				provider: provider || s.aiProvider || 'openrouter',
				lastMessageAt: Date.now(),
				lastActivity: '',
				archived: false,
				createdAt: Date.now()
			};
			agents = [...agents, agent];
			agentMessages[id] = [];
			agentLoading[id] = false;
			focusedAgentId = id;
			return id;
		},

		removeAgent(id: string): void {
			const engine = chatEngines.get(id);
			if (engine) {
				engine.abort();
				chatEngines.delete(id);
			}

			agents = agents.filter(a => a.id !== id);
			delete agentMessages[id];
			delete agentLoading[id];
			if (focusedAgentId === id) {
				const remaining = agents.filter(a => !a.archived);
				focusedAgentId = remaining.length > 0 ? remaining[0].id : null;
			}
		},

		archiveAgent(id: string): void {
			agents = agents.map(a => a.id === id ? { ...a, archived: true } : a);
			if (focusedAgentId === id) {
				const remaining = agents.filter(a => !a.archived);
				focusedAgentId = remaining.length > 0 ? remaining[0].id : null;
			}
		},

		unarchiveAgent(id: string): void {
			agents = agents.map(a => a.id === id ? { ...a, archived: false } : a);
		},

		renameAgent(id: string, title: string): void {
			agents = agents.map(a => a.id === id ? { ...a, title } : a);
		},

		updateAgentMeta(id: string, updates: Partial<Pick<Agent, 'lastActivity' | 'lastMessageAt' | 'title'>>): void {
			agents = agents.map(a => a.id === id ? { ...a, ...updates } : a);
		},

		getMessages(agentId: string): AgentMessage[] {
			return agentMessages[agentId] || [];
		},

		addMessage(agentId: string, message: AgentMessage): void {
			if (!agentMessages[agentId]) {
				agentMessages[agentId] = [];
			}
			agentMessages[agentId] = [...agentMessages[agentId], message];
			// Update lastMessageAt
			agents = agents.map(a => a.id === agentId ? { ...a, lastMessageAt: Date.now() } : a);
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
			const engine = chatEngines.get(agentId);
			if (engine) {
				engine.getSession().clear();
			}
		},

		abortAgent(agentId: string): void {
			const engine = chatEngines.get(agentId);
			if (engine) {
				engine.abort();
			}
			this.setStatus(agentId, 'idle');
			this.setActivity(agentId, undefined);
			this.setLoading(agentId, false);
		},

		async sendMessage(agentId: string, content: string): Promise<void> {
			const userMsg: AgentMessage = {
				id: generateId(),
				role: 'user',
				content,
				timestamp: Date.now()
			};
			this.addMessage(agentId, userMsg);

			// Auto-title from first user message
			const agent = agents.find(a => a.id === agentId);
			if (agent && agent.title === 'New Chat') {
				this.updateAgentMeta(agentId, { title: autoTitle(content) });
			}

			this.setStatus(agentId, 'thinking');
			this.setLoading(agentId, true);
			this.setActivity(agentId, 'Thinking...');

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

			const engine = getOrCreateEngine(agentId);
			try {
				await engine.sendMessage(content);
			} catch (err) {
				console.error('[agent] sendMessage error:', err);
			}
		}
	};
}
