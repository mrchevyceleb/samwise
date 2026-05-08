import { json, error } from '@sveltejs/kit';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async () => {
  const supabase = getSupabaseAdmin();
  const { data, error: dbError } = await supabase
    .from('ae_projects')
    .select('id,name,repo_url,repo_path,preview_url,client,deploy_method,dev_command,created_at')
    .order('client', { ascending: true })
    .order('name', { ascending: true });

  if (dbError) throw error(500, dbError.message);
  return json(data ?? []);
};
