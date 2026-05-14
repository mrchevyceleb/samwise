import type { Handle } from '@sveltejs/kit';

const CROSS_ORIGIN_API_PREFIXES = ['/api/qa-testers', '/api/send-to-qa', '/api/qa-callback'];

export const handle: Handle = async ({ event, resolve }) => {
  const isCrossOriginApi = CROSS_ORIGIN_API_PREFIXES.some((p) => event.url.pathname.startsWith(p));

  if (isCrossOriginApi && event.request.method === 'OPTIONS') {
    return new Response(null, {
      headers: {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'content-type, x-qa-callback-secret'
      }
    });
  }

  const response = await resolve(event);
  if (isCrossOriginApi) {
    response.headers.set('Access-Control-Allow-Origin', '*');
    response.headers.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    response.headers.set('Access-Control-Allow-Headers', 'content-type');
  }
  return response;
};
