// task-webhook: generic inbound endpoint for firing tasks at Sam from
// external systems (Sentry, GitHub, shell scripts, anything that can POST).
//
// Auth: `x-webhook-secret` header must equal TASK_WEBHOOK_SECRET env var.
// Minimum body: { title, description }
// Optional body: project, priority, task_type, source, base_branch, attachments,
//   origin_system, origin_id, origin_url
//
// Attachments: array. Each entry may be either
//   - a plain string URL, or
//   - a string data URL ("data:image/png;base64,..."), or
//   - an object { url?, data?, name?, mime? } where data is a base64 string
//     (optionally with a data: prefix). Inline data is uploaded to the
//     task-attachments storage bucket and the resulting public URL is stored.
//
// Behavior:
//   - If project omitted, try to infer by scanning title+description for any
//     registered name in ae_projects.
//   - If project resolved, backfill repo_url and repo_path from the registry.
//   - Insert into ae_tasks with status="queued". Sam's worker picks it up on
//     its next poll (~10s cadence).

import { createClient } from "https://esm.sh/@supabase/supabase-js@2.45.4";

type AttachmentInput =
  | string
  | { url?: string; data?: string; name?: string; mime?: string };

type OriginSystem = "operly_triage" | "banana_triage" | "sentry" | "manual";

type Body = {
  title?: string;
  description?: string;
  project?: string;
  priority?: "critical" | "high" | "medium" | "low";
  task_type?: "code" | "research" | "qa-verify";
  source?: string;
  repo_url?: string;
  base_branch?: string;
  attachments?: AttachmentInput[];
  callback_url?: string;
  callback_secret?: string;
  origin_system?: OriginSystem;
  origin_id?: string;
  origin_url?: string;
};

const ORIGIN_SYSTEMS: ReadonlySet<OriginSystem> = new Set([
  "operly_triage",
  "banana_triage",
  "sentry",
  "manual",
]);

