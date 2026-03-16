/** Claude Code CLI integration store using Svelte 5 runes */

import { listen } from '@tauri-apps/api/event';
import { spawnClaudeCode, closeClaudeCode, writeClaudeCode } from '$lib/utils/tauri';

// ── Types ────────────────────────────────────────────────────────────

let messageSeq = 0;

export interface ClaudeCodeMessage {
	type: 'system' | 'assistant' | 'user' | 'tool_result' | 'result' | 'rate_limit_event' | 'error' | 'stderr';
	raw: any;
	timestamp: number;
	seq: number;
}

export interface ContextUsage {
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheCreationTokens: number;
	contextWindow: number;
}

export interface ClaudeCodeSession {
	id: string;
	status: 'ready' | 'running' | 'error';
	messages: ClaudeCodeMessage[];
	model?: string;
	selectedModel?: string;
	processAlive: boolean;
	claudeSessionId?: string;
	pendingModelRestart: boolean;
	deliberateKill?: boolean;
	totalCost?: number;
	contextUsage?: ContextUsage;
	slashCommands: string[];
	error?: string;
	cwd: string;
	// Sidebar metadata
	title: string;
	lastMessageAt: number;
	lastActivity: string;
	archived: boolean;
}

export interface ImageAttachment {
	mediaType: string;
	base64: string;
}

// ── State ────────────────────────────────────────────────────────────

let sessions = $state<Map<string, ClaudeCodeSession>>(new Map());
let activeSessionId = $state<string | null>(null);

function getSession(id: string): ClaudeCodeSession | undefined {
	return sessions.get(id);
}

function updateSession(id: string, updater: (session: ClaudeCodeSession) => Partial<ClaudeCodeSession>) {
	const session = sessions.get(id);
	if (!session) return;
	const updates = updater(session);
	const next = new Map(sessions);
	next.set(id, { ...session, ...updates });
	sessions = next;
}

// ── Stream Parsing ───────────────────────────────────────────────────

function parseContextWindow(model: string): number {
	const match = model.match(/\[(\d+)([km])\]/i);
	if (match) {
		const num = parseInt(match[1]);
		const unit = match[2].toLowerCase();
		return unit === 'm' ? num * 1_000_000 : num * 1_000;
	}
	if (model.includes('opus')) return 1_000_000;
	if (model.includes('sonnet')) return 200_000;
	if (model.includes('haiku')) return 200_000;
	return 200_000;
}

function extractUsage(parsed: any): Partial<ContextUsage> | null {
	const usage = parsed.usage || parsed.message?.usage;
	if (!usage) return null;
	return {
		inputTokens: usage.input_tokens || 0,
		outputTokens: usage.output_tokens || 0,
		cacheReadTokens: usage.cache_read_input_tokens || 0,
		cacheCreationTokens: usage.cache_creation_input_tokens || 0,
	};
}

const DEFAULT_USAGE: ContextUsage = {
	inputTokens: 0,
	outputTokens: 0,
	cacheReadTokens: 0,
	cacheCreationTokens: 0,
	contextWindow: 200_000,
};

// ── Event Listeners ──────────────────────────────────────────────────

let listenersInitialized = false;

// Streaming state per session (outside reactive state to avoid churn)
const streamingState = new Map<string, {
	text: string;
	toolUses: any[];
	seq: number;
	currentBlockType: string;
}>();

