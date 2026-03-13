import { getSettings, updateSetting } from '$lib/stores/settings';
import {
  aiExchangeOpenRouterOAuthCode,
  aiOpenAIDeviceStart,
  aiOpenAIDevicePoll,
  aiOpenAIExchangeAuthorizationCode,
  aiOpenAIRefreshOAuthToken,
} from '$lib/utils/tauri';

export const OPENAI_AUTH_ISSUER = 'https://auth.openai.com';
export const OPENAI_CODEX_CLIENT_ID = 'app_EMoamEEZ73f0CkXaXp7hrann';

interface OpenAIDeviceStartPayload {
  issuer: string;
  client_id: string;
  verification_url: string;
  user_code: string;
  device_auth_id: string;
  interval: number;
}

export interface OpenAIDeviceAuthState {
  issuer: string;
  clientId: string;
  verificationUrl: string;
  userCode: string;
  deviceAuthId: string;
  interval: number;
  startedAt: number;
}

let openRouterCodeVerifier = '';
let pendingOpenAIDeviceAuth: OpenAIDeviceAuthState | null = null;

function randomVerifier(length = 64): string {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~';
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  let out = '';
  for (let i = 0; i < bytes.length; i++) {
    out += alphabet[bytes[i] % alphabet.length];
  }
  return out;
}

