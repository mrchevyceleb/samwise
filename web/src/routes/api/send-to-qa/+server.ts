import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import type { RequestHandler } from './$types';

const DEFAULT_QAHUB_URL = 'https://iycloielqcjnjqddeuet.supabase.co';
const DEFAULT_QAHUB_ANON_KEY =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Iml5Y2xvaWVscWNqbmpxZGRldWV0Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3Njg5NDIzNjgsImV4cCI6MjA4NDUxODM2OH0.OEuHnHLx6aaT_jCdiGD2SKBtNu96AOl7CiiuYxW3G0o';

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
  'Access-Control-Allow-Headers': 'content-type'
};

export const OPTIONS: RequestHandler = () => new Response(null, { headers: CORS_HEADERS });

// Map AutoSam project name to QA Hub client key. Unknown projects default to rlink-rebuild.
function clientFromProject(project: string | null | undefined): string {
  const p = (project || '').toLowerCase();
  if (p.includes('operly')) return 'operly';
  if (p.includes('nova') && p.includes('cs')) return 'nova-cs';
  if (p.includes('banana')) return 'banana-code';
  if (p.includes('profit') && p.includes('wizard')) return 'profit-wizard';
  if (p.includes('budget') && p.includes('genius')) return 'budget-genius';
  if (p.includes('fiscal') || p.includes('pilot')) return 'fiscal-pilot';
  if (p.includes('bridge')) return 'the-bridge';
  if (p.includes('wecare')) return 'wecare';
  if (p.includes('wwyh')) return 'wwyh';
  if (p.includes('ypp')) return 'ypp';
  if (p.includes('elite')) return 'eliteteam';
  return 'rlink-rebuild';
}

function trim(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

export const POST: RequestHandler = async ({ request }) => {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }

  const input = body as Record<string, unknown>;
  const taskId = trim(input.task_id);
  const tester = trim(input.tester);
  if (!taskId) throw error(400, 'task_id required');
  if (!tester) throw error(400, 'tester required');

  const supabase = getSupabaseAdmin();
  const { data: taskRow, error: taskErr } = await supabase
    .from('ae_tasks')
    .select(
      'id,title,description,project,status,pr_url,pr_number,preview_url,branch,commit_message,context,repo_url'
    )
    .eq('id', taskId)
    .limit(1)
    .single();

  if (taskErr || !taskRow) throw error(404, 'task not found');

  const qaUrl = env.QAHUB_SUPABASE_URL || DEFAULT_QAHUB_URL;
  const qaKey = env.QAHUB_ANON_KEY || DEFAULT_QAHUB_ANON_KEY;
  const client = clientFromProject(taskRow.project);

  const reproLines: string[] = [];
  reproLines.push(`1. Review what was changed. Read the PR diff at ${taskRow.pr_url || '(no PR URL on record)'}.`);
  if (taskRow.preview_url) {
    reproLines.push(`2. Open the preview at ${taskRow.preview_url} and exercise the affected flow end-to-end.`);
  } else {
    reproLines.push('2. Pull the branch locally (or wait for staging) and exercise the affected flow end-to-end.');
  }
  reproLines.push('3. Keep the browser DevTools console open. Note any errors, warnings, or failed network requests.');
  reproLines.push('4. Verify the original goal is met. Screenshot anything notable.');
  reproLines.push('5. If everything passes: mark Verified - Fix Works. If anything is wrong: mark Still Broken with notes.');

  const descParts: string[] = [];
  descParts.push(`Sam finished work on this and it is ready for QA verification before merge.`);
  if (taskRow.description) descParts.push(`Original goal: ${taskRow.description}`);
  if (taskRow.commit_message) descParts.push(`What Sam changed (commit message):\n${taskRow.commit_message}`);
  if (taskRow.pr_url) descParts.push(`PR: ${taskRow.pr_url}`);
  if (taskRow.preview_url) descParts.push(`Preview: ${taskRow.preview_url}`);

  const ticketTitle = `QA: ${(taskRow.title || 'AutoSam task').slice(0, 70)}`;
  const insertPayload = {
    client,
    submitted_by: 'AutoSam',
    raw_text: `AutoSam task ${taskId} ready for QA. ${descParts.join('\n\n')}`,
    title: ticketTitle,
    description: descParts.join('\n\n'),
    repro_steps: reproLines.join('\n'),
    severity: 'medium',
    category: 'qa_task',
    ticket_type: 'task',
    status: 'new',
    priority: 'normal',
    assigned_to: tester,
    environment: 'staging',
    ai_processing_status: 'complete',
    metadata: {
      from_autosam: true,
      autosam_task_id: taskId,
      autosam_pr_url: taskRow.pr_url,
      autosam_preview_url: taskRow.preview_url,
      autosam_branch: taskRow.branch,
      feedback_ticket_id:
        (taskRow.context as Record<string, unknown> | null)?.ticket_id ?? null
    }
  };

  const insertRes = await fetch(`${qaUrl}/rest/v1/qa_tickets`, {
    method: 'POST',
    headers: {
      apikey: qaKey,
      Authorization: `Bearer ${qaKey}`,
      'Content-Type': 'application/json',
      Prefer: 'return=representation'
    },
    body: JSON.stringify(insertPayload)
  });
  if (!insertRes.ok) {
    const detail = await insertRes.text();
    throw error(502, `QA Hub insert failed: ${detail.slice(0, 300)}`);
  }
  const inserted = await insertRes.json();
  const qaTicket = Array.isArray(inserted) ? inserted[0] : inserted;
  const qaTicketId = qaTicket?.id as string | undefined;
  if (!qaTicketId) throw error(502, 'QA Hub insert returned no id');

  const existingContext = (taskRow.context as Record<string, unknown> | null) || {};
  const updatedContext = {
    ...existingContext,
    qa: {
      ticket_id: qaTicketId,
      tester,
      sent_at: new Date().toISOString()
    }
  };

  const { error: updateErr } = await supabase
    .from('ae_tasks')
    .update({ status: 'qa', context: updatedContext })
    .eq('id', taskId);

  if (updateErr) {
    throw error(502, `Failed to update AutoSam task: ${updateErr.message}`);
  }

  return json(
    {
      ok: true,
      qa_ticket_id: qaTicketId,
      qa_ticket_url: `https://qa.stonelabs.app/dashboard.html?ticket=${qaTicketId}`,
      tester
    },
    { headers: CORS_HEADERS }
  );
};
