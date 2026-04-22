import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import type { RequestHandler } from './$types';

const MAX_BYTES = 20 * 1024 * 1024;
const ALLOWED_PREFIXES = ['image/', 'application/pdf', 'text/'];

function extFor(mime: string, name: string): string {
  if (name.includes('.')) return name.slice(name.lastIndexOf('.'));
  const map: Record<string, string> = {
    'image/png': '.png',
    'image/jpeg': '.jpg',
    'image/jpg': '.jpg',
    'image/gif': '.gif',
    'image/webp': '.webp',
    'image/svg+xml': '.svg',
    'application/pdf': '.pdf'
  };
  return map[mime] ?? '.bin';
}

export const POST: RequestHandler = async ({ request }) => {
  const serviceKey = env.SUPABASE_SERVICE_ROLE_KEY;
  if (!serviceKey) throw error(500, 'upload not configured (missing service key)');

  const form = await request.formData();
  const file = form.get('file');
  if (!(file instanceof File)) throw error(400, 'file required');
  if (file.size === 0) throw error(400, 'empty file');
  if (file.size > MAX_BYTES) throw error(413, `file too large (max ${MAX_BYTES} bytes)`);
  const mime = file.type || 'application/octet-stream';
  if (!ALLOWED_PREFIXES.some((p) => mime.startsWith(p))) {
    throw error(415, `mime not allowed: ${mime}`);
  }

  const key = `${crypto.randomUUID()}${extFor(mime, file.name)}`;
  const uploadUrl = `${PUBLIC_SUPABASE_URL}/storage/v1/object/task-attachments/${key}`;
  const res = await fetch(uploadUrl, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${serviceKey}`,
      'content-type': mime,
      'x-upsert': 'false'
    },
    body: file.stream(),
    // @ts-expect-error — required by Node fetch for ReadableStream body
    duplex: 'half'
  });
  if (!res.ok) {
    const text = await res.text();
    throw error(res.status, `storage: ${text}`);
  }

  const publicUrl = `${PUBLIC_SUPABASE_URL}/storage/v1/object/public/task-attachments/${key}`;
  return json({ url: publicUrl, name: file.name || key, mime });
};