async function sha256Base64Url(value: string): Promise<string> {
  const encoded = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest('SHA-256', encoded);
  const bytes = Array.from(new Uint8Array(digest));
  const base64 = btoa(String.fromCharCode(...bytes));
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function toIsoExpiry(expiresInSeconds: unknown): string {
  const seconds = Number(expiresInSeconds || 0);
  if (!Number.isFinite(seconds) || seconds <= 0) return '';
  return new Date(Date.now() + (seconds * 1000)).toISOString();
}

function isExpired(expiresAt: string, skewSeconds = 60): boolean {
  if (!expiresAt) return false;
  const ts = new Date(expiresAt).getTime();
  if (!Number.isFinite(ts)) return false;
  return Date.now() >= (ts - skewSeconds * 1000);
}

function normalizeClientVersion(raw: string): string {
  const fallback = '4.0.0';
  const value = (raw || '').trim();
  if (!value) return fallback;

  const exact = value.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (exact) return `${exact[1]}.${exact[2]}.${exact[3]}`;

  const embedded = value.match(/(\d+)\.(\d+)\.(\d+)/);
  if (embedded) return `${embedded[1]}.${embedded[2]}.${embedded[3]}`;

  return fallback;
}

export async function startOpenRouterOAuth(): Promise<string> {
  const verifier = randomVerifier(64);
  openRouterCodeVerifier = verifier;
  const challenge = await sha256Base64Url(verifier);
  const callbackUrl = 'http://localhost:3000/openrouter-oauth-callback';
  const authUrl = `https://openrouter.ai/auth?callback_url=${encodeURIComponent(callbackUrl)}&code_challenge=${encodeURIComponent(challenge)}&code_challenge_method=S256`;
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(authUrl);
  return 'Browser opened. After login, paste the returned code below.';
}

export async function exchangeOpenRouterOAuthCode(code: string): Promise<string> {
  if (!code.trim()) throw new Error('No code provided');
  if (!openRouterCodeVerifier) throw new Error('Start OAuth first so a code verifier is generated.');

  const raw = await aiExchangeOpenRouterOAuthCode(code.trim(), openRouterCodeVerifier);
  const parsed = JSON.parse(raw || '{}');
  const key = parsed?.key;
  if (!key || typeof key !== 'string') {
    throw new Error('OAuth exchange succeeded but no key was returned.');
  }

  updateSetting('aiProvider', 'openrouter');
  updateSetting('aiAuthMode', 'oauth');
  updateSetting('aiOpenRouterApiKey', key);
  updateSetting('aiApiKey', key);
  openRouterCodeVerifier = '';
  return 'Connected. OpenRouter OAuth key saved.';
}

export async function startOpenAIDeviceOAuth(): Promise<OpenAIDeviceAuthState> {
  const raw = await aiOpenAIDeviceStart(OPENAI_AUTH_ISSUER, OPENAI_CODEX_CLIENT_ID);
  const parsed = JSON.parse(raw || '{}') as Partial<OpenAIDeviceStartPayload>;

  const userCode = String(parsed.user_code || '').trim();
  const deviceAuthId = String(parsed.device_auth_id || '').trim();
  if (!userCode || !deviceAuthId) {
    throw new Error('OpenAI device auth failed: missing user code or device auth id.');
  }

  const state: OpenAIDeviceAuthState = {
    issuer: String(parsed.issuer || OPENAI_AUTH_ISSUER).trim(),
    clientId: String(parsed.client_id || OPENAI_CODEX_CLIENT_ID).trim(),
    verificationUrl: String(parsed.verification_url || `${OPENAI_AUTH_ISSUER}/codex/device`).trim(),
    userCode,
    deviceAuthId,
    interval: Math.max(2, Number(parsed.interval || 5)),
    startedAt: Date.now(),
  };

  pendingOpenAIDeviceAuth = state;

  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(state.verificationUrl);
  return state;
}

export function getPendingOpenAIDeviceOAuth(): OpenAIDeviceAuthState | null {
  return pendingOpenAIDeviceAuth;
}

export function cancelOpenAIDeviceOAuth(): void {
  pendingOpenAIDeviceAuth = null;
}

export async function completeOpenAIDeviceOAuth(timeoutMs = 15 * 60 * 1000): Promise<string> {
  if (!pendingOpenAIDeviceAuth) {
    throw new Error('Start OpenAI OAuth first.');
  }

  const started = Date.now();
  const state = pendingOpenAIDeviceAuth;

  while (Date.now() - started < timeoutMs) {
    const rawPoll = await aiOpenAIDevicePoll(state.issuer, state.deviceAuthId, state.userCode);
    if (!rawPoll.trim()) {
      await sleep(state.interval * 1000);
      continue;
    }

    const pollPayload = JSON.parse(rawPoll || '{}') as Record<string, unknown>;
    const authorizationCode = String(pollPayload.authorization_code || '').trim();
    const codeVerifier = String(pollPayload.code_verifier || '').trim();

    if (!authorizationCode || !codeVerifier) {
      await sleep(state.interval * 1000);
      continue;
    }

    const redirectUri = `${state.issuer.replace(/\/+$/, '')}/deviceauth/callback`;
    const rawToken = await aiOpenAIExchangeAuthorizationCode(
      state.issuer,
      state.clientId,
      authorizationCode,
      codeVerifier,
      redirectUri,
    );

    const tokenPayload = JSON.parse(rawToken || '{}') as Record<string, unknown>;
    const accessToken = String(tokenPayload.access_token || '').trim();
    if (!accessToken) {
      throw new Error('OpenAI OAuth completed but no access token was returned.');
    }

    const refreshToken = String(tokenPayload.refresh_token || '').trim();
    const expiresAt = toIsoExpiry(tokenPayload.expires_in);

    updateSetting('aiProvider', 'openai');
    updateSetting('aiAuthMode', 'oauth');
    updateSetting('aiModel', 'openai/gpt-5.4');
    updateSetting('aiOpenAIApiKey', accessToken);
    updateSetting('aiOpenAIOAuthAccessToken', accessToken);
    updateSetting('aiOpenAIOAuthRefreshToken', refreshToken);
    updateSetting('aiOpenAIOAuthExpiresAt', expiresAt);
    updateSetting('aiOpenAICodexBaseUrl', 'https://chatgpt.com/backend-api/codex');
    const nextSettings = getSettings();
    updateSetting('aiOpenAICodexClientVersion', normalizeClientVersion(nextSettings.aiOpenAICodexClientVersion));
    const toEnsure = [
      'openai/gpt-5.4',
      'openai/gpt-5.3-codex-medium',
      'openai/gpt-5.3-codex',
      'openai/gpt-5.3-codex-spark',
    ];
    const nextEnabled = [...nextSettings.aiEnabledModels];
    for (const modelId of toEnsure) {
      if (!nextEnabled.includes(modelId)) nextEnabled.push(modelId);
    }
    if (nextEnabled.length !== nextSettings.aiEnabledModels.length) {
      updateSetting('aiEnabledModels', nextEnabled);
    }

    pendingOpenAIDeviceAuth = null;
    return 'Connected. OpenAI OAuth token saved.';
  }

  throw new Error('OpenAI device authorization timed out after 15 minutes.');
}

export async function refreshOpenAIOAuthIfNeeded(): Promise<void> {
  const s = getSettings();
  if (s.aiProvider !== 'openai') return;

  const accessToken = (s.aiOpenAIOAuthAccessToken || '').trim();
  if (!accessToken) return;
  if (!isExpired(s.aiOpenAIOAuthExpiresAt || '')) return;

  const refreshToken = (s.aiOpenAIOAuthRefreshToken || '').trim();
  if (!refreshToken) return;

  const raw = await aiOpenAIRefreshOAuthToken(
    OPENAI_AUTH_ISSUER,
    OPENAI_CODEX_CLIENT_ID,
    refreshToken,
  );
  const payload = JSON.parse(raw || '{}') as Record<string, unknown>;

  const nextAccessToken = String(payload.access_token || '').trim();
  if (!nextAccessToken) return;

  const nextRefreshToken = String(payload.refresh_token || '').trim();
  updateSetting('aiOpenAIOAuthAccessToken', nextAccessToken);
  updateSetting('aiOpenAIApiKey', nextAccessToken);
  if (nextRefreshToken) {
    updateSetting('aiOpenAIOAuthRefreshToken', nextRefreshToken);
  }
  updateSetting('aiOpenAIOAuthExpiresAt', toIsoExpiry(payload.expires_in));
}
