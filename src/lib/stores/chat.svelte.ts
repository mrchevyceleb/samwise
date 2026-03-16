/** Chat store using Svelte 5 runes - flat message model (no conversations) */

import type { AeMessage } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

let messages = $state<AeMessage[]>([]);
let loading = $state(false);
let sendingMessage = $state(false);

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
        const result = await safeInvoke<AeMessage[]>('supabase_send_message', {
          message: { role: 'user', content },
        });
        if (result && Array.isArray(result) && result.length > 0) {
          const newMsg = result[0] as AeMessage;
          if (!messages.find(m => m.id === newMsg.id)) {
            messages = [...messages, newMsg];
          }
        }
      } catch (e) {
        console.warn('[chat] send message failed:', e);
      } finally {
        sendingMessage = false;
      }
    },

    /** Apply a realtime message insert (deduplicates) */
    applyRealtimeMessage(msg: AeMessage) {
      if (!messages.find(m => m.id === msg.id)) {
        messages = [...messages, msg];
      }
    },
  };
}
