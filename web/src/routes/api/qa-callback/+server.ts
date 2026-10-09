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
    .select('id,status,context,on_hold,failure_reason')
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

  // A QA fail must start a real fix cycle on the same branch/PR, not just park
  // the card. The worker's fixes_needed sweep only re-fires when the newest
  // "## Blockers"-bearing comment carries a substantive blocker, so mirror the
  // QA findings into a review-format comment. Fail-closed on human ownership:
  // an atomic conditional claim on the task row (status fixes_needed, not
  // on_hold, failure_reason not 'Reserved for ...') must succeed before
  // anything is posted, and the new capability only arms when the route's
  // auth secret is configured (blocker text drives an autonomous agent).
  let fix_cycle_armed = false;
  let fix_cycle_error: string | null = null;
  if (outcome === 'still_broken') {
    if (!env.QA_CALLBACK_SECRET) {
      fix_cycle_error = 'fix-cycle arming requires QA_CALLBACK_SECRET';
    } else {
      const failureReason = typeof taskRow?.failure_reason === 'string' ? taskRow.failure_reason : '';
      const reserved = failureReason.trimStart().toLowerCase().startsWith('reserved for');
      if (taskRow?.on_hold === true || reserved) {
        fix_cycle_error = taskRow?.on_hold === true ? 'task is on hold' : 'task is reserved for a human';
      } else {
        const bullets = findings
          .split(/\r?\n/)
          .map((line) => line.trim().replace(/^[-*]\s*/, ''))
          .filter((line) => line.length > 0)
          .slice(0, 15)
          .map((line) => line.slice(0, 400));
        if (bullets.length === 0) {
          fix_cycle_error = 'no parseable findings';
        } else {
          const marker = qaTicketId ? `Sud QA ticket ${qaTicketId}` : null;
          let alreadyPosted = false;
          if (marker) {
            const { data: recent, error: scanErr } = await supabase
              .from('ae_comments')
              .select('content')
              .eq('task_id', taskId)
              .order('created_at', { ascending: false })
              .limit(50);
            if (scanErr) {
              alreadyPosted = true; // fail closed: an unreadable dup scan never re-arms
              fix_cycle_error = `dup scan failed: ${scanErr.message}`;
            } else {
              alreadyPosted = (recent || []).some(
                (row) => typeof row?.content === 'string' && row.content.includes(marker as string)
              );
              if (alreadyPosted) fix_cycle_error = 'fix comment already posted for this QA ticket';
            }
          }
          if (!alreadyPosted) {
            // Atomic claim: the guards are re-evaluated at write time, so a
            // hold/reservation that landed after the read above still blocks.
            const { data: claimed, error: claimErr } = await supabase
              .from('ae_tasks')
              .update({ updated_at: new Date().toISOString() })
              .eq('id', taskId)
              .eq('status', 'fixes_needed')
              .eq('on_hold', false)
              .not('failure_reason', 'like', 'Reserved for%')
              .select('id');
            if (claimErr) {
              fix_cycle_error = `guard claim failed: ${claimErr.message}`;
            } else if (!claimed || claimed.length === 0) {
              fix_cycle_error = 'task no longer eligible (claimed by a human or moved on)';
            } else {
              const prUrl = typeof taskRow?.pr_url === 'string' ? taskRow.pr_url : null;
              const content = [
                '## Summary',
                `Sud QA failed this PR${prUrl ? ` (${prUrl})` : ''}: QA marked Still Broken${marker ? ` (${marker})` : ''}`,
                '',
                '## Blockers',
                ...bullets.map((line) => `- ${line}`)
              ].join('\n');
              const { error: fixCommentErr } = await supabase
                .from('ae_comments')
                .insert({ task_id: taskId, author: 'system', content });
              if (fixCommentErr) {
                fix_cycle_error = `blockers comment failed: ${fixCommentErr.message}`;
                console.error('qa-callback fix-cycle insert failed:', fixCommentErr);
              } else {
                fix_cycle_armed = true;
              }
            }
          }
        }
      }
    }
  }

  return json(
    { ok: true, new_status: nextStatus, autosam_task_id: taskId, fix_cycle_armed, ...(fix_cycle_error ? { fix_cycle_error } : {}) },
    { headers: CORS_HEADERS }
  );
};
