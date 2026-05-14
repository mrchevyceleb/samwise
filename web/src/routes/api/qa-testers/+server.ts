import { json, error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

const DEFAULT_QAHUB_URL = 'https://iycloielqcjnjqddeuet.supabase.co';
const DEFAULT_QAHUB_ANON_KEY =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Iml5Y2xvaWVscWNqbmpxZGRldWV0Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3Njg5NDIzNjgsImV4cCI6MjA4NDUxODM2OH0.OEuHnHLx6aaT_jCdiGD2SKBtNu96AOl7CiiuYxW3G0o';

type QaUser = { name: string; role: string };

export const GET: RequestHandler = async () => {
  const url = env.QAHUB_SUPABASE_URL || DEFAULT_QAHUB_URL;
  const key = env.QAHUB_ANON_KEY || DEFAULT_QAHUB_ANON_KEY;

  const res = await fetch(`${url}/rest/v1/qa_users?select=name,role&order=name.asc`, {
    headers: { apikey: key, Authorization: `Bearer ${key}` }
  });
  if (!res.ok) throw error(502, `qa_users lookup failed: ${await res.text()}`);
  const rows = (await res.json()) as QaUser[];
  const testers = rows.filter((u) => u.role === 'qa' || u.role === 'admin');
  return json({ testers });
};
