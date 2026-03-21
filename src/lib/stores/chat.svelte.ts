/** Chat store using Svelte 5 runes - flat message model with direct Sam responses */

import type { AeMessage } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

let messages = $state<AeMessage[]>([]);
let loading = $state(false);
let sendingMessage = $state(false);

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
        // Call direct chat command - saves user msg, gets Sam's response via Claude Code CLI
        // No optimistic message - realtime subscription delivers the user message fast,
        // and sendingMessage=true shows the "Thinking..." indicator.
        const result = await safeInvoke<ChatResponseResult>('chat_respond', {
          userMessage: content,
        });

        if (result) {
          // Push agent response directly so it shows even without realtime
          if (result.content && result.message_id) {
            const agentMsg: AeMessage = {
              id: result.message_id,
              conversation_id: 'default',
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
      } finally {
        sendingMessage = false;
      }
    },

    /** Apply a realtime message insert (deduplicates by ID) */
    applyRealtimeMessage(msg: AeMessage) {
      if (!messages.find(m => m.id === msg.id)) {
        messages = [...messages, msg];
      }
    },
  };
}
