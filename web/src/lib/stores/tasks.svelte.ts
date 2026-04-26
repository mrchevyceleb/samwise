import { supabase } from '$lib/supabase';
import type { AeTask, AeComment, TaskStatus } from '$lib/types';

class TasksStore {
  tasks = $state<AeTask[]>([]);
  comments = $state<Record<string, AeComment[]>>({});
  loading = $state(true);
  error = $state<string | null>(null);
  connected = $state(false);

  private channel: ReturnType<typeof supabase.channel> | null = null;

  async init() {
    try {
      const { data, error } = await supabase
        .from('ae_tasks')
        .select('*')
        .order('updated_at', { ascending: false })
        .limit(500);
      if (error) throw error;
      this.tasks = (data ?? []) as AeTask[];
      this.loading = false;
      this.prefetchReviewComments();
    } catch (e: unknown) {
      this.error = e instanceof Error ? e.message : String(e);
      this.loading = false;
      return;
    }

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
    await this.updateTask(taskId, updates);
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
    return !!task.pr_url && (task.status === 'review' || task.status === 'fixes_needed' || task.status === 'approved');
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
