// task-webhook: generic inbound endpoint for firing tasks at Sam from
// external systems (Sentry, GitHub, shell scripts, anything that can POST).
//
// Auth: `x-webhook-secret` header must equal TASK_WEBHOOK_SECRET env var.
// Minimum body: { title, description }
// Optional body: project, priority, task_type, source, base_branch
//
// Behavior:
//   - If project omitted, try to infer by scanning title+description for any
//     registered name in ae_projects.
//   - If project resolved, backfill repo_url and repo_path from the registry.
//   - Insert into ae_tasks with status="queued". Sam's worker picks it up on
//     its next poll (~10s cadence).

import { createClient } from "https://esm.sh/@supabase/supabase-js@2.45.4";

type Body = {
  title?: string;
  description?: string;
  project?: string;
  priority?: "critical" | "high" | "medium" | "low";
  task_type?: "code" | "research";
  source?: string;
  base_branch?: string;
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
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
