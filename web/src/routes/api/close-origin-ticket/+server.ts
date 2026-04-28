import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import { createHmac } from 'node:crypto';
import type { RequestHandler } from './$types';

const DEFAULT_CLOSE_ORIGIN_TICKET_URL =
  'https://iycloielqcjnjqddeuet.supabase.co/functions/v1/close-origin-ticket';

type TaskRow = {
  id: string;
  source: string | null;
  status: string | null;
  origin_system: string | null;
  origin_id: string | null;
  callback_url: string | null;
  pr_url: string | null;
};

function text(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

async function loadTask(taskId: string, serviceKey: string): Promise<TaskRow | null> {
  const params = new URLSearchParams({
    select: 'id,source,status,origin_system,origin_id,callback_url,pr_url',
    id: `eq.${taskId}`,
    limit: '1'
  });
  const res = await fetch(`${PUBLIC_SUPABASE_URL}/rest/v1/ae_tasks?${params}`, {
    headers: {
      apikey: serviceKey,
      authorization: `Bearer ${serviceKey}`
    }
  });
  if (!res.ok) throw error(502, `task lookup failed: ${await res.text()}`);
  const rows = (await res.json()) as TaskRow[];
  return rows[0] ?? null;
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
  if (!serviceKey) throw error(500, 'close-origin not configured');

  const task = await loadTask(taskId, serviceKey);
  if (!task) throw error(404, 'task not found');
  if (task.status !== 'done') {
    return json({ ok: true, skipped: true, reason: 'task is not done' });
  }

  const source = text(task.source);
  const rawSystem = text(task.origin_system);
  if (source === 'manual' || rawSystem === 'manual') {
    return json({ ok: true, skipped: true });
  }

  const callbackUrl = text(task.callback_url);
  if (!rawSystem && !callbackUrl) {
    return json({ ok: true, skipped: true });
  }

  const secret = env.SAM_CALLBACK_SECRET;
  if (!secret) throw error(500, 'close-origin not configured');

  const payload = {
    task_id: taskId,
    pr_url: text(task.pr_url),
    system: rawSystem || 'unknown',
    origin_id: text(task.origin_id)
  };
  const payloadJson = JSON.stringify(payload);
  const signature = createHmac('sha256', secret).update(payloadJson).digest('hex');
  const closeoutUrl = env.CLOSE_ORIGIN_TICKET_URL || DEFAULT_CLOSE_ORIGIN_TICKET_URL;

  const res = await fetch(closeoutUrl, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-samwise-signature': `sha256=${signature}`,
      'user-agent': 'samwise-web/1'
    },
    body: payloadJson
  });
  const responseText = await res.text();

  if (!res.ok) {
    return json(
      { ok: false, status: res.status, error: responseText || res.statusText },
      { status: 502 }
    );
  }

  return json({ ok: true });
};
