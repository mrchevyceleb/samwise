import { json, error } from '@sveltejs/kit';
import { assertAdminSession } from '$lib/server/admin-auth';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ cookies, url }) => {
  assertAdminSession(cookies);
  const supabase = getSupabaseAdmin();
  const cronId = url.searchParams.get('cron_id') || url.searchParams.get('cronId');

  let query = supabase
    .from('ae_cron_runs')
    .select('*')
    .order('started_at', { ascending: false })
    .limit(200);

  if (cronId) {
    query = query.eq('cron_id', cronId);
  }

  const { data, error: dbError } = await query;
  if (dbError) throw error(500, dbError.message);
  return json(data ?? []);
};
