import { json, error } from '@sveltejs/kit';
import { getSupabaseAdmin } from '$lib/server/supabase-admin';
import type { RequestHandler } from './$types';

type RepoMode = 'project' | 'none' | 'multiple';
type TaskType = 'code' | 'research' | 'qa-verify';

function cleanString(value: unknown) {
  return typeof value === 'string' ? value.trim() : '';
}

function cleanBaseBranch(value: unknown) {
  let branch = cleanString(value).replace(/^['"`]+|['"`]+$/g, '');
  branch = branch.replace(/^refs\/heads\//, '').replace(/^origin\//, '');
  branch = branch.replace(/\\+$/g, '').trim();
  if (!branch) return null;
  if (!/^[A-Za-z0-9._/-]+$/.test(branch)) return null;
  if (
    branch.startsWith('/') ||
    branch.endsWith('/') ||
    branch.includes('//') ||
    branch.includes('..') ||
    branch.includes('@{')
  ) {
    return null;
  }
  return branch;
}

function titleFromPrompt(prompt: string) {
  const first = prompt
    .split(/\n+/)
    .map((line) => line.trim())
    .find(Boolean) || 'New Samwise task';
  return first.length > 90 ? `${first.slice(0, 87).trimEnd()}...` : first;
}

function repoModeFrom(value: unknown): RepoMode {
  return value === 'none' || value === 'multiple' ? value : 'project';
}

function taskTypeFrom(value: unknown): TaskType {
  if (value === 'research') return 'research';
  if (value === 'qa-verify') return 'qa-verify';
  return 'code';
}

export const POST: RequestHandler = async ({ request }) => {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'invalid JSON');
  }

  const payload = body as Record<string, unknown>;
  const prompt = cleanString(payload.prompt) || cleanString(payload.description) || cleanString(payload.title);
  if (!prompt) throw error(400, 'prompt required');

  const repoMode = repoModeFrom(payload.repo_mode);
  const taskType = taskTypeFrom(payload.task_type);
  const supabase = getSupabaseAdmin();

  let projectName: string | null = null;
  let repoUrl: string | null = null;
  let repoPath: string | null = null;
  let previewUrl: string | null = null;

  if (repoMode === 'project') {
    const projectId = cleanString(payload.project_id);
    const projectNameInput = cleanString(payload.project);

    let query = supabase
      .from('ae_projects')
      .select('id,name,repo_url,repo_path,preview_url')
      .limit(1);

    if (projectId) {
      query = query.eq('id', projectId);
    } else if (projectNameInput) {
      query = query.eq('name', projectNameInput);
    } else {
      throw error(400, 'repo required');
    }

    const { data, error: projectError } = await query.maybeSingle();
    if (projectError) throw error(500, projectError.message);
    if (!data) throw error(400, 'selected repo not found');

    projectName = data.name ?? null;
    repoUrl = data.repo_url ?? null;
    repoPath = data.repo_path ?? null;
    previewUrl = data.preview_url ?? null;
  }

  const context: Record<string, unknown> = {
    repo_mode: repoMode,
    original_prompt: prompt,
  };
  if (repoMode === 'none') context.repo_label = 'No repo';
  if (repoMode === 'multiple') context.repo_label = 'Multiple repos';
  if (repoMode === 'project') context.project_id = cleanString(payload.project_id) || undefined;

  // qa-verify: stamp the environment and let the worker resolve the QA target
  // (staging preview_url vs production_url) at run time. Don't pin the project
  // preview_url here unless the caller passed an explicit override.
  if (taskType === 'qa-verify') {
    context.qa_environment = payload.environment === 'production' ? 'production' : 'staging';
    const explicitPreview = cleanString(payload.preview_url);
    previewUrl = explicitPreview || null;
  }

  const attachments = Array.isArray(payload.attachments) ? payload.attachments : undefined;

  const row: Record<string, unknown> = {
    title: cleanString(payload.title) || titleFromPrompt(prompt),
    description: prompt,
    status: 'queued',
    priority: 'medium',
    task_type: taskType,
    source: 'web-board',
    project: projectName,
    repo_url: repoUrl,
    repo_path: repoPath,
    preview_url: previewUrl,
    base_branch: cleanBaseBranch(payload.base_branch),
    context,
  };

  if (attachments && attachments.length > 0) row.attachments = attachments;

  const { data, error: insertError } = await supabase
    .from('ae_tasks')
    .insert(row)
    .select()
    .single();

  if (insertError) {
    return json({ ok: false, error: insertError.message }, { status: 500 });
  }

  return json({ ok: true, result: data });
};
