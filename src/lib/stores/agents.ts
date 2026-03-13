/** Multi-agent store using Svelte 5 runes */

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

function generateId(): string {
	return crypto.randomUUID();
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
			const agent: Agent = {
				id,
				name: name || `Agent ${nextAgentNum++}`,
				status: 'idle',
				model: model || 'claude-sonnet-4-20250514',
				provider: provider || 'anthropic',
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
		},

		/** Send a user message to an agent (placeholder, will integrate with backend) */
		async sendMessage(agentId: string, content: string): Promise<void> {
			const userMsg: AgentMessage = {
				id: generateId(),
				role: 'user',
				content,
				timestamp: Date.now()
			};
			this.addMessage(agentId, userMsg);
			this.setStatus(agentId, 'thinking');
			this.setLoading(agentId, true);

			// Simulate a response for now (will be replaced with real backend call)
			setTimeout(() => {
				const assistantMsg: AgentMessage = {
					id: generateId(),
					role: 'assistant',
					content: `I received your message. Backend integration coming soon.\n\nYou said: "${content}"`,
					thinking: 'Analyzing the request and determining the best approach...',
					timestamp: Date.now()
				};
				this.addMessage(agentId, assistantMsg);
				this.setStatus(agentId, 'idle');
				this.setLoading(agentId, false);
			}, 1500);
		}
	};
}
