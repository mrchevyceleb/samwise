import { error } from '@sveltejs/kit';
import { createClient, type SupabaseClient } from '@supabase/supabase-js';
import { env } from '$env/dynamic/private';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';

let cachedClient: SupabaseClient | null = null;
let cachedUrl = '';
let cachedKey = '';

export function getSupabaseAdmin(): SupabaseClient {
  const url = env.SB_URL || env.SUPABASE_URL || PUBLIC_SUPABASE_URL;
  const key = env.SB_SERVICE_ROLE_KEY || env.SUPABASE_SERVICE_ROLE_KEY;

  if (!url || !key) {
    throw error(500, 'Supabase admin credentials are not configured');
  }

  if (!cachedClient || cachedUrl !== url || cachedKey !== key) {
    cachedUrl = url;
    cachedKey = key;
    cachedClient = createClient(url, key, {
      auth: { persistSession: false },
      realtime: { params: { eventsPerSecond: 10 } }
    });
  }

  return cachedClient;
}
