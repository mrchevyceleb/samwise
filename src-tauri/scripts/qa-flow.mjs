#!/usr/bin/env node
// Local headless QA runner for Sam (replaces Browserbase by default).
//
//   node qa-flow.mjs <flow.mjs> [--out DIR] [--viewport 1280x800] [--timeout SECONDS]
//
// The flow file is an ES module: `export default async function (page, ctx) { ... }`.
//   page            a Playwright Page (headless Chromium, one context, popups are tracked too)
//   ctx.login(url)  signs in with the staging test account for that app (operly or studio), prints no secrets;
//                   returns { ok, reason, url }. reason is 'needs_2fa', 'login_failed', 'login_requires_https', 'login_host_mismatch', 'login_form_not_found' or 'no_credentials_for_host' when ok is false
//   ctx.shot(name)  full-page screenshot into the out dir, returns the path
//   ctx.snapshot()  accessibility-tree text of the page (read this like a page snapshot)
//   ctx.note(text)  records a step in the report (what you actually exercised)
// Anything the flow console.logs shows up in stdout. The last stdout line is `QA_FLOW_REPORT: <json>` with
// console errors/warnings, uncaught page errors, failed requests and HTTP 4xx/5xx seen across the whole run.
// Exit code: 0 flow finished, 2 flow threw (report has the error and failure.png), 3 could not launch.
import { createRequire } from 'node:module';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { format } from 'node:util';

const args = process.argv.slice(2);
const flowPath = args.find((a, i) => !a.startsWith('--') && !(i && args[i - 1].startsWith('--')));
const opt = (n, d) => { const i = args.indexOf(`--${n}`); return i >= 0 ? args[i + 1] : d; };
if (!flowPath) { console.error('usage: node qa-flow.mjs <flow.mjs> [--out DIR] [--viewport WxH] [--timeout S]'); process.exit(3); }
const out = resolve(opt('out', join(process.cwd(), 'qa-out')));
const [vw, vh] = opt('viewport', '1280x800').split('x').map(Number);
const timeoutArg = Number(opt('timeout', '100'));
const timeoutMs = (Number.isFinite(timeoutArg) && timeoutArg > 0 ? Math.min(timeoutArg, 300) : 100) * 1000;
try { mkdirSync(out, { recursive: true }); } catch (e) { console.error(`cannot create the out dir ${out}: ${e.message}`); process.exit(3); }

let chromium;
try {
  chromium = createRequire(join(homedir(), 'node_modules', '_'))('playwright').chromium;
} catch (e) {
  console.error(`playwright is not installed in ~/node_modules: ${e.message}`);
  process.exit(3);
}

