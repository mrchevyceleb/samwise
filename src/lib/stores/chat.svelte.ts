/** Chat store using Svelte 5 runes - persistent Claude Code session for Sam */

import type { AeMessage } from '$lib/types';
import { safeInvoke, spawnClaudeCode, writeClaudeCode, closeClaudeCode } from '$lib/utils/tauri';
import { getWorkerStore } from './worker.svelte';

/** Like safeInvoke but re-throws errors so callers can handle them */
async function strictInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
}

let messages = $state<AeMessage[]>([]);
let loading = $state(false);
let sendingMessage = $state(false);
let waitingForSam = $state(false);
let waitingTimeout: ReturnType<typeof setTimeout> | null = null;

// Persistent session state
const CHAT_SESSION_ID = 'sam-chat';
let sessionAlive = $state(false);
let sessionSpawning = false;
let streamingText = '';
let lastUserMessage = '';
let initPhase = false; // true while processing the system prompt init
let listenersInitialized = false;
let resultUnlisten: (() => void) | null = null;
let closedUnlisten: (() => void) | null = null;
let initResolve: ((ok: boolean) => void) | null = null;
// Hard timeout for master mode: if no result event within 120s, give up
let responseTimeout: ReturnType<typeof setTimeout> | null = null;

interface ChatResponseResult {
  content: string;
  message_id: string | null;
  created_tasks: Array<{ id: string; title: string; task_type: string }>;
}

interface FastPathResult {
  handled: boolean;
  response: string | null;
  message_id: string | null;
}

const CONVO_ID = '00000000-0000-0000-0000-000000000001';

function makeMsg(overrides: Partial<AeMessage> & { id: string; role: string; content: string }): AeMessage {
  return {
    conversation_id: CONVO_ID,
    task_id: null,
    attachments: null,
    created_at: new Date().toISOString(),
    ...overrides,
  } as AeMessage;
}

function clearResponseTimeout() {
  if (responseTimeout) { clearTimeout(responseTimeout); responseTimeout = null; }
}

/** Extract text from a stream-json assistant message */
function extractTextFromAssistant(parsed: any): string {
  const content = parsed?.message?.content;
  if (!content) return '';
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content
      .filter((b: any) => b.type === 'text')
      .map((b: any) => b.text || '')
      .join('');
  }
  return '';
}

/** Initialize event listeners for Claude Code streaming output */
async function initListeners() {
  if (listenersInitialized) return;
  listenersInitialized = true;

  const { listen } = await import('@tauri-apps/api/event');

  resultUnlisten = await listen<{ id: string; data: string }>('claude-code-output', (event) => {
    const { id, data } = event.payload;
    if (id !== CHAT_SESSION_ID) return;
    if (data.startsWith('[stderr]')) return;

    let parsed: any;
    try { parsed = JSON.parse(data); } catch { return; }

    // During init phase, ignore streaming output (it's Sam's init response, not shown)
    if (initPhase) {
      if (parsed.type === 'result') {
        initPhase = false;
        if (initResolve) { initResolve(true); initResolve = null; }
      }
      return;
    }

    // Handle streaming deltas - reset timeout on each delta (session is alive)
    const streamEvent = parsed.type === 'stream_event' ? parsed.event : null;
    if (streamEvent?.type === 'content_block_delta') {
      const delta = streamEvent.delta;
      if (delta?.type === 'text_delta' && delta.text) {
        streamingText += delta.text;
        updateStreamingMessage();
        // Reset timeout since we're receiving data
        startResponseTimeout();
      }
      return;
    }

    // Final assistant message (replaces streaming)
    if (parsed.type === 'assistant') {
      const text = extractTextFromAssistant(parsed);
      if (text) {
        streamingText = text;
        updateStreamingMessage();
      }
      return;
    }

    // Result event = response complete
    if (parsed.type === 'result') {
      clearResponseTimeout();
      const resultText = parsed.result_text || streamingText;
      finalizeResponse(resultText);
      return;
    }
  });

  closedUnlisten = await listen<{ id: string; exit_code: number | null }>('claude-code-closed', (event) => {
    if (event.payload.id !== CHAT_SESSION_ID) return;
    console.warn('[chat] Sam session closed, exit code:', event.payload.exit_code);
    sessionAlive = false;
    clearResponseTimeout();

    // If init was in progress, signal failure
    if (initPhase) {
      initPhase = false;
      if (initResolve) { initResolve(false); initResolve = null; }
    }

    // If mid-response, finalize whatever we have
    if (sendingMessage && streamingText) {
      finalizeResponse(streamingText);
    } else if (sendingMessage) {
      sendingMessage = false;
      messages = [...messages, makeMsg({
        id: `error-${Date.now()}`,
        role: 'system',
        content: 'Sam\'s session ended unexpectedly. Send another message and he\'ll reconnect.',
      })];
    }
  });
}

