import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import type { RequestHandler } from './$types';

/**
 * Close a GitHub PR without merging, on behalf of the web app.
 *
 * The web app has no GitHub credentials of its own, so this route delegates to
 * the `close-pr` Supabase Edge Function (Samwise project), which holds a
 * GH_TOKEN secret and calls the GitHub REST API. Auth between this route and
 * the function is the Samwise service role key (the same secret the web
 * backend already uses for privileged Supabase writes).
 *
 * Body: { task_id: string }. The task row is loaded to resolve pr_url so the
 * caller can't close an arbitrary PR by guessing a URL — only a PR already
 * attached to a real Samwise task can be closed this way.
 */
const DEFAULT_CLOSE_PR_URL =
  'https://meqtadfevxguishrlxyx.supabase.co/functions/v1/close-pr';

type TaskRow = { id: string; pr_url: string | null };

function text(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

async function loadTaskPrUrl(taskId: string, serviceKey: string): Promise<string | null> {
  const params = new URLSearchParams({
    select: 'id,pr_url',
    id: `eq.${taskId}`,
    limit: '1'
  });
  const res = await fetch(`${PUBLIC_SUPABASE_URL}/rest/v1/ae_tasks?${params}`, {
    headers: { apikey: serviceKey, authorization: `Bearer ${serviceKey}` }
  });
  if (!res.ok) throw error(502, `task lookup failed: ${await res.text()}`);
  const rows = (await res.json()) as TaskRow[];
  return text(rows[0]?.pr_url) || null;
}

export const POST: RequestHandler = async ({ request }) => {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }

  const input = body as Record<string, unknown>;
  const taskId = text(input.task_id);
  if (!taskId) throw error(400, 'task_id required');

  const serviceKey = env.SUPABASE_SERVICE_ROLE_KEY;
  if (!serviceKey) throw error(500, 'close-pr not configured (missing service key)');

  const prUrl = await loadTaskPrUrl(taskId, serviceKey);
  if (!prUrl) throw error(404, 'task has no PR to close');

  const closePrUrl = env.CLOSE_PR_URL || DEFAULT_CLOSE_PR_URL;
  const res = await fetch(closePrUrl, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${serviceKey}`
    },
    body: JSON.stringify({ pr_url: prUrl })
  });

  if (!res.ok) {
    const responseText = await res.text();
    return json(
      { ok: false, status: res.status, error: responseText || res.statusText },
      { status: 502 }
    );
  }

  // The function returns { ok, state } where state is "closed" or
  // "already-closed". Forward ok:true either way.
  const fnResult = await res.json().catch(() => ({}));
  return json({ ok: true, state: fnResult?.state ?? 'closed' });
};