const CAP = 40;
const report = { ok: true, notes: [], screenshots: [], console: [], pageErrors: [], failedRequests: [], httpErrors: [] };
// Entries past the cap are counted in report.truncated so a noisy run cannot hide later errors silently.
const push = (name, item) => {
  const list = report[name];
  if (list.some((x) => JSON.stringify(x) === JSON.stringify(item))) return;
  if (list.length < CAP) list.push(item);
  else { report.truncated ??= {}; report.truncated[name] = (report.truncated[name] || 0) + 1; }
};
// Console text, URLs and errors can echo credentials or tokens: strip the loaded login and sensitive query params.
const secrets = [];
const SENSITIVE = /([?&#][^=&#\s]*(?:token|key|secret|sig|auth|code|password|session)[^=&#\s]*=)[^&#\s]*/gi;
const redact = (v) => { let t = String(v).replace(SENSITIVE, '$1[redacted]'); for (const x of secrets) t = t.split(x).join('[redacted]'); return t; };
// The flow's own console output goes through redact too.
for (const m of ['log', 'info', 'warn', 'error']) { const orig = console[m].bind(console); console[m] = (...a) => orig(redact(format(...a))); }

function creds(url) {
  let all;
  try { all = JSON.parse(readFileSync(join(homedir(), '.claude', 'test-credentials.json'), 'utf8')); } catch { return null; }
  // Credentials are only ever used on the exact host they are stored for, never on a host the URL merely mentions.
  let host;
  try { host = new URL(url).hostname.toLowerCase(); } catch { return null; }
  const c = Object.hasOwn(all, host) ? all[host] : null;
  return c?.email && c?.password ? { ...c, host } : null;
}

async function login(page, url) {
  // Credentials only ever go over HTTPS, so a plain-http or odd-scheme URL never receives them.
  let scheme;
  try { scheme = new URL(url).protocol; } catch { return { ok: false, reason: 'no_credentials_for_host', url }; }
  if (scheme !== 'https:') return { ok: false, reason: 'login_requires_https', url };
  const c = creds(url);
  if (!c) return { ok: false, reason: 'no_credentials_for_host', url };
  secrets.push(c.email, c.password);
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  const pw = page.locator('input[type=password]:visible').first();
  if (!(await pw.count())) {
    await page.goto(new URL('/login', url).href, { waitUntil: 'domcontentloaded' });
    await pw.waitFor({ state: 'visible', timeout: 15000 }).catch(() => {});
  }
  // Every run starts a fresh browser, so there is no session to be "already signed in" with: no form means a broken page.
  if (!(await pw.count())) return { ok: false, reason: 'login_form_not_found', url: redact(page.url()) };
  // Re-checked right before each credential is typed, in case the page navigated away while the form was being filled.
  const onHost = () => new URL(page.url()).hostname.toLowerCase() === c.host;
  if (!onHost()) return { ok: false, reason: 'login_host_mismatch', url: redact(page.url()) };
  const emailField = page.locator('input:visible:not([type=password]):not([type=checkbox]):not([type=radio]):not([type=hidden]):not([type=submit])').first();
  await emailField.click(); // Operly renders the email input readOnly until it is focused
  if (!onHost()) return { ok: false, reason: 'login_host_mismatch', url: redact(page.url()) };
  await emailField.fill(c.email);
  await pw.click();
  if (!onHost()) return { ok: false, reason: 'login_host_mismatch', url: redact(page.url()) };
  await pw.fill(c.password);
  const submit = page.locator('button[type=submit]:visible').first();
  if (await submit.count()) await submit.click(); else await pw.press('Enter');
  await page.waitForFunction(() => !document.querySelector('input[type=password]'), null, { timeout: 20000 }).catch(() => {});
  if (await page.locator('input[autocomplete=one-time-code], input[name*=otp i], input[name*=code i]').first().count()) return { ok: false, reason: 'needs_2fa', url: redact(page.url()) };
  // Success needs positive evidence: the password field is gone AND the page left the /login route.
  if ((await page.locator('input[type=password]:visible').count()) || /^\/login(\/|$)/i.test(new URL(page.url()).pathname)) return { ok: false, reason: 'login_failed', url: redact(page.url()) };
  return { ok: true, url: redact(page.url()) };
}

function watch(page) {
  page.on('console', (m) => { if (['error', 'warning'].includes(m.type())) push('console', { type: m.type(), text: redact(m.text()).slice(0, 300), page: redact(page.url()) }); });
  page.on('pageerror', (e) => push('pageErrors', { text: redact(e.message || e).slice(0, 300), page: redact(page.url()) }));
  page.on('requestfailed', (r) => push('failedRequests', { url: redact(r.url()).slice(0, 200), method: r.method(), error: r.failure()?.errorText }));
  page.on('response', (r) => { if (r.status() >= 400) push('httpErrors', { url: redact(r.url()).slice(0, 200), method: r.request().method(), status: r.status() }); });
}

let browser, context, page;
try {
  browser = await chromium.launch({ headless: true });
  context = await browser.newContext({ viewport: { width: vw, height: vh } });
  context.on('page', watch);
  page = await context.newPage();
} catch (e) {
  console.error(`could not start headless Chromium: ${e.message}`);
  await browser?.close().catch(() => {});
  process.exit(3);
}
let shots = 0;
const ctx = {
  out,
  note: (t) => report.notes.push(String(t).slice(0, 300)),
  login: (url) => login(page, url),
  shot: async (name = 'shot') => { const p = join(out, `${String(++shots).padStart(2, '0')}-${name.replace(/[^\w-]+/g, '_')}.png`); await page.screenshot({ path: p, fullPage: true }); report.screenshots.push(p); return p; },
  snapshot: async () => (await page.locator('body').ariaSnapshot()).slice(0, 8000),
};

const timer = setTimeout(async () => {
  report.ok = false; report.error = `flow exceeded ${timeoutMs / 1000}s`;
  await finish(2);
}, timeoutMs);

let finished = false;
async function finish(code) {
  if (finished) return;
  finished = true;
  clearTimeout(timer);
  writeFileSync(join(out, 'report.json'), JSON.stringify(report, null, 2));
  console.log(`QA_FLOW_REPORT: ${JSON.stringify(report)}`);
  await browser.close().catch(() => {});
  process.exit(code);
}

try {
  const mod = await import(pathToFileURL(resolve(flowPath)).href);
  await mod.default(page, ctx);
  await finish(0);
} catch (e) {
  report.ok = false; report.error = redact(String(e?.stack || e).split('\n').slice(0, 4).join(' | ')).slice(0, 500);
  await ctx.shot('failure').catch(() => {});
  await finish(2);
}
