// Agent One API - Webhook receiver + CRUD for crons, triggers, and tasks
// Deploy: supabase functions deploy agent-api --project-ref iycloielqcjnjqddeuet

import { createClient } from "https://esm.sh/@supabase/supabase-js@2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SUPABASE_SERVICE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;

const supabase = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY);

const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, PATCH, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization, x-api-key",
};

function json(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { ...corsHeaders, "Content-Type": "application/json" },
  });
}

function err(message: string, status = 400) {
  return json({ error: message }, status);
}

// Constant-time API key auth - checks x-api-key header against stored secret
// Webhook endpoints skip auth (they use trigger_id as the "key")
function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

function requireAuth(req: Request): boolean {
  const apiKey = req.headers.get("x-api-key");
  const expected = Deno.env.get("AGENT_API_KEY");
  if (!expected) return false; // No key configured = locked down
  if (!apiKey) return false;
  return timingSafeEqual(apiKey, expected);
}

// Field whitelists for PATCH operations
const CRON_FIELDS = new Set(["name", "schedule", "task_template", "enabled", "next_run"]);
const TRIGGER_FIELDS = new Set(["name", "source_type", "source_config", "task_template", "enabled"]);
const TASK_FIELDS = new Set(["title", "description", "status", "priority", "task_type", "project", "repo_url", "repo_path", "assignee", "context", "branch", "pr_url", "pr_number"]);

const VALID_STATUS = new Set(["queued", "in_progress", "testing", "review", "approved", "done", "failed"]);
const VALID_PRIORITY = new Set(["critical", "high", "medium", "low"]);
const VALID_TASK_TYPE = new Set(["code", "research"]);
const VALID_SOURCE = new Set(["manual", "trigger", "cron", "chat"]);
const VALID_SOURCE_TYPE = new Set(["supabase", "webhook", "github", "triage"]);

function pick(obj: Record<string, unknown>, allowed: Set<string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const key of Object.keys(obj)) {
    if (allowed.has(key)) result[key] = obj[key];
  }
  return result;
}

function validateEnums(b: Record<string, unknown>): string | null {
  if (b.status !== undefined && !VALID_STATUS.has(b.status as string)) return `Invalid status: ${b.status}`;
  if (b.priority !== undefined && !VALID_PRIORITY.has(b.priority as string)) return `Invalid priority: ${b.priority}`;
  if (b.task_type !== undefined && !VALID_TASK_TYPE.has(b.task_type as string)) return `Invalid task_type: ${b.task_type}`;
  if (b.source !== undefined && !VALID_SOURCE.has(b.source as string)) return `Invalid source: ${b.source}`;
  if (b.source_type !== undefined && !VALID_SOURCE_TYPE.has(b.source_type as string)) return `Invalid source_type: ${b.source_type}`;
  return null;
}

// Parse route: /agent-api/crons/abc-123 -> { resource: "crons", id: "abc-123", action: null }
// Parse route: /agent-api/crons/abc-123/fire -> { resource: "crons", id: "abc-123", action: "fire" }
function parseRoute(url: URL): { resource: string; id: string | null; action: string | null } {
  // pathname is like /agent-api/crons/abc-123/fire or /agent-api/webhook/trigger-id
  const parts = url.pathname.replace(/^\/agent-api\/?/, "").split("/").filter(Boolean);
  return {
    resource: parts[0] || "",
    id: parts[1] || null,
    action: parts[2] || null,
  };
}

// ─── Webhook handler ─────────────────────────────────────────────
// POST /agent-api/webhook/:trigger_id
// No auth required - the trigger_id itself acts as an unguessable token
async function handleWebhook(triggerId: string, body: unknown) {
  // Verify trigger exists and is enabled
  const { data: trigger, error: trigErr } = await supabase
    .from("ae_triggers")
    .select("*")
    .eq("id", triggerId)
    .single();

  if (trigErr || !trigger) return err("Trigger not found", 404);
  if (!trigger.enabled) return err("Trigger is disabled", 403);

  // Insert event into ae_trigger_events for the worker to pick up
  const { data: event, error: evtErr } = await supabase
    .from("ae_trigger_events")
    .insert({
      trigger_id: triggerId,
      payload: body || {},
      processed: false,
    })
    .select()
    .single();

  if (evtErr) return err(`Failed to create event: ${evtErr.message}`, 500);

  return json({ ok: true, event_id: event.id, message: "Webhook received, task will be created by worker" }, 201);
}

