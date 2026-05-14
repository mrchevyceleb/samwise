import { createHash, timingSafeEqual } from 'node:crypto';
import { error, type Cookies } from '@sveltejs/kit';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';

const ADMIN_COOKIE = 'samwise_web_admin';

function adminKey() {
  return env.SAMWISE_WEB_ADMIN_KEY || '';
}

function adminRequired() {
  return !!adminKey();
}

function digest(value: string) {
  return createHash('sha256').update(`samwise-web-admin:${value}`).digest('hex');
}

function timingSafeStringEqual(a: string, b: string) {
  const aBuffer = Buffer.from(a);
  const bBuffer = Buffer.from(b);
  return aBuffer.length === bBuffer.length && timingSafeEqual(aBuffer, bBuffer);
}

export function hasAdminSession(cookies: Cookies) {
  if (!adminRequired()) return true;

  const key = adminKey();
  const token = cookies.get(ADMIN_COOKIE);
  return !!key && !!token && timingSafeStringEqual(token, digest(key));
}

export function isValidAdminKey(candidate: unknown) {
  const key = adminKey();
  return typeof candidate === 'string' && !!key && timingSafeStringEqual(candidate, key);
}

export function setAdminSession(cookies: Cookies) {
  const key = adminKey();
  if (!key) return;

  cookies.set(ADMIN_COOKIE, digest(key), {
    httpOnly: true,
    sameSite: 'strict',
    secure: !dev,
    path: '/',
    maxAge: 60 * 60 * 12
  });
}

export function clearAdminSession(cookies: Cookies) {
  cookies.delete(ADMIN_COOKIE, { path: '/' });
}

export function assertAdminSession(cookies: Cookies) {
  if (!hasAdminSession(cookies)) {
    throw error(401, 'Admin unlock required');
  }
}
