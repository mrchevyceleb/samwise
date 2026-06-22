// close-pr: close a GitHub pull request without merging.
//
// Called by the Samwise web app's /api/close-pr route (which holds the
// Samwise service role key). The function authenticates the caller by
// comparing the supplied Authorization/apikey header against the
// auto-injected SUPABASE_SERVICE_ROLE_KEY, then closes the PR via the
// GitHub REST API using a GH_TOKEN secret.
//
// Closing an already-closed or already-merged PR is treated as success so
// the "Close PR & Mark Done" button still completes the task in those cases.
//
// Deploy: supabase functions deploy close-pr --project-ref meqtadfevxguishrlxyx

const SERVICE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY");

const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization, apikey",
};

function json(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { ...corsHeaders, "Content-Type": "application/json" },
  });
}

function err(message: string, status = 400) {
  return json({ ok: false, error: message }, status);
}

/** Constant-time string compare. */
function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

/** The web backend is the only legitimate caller; it holds the service key. */
function requireServiceKey(req: Request): boolean {
  if (!SERVICE_KEY) return false;
  const authHeader = req.headers.get("authorization") ?? "";
  const bearer = authHeader.startsWith("Bearer ") ? authHeader.slice(7).trim() : "";
  const apiKey = req.headers.get("apikey") ?? "";
  return timingSafeEqual(bearer, SERVICE_KEY) || timingSafeEqual(apiKey, SERVICE_KEY);
}

interface ParsedPr {
  owner: string;
  repo: string;
  number: number;
}

/** Parse https://github.com/{owner}/{repo}/pull/{number} (+ /files, /commits suffixes). */
function parsePrUrl(raw: string): ParsedPr | null {
  try {
    const u = new URL(raw.trim());
    if (u.hostname !== "github.com" && u.hostname !== "www.github.com") return null;
    const parts = u.pathname.split("/").filter(Boolean);
    // [owner, repo, "pull", "123", ...]
    if (parts.length < 4 || parts[2].toLowerCase() !== "pull") return null;
    const owner = parts[0];
    const repo = parts[1];
    const number = parseInt(parts[3], 10);
    if (!owner || !repo || !Number.isFinite(number) || number <= 0) return null;
    return { owner, repo, number };
  } catch {
    return null;
  }
}

async function closePullRequest(pr: ParsedPr): Promise<Response> {
  const token = Deno.env.get("GH_TOKEN");
  if (!token) {
    return err("Server is not configured to close PRs (GH_TOKEN missing).", 500);
  }

  const endpoint = `https://api.github.com/repos/${pr.owner}/${pr.repo}/pulls/${pr.number}`;
  const resp = await fetch(endpoint, {
    method: "PATCH",
    headers: {
      "Accept": "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json",
      "User-Agent": "samwise-close-pr",
    },
    body: JSON.stringify({ state: "closed" }),
  });

  if (resp.ok) {
    return json({ ok: true, state: "closed" });
  }

  // GitHub returns 422 for "already closed" / "already merged". Both mean the
  // PR is no longer open, which is the goal of the caller — treat as success.
  if (resp.status === 422) {
    const body = await resp.json().catch(() => ({}));
    const message: string = body?.message ?? "";
    if (/already (closed|merged)/i.test(message)) {
      return json({ ok: true, state: "already-closed", note: message });
    }
  }

  const text = await resp.json().catch(() => resp.text());
  const detail = typeof text === "string" ? text : (text?.message ?? JSON.stringify(text));
  return err(`GitHub API ${resp.status}: ${detail}`, resp.status === 401 || resp.status === 403 ? 401 : 502);
}

Deno.serve(async (req) => {
  if (req.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: corsHeaders });
  }
  if (req.method !== "POST") {
    return err("POST only", 405);
  }
  if (!requireServiceKey(req)) {
    return err("Unauthorized", 401);
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return err("invalid JSON");
  }
  const input = body as Record<string, unknown>;
  const prUrl = typeof input?.pr_url === "string" ? input.pr_url.trim() : "";
  if (!prUrl) {
    return err("pr_url required");
  }

  const parsed = parsePrUrl(prUrl);
  if (!parsed) {
    return err(`Could not parse GitHub PR URL: ${prUrl}`);
  }

  return closePullRequest(parsed);
});
