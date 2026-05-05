import { json, error } from '@sveltejs/kit';
import { assertAdminSession } from '$lib/server/admin-auth';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import { normalizeCronInsert } from '$lib/server/crons';
import type { RequestHandler } from './$types';

async function readJson(request: Request): Promise<unknown> {
  try {
    return await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }
}

export const GET: RequestHandler = async ({ cookies }) => {
  assertAdminSession(cookies);
  const supabase = getSupabaseAdmin();
  const { data, error: dbError } = await supabase
    .from('ae_crons')
    .select('*')
    .order('created_at', { ascending: false });

  if (dbError) throw error(500, dbError.message);
  return json(data ?? []);
};

export const POST: RequestHandler = async ({ request, cookies }) => {
  assertAdminSession(cookies);
  const supabase = getSupabaseAdmin();
  const payload = normalizeCronInsert(await readJson(request));
  const { data, error: dbError } = await supabase
    .from('ae_crons')
    .insert(payload)
    .select()
    .single();

  if (dbError) throw error(500, dbError.message);
  return json(data, { status: 201 });
};
