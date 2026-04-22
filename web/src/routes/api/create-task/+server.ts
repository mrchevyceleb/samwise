import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
  const url = env.TASK_WEBHOOK_URL;
  const secret = env.TASK_WEBHOOK_SECRET;
  if (!url || !secret) throw error(500, 'webhook not configured');

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }

  const payload = body as Record<string, unknown>;
  const title = typeof payload.title === 'string' ? payload.title.trim() : '';
  if (!title) throw error(400, 'title required');

  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-webhook-secret': secret
    },
    body: JSON.stringify({
      title,
      description: typeof payload.description === 'string' ? payload.description : '',
      project: typeof payload.project === 'string' && payload.project ? payload.project : undefined,
      priority: payload.priority ?? 'medium',
      task_type: payload.task_type ?? 'code',
      source: 'web-board',
      base_branch: typeof payload.base_branch === 'string' && payload.base_branch ? payload.base_branch : undefined,
      attachments: Array.isArray(payload.attachments) ? payload.attachments : undefined
    })
  });

  const text = await res.text();
  let data: unknown = text;
  try { data = JSON.parse(text); } catch { /* leave as text */ }

  if (!res.ok) {
    return json({ ok: false, status: res.status, error: data }, { status: res.status });
  }
  return json({ ok: true, result: data });
};
