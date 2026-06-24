import { supabase } from '$lib/supabase';
import type { AeTask, AeComment, TaskStatus } from '$lib/types';

const ACTIVE_STATUSES: TaskStatus[] = [
  'queued',
  'pending_confirmation',
  'in_progress',
  'testing',
  'review',
  'fixes_needed',
  'approved',
  'qa'
];

class TasksStore {
  tasks = $state<AeTask[]>([]);
  comments = $state<Record<string, AeComment[]>>({});
  loading = $state(true);
  error = $state<string | null>(null);
  connected = $state(false);

  private channel: ReturnType<typeof supabase.channel> | null = null;

  async refresh() {
    try {
      const active = await supabase
        .from('ae_tasks')
        .select('*')
        .in('status', ACTIVE_STATUSES)
        .order('priority', { ascending: true })
        .order('created_at', { ascending: true })
        .limit(1000);
      if (active.error) throw active.error;

      const terminal = await supabase
        .from('ae_tasks')
        .select('*')
        .in('status', ['done', 'failed'])
        .order('updated_at', { ascending: false })
        .limit(250);
      if (terminal.error) throw terminal.error;

      const seen = new Set<string>();
      const rows = [...(active.data ?? []), ...(terminal.data ?? [])].filter((task) => {
        if (seen.has(task.id)) return false;
        seen.add(task.id);
        return true;
      });
      this.tasks = rows as AeTask[];
      this.error = null;
      this.prefetchReviewComments();
      if (!this.channel) this.subscribeRealtime();
      return true;
    } catch (e: unknown) {
      this.error = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  async init() {
    const ok = await this.refresh();
    this.loading = false;
    if (!ok) return;
    if (!this.channel) this.subscribeRealtime();
  }

  private subscribeRealtime() {
    this.channel = supabase
      .channel('samwise-board')
      .on(
        'postgres_changes',
        { event: '*', schema: 'public', table: 'ae_tasks' },
        (payload) => this.applyTaskChange(payload)
      )
      .on(
        'postgres_changes',
        { event: '*', schema: 'public', table: 'ae_comments' },
        (payload) => this.applyCommentChange(payload)
      )
      .subscribe((status) => {
        this.connected = status === 'SUBSCRIBED';
      });
  }

  private applyTaskChange(payload: { eventType: string; new: Record<string, unknown>; old: Record<string, unknown> }) {
    if (payload.eventType === 'DELETE') {
      const id = payload.old?.id as string | undefined;
      if (id) this.tasks = this.tasks.filter((t) => t.id !== id);
      return;
    }
    const row = payload.new as unknown as AeTask;
    const idx = this.tasks.findIndex((t) => t.id === row.id);
    if (idx >= 0) {
      const next = this.tasks.slice();
      next[idx] = row;
      this.tasks = next;
    } else {
      this.tasks = [row, ...this.tasks];
    }
    if (this.shouldPrefetchComments(row)) void this.loadCommentsFor(row.id);
  }

  private applyCommentChange(payload: { eventType: string; new: Record<string, unknown>; old: Record<string, unknown> }) {
    if (payload.eventType === 'DELETE') {
      const id = payload.old?.id as string | undefined;
      const task_id = payload.old?.task_id as string | undefined;
      if (id && task_id && this.comments[task_id]) {
        this.comments = {
          ...this.comments,
          [task_id]: this.comments[task_id].filter((c) => c.id !== id)
        };
      }
      return;
    }
    const row = payload.new as unknown as AeComment;
    const list = this.comments[row.task_id] ?? [];
    const exists = list.some((c) => c.id === row.id);
    const merged = exists
      ? list.map((c) => (c.id === row.id ? row : c))
      : [...list, row];
    merged.sort((a, b) => a.created_at.localeCompare(b.created_at));
    this.comments = { ...this.comments, [row.task_id]: merged };
  }

  async updateTask(taskId: string, updates: Partial<AeTask>) {
    const idx = this.tasks.findIndex((t) => t.id === taskId);
    if (idx < 0) {
      this.error = 'Task no longer exists locally. Refresh and try again.';
      return false;
    }
    const prev = this.tasks[idx];
    const now = new Date().toISOString();
    const payload = { ...updates, updated_at: now };

    // Optimistic local update so the card moves instantly instead of waiting
    // on the realtime echo. The postgres_changes payload will overwrite this
    // with the server's canonical row.
    const next = this.tasks.slice();
    next[idx] = { ...prev, ...payload };
    this.tasks = next;

    const { error } = await supabase
      .from('ae_tasks')
      .update(payload)
      .eq('id', taskId);

    if (error) {
      // Roll back. Use a fresh index lookup in case realtime moved things.
      const curIdx = this.tasks.findIndex((t) => t.id === taskId);
      if (curIdx >= 0) {
        const rev = this.tasks.slice();
        rev[curIdx] = prev;
        this.tasks = rev;
      }
      this.error = error.message;
      return false;
    }
    this.error = null;
    return true;
  }

  async setStatus(taskId: string, status: TaskStatus) {
    const task = this.tasks.find((t) => t.id === taskId);
    if (!task || task.status === status) return;
    const updates: Partial<AeTask> = { status };
    if (status === 'done') updates.completed_at = new Date().toISOString();
    const ok = await this.updateTask(taskId, updates);
    if (ok && status === 'done') void this.closeOriginTicket(task);
  }

  /**
   * Force a stuck card back into `queued` and clear every field a stale
   * worker claim leaves behind. Mirrors the desktop store's requeueTask;
   * the worker treats rows with a populated `worker_id` as still claimed,
   * so a plain status flip can leave the card invisible to the queue.
   */
  async requeueTask(taskId: string) {
    const task = this.tasks.find((t) => t.id === taskId);
    if (!task) return false;
    return this.updateTask(taskId, {
      status: 'queued',
      worker_id: null,
      claimed_at: null,
      failure_reason: null,
    } as Partial<AeTask>);
  }

  async stopTask(taskId: string) {
    return this.updateTask(taskId, {
      status: 'failed',
      failure_reason: 'Stopped by user.',
      worker_id: null,
      claimed_at: null,
    } as Partial<AeTask>);
  }

  /**
   * Restart a failed task: kick it back to queued and clear the stale claim
   * so the worker re-picks it up. Semantically the same write as requeueTask
   * but expressed as a distinct action for the failed-card "Restart" button.
   */
  async restartTask(taskId: string) {
    const task = this.tasks.find((t) => t.id === taskId);
    if (!task) return false;
    return this.updateTask(taskId, {
      status: 'queued',
      worker_id: null,
      claimed_at: null,
      failure_reason: null,
    } as Partial<AeTask>);
  }

  /**
   * Post a comment as the given author. The realtime channel (ae_comments)
   * pushes the new row back into this.comments, so the caller does not need
   * to insert it locally.
   */
  async postComment(taskId: string, author: string, content: string): Promise<boolean> {
    try {
      const { error } = await supabase
        .from('ae_comments')
        .insert({ task_id: taskId, author, content });
      if (error) throw error;
      this.error = null;
      return true;
    } catch (e: unknown) {
      this.error = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  async deleteTask(taskId: string) {
    const { error } = await supabase.from('ae_tasks').delete().eq('id', taskId);
    if (error) {
      this.error = error.message;
      return false;
    }
    this.tasks = this.tasks.filter((t) => t.id !== taskId);
    this.error = null;
    return true;
  }

  private async closeOriginTicket(task: AeTask) {
    if (task.origin_system === 'manual' || task.source === 'manual') return;
    if (!task.origin_system && !task.callback_url) return;
    try {
      const res = await fetch('/api/close-origin-ticket', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ task_id: task.id })
      });
      if (!res.ok) {
        console.warn('[tasks] close-origin failed:', await res.text());
      }
    } catch (e) {
      console.warn('[tasks] close-origin failed:', e);
    }
  }

  /**
   * Close a task's GitHub PR without merging, via the close-pr edge function.
   * Returns true if the PR was closed (or was already closed/merged). Does NOT
   * touch the task status — the caller decides whether to then setStatus done.
   */
  async closePr(taskId: string): Promise<{ ok: boolean; error?: string }> {
    const task = this.tasks.find((t) => t.id === taskId);
    if (!task?.pr_url) return { ok: false, error: 'This task has no PR.' };
    try {
      const res = await fetch('/api/close-pr', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ task_id: taskId })
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data?.ok === false) {
        return { ok: false, error: data?.error || `close-pr failed (${res.status})` };
      }
      return { ok: true };
    } catch (e) {
      return { ok: false, error: e instanceof Error ? e.message : String(e) };
    }
  }

  async loadCommentsFor(taskId: string) {
    if (this.comments[taskId]) return;
    const { data } = await supabase
      .from('ae_comments')
      .select('*')
      .eq('task_id', taskId)
      .order('created_at', { ascending: true });
    this.comments = { ...this.comments, [taskId]: (data ?? []) as AeComment[] };
  }

  private shouldPrefetchComments(task: AeTask) {
    return task.status === 'in_progress' || task.status === 'testing' ||
      (!!task.pr_url && (task.status === 'review' || task.status === 'fixes_needed' || task.status === 'approved'));
  }

  private prefetchReviewComments() {
    for (const task of this.tasks) {
      if (this.shouldPrefetchComments(task)) void this.loadCommentsFor(task.id);
    }
  }

  destroy() {
    if (this.channel) supabase.removeChannel(this.channel);
  }
}

export const tasksStore = new TasksStore();
