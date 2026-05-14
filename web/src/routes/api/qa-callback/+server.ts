import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import type { RequestHandler } from './$types';

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
  'Access-Control-Allow-Headers': 'content-type, x-qa-callback-secret'
};

export const OPTIONS: RequestHandler = () => new Response(null, { headers: CORS_HEADERS });

type CallbackBody = {
  autosam_task_id?: string;
  outcome?: 'verified' | 'still_broken';
  qa_ticket_id?: string;
  qa_findings?: string;
};

function trim(v: unknown) {
  return typeof v === 'string' ? v.trim() : '';
}

export const POST: RequestHandler = async ({ request }) => {
  const secret = env.QA_CALLBACK_SECRET;
  if (secret) {
    const got = request.headers.get('x-qa-callback-secret');
    if (got !== secret) {
      return new Response('unauthorized', { status: 401, headers: CORS_HEADERS });
    }
  }

  let body: CallbackBody;
  try {
    body = (await request.json()) as CallbackBody;
  } catch {
    throw error(400, 'invalid JSON');
  }

  const taskId = trim(body.autosam_task_id);
  const outcome = body.outcome;
  const qaTicketId = trim(body.qa_ticket_id);
  if (!taskId) throw error(400, 'autosam_task_id required');
  if (outcome !== 'verified' && outcome !== 'still_broken') {
    throw error(400, 'outcome must be verified or still_broken');
  }

  const supabase = getSupabaseAdmin();
  const { data: taskRow, error: lookupErr } = await supabase
    .from('ae_tasks')
    .select('id,status,context')
    .eq('id', taskId)
    .single();
  if (lookupErr || !taskRow) throw error(404, 'autosam task not found');

  const nextStatus = outcome === 'verified' ? 'approved' : 'fixes_needed';
  const existingContext = (taskRow.context as Record<string, unknown> | null) || {};
  const prevQa = (existingContext.qa as Record<string, unknown> | undefined) || {};
  const updatedContext = {
    ...existingContext,
    qa: {
      ...prevQa,
      outcome,
      qa_ticket_id: qaTicketId || prevQa.qa_ticket_id || null,
      resolved_at: new Date().toISOString()
    }
  };

  const { error: updateErr } = await supabase
    .from('ae_tasks')
    .update({ status: nextStatus, context: updatedContext })
    .eq('id', taskId);
  if (updateErr) throw error(502, `ae_tasks update failed: ${updateErr.message}`);

  const findings = trim(body.qa_findings);
  if (findings) {
    await supabase.from('ae_comments').insert({
      task_id: taskId,
      author: 'system',
      content:
        outcome === 'verified'
          ? `QA Verified the fix.\n\n${findings}`
          : `QA marked Still Broken.\n\n${findings}`
    });
  }

  return json(
    { ok: true, new_status: nextStatus, autosam_task_id: taskId },
    { headers: CORS_HEADERS }
  );
};
