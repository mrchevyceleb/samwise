/** Chat store using Svelte 5 runes - flat message model with direct Sam responses */

import type { AeMessage } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';
import { getWorkerStore } from './worker.svelte';

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
          messages = result;
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
          // Viewer mode: save message to Supabase directly, response comes via realtime
          const sendResult = await safeInvoke('supabase_send_message', {
            message: { role: 'user', content, conversation_id: '00000000-0000-0000-0000-000000000001', needs_response: true },
          });
          if (!sendResult) {
            console.warn('[chat] Failed to send message to Supabase');
            sendingMessage = false;
            return;
          }
          // Optimistically show the user's message locally
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
        const result = await safeInvoke<ChatResponseResult>('chat_respond', {
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
          content: `Sam couldn't respond: ${e instanceof Error ? e.message : String(e)}. Is Claude Code installed and authenticated?`,
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
