export type TaskStatus =
  | 'queued'
  | 'in_progress'
  | 'testing'
  | 'review'
  | 'approved'
  | 'done'
  | 'failed'
  | 'pending_confirmation';
export type TaskPriority = 'critical' | 'high' | 'medium' | 'low';

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
  source: string;
  task_type: string;
  repo_url: string | null;
  branch: string | null;
  base_branch: string | null;
  preview_url: string | null;
  pr_url: string | null;
  pr_number: number | null;
  screenshots_before: string[] | null;
  screenshots_after: string[] | null;
  visual_qa_result: { pass: boolean; explanation: string } | null;
  assignee: string;
  worker_id: string | null;
  subtasks: Subtask[] | null;
  failure_reason?: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface AeComment {
  id: string;
  task_id: string;
  author: 'matt' | 'agent' | 'system';
  content: string;
  created_at: string;
}

export const STATUSES: TaskStatus[] = [
  'queued',
  'in_progress',
  'testing',
  'review',
  'approved',
  'done',
  'failed'
];

export const STATUS_LABEL: Record<TaskStatus, string> = {
  queued: 'Queued',
  in_progress: 'In Progress',
  testing: 'Testing',
  review: 'Review',
  approved: 'Approved',
  done: 'Done',
  failed: 'Failed',
  pending_confirmation: 'Awaiting Confirmation'
};

export const PRIORITY_COLOR: Record<TaskPriority, string> = {
  critical: 'bg-rose-500/20 text-rose-300 border-rose-500/40',
  high: 'bg-amber-500/20 text-amber-300 border-amber-500/40',
  medium: 'bg-sky-500/20 text-sky-300 border-sky-500/40',
  low: 'bg-slate-500/20 text-slate-300 border-slate-500/40'
};