function initListeners() {
	if (listenersInitialized) return;
	listenersInitialized = true;

	listen<{ id: string; data: string }>('claude-code-output', (event) => {
		const { id, data } = event.payload;

		if (data.startsWith('[stderr] ')) {
			updateSession(id, (s) => ({
				messages: [...s.messages, {
					type: 'stderr' as const,
					raw: data.slice(9),
					timestamp: Date.now(),
					seq: messageSeq++,
				}],
			}));
			return;
		}

		try {
			const parsed = JSON.parse(data);

			// Unwrap stream_event wrapper
			const streamEvent = parsed.type === 'stream_event' ? parsed.event : null;

			// message_start: input token usage
			if (streamEvent?.type === 'message_start') {
				const msgUsage = streamEvent.message?.usage;
				if (msgUsage) {
					updateSession(id, (s) => ({
						contextUsage: {
							...(s.contextUsage || { ...DEFAULT_USAGE }),
							inputTokens: msgUsage.input_tokens || 0,
							cacheReadTokens: msgUsage.cache_read_input_tokens || 0,
							cacheCreationTokens: msgUsage.cache_creation_input_tokens || 0,
						},
					}));
				}
				return;
			}

			// message_delta: output token usage
			if (streamEvent?.type === 'message_delta') {
				const deltaUsage = streamEvent.usage;
				if (deltaUsage) {
					updateSession(id, (s) => ({
						contextUsage: {
							...(s.contextUsage || { ...DEFAULT_USAGE }),
							outputTokens: deltaUsage.output_tokens || 0,
						},
					}));
				}
				return;
			}

			// content_block_start: begin accumulating text or tool_use
			if (streamEvent?.type === 'content_block_start') {
				if (!streamingState.has(id)) {
					streamingState.set(id, { text: '', toolUses: [], seq: messageSeq++, currentBlockType: 'text' });
				}
				const state = streamingState.get(id)!;
				state.currentBlockType = streamEvent.content_block?.type || 'text';
				if (streamEvent.content_block?.type === 'tool_use') {
					state.toolUses.push({
						type: 'tool_use',
						id: streamEvent.content_block.id,
						name: streamEvent.content_block.name,
						input: {},
					});
				}
				return;
			}

			// content_block_delta: accumulate text/tool input
			if (streamEvent?.type === 'content_block_delta') {
				const state = streamingState.get(id);
				if (state) {
					const delta = streamEvent.delta;
					if (delta?.type === 'text_delta' && delta.text) {
						state.text += delta.text;
					} else if (delta?.type === 'input_json_delta' && delta.partial_json && state.currentBlockType === 'tool_use') {
						const lastTool = state.toolUses[state.toolUses.length - 1];
						if (lastTool) {
							lastTool._rawInput = (lastTool._rawInput || '') + delta.partial_json;
						}
					}
					// Update the live streaming message
					updateSession(id, (s) => {
						const streamMsg: ClaudeCodeMessage = {
							type: 'assistant',
							raw: {
								type: 'assistant',
								message: {
									role: 'assistant',
									content: [
										...(state.text ? [{ type: 'text', text: state.text }] : []),
										...state.toolUses.map((t: any) => ({ ...t })),
									],
								},
							},
							timestamp: Date.now(),
							seq: state.seq,
						};
						const existingIdx = s.messages.findIndex((m) => m.seq === state.seq);
						const messages = [...s.messages];
						if (existingIdx >= 0) {
							messages[existingIdx] = streamMsg;
						} else {
							messages.push(streamMsg);
						}
						return { messages };
					});
				}
				return;
			}

			if (streamEvent?.type === 'content_block_stop') {
				return;
			}

			// ── Final message handling ──
			updateSession(id, (s) => {
				const updates: Partial<ClaudeCodeSession> = {};

				// Init message
				if (parsed.type === 'system' && parsed.subtype === 'init') {
					updates.model = parsed.model;
					if (parsed.session_id) {
						updates.claudeSessionId = parsed.session_id;
					}
					if (Array.isArray(parsed.slash_commands)) {
						updates.slashCommands = parsed.slash_commands;
					}
					if (parsed.model) {
						updates.contextUsage = {
							...(s.contextUsage || { ...DEFAULT_USAGE }),
							contextWindow: parseContextWindow(parsed.model),
						};
					}
				}

				// Final assistant message replaces streaming message
				if (parsed.type === 'assistant') {
					const sState = streamingState.get(id);
					if (sState) {
						const existingIdx = s.messages.findIndex((m) => m.seq === sState.seq);
						const messages = [...s.messages];
						const finalMsg: ClaudeCodeMessage = {
							type: 'assistant',
							raw: parsed,
							timestamp: Date.now(),
							seq: sState.seq,
						};
						if (existingIdx >= 0) {
							messages[existingIdx] = finalMsg;
						} else {
							messages.push(finalMsg);
						}
						updates.messages = messages;
						streamingState.delete(id);
					} else {
						updates.messages = [...s.messages, {
							type: 'assistant' as const,
							raw: parsed,
							timestamp: Date.now(),
							seq: messageSeq++,
						}];
					}
					const usage = extractUsage(parsed);
					if (usage) {
						updates.contextUsage = {
							...(s.contextUsage || { ...DEFAULT_USAGE }),
							...usage,
						};
					}
				}

				// Result with cost
				if (parsed.type === 'result') {
					updates.totalCost = (s.totalCost || 0) + (parsed.total_cost_usd || 0);
					updates.status = 'ready';
					updates.lastMessageAt = Date.now();
					// Set lastActivity from result text
					const resultText = (parsed.result || '').replace(/\s+/g, ' ').trim();
					if (resultText) {
						updates.lastActivity = resultText.slice(0, 60) + (resultText.length > 60 ? '...' : '');
					}
					streamingState.delete(id);
					const resultUsage = parsed.usage;
					if (resultUsage) {
						updates.contextUsage = {
							...(s.contextUsage || { ...DEFAULT_USAGE }),
							inputTokens: resultUsage.input_tokens || 0,
							outputTokens: resultUsage.output_tokens || 0,
							cacheReadTokens: resultUsage.cache_read_input_tokens || 0,
							cacheCreationTokens: resultUsage.cache_creation_input_tokens || 0,
						};
					}
				}

				// Append other event types as messages
				if (!updates.messages && parsed.type !== 'assistant' && parsed.type !== 'stream_event') {
					const msg: ClaudeCodeMessage = {
						type: parsed.type || 'error',
						raw: parsed,
						timestamp: Date.now(),
						seq: messageSeq++,
					};
					updates.messages = [...s.messages, msg];
				}

				return updates;
			});
		} catch {
			updateSession(id, (s) => ({
				messages: [...s.messages, {
					type: 'stderr' as const,
					raw: data,
					timestamp: Date.now(),
					seq: messageSeq++,
				}],
			}));
		}
	});

	listen<{ id: string; exit_code: number | null }>('claude-code-closed', (event) => {
		const { id, exit_code } = event.payload;
		const orphanedStream = streamingState.get(id);
		if (orphanedStream) {
			streamingState.delete(id);
		}
		updateSession(id, (s) => {
			const updates: Partial<ClaudeCodeSession> = { processAlive: false };
			// Stamp orphaned streaming message so isStreaming becomes false
			if (orphanedStream) {
				const idx = s.messages.findIndex((m) => m.seq === orphanedStream.seq);
				if (idx >= 0) {
					const messages = [...s.messages];
					const msg = { ...messages[idx] };
					msg.raw = { ...msg.raw, message: { ...msg.raw.message, stop_reason: 'interrupted' } };
					messages[idx] = msg;
					updates.messages = messages;
				}
			}
			if (s.status === 'running') {
				if (s.deliberateKill) {
					updates.status = 'ready';
					updates.deliberateKill = false;
				} else if (exit_code !== null && exit_code !== 0) {
					updates.status = 'error';
					updates.error = `Claude process exited with code ${exit_code}`;
				} else {
					updates.status = 'ready';
				}
			}
			return updates;
		});
	});
}

