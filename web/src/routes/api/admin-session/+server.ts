import { json, error } from '@sveltejs/kit';
import {
  clearAdminSession,
  hasAdminSession,
  isValidAdminKey,
  setAdminSession
} from '$lib/server/admin-auth';
import type { RequestHandler } from './$types';

async function readJson(request: Request): Promise<Record<string, unknown>> {
  try {
    const body = await request.json();
    return body && typeof body === 'object' && !Array.isArray(body)
      ? body as Record<string, unknown>
      : {};
  } catch {
    throw error(400, 'invalid JSON');
  }
}

export const GET: RequestHandler = async ({ cookies }) => {
  return json({ ok: hasAdminSession(cookies) });
};

export const POST: RequestHandler = async ({ request, cookies }) => {
  const body = await readJson(request);
  if (!isValidAdminKey(body.key)) {
    throw error(401, 'Invalid admin key');
  }

  setAdminSession(cookies);
  return json({ ok: true });
};

export const DELETE: RequestHandler = async ({ cookies }) => {
  clearAdminSession(cookies);
  return json({ ok: true });
};
