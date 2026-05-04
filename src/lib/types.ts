/** Agent One - Supabase table types */

export type TaskStatus = 'queued' | 'in_progress' | 'testing' | 'review' | 'fixes_needed' | 'approved' | 'done' | 'failed' | 'pending_confirmation';
export type TaskPriority = 'critical' | 'high' | 'medium' | 'low';
export type TaskSource = 'manual' | 'trigger' | 'cron' | 'chat';
export type TaskType = 'code' | 'research';
export type MessageRole = 'user' | 'agent' | 'system';
export type CommentAuthor = 'matt' | 'agent' | 'system';
export type WorkerStatus = 'online' | 'offline' | 'busy';
export type TriggerSourceType = 'supabase' | 'webhook' | 'github' | 'triage';
export type OriginSystem = 'operly_triage' | 'banana_triage' | 'sentry' | 'manual';

export interface Subtask {
	id: string;
	title: string;
	done: boolean;
	order: number;
}

export interface AeTask {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  project: string | null;
  source: TaskSource;
  task_type: TaskType;
  repo_url: string | null;
  repo_path: string | null;
  branch: string | null;
  base_branch: string | null;
  preview_url: string | null;
  pr_url: string | null;
  pr_number: number | null;
  report_url: string | null;
  screenshots: unknown[] | null;
  screenshots_before: string[] | null;
  screenshots_after: string[] | null;
  visual_qa_result: { pass: boolean; explanation: string } | null;
  assignee: string;
  worker_id: string | null;
  trigger_id: string | null;
  cron_id: string | null;
  context: Record<string, unknown> | null;
  subtasks: Subtask[] | null;
  created_at: string;
  updated_at: string;
  claimed_at: string | null;
  completed_at: string | null;
  review_scores: Record<string, number> | null;
  review_summary: string | null;
  auto_merged: boolean | null;
  auto_merge_blocked_reason: string | null;
  failure_reason?: string | null;
  origin_system?: OriginSystem | null;
  origin_id?: string | null;
  origin_url?: string | null;
  callback_url?: string | null;
  on_hold?: boolean;
  commit_message?: string | null;
}

export interface OriginBadgeMeta {
  label: string;
  color: string;
  bg: string;
  border: string;
}

export const ORIGIN_BADGES: Record<Exclude<OriginSystem, 'manual'>, OriginBadgeMeta> = {
  operly_triage: {
    label: 'Operly',
    color: '#a78bfa',
    bg: 'rgba(167, 139, 250, 0.10)',
    border: 'rgba(167, 139, 250, 0.32)',
  },
  banana_triage: {
    label: 'Banana',
    color: '#facc15',
    bg: 'rgba(250, 204, 21, 0.10)',
    border: 'rgba(250, 204, 21, 0.30)',
  },
  sentry: {
    label: 'Sentry',
    color: '#fb7185',
    bg: 'rgba(251, 113, 133, 0.10)',
    border: 'rgba(251, 113, 133, 0.30)',
  },
};

export interface AeComment {
  id: string;
  task_id: string;
  author: CommentAuthor;
  content: string;
  mentions: string[];
  created_at: string;
}

export interface AeMessage {
  id: string;
  conversation_id: string;
  role: MessageRole;
  content: string;
  task_id: string | null;
  attachments: unknown[] | null;
  needs_response?: boolean;
  created_at: string;
}

export interface AeCron {
  id: string;
  name: string;
  schedule: string;
  task_template: Record<string, unknown>;
  enabled: boolean;
  last_run: string | null;
  next_run: string | null;
  created_at: string;
}

export interface AeTrigger {
  id: string;
  name: string;
  source_type: TriggerSourceType;
  source_config: Record<string, unknown>;
  task_template: Record<string, unknown>;
  enabled: boolean;
  last_checked: string | null;
  created_at: string;
}

export interface AeTriggerEvent {
  id: string;
  trigger_id: string;
  payload: Record<string, unknown>;
  processed: boolean;
  created_at: string;
}

export interface AeWorker {
  id: string;
  machine_name: string;
  status: WorkerStatus;
  current_task_id: string | null;
  last_heartbeat: string;
  created_at: string;
}

export interface AeProject {
  id: string;
  name: string;
  repo_url: string | null;
  repo_path: string | null;
  preview_url: string | null;
  client: string | null;
  deploy_method: string | null;
  dev_command: string | null;
  created_at: string;
}

/** Kanban column definition */
export interface KanbanColumn {
  status: TaskStatus;
  label: string;
  color: string;
  glowColor: string;
  icon: string;
}

export const KANBAN_COLUMNS: KanbanColumn[] = [
  { status: 'queued', label: 'Queued', color: '#6e7681', glowColor: 'rgba(110, 118, 129, 0.15)', icon: '()' },
  { status: 'pending_confirmation', label: 'Awaiting Confirmation', color: '#d29922', glowColor: 'rgba(210, 153, 34, 0.18)', icon: '?' },
  { status: 'in_progress', label: 'In Progress', color: '#6366f1', glowColor: 'rgba(99, 102, 241, 0.2)', icon: '>>' },
  { status: 'testing', label: 'Testing', color: '#f59e0b', glowColor: 'rgba(245, 158, 11, 0.15)', icon: '??' },
  { status: 'review', label: 'Review', color: '#3fb950', glowColor: 'rgba(63, 185, 80, 0.15)', icon: 'PR' },
  { status: 'fixes_needed', label: 'Fixes Needed', color: '#f97316', glowColor: 'rgba(249, 115, 22, 0.18)', icon: '!!' },
  { status: 'approved', label: 'Ready to Merge', color: '#58a6ff', glowColor: 'rgba(88, 166, 255, 0.15)', icon: '++' },
  { status: 'done', label: 'Done', color: '#8b949e', glowColor: 'rgba(139, 148, 158, 0.1)', icon: 'ok' },
  { status: 'failed', label: 'Failed', color: '#f85149', glowColor: 'rgba(248, 81, 73, 0.2)', icon: 'X' },
];

export const PRIORITY_COLORS: Record<TaskPriority, string> = {
  critical: '#f85149',
  high: '#d29922',
  medium: '#6366f1',
  low: '#6e7681',
};

export const SOURCE_ICONS: Record<TaskSource, string> = {
  manual: '+',
  trigger: '!',
  cron: '@',
  chat: '#',
};