// ── Process Management ───────────────────────────────────────────────

async function ensureProcess(session: ClaudeCodeSession): Promise<void> {
	if (session.processAlive) return;
	const extraArgs: string[] = [];
	if (session.selectedModel) {
		extraArgs.push('--model', session.selectedModel);
	}
	if (session.claudeSessionId) {
		extraArgs.push('--resume', session.claudeSessionId);
	}
	await spawnClaudeCode(session.id, session.cwd, extraArgs);
	updateSession(session.id, () => ({ processAlive: true }));
}

// ── Public Store ─────────────────────────────────────────────────────

export function getClaudeCodeStore() {
	initListeners();

	return {
		get sessions() { return sessions; },
		get activeSessionId() { return activeSessionId; },
		set activeSessionId(id: string | null) { activeSessionId = id; },

		getSession(id: string): ClaudeCodeSession | undefined {
			return getSession(id);
		},

		getActiveSession(): ClaudeCodeSession | undefined {
			if (!activeSessionId) return undefined;
			return sessions.get(activeSessionId);
		},

		launchSession(cwd: string): string {
			const id = `cc-${Date.now()}-${Math.random().toString(36).slice(2)}`;
			const session: ClaudeCodeSession = {
				id,
				status: 'ready',
				messages: [],
				slashCommands: [],
				cwd,
				processAlive: false,
				pendingModelRestart: false,
				title: 'Claude Code',
				lastMessageAt: Date.now(),
				lastActivity: '',
				archived: false,
			};
			const next = new Map(sessions);
			next.set(id, session);
			sessions = next;
			activeSessionId = id;
			return id;
		},

		async sendMessage(id: string, message: string, images?: ImageAttachment[]): Promise<void> {
			const session = sessions.get(id);
			if (!session) throw new Error('Session not found');
			if (session.status === 'running') throw new Error('Already running');

			// Auto-title from first user message
			const isFirstMessage = session.messages.filter(m => m.type === 'user').length === 0;
			const titleUpdate = isFirstMessage && session.title === 'Claude Code'
				? { title: message.replace(/\s+/g, ' ').trim().slice(0, 40) + (message.length > 40 ? '...' : '') }
				: {};

			updateSession(id, (s) => ({
				...titleUpdate,
				lastMessageAt: Date.now(),
				messages: [...s.messages, {
					type: 'user' as const,
					raw: { text: message, images: images?.map((img) => ({ base64: img.base64, mediaType: img.mediaType })) },
					timestamp: Date.now(),
					seq: messageSeq++,
				}],
				status: 'running',
				error: undefined,
			}));

			try {
				let current = sessions.get(id);
				if (!current) throw new Error('Session lost');

				// Restart process if model changed
				if (current.pendingModelRestart && current.processAlive) {
					updateSession(id, () => ({ deliberateKill: true }));
					await closeClaudeCode(id);
					updateSession(id, () => ({ processAlive: false, pendingModelRestart: false, deliberateKill: false, claudeSessionId: undefined }));
					current = sessions.get(id);
					if (!current) throw new Error('Session lost during model restart');
				}

				await ensureProcess(current);

				// Build content
				let content: any;
				if (images && images.length > 0) {
					content = [];
					for (const img of images) {
						content.push({
							type: 'image',
							source: {
								type: 'base64',
								media_type: img.mediaType,
								data: img.base64,
							},
						});
					}
					if (message) {
						content.push({ type: 'text', text: message });
					}
				} else {
					content = message;
				}

				const inputMsg = JSON.stringify({
					type: 'user',
					message: { role: 'user', content },
				});
				await writeClaudeCode(id, inputMsg);
			} catch (err) {
				updateSession(id, () => ({ status: 'error', error: String(err) }));
			}
		},

		async setModel(id: string, model: string): Promise<void> {
			const session = sessions.get(id);
			updateSession(id, () => ({ selectedModel: model || undefined }));
			if (session?.processAlive) {
				if (session.status === 'running') {
					updateSession(id, () => ({ pendingModelRestart: true }));
				} else {
					updateSession(id, () => ({ deliberateKill: true }));
					await closeClaudeCode(id).catch(() => {});
					updateSession(id, () => ({ processAlive: false, claudeSessionId: undefined }));
				}
			}
		},

		async stopSession(id: string): Promise<void> {
			updateSession(id, () => ({ deliberateKill: true }));
			await closeClaudeCode(id);
			updateSession(id, () => ({ status: 'ready', processAlive: false }));
		},

		removeSession(id: string): void {
			streamingState.delete(id);
			const session = sessions.get(id);
			if (session?.processAlive) {
				closeClaudeCode(id).catch(() => {});
			}
			const next = new Map(sessions);
			next.delete(id);
			sessions = next;
			if (activeSessionId === id) {
				activeSessionId = null;
			}
		},

		clearMessages(id: string): void {
			updateSession(id, () => ({ messages: [] }));
		},

		get allSessions(): ClaudeCodeSession[] {
			return Array.from(sessions.values());
		},

		archiveSession(id: string): void {
			updateSession(id, () => ({ archived: true }));
		},

		unarchiveSession(id: string): void {
			updateSession(id, () => ({ archived: false }));
		},

		renameSession(id: string, title: string): void {
			updateSession(id, () => ({ title }));
		},

		updateSessionMeta(id: string, updates: Partial<Pick<ClaudeCodeSession, 'title' | 'lastMessageAt' | 'lastActivity'>>): void {
			updateSession(id, () => updates);
		},
	};
}