// ─── Crons CRUD ──────────────────────────────────────────────────
async function handleCrons(method: string, id: string | null, action: string | null, body: unknown) {
  // POST /crons/:id/fire - manually fire a cron (create task immediately)
  if (method === "POST" && id && action === "fire") {
    const { data: cron, error: cronErr } = await supabase
      .from("ae_crons")
      .select("*")
      .eq("id", id)
      .single();

    if (cronErr || !cron) return err("Cron not found", 404);

    const template = cron.task_template as Record<string, unknown>;

    // Fan-out crons (`repo_parent`) need filesystem access on the worker to
    // enumerate git subdirs. The Edge Function can't do that, so backdate
    // next_run instead — the worker will pick it up on its next tick (~60s)
    // and run the proper fan-out path.
    if (typeof template.repo_parent === "string" && template.repo_parent.trim().length > 0) {
      const due = new Date(Date.now() - 1000).toISOString();
      const { error: updErr } = await supabase
        .from("ae_crons")
        .update({ next_run: due })
        .eq("id", id);
      if (updErr) return err(`Failed to queue cron: ${updErr.message}`, 500);
      return json(
        { ok: true, queued: true, message: "Fan-out cron queued; worker will fire on its next tick (~60s)." },
        202,
      );
    }

    const { data: task, error: taskErr } = await supabase
      .from("ae_tasks")
      .insert({
        title: template.title || cron.name,
        description: template.description || null,
        status: (template.status as string) || "queued",
        priority: (template.priority as string) || "medium",
        task_type: (template.task_type as string) || "code",
        project: (template.project as string) || null,
        repo_url: (template.repo_url as string) || null,
        repo_path: (template.repo_path as string) || null,
        source: "cron",
        cron_id: cron.id,
        assignee: "sam",
        context: template.context || null,
      })
      .select()
      .single();

    if (taskErr) return err(`Failed to create task: ${taskErr.message}`, 500);

    // Update last_run
    await supabase.from("ae_crons").update({ last_run: new Date().toISOString() }).eq("id", id);

    return json({ ok: true, task }, 201);
  }

  // GET /crons - list all
  if (method === "GET" && !id) {
    const { data, error } = await supabase.from("ae_crons").select("*").order("created_at", { ascending: false });
    if (error) return err(error.message, 500);
    return json(data);
  }

  // GET /crons/:id
  if (method === "GET" && id) {
    const { data, error } = await supabase.from("ae_crons").select("*").eq("id", id).single();
    if (error) return err("Cron not found", 404);
    return json(data);
  }

  // POST /crons - create
  if (method === "POST" && !id) {
    const b = body as Record<string, unknown>;
    if (!b?.name || !b?.schedule) return err("name and schedule are required");

    const { data, error } = await supabase
      .from("ae_crons")
      .insert({
        name: b.name,
        schedule: b.schedule,
        task_template: b.task_template || {},
        enabled: b.enabled !== false,
        next_run: b.next_run || null,
      })
      .select()
      .single();

    if (error) return err(error.message, 500);
    return json(data, 201);
  }

  // PATCH /crons/:id - update
  if (method === "PATCH" && id) {
    const updates = pick(body as Record<string, unknown>, CRON_FIELDS);
    if (Object.keys(updates).length === 0) return err("No valid fields to update");
    const { data, error } = await supabase.from("ae_crons").update(updates).eq("id", id).select().single();
    if (error) return err(error.message, 500);
    return json(data);
  }

  // DELETE /crons/:id
  if (method === "DELETE" && id) {
    const { error } = await supabase.from("ae_crons").delete().eq("id", id);
    if (error) return err(error.message, 500);
    return json({ ok: true, deleted: id });
  }

  return err("Method not allowed", 405);
}

