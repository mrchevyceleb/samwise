/** Task store using Svelte 5 runes - manages Kanban task state */

import type { AeTask, TaskStatus, TaskPriority, TaskSource } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

let tasks = $state<AeTask[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

/** Tasks grouped by status column */
function getTasksByColumn(): Record<TaskStatus, AeTask[]> {
  const grouped: Record<TaskStatus, AeTask[]> = {
    queued: [],
    in_progress: [],
    testing: [],
    review: [],
    approved: [],
    done: [],
    failed: [],
  };
  for (const t of tasks) {
    if (grouped[t.status]) {
      grouped[t.status].push(t);
    }
  }
  // Sort each column by priority then created_at
  const priorityOrder: Record<TaskPriority, number> = { critical: 0, high: 1, medium: 2, low: 3 };
  for (const key of Object.keys(grouped) as TaskStatus[]) {
    grouped[key].sort((a, b) => {
      const pd = priorityOrder[a.priority] - priorityOrder[b.priority];
      if (pd !== 0) return pd;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    });
  }
  return grouped;
}

export function getTaskStore() {
  return {
    get tasks() { return tasks; },
    get loading() { return loading; },
    get error() { return error; },
    get tasksByColumn() { return getTasksByColumn(); },

    get taskCounts() {
      return {
        total: tasks.length,
        queued: tasks.filter(t => t.status === 'queued').length,
        inProgress: tasks.filter(t => t.status === 'in_progress').length,
        testing: tasks.filter(t => t.status === 'testing').length,
        review: tasks.filter(t => t.status === 'review').length,
        done: tasks.filter(t => t.status === 'done').length,
        failed: tasks.filter(t => t.status === 'failed').length,
      };
    },

    async fetchTasks() {
      loading = true;
      error = null;
      try {
        const result = await safeInvoke<AeTask[]>('supabase_fetch_tasks');
        if (result) {
          tasks = result;
        }
      } catch (e) {
        error = String(e);
        console.warn('[tasks] fetch failed:', e);
      } finally {
        loading = false;
      }
    },

    async createTask(data: {
      title: string;
      description?: string;
      priority?: TaskPriority;
      source?: TaskSource;
      project?: string;
      repo_url?: string;
      repo_path?: string;
      preview_url?: string;
      context?: Record<string, unknown>;
    }) {
      try {
        const result = await safeInvoke<AeTask[]>('supabase_create_task', {
          task: {
            title: data.title,
            description: data.description || null,
            priority: data.priority || 'medium',
            source: data.source || 'manual',
            status: 'queued',
            project: data.project || null,
            repo_url: data.repo_url || null,
            repo_path: data.repo_path || null,
            preview_url: data.preview_url || null,
            context: data.context || null,
          },
        });
        if (result && Array.isArray(result) && result.length > 0) {
          const newTask = result[0] as AeTask;
          tasks = [newTask, ...tasks];
          return newTask;
        }
      } catch (e) {
        console.warn('[tasks] create failed:', e);
      }
      return null;
    },

    async updateTask(id: string, updates: Partial<AeTask>) {
      try {
        await safeInvoke('supabase_update_task', {
          id,
          updates,
        });
        tasks = tasks.map(t => t.id === id ? { ...t, ...updates, updated_at: new Date().toISOString() } : t);
      } catch (e) {
        console.warn('[tasks] update failed:', e);
      }
    },

    async deleteTask(id: string) {
      try {
        await safeInvoke('supabase_delete_task', { id });
        tasks = tasks.filter(t => t.id !== id);
      } catch (e) {
        console.warn('[tasks] delete failed:', e);
      }
    },

    async moveTask(id: string, newStatus: TaskStatus) {
      const updates: Partial<AeTask> = { status: newStatus };
      if (newStatus === 'done') {
        updates.completed_at = new Date().toISOString();
      }
      await this.updateTask(id, updates);
    },

    getTask(id: string): AeTask | undefined {
      return tasks.find(t => t.id === id);
    },

    /** Apply a realtime update from Supabase */
    applyRealtimeUpdate(eventType: string, payload: AeTask) {
      if (eventType === 'INSERT') {
        if (!tasks.find(t => t.id === payload.id)) {
          tasks = [payload, ...tasks];
        }
      } else if (eventType === 'UPDATE') {
        tasks = tasks.map(t => t.id === payload.id ? payload : t);
      } else if (eventType === 'DELETE') {
        tasks = tasks.filter(t => t.id !== payload.id);
      }
    },
  };
}
