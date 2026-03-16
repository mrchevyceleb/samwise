/** Unified conversation getter - merges Agent + Claude Code sessions into one sorted list */

import { getAgentStore, type ConversationRef } from './agents.svelte';
import { getClaudeCodeStore } from './claude-code.svelte';

export type { ConversationRef };

export function getConversations() {
	const agents = getAgentStore();
	const cc = getClaudeCodeStore();

	return {
		get active(): ConversationRef[] {
			const agentRefs: ConversationRef[] = agents.agents
				.filter(a => !a.archived)
				.map(a => ({
					id: a.id,
					type: 'agent' as const,
					title: a.title,
					status: a.status,
					lastMessageAt: a.lastMessageAt,
					lastActivity: a.lastActivity,
					archived: false,
				}));

			const ccRefs: ConversationRef[] = cc.allSessions
				.filter(s => !s.archived)
				.map(s => ({
					id: s.id,
					type: 'claude-code' as const,
					title: s.title,
					status: s.status,
					lastMessageAt: s.lastMessageAt,
					lastActivity: s.lastActivity,
					archived: false,
				}));

			return [...agentRefs, ...ccRefs].sort((a, b) => b.lastMessageAt - a.lastMessageAt);
		},

		get archived(): ConversationRef[] {
			const agentRefs: ConversationRef[] = agents.agents
				.filter(a => a.archived)
				.map(a => ({
					id: a.id,
					type: 'agent' as const,
					title: a.title,
					status: a.status,
					lastMessageAt: a.lastMessageAt,
					lastActivity: a.lastActivity,
					archived: true,
				}));

			const ccRefs: ConversationRef[] = cc.allSessions
				.filter(s => s.archived)
				.map(s => ({
					id: s.id,
					type: 'claude-code' as const,
					title: s.title,
					status: s.status,
					lastMessageAt: s.lastMessageAt,
					lastActivity: s.lastActivity,
					archived: true,
				}));

			return [...agentRefs, ...ccRefs].sort((a, b) => b.lastMessageAt - a.lastMessageAt);
		},

		filtered(query: string): { active: ConversationRef[]; archived: ConversationRef[] } {
			const q = query.toLowerCase().trim();
			if (!q) return { active: this.active, archived: this.archived };
			const match = (c: ConversationRef) =>
				c.title.toLowerCase().includes(q) || (c.lastActivity ?? '').toLowerCase().includes(q);
			return {
				active: this.active.filter(match),
				archived: this.archived.filter(match),
			};
		},

		archive(ref: Pick<ConversationRef, 'id' | 'type'>): void {
			if (ref.type === 'agent') agents.archiveAgent(ref.id);
			else cc.archiveSession(ref.id);
		},

		unarchive(ref: Pick<ConversationRef, 'id' | 'type'>): void {
			if (ref.type === 'agent') agents.unarchiveAgent(ref.id);
			else cc.unarchiveSession(ref.id);
		},

		rename(ref: Pick<ConversationRef, 'id' | 'type'>, title: string): void {
			if (ref.type === 'agent') agents.renameAgent(ref.id, title);
			else cc.renameSession(ref.id, title);
		},

		remove(ref: Pick<ConversationRef, 'id' | 'type'>): void {
			if (ref.type === 'agent') agents.removeAgent(ref.id);
			else cc.removeSession(ref.id);
		},
	};
}