/** Start/reset the 120s response timeout for master mode */
function startResponseTimeout() {
  clearResponseTimeout();
  responseTimeout = setTimeout(() => {
    if (!sendingMessage) return;
    // Timed out. If we have partial text, show it. Otherwise show error.
    if (streamingText) {
      finalizeResponse(streamingText);
    } else {
      sendingMessage = false;
      messages = [...messages, makeMsg({
        id: `timeout-${Date.now()}`,
        role: 'system',
        content: 'Sam took too long to respond (timed out after 2 minutes). Try again.',
      })];
    }
  }, 120_000);
}

/** Update or create the streaming message bubble */
function updateStreamingMessage() {
  const streamId = 'streaming-sam';
  const existing = messages.findIndex(m => m.id === streamId);
  const msg = makeMsg({ id: streamId, role: 'agent', content: streamingText });
  if (existing >= 0) {
    messages = [...messages.slice(0, existing), msg, ...messages.slice(existing + 1)];
  } else {
    messages = [...messages, msg];
  }
}

/** Finalize a completed response: process tasks, save to Supabase, update UI */
async function finalizeResponse(text: string) {
  sendingMessage = false;
  clearResponseTimeout();
  const streamIdx = messages.findIndex(m => m.id === 'streaming-sam');

  try {
    const result = await strictInvoke<ChatResponseResult>('chat_process_response', {
      responseText: text,
      userMessage: lastUserMessage,
    });

    const finalMsg = makeMsg({
      id: result.message_id ?? `local-${Date.now()}`,
      role: 'agent',
      content: result.content,
    });

    if (streamIdx >= 0) {
      messages = [...messages.slice(0, streamIdx), finalMsg, ...messages.slice(streamIdx + 1)];
    } else if (!messages.find(m => m.id === finalMsg.id)) {
      messages = [...messages, finalMsg];
    }

    if (result.created_tasks && result.created_tasks.length > 0) {
      const { getTaskStore } = await import('./tasks.svelte');
      getTaskStore().fetchTasks();
    }
  } catch (e) {
    console.warn('[chat] Failed to process response:', e);
    if (streamIdx >= 0) {
      const fallback = makeMsg({ id: `local-${Date.now()}`, role: 'agent', content: text });
      messages = [...messages.slice(0, streamIdx), fallback, ...messages.slice(streamIdx + 1)];
    }
  }

  streamingText = '';
}

/** Ensure a persistent Claude Code session is alive */
async function ensureSession(): Promise<void> {
  if (sessionAlive) return;
  if (sessionSpawning) {
    // Wait for the existing spawn to finish (max 60s to prevent infinite poll)
    await new Promise<void>((resolve, reject) => {
      let elapsed = 0;
      const check = setInterval(() => {
        elapsed += 100;
        if (!sessionSpawning) { clearInterval(check); resolve(); }
        else if (elapsed > 60_000) { clearInterval(check); reject(new Error('Session spawn timed out')); }
      }, 100);
    });
    if (sessionAlive) return;
  }

  sessionSpawning = true;
  try {
    await initListeners();

    // Build system prompt with current board context
    const systemPrompt = await strictInvoke<string>('chat_build_system_prompt');

    // Spawn persistent session
    await spawnClaudeCode(CHAT_SESSION_ID, '.', []);
    sessionAlive = true;
    initPhase = true;

    // Send system prompt as first message to set Sam's personality/context
    const initMsg = JSON.stringify({
      type: 'user',
      message: { role: 'user', content: systemPrompt },
    });
    await writeClaudeCode(CHAT_SESSION_ID, initMsg);

    // Wait for the init result (max 30s) so the session is primed
    const initOk = await new Promise<boolean>((resolve) => {
      initResolve = resolve;
      setTimeout(() => {
        if (initPhase) {
          initPhase = false;
          initResolve = null;
        }
        resolve(sessionAlive); // resolve with current session state
      }, 30_000);
    });

    // If session died during init, throw so caller sees the error
    if (!initOk || !sessionAlive) {
      throw new Error('Sam\'s session failed to start. Check that Claude Code is installed and authenticated.');
    }
  } catch (e) {
    console.error('[chat] Failed to spawn session:', e);
    sessionAlive = false;
    throw e;
  } finally {
    sessionSpawning = false;
  }
}