// ─── Triggers CRUD ───────────────────────────────────────────────
async function handleTriggers(method: string, id: string | null, _action: string | null, body: unknown) {
  // GET /triggers
  if (method === "GET" && !id) {
    const { data, error } = await supabase.from("ae_triggers").select("*").order("created_at", { ascending: false });
    if (error) return err(error.message, 500);
    return json(data);
  }

  // GET /triggers/:id
  if (method === "GET" && id) {
    const { data, error } = await supabase.from("ae_triggers").select("*").eq("id", id).single();
    if (error) return err("Trigger not found", 404);
    return json(data);
  }

  // POST /triggers - create
  if (method === "POST" && !id) {
    const b = body as Record<string, unknown>;
    if (!b?.name || !b?.source_type) return err("name and source_type are required");
    const enumErr = validateEnums(b);
    if (enumErr) return err(enumErr);

    const { data, error } = await supabase
      .from("ae_triggers")
      .insert({
        name: b.name,
        source_type: b.source_type,
        source_config: b.source_config || {},
        task_template: b.task_template || {},
        enabled: b.enabled !== false,
      })
      .select()
      .single();

    if (error) return err(error.message, 500);
    return json(data, 201);
  }

  // PATCH /triggers/:id
  if (method === "PATCH" && id) {
    const updates = pick(body as Record<string, unknown>, TRIGGER_FIELDS);
    if (Object.keys(updates).length === 0) return err("No valid fields to update");
    const enumErr = validateEnums(updates);
    if (enumErr) return err(enumErr);
    const { data, error } = await supabase.from("ae_triggers").update(updates).eq("id", id).select().single();
    if (error) return err(error.message, 500);
    return json(data);
  }

  // DELETE /triggers/:id
  if (method === "DELETE" && id) {
    const { error } = await supabase.from("ae_triggers").delete().eq("id", id);
    if (error) return err(error.message, 500);
    return json({ ok: true, deleted: id });
  }

  return err("Method not allowed", 405);
}

// ─── Tasks - direct creation ─────────────────────────────────────
async function handleTasks(method: string, id: string | null, _action: string | null, body: unknown) {
  // GET /tasks - list (with optional status filter)
  if (method === "GET" && !id) {
    const { data, error } = await supabase
      .from("ae_tasks")
      .select("*")
      .order("created_at", { ascending: false })
      .limit(50);
    if (error) return err(error.message, 500);
    return json(data);
  }

  // POST /tasks - create task directly via API
  if (method === "POST" && !id) {
    const b = body as Record<string, unknown>;
    if (!b?.title) return err("title is required");
    const enumErr = validateEnums(b);
    if (enumErr) return err(enumErr);

    const { data, error } = await supabase
      .from("ae_tasks")
      .insert({
        title: b.title,
        description: (b.description as string) || null,
        status: (b.status as string) || "queued",
        priority: (b.priority as string) || "medium",
        task_type: (b.task_type as string) || "code",
        project: (b.project as string) || null,
        repo_url: (b.repo_url as string) || null,
        repo_path: (b.repo_path as string) || null,
        source: (b.source as string) || "manual",
        assignee: (b.assignee as string) || "sam",
        context: (b.context as Record<string, unknown>) || null,
      })
      .select()
      .single();

    if (error) return err(error.message, 500);
    return json(data, 201);
  }

  // PATCH /tasks/:id - update task status etc
  if (method === "PATCH" && id) {
    const updates = pick(body as Record<string, unknown>, TASK_FIELDS);
    if (Object.keys(updates).length === 0) return err("No valid fields to update");
    const enumErr = validateEnums(updates);
    if (enumErr) return err(enumErr);
    const { data, error } = await supabase.from("ae_tasks").update(updates).eq("id", id).select().single();
    if (error) return err(error.message, 500);
    return json(data);
  }

  return err("Method not allowed", 405);
}

// ─── Main router ─────────────────────────────────────────────────
Deno.serve(async (req) => {
  // CORS preflight
  if (req.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: corsHeaders });
  }

  const url = new URL(req.url);
  const { resource, id, action } = parseRoute(url);

  // Parse body for non-GET requests
  let body: unknown = null;
  if (req.method !== "GET" && req.method !== "DELETE") {
    try {
      body = await req.json();
    } catch {
      body = {};
    }
  }

  // Webhook endpoint - no auth required
  if (resource === "webhook") {
    if (req.method !== "POST") return err("POST only", 405);
    if (!id) return err("Trigger ID required: POST /agent-api/webhook/:trigger_id");
    return handleWebhook(id, body);
  }

  // All other endpoints require API key (if configured)
  if (!requireAuth(req)) {
    return err("Invalid or missing x-api-key", 401);
  }

  // Health check
  if (resource === "health" || resource === "") {
    return json({ status: "ok", version: "1.0.0", routes: ["webhook", "crons", "triggers", "tasks"] });
  }

  switch (resource) {
    case "crons":
      return handleCrons(req.method, id, action, body);
    case "triggers":
      return handleTriggers(req.method, id, action, body);
    case "tasks":
      return handleTasks(req.method, id, action, body);
    default:
      return err(`Unknown resource: ${resource}`, 404);
  }
});
