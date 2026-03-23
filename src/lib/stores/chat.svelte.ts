/** Chat store using Svelte 5 runes - flat message model with direct Sam responses */

import type { AeMessage } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';
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

interface ChatResponseResult {
  content: string;
  message_id: string | null;
  created_tasks: Array<{ id: string; title: string; task_type: string }>;
}

export function getChatStore() {
  return {
    get messages() { return messages; },
    get loading() { return loading; },
    get sendingMessage() { return sendingMessage; },
    get waitingForSam() { return waitingForSam; },

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
          // Preserve any optimistic/local messages (pending-, error-, timeout-, local-)
          // that haven't been confirmed by the server yet. Without this, a fetch that
          // races with sendMessage() will blow away the user's just-sent message.
          if (sendingMessage || waitingForSam) {
            const localMsgs = messages.filter(m =>
              m.id.startsWith('pending-') || m.id.startsWith('error-') ||
              m.id.startsWith('timeout-') || m.id.startsWith('local-')
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
      try {
        if (getWorkerStore().isViewer) {
          // Viewer mode: show message immediately, then save to Supabase
          const optimisticMsg: AeMessage = {
            id: `pending-${Date.now()}`,
            conversation_id: '00000000-0000-0000-0000-000000000001',
            role: 'user',
            content,
            task_id: null,
            attachments: null,
            created_at: new Date().toISOString(),
          };
          messages = [...messages, optimisticMsg];
          await strictInvoke('supabase_send_message', {
            message: { role: 'user', content, conversation_id: '00000000-0000-0000-0000-000000000001', needs_response: true },
          });
          waitingForSam = true;
          sendingMessage = false;
          // Timeout after 120s so viewer is never permanently stuck
          if (waitingTimeout) clearTimeout(waitingTimeout);
          waitingTimeout = setTimeout(() => {
            if (waitingForSam) {
              waitingForSam = false;
              // Show timeout message so user knows what happened
              const timeoutMsg: AeMessage = {
                id: `timeout-${Date.now()}`,
                conversation_id: '00000000-0000-0000-0000-000000000001',
                role: 'system',
                content: 'Sam didn\'t respond in time. The master worker may be offline or busy. Try again?',
                task_id: null,
                attachments: null,
                created_at: new Date().toISOString(),
              };
              messages = [...messages, timeoutMsg];
              console.warn('[chat] Timed out waiting for Sam response');
            }
          }, 120_000);
          return;
        }

        // Master mode: optimistically show the user's message immediately
        // (the backend saves it to Supabase, but realtime may be slow)
        const optimisticUserMsg: AeMessage = {
          id: `pending-${Date.now()}`,
          conversation_id: '00000000-0000-0000-0000-000000000001',
          role: 'user',
          content,
          task_id: null,
          attachments: null,
          created_at: new Date().toISOString(),
        };
        messages = [...messages, optimisticUserMsg];

        // Master mode: call Claude Code CLI locally
        // Use strictInvoke (not safeInvoke) so errors propagate to the catch block
        // and the user sees WHY Sam couldn't respond instead of silent failure.
        const result = await strictInvoke<ChatResponseResult>('chat_respond', {
          userMessage: content,
        });

        if (result) {
          // Push agent response directly so it shows even without realtime.
          // message_id can be null if the Supabase save failed, but we still
          // have the content - always show it rather than silently drop it.
          if (result.content) {
            const agentMsg: AeMessage = {
              id: result.message_id ?? `local-${Date.now()}`,
              conversation_id: '00000000-0000-0000-0000-000000000001',
              role: 'agent',
              content: result.content,
              task_id: null,
              attachments: null,
              created_at: new Date().toISOString(),
            };
            if (!messages.find(m => m.id === agentMsg.id)) {
              messages = [...messages, agentMsg];
            }
          }

          // If tasks were created, refresh the task board
          if (result.created_tasks && result.created_tasks.length > 0) {
            const { getTaskStore } = await import('./tasks.svelte');
            getTaskStore().fetchTasks();
          }
        }
      } catch (e) {
        console.warn('[chat] send message failed:', e);
        waitingForSam = false;
        if (waitingTimeout) { clearTimeout(waitingTimeout); waitingTimeout = null; }
        // Surface the error to the user so they know Sam couldn't respond
        const errorMsg: AeMessage = {
          id: `error-${Date.now()}`,
          conversation_id: '00000000-0000-0000-0000-000000000001',
          role: 'system',
          content: `Sam couldn't respond right now. Check that the worker is running and Claude Code is installed. (${e instanceof Error ? e.message.split('\n')[0] : 'Unknown error'})`,
          task_id: null,
          attachments: null,
          created_at: new Date().toISOString(),
        };
        messages = [...messages, errorMsg];
      } finally {
        sendingMessage = false;
      }
    },

    /** Apply a realtime message insert (deduplicates by ID, replaces optimistic messages) */
    applyRealtimeMessage(msg: AeMessage) {
      if (msg.role === 'user') {
        // Replace optimistic pending message with real one from Supabase
        const hasPending = messages.some(m => m.id.startsWith('pending-') && m.role === 'user' && m.content === msg.content);
        if (hasPending) {
          messages = messages.filter(m => !(m.id.startsWith('pending-') && m.role === 'user' && m.content === msg.content));
        }
      }
      if (!messages.find(m => m.id === msg.id)) {
        messages = [...messages, msg];
      }
      // Clear waiting state when Sam responds
      if (msg.role === 'agent' && waitingForSam) {
        waitingForSam = false;
        if (waitingTimeout) { clearTimeout(waitingTimeout); waitingTimeout = null; }
      }
    },
  };
}
