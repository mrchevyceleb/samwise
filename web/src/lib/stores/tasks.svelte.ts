import { supabase } from '$lib/supabase';
import type { AeTask, AeComment } from '$lib/types';

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

  async loadCommentsFor(taskId: string) {
    if (this.comments[taskId]) return;
    const { data } = await supabase
      .from('ae_comments')
      .select('*')
      .eq('task_id', taskId)
      .order('created_at', { ascending: true });
    this.comments = { ...this.comments, [taskId]: (data ?? []) as AeComment[] };
  }

  destroy() {
    if (this.channel) supabase.removeChannel(this.channel);
  }
}

export const tasksStore = new TasksStore();
