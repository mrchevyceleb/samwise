import { json, error } from '@sveltejs/kit';
import { assertAdminSession } from '$lib/server/admin-auth';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import { nextRunForSchedule, normalizeCronUpdate } from '$lib/server/crons';
import type { RequestHandler } from './$types';

async function readJson(request: Request): Promise<unknown> {
  try {
    return await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }
}

export const PATCH: RequestHandler = async ({ params, request, cookies }) => {
  assertAdminSession(cookies);
  const supabase = getSupabaseAdmin();
  const updates = normalizeCronUpdate(await readJson(request));

  if (updates.enabled === true || typeof updates.schedule === 'string') {
    const { data: current, error: currentError } = await supabase
      .from('ae_crons')
      .select('schedule,enabled')
      .eq('id', params.id)
      .single();

    if (currentError) throw error(500, currentError.message);
    const nextEnabled = typeof updates.enabled === 'boolean' ? updates.enabled : current.enabled;
    if (nextEnabled) {
      updates.next_run = nextRunForSchedule(
        typeof updates.schedule === 'string' ? updates.schedule : current.schedule
      );
    }
  } else if (updates.enabled === false) {
    updates.next_run = null;
  }

  const { data, error: dbError } = await supabase
    .from('ae_crons')
    .update(updates)
    .eq('id', params.id)
    .select()
    .single();

  if (dbError) throw error(500, dbError.message);
  return json(data);
};

export const DELETE: RequestHandler = async ({ params, cookies }) => {
  assertAdminSession(cookies);
  const supabase = getSupabaseAdmin();
  const { error: dbError } = await supabase
    .from('ae_crons')
    .delete()
    .eq('id', params.id);

  if (dbError) throw error(500, dbError.message);
  return json({ ok: true, deleted: params.id });
};
