// task-webhook: generic inbound endpoint for firing tasks at Sam from
// external systems (Sentry, GitHub, shell scripts, anything that can POST).
//
// Auth: `x-webhook-secret` header must equal TASK_WEBHOOK_SECRET env var.
// Minimum body: { title, description }
// Optional body: project, priority, task_type, source, base_branch, attachments
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

type Body = {
  title?: string;
  description?: string;
  project?: string;
  priority?: "critical" | "high" | "medium" | "low";
  task_type?: "code" | "research";
  source?: string;
  base_branch?: string;
  attachments?: AttachmentInput[];
};

type StoredAttachment = { url: string; name: string; mime: string };

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
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
  if (!project && projects && projects.length > 0) {
    const haystack = `${title.toLowerCase()}\n${description.toLowerCase()}`;
    const names = projects.map((p) => p.name as string).sort((a, b) => b.length - a.length);
    for (const n of names) {
      if (haystack.includes(n.toLowerCase())) {
        project = n;
        break;
      }
    }
  }

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
    const row = (projects ?? []).find((p) => p.name === project);
    if (row) {
      if (row.repo_url) task.repo_url = row.repo_url;
      if (row.repo_path) task.repo_path = row.repo_path;
      if (row.preview_url) task.preview_url = row.preview_url;
    }
  }

  if (body.base_branch) task.base_branch = body.base_branch;

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