export function getChatStore() {
  return {
    get messages() { return messages; },
    get loading() { return loading; },
    get sendingMessage() { return sendingMessage; },
    get waitingForSam() { return waitingForSam; },
    get sessionAlive() { return sessionAlive; },

    get sortedMessages(): AeMessage[] {
      return [...messages].sort(
        (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      );
    },

    async fetchMessages() {
      loading = true;
      try {
        const result = await safeInvoke<AeMessage[]>('supabase_fetch_messages');
        if (result) {
          if (sendingMessage || waitingForSam) {
            const localMsgs = messages.filter(m =>
              m.id.startsWith('pending-') || m.id.startsWith('error-') ||
              m.id.startsWith('timeout-') || m.id.startsWith('local-') ||
              m.id === 'streaming-sam'
            );
            const serverIds = new Set(result.map(m => m.id));
            const keepLocal = localMsgs.filter(m => !serverIds.has(m.id));
            messages = [...result, ...keepLocal];
          } else {
            messages = result;
          }
        }
      } catch (e) {
        console.warn('[chat] fetch messages failed:', e);
      } finally {
        loading = false;
      }
    },

    async sendMessage(content: string) {
      sendingMessage = true;
      lastUserMessage = content;
      streamingText = '';

      try {
        if (getWorkerStore().isViewer) {
          // Viewer mode: save to Supabase, wait for master to respond via realtime
          messages = [...messages, makeMsg({
            id: `pending-${Date.now()}`, role: 'user', content,
          })];
          await strictInvoke('supabase_send_message', {
            message: { role: 'user', content, conversation_id: CONVO_ID, needs_response: true },
          });
          waitingForSam = true;
          sendingMessage = false;
          if (waitingTimeout) clearTimeout(waitingTimeout);
          waitingTimeout = setTimeout(() => {
            if (waitingForSam) {
              waitingForSam = false;
              messages = [...messages, makeMsg({
                id: `timeout-${Date.now()}`,
                role: 'system',
                content: 'Sam didn\'t respond in time. The master worker may be offline or busy. Try again?',
              })];
            }
          }, 120_000);
          return;
        }

        // Master mode: show user message immediately
        messages = [...messages, makeMsg({
          id: `pending-${Date.now()}`, role: 'user', content,
        })];

        // Check fast-paths first (confirmations, status queries) - no Claude needed
        const fastPath = await strictInvoke<FastPathResult>('chat_check_fast_path', {
          userMessage: content,
        });

        if (fastPath.handled && fastPath.response) {
          messages = [...messages, makeMsg({
            id: fastPath.message_id ?? `local-${Date.now()}`,
            role: 'agent',
            content: fastPath.response,
          })];
          sendingMessage = false;
          return;
        }

        // Needs Claude - ensure persistent session is alive, then write
        await ensureSession();

        // Verify session is still alive (could have died between ensure and write)
        if (!sessionAlive) {
          throw new Error('Session died before message could be sent');
        }

        const inputMsg = JSON.stringify({
          type: 'user',
          message: { role: 'user', content },
        });
        await writeClaudeCode(CHAT_SESSION_ID, inputMsg);

        // Start response timeout (120s hard limit)
        startResponseTimeout();

      } catch (e) {
        console.warn('[chat] send message failed:', e);
        sendingMessage = false;
        waitingForSam = false;
        clearResponseTimeout();
        if (waitingTimeout) { clearTimeout(waitingTimeout); waitingTimeout = null; }
        messages = [...messages, makeMsg({
          id: `error-${Date.now()}`,
          role: 'system',
          content: `Sam couldn't respond right now. (${e instanceof Error ? e.message.split('\n')[0] : 'Unknown error'})`,
        })];
      }
    },

    /** Apply a realtime message insert (deduplicates by ID, replaces optimistic messages) */
    applyRealtimeMessage(msg: AeMessage) {
      if (msg.role === 'user') {
        const hasPending = messages.some(m => m.id.startsWith('pending-') && m.role === 'user' && m.content === msg.content);
        if (hasPending) {
          messages = messages.filter(m => !(m.id.startsWith('pending-') && m.role === 'user' && m.content === msg.content));
        }
      }
      if (!messages.find(m => m.id === msg.id)) {
        messages = [...messages, msg];
      }
      if (msg.role === 'agent' && waitingForSam) {
        waitingForSam = false;
        if (waitingTimeout) { clearTimeout(waitingTimeout); waitingTimeout = null; }
      }
    },

    /** Kill the persistent session (e.g. on app close) */
    async destroySession() {
      sessionSpawning = false;
      initPhase = false;
      initResolve = null;
      clearResponseTimeout();
      if (sessionAlive) {
        try { await closeClaudeCode(CHAT_SESSION_ID); } catch { /* ignore */ }
        sessionAlive = false;
      }
      if (resultUnlisten) { resultUnlisten(); resultUnlisten = null; }
      if (closedUnlisten) { closedUnlisten(); closedUnlisten = null; }
      listenersInitialized = false;
    },
  };
}
