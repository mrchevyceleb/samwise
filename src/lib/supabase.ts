/** Supabase client for realtime subscriptions */
import { createClient, type RealtimeChannel, type SupabaseClient } from '@supabase/supabase-js';

let client: SupabaseClient | null = null;
let currentUrl = '';
let currentKey = '';

export function getSupabaseClient(url: string, anonKey: string): SupabaseClient {
  if (!client || url !== currentUrl || anonKey !== currentKey) {
    client = createClient(url, anonKey, {
      realtime: { params: { eventsPerSecond: 10 } },
    });
    currentUrl = url;
    currentKey = anonKey;
  }
  return client;
}

export function subscribeToTable(
  supabaseUrl: string,
  supabaseKey: string,
  table: string,
  callback: (payload: { eventType: string; new: unknown; old: unknown }) => void,
): RealtimeChannel {
  const sb = getSupabaseClient(supabaseUrl, supabaseKey);
  return sb
    .channel(`${table}-changes`)
    .on(
      'postgres_changes',
      { event: '*', schema: 'public', table },
      (payload) => callback(payload as any),
    )
    .subscribe();
}

export function subscribeToMessages(
  supabaseUrl: string,
  supabaseKey: string,
  callback: (payload: { eventType: string; new: unknown }) => void,
): RealtimeChannel {
  const sb = getSupabaseClient(supabaseUrl, supabaseKey);
  return sb
    .channel('messages-all')
    .on(
      'postgres_changes',
      {
        event: 'INSERT',
        schema: 'public',
        table: 'ae_messages',
      },
      (payload) => callback(payload as any),
    )
    .subscribe();
}

export function subscribeToComments(
  supabaseUrl: string,
  supabaseKey: string,
  callback: (payload: { eventType: string; new: unknown }) => void,
): RealtimeChannel {
  const sb = getSupabaseClient(supabaseUrl, supabaseKey);
  return sb
    .channel('comments-all')
    .on(
      'postgres_changes',
      {
        event: 'INSERT',
        schema: 'public',
        table: 'ae_comments',
      },
      (payload) => callback(payload as any),
    )
    .subscribe();
}

export function unsubscribe(channel: RealtimeChannel): void {
  channel.unsubscribe();
}