type StoredAttachment = { url: string; name: string; mime: string };
type ProjectRow = {
  name: string;
  repo_url?: string | null;
  repo_path?: string | null;
  preview_url?: string | null;
};
type ProjectResolution = {
  row: ProjectRow;
  reason: string;
  hint: string;
  score: number;
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function normalizeRepoUrl(value: string): string {
  return value.trim().toLowerCase().replace(/\/+$/, "").replace(/\.git$/, "").replace(/\/+$/, "");
}

function compactKey(input: string): string {
  return input.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function tokens(input: string): string[] {
  return input
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function tokenMatches(query: string, candidate: string): boolean {
  return query === candidate ||
    (query.length >= 3 && candidate.startsWith(query)) ||
    (candidate.length >= 3 && query.startsWith(candidate));
}

function scoreProjectHint(input: string, name: string): number {
  const queryCompact = compactKey(input);
  const nameCompact = compactKey(name);
  if (!queryCompact || !nameCompact) return 0;
  if (queryCompact === nameCompact) return 100;
  if (nameCompact.startsWith(queryCompact) && queryCompact.length >= 4) return 92;
  if (queryCompact.startsWith(nameCompact) && nameCompact.length >= 4) return 90;
  if (nameCompact.includes(queryCompact) && queryCompact.length >= 4) return 86;

  const queryTokens = tokens(input);
  const nameTokens = tokens(name);
  if (!queryTokens.length || !nameTokens.length) return 0;

  const matchedQuery = queryTokens.filter((q) => nameTokens.some((n) => tokenMatches(q, n))).length;
  const matchedName = nameTokens.filter((n) => queryTokens.some((q) => tokenMatches(q, n))).length;
  let score = Math.round((matchedQuery / queryTokens.length) * 72 + (matchedName / nameTokens.length) * 20);
  if (matchedQuery === queryTokens.length && queryTokens.length > 1) score += 6;
  return Math.min(score, 99);
}

function pushHint(hints: string[], value: unknown) {
  if (typeof value !== "string") return;
  const hint = value.trim().replace(/^['"`]+|['"`]+$/g, "");
  if (hint.length < 3 || hint.length > 100 || !/[a-z]/i.test(hint)) return;
  if (!hints.some((existing) => existing.toLowerCase() === hint.toLowerCase())) {
    hints.push(hint);
  }
}

function projectHints(project: string, title: string, description: string): string[] {
  const hints: string[] = [];
  pushHint(hints, project);
  for (const text of [title, description]) {
    for (const line of text.split(/\r?\n/)) {
      const trimmed = line.trim();
      const lower = trimmed.toLowerCase();
      for (const prefix of ["project:", "app:", "source:", "repository:", "repo:"]) {
        if (lower.startsWith(prefix)) {
          pushHint(hints, trimmed.slice(prefix.length).split("|")[0]);
        }
      }
    }
  }
  return hints;
}

function resolveProject(
  project: string,
  repoUrl: string,
  title: string,
  description: string,
  projects: ProjectRow[],
): ProjectResolution | null {
  if (repoUrl.trim()) {
    const normalized = normalizeRepoUrl(repoUrl);
    const byUrl = projects.find((row) => row.repo_url && normalizeRepoUrl(row.repo_url) === normalized);
    if (byUrl) return { row: byUrl, reason: "repo_url", hint: repoUrl.trim(), score: 100 };
  }

  if (project.trim()) {
    const exact = projects.find((row) => row.name.toLowerCase() === project.trim().toLowerCase());
    if (exact?.repo_path) return { row: exact, reason: "exact_project", hint: project.trim(), score: 100 };
  }

  for (const hint of projectHints(project, title, description)) {
    const scored = projects
      .filter((row) => row.repo_path)
      .map((row) => ({ row, score: scoreProjectHint(hint, row.name) }))
      .filter((item) => item.score >= 65)
      .sort((a, b) => b.score - a.score || a.row.name.localeCompare(b.row.name));
    const best = scored[0];
    if (!best) continue;
    const second = scored[1]?.score ?? 0;
    if (best.score >= 94 || best.score - second >= 4) {
      return { row: best.row, reason: "hint", hint, score: best.score };
    }
  }

  return null;
}

const ATTACHMENT_BUCKET = "task-attachments";
const MAX_ATTACHMENT_BYTES = 20 * 1024 * 1024; // 20 MB per file

function guessExtension(mime: string, name?: string): string {
  const fromName = name?.includes(".") ? name.slice(name.lastIndexOf(".")) : "";
  if (fromName) return fromName;
  const map: Record<string, string> = {
    "image/png": ".png",
    "image/jpeg": ".jpg",
    "image/jpg": ".jpg",
    "image/gif": ".gif",
    "image/webp": ".webp",
    "image/svg+xml": ".svg",
    "application/pdf": ".pdf",
    "text/plain": ".txt",
  };
  return map[mime] ?? ".bin";
}

function parseDataUrl(s: string): { mime: string; bytes: Uint8Array } | null {
  const m = s.match(/^data:([^;]+);base64,(.*)$/s);
  if (!m) return null;
  const mime = m[1];
  const b64 = m[2];
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return { mime, bytes };
}

function decodeBase64(raw: string): Uint8Array {
  const bin = atob(raw.replace(/\s+/g, ""));
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

async function ingestAttachment(
  sb: ReturnType<typeof createClient>,
  supabaseUrl: string,
  entry: AttachmentInput,
): Promise<StoredAttachment | null> {
  if (typeof entry === "string") {
    const s = entry.trim();
    if (!s) return null;
    if (s.startsWith("data:")) {
      const parsed = parseDataUrl(s);
      if (!parsed) throw new Error("invalid data URL");
      return await uploadBytes(sb, supabaseUrl, parsed.bytes, parsed.mime, undefined);
    }
    if (s.startsWith("http://") || s.startsWith("https://")) {
      return { url: s, name: s.split("/").pop() ?? "attachment", mime: "application/octet-stream" };
    }
    throw new Error("attachment string must be https://, http://, or data: URL");
  }

  if (!entry || typeof entry !== "object") return null;

  if (typeof entry.url === "string" && entry.url) {
    return {
      url: entry.url,
      name: entry.name ?? (entry.url.split("/").pop() ?? "attachment"),
      mime: entry.mime ?? "application/octet-stream",
    };
  }

  if (typeof entry.data === "string" && entry.data) {
    if (entry.data.startsWith("data:")) {
      const parsed = parseDataUrl(entry.data);
      if (!parsed) throw new Error("invalid data URL");
      return await uploadBytes(sb, supabaseUrl, parsed.bytes, entry.mime ?? parsed.mime, entry.name);
    }
    const bytes = decodeBase64(entry.data);
    return await uploadBytes(sb, supabaseUrl, bytes, entry.mime ?? "application/octet-stream", entry.name);
  }

  return null;
}

async function uploadBytes(
  sb: ReturnType<typeof createClient>,
  supabaseUrl: string,
  bytes: Uint8Array,
  mime: string,
  name?: string,
): Promise<StoredAttachment> {
  if (bytes.byteLength === 0) throw new Error("empty file");
  if (bytes.byteLength > MAX_ATTACHMENT_BYTES) {
    throw new Error(`file too large (${bytes.byteLength} bytes, max ${MAX_ATTACHMENT_BYTES})`);
  }
  const ext = guessExtension(mime, name);
  const key = `${crypto.randomUUID()}${ext}`;
  const { error } = await sb.storage
    .from(ATTACHMENT_BUCKET)
    .upload(key, bytes, { contentType: mime, upsert: false });
  if (error) throw new Error(`storage upload: ${error.message}`);
  const url = `${supabaseUrl}/storage/v1/object/public/${ATTACHMENT_BUCKET}/${key}`;
  return { url, name: name ?? key, mime };
}

Deno.serve(async (req) => {
  if (req.method === "OPTIONS") {
    return new Response("ok", {
      headers: {
        "access-control-allow-origin": "*",
        "access-control-allow-methods": "POST, OPTIONS",
        "access-control-allow-headers": "content-type, x-webhook-secret",
      },
    });
  }

  if (req.method !== "POST") {
    return json(405, { error: "POST only" });
  }

  const expectedSecret = Deno.env.get("TASK_WEBHOOK_SECRET");
  if (!expectedSecret) {
    return json(500, { error: "Webhook secret not configured on server" });
  }
  const gotSecret = req.headers.get("x-webhook-secret");
  if (gotSecret !== expectedSecret) {
    return json(401, { error: "Invalid or missing x-webhook-secret" });
  }

  let body: Body;
  try {
    body = await req.json();
  } catch {
    return json(400, { error: "Body must be valid JSON" });
  }

  const title = (body.title ?? "").trim();
  const description = (body.description ?? "").trim();
  if (!title) return json(400, { error: "title is required" });
  if (!description) return json(400, { error: "description is required" });

  const supabaseUrl = Deno.env.get("SUPABASE_URL")!;
  const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
  const sb = createClient(supabaseUrl, serviceKey);

  // Load registry once for project lookup + inference + backfill.
  const { data: projects, error: projErr } = await sb
    .from("ae_projects")
    .select("name, repo_url, repo_path, preview_url");
  if (projErr) {
    return json(500, { error: `Failed to load ae_projects: ${projErr.message}` });
  }

  let project = (body.project ?? "").trim();
  const repoUrl = (body.repo_url ?? "").trim();
  const resolution = resolveProject(project, repoUrl, title, description, projects ?? []);
  if (resolution) project = resolution.row.name;

  const task: Record<string, unknown> = {
    title,
    description,
    status: "queued",
    priority: body.priority ?? "medium",
    task_type: body.task_type ?? "code",
    source: body.source ?? "webhook",
  };

  if (project) {
    task.project = project;
    const row = resolution?.row ?? (projects ?? []).find((p) => p.name === project);
    if (row) {
      if (row.repo_url) task.repo_url = row.repo_url;
      if (row.repo_path) task.repo_path = row.repo_path;
      if (row.preview_url) task.preview_url = row.preview_url;
    }
  }
  if (!task.repo_url && repoUrl) task.repo_url = repoUrl;
  if (resolution) {
    task.context = {
      project_resolution: {
        reason: resolution.reason,
        hint: resolution.hint,
        score: resolution.score,
        previous_project: body.project ?? null,
      },
    };
  }

  if (body.base_branch) task.base_branch = body.base_branch;

  if (typeof body.callback_url === "string") {
    const cb = body.callback_url.trim();
    if (cb) {
      if (!/^https?:\/\//i.test(cb)) {
        return json(400, { error: "callback_url must be http:// or https://" });
      }
      task.callback_url = cb;
      if (typeof body.callback_secret === "string" && body.callback_secret.trim()) {
        task.callback_secret = body.callback_secret.trim();
      }
    }
  }

  if (typeof body.origin_system === "string") {
    const os = body.origin_system.trim() as OriginSystem;
    if (os) {
      if (!ORIGIN_SYSTEMS.has(os)) {
        return json(400, {
          error: `origin_system must be one of: ${[...ORIGIN_SYSTEMS].join(", ")}`,
        });
      }
      task.origin_system = os;
    }
  }
  if (typeof body.origin_id === "string" && body.origin_id.trim()) {
    task.origin_id = body.origin_id.trim();
  }
  if (typeof body.origin_url === "string") {
    const ou = body.origin_url.trim();
    if (ou) {
      if (!/^https?:\/\//i.test(ou)) {
        return json(400, { error: "origin_url must be http:// or https://" });
      }
      task.origin_url = ou;
    }
  }

  if (Array.isArray(body.attachments) && body.attachments.length > 0) {
    const stored: StoredAttachment[] = [];
    for (const entry of body.attachments) {
      try {
        const saved = await ingestAttachment(sb, supabaseUrl, entry);
        if (saved) stored.push(saved);
      } catch (e) {
        return json(400, { error: `Attachment ingest failed: ${e instanceof Error ? e.message : String(e)}` });
      }
    }
    if (stored.length > 0) task.attachments = stored;
  }

  const { data: inserted, error: insErr } = await sb
    .from("ae_tasks")
    .insert(task)
    .select("id, title, project, status, priority")
    .single();

  if (insErr) {
    return json(500, { error: `Insert failed: ${insErr.message}` });
  }

  return json(200, {
    ok: true,
    task: inserted,
    note: project
      ? `Task queued on ${project}. Sam will pick it up within ~10s.`
      : "Task queued but no project was resolved; Sam may ask for clarification.",
  });
});
