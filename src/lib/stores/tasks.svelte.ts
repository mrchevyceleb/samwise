/** Task store using Svelte 5 runes - manages Kanban task state */

import type { AeTask, TaskStatus, TaskPriority, TaskType, TaskSource } from '$lib/types';
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
    fixes_needed: [],
    approved: [],
    done: [],
    failed: [],
    pending_confirmation: [],
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
        fixesNeeded: tasks.filter(t => t.status === 'fixes_needed').length,
        approved: tasks.filter(t => t.status === 'approved').length,
        done: tasks.filter(t => t.status === 'done').length,
        failed: tasks.filter(t => t.status === 'failed').length,
        pendingConfirmation: tasks.filter(t => t.status === 'pending_confirmation').length,
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
      task_type?: TaskType;
      source?: TaskSource;
      project?: string;
      repo_url?: string;
      repo_path?: string;
      preview_url?: string;
      base_branch?: string;
      context?: Record<string, unknown>;
    }) {
      try {
        const result = await safeInvoke<AeTask[]>('supabase_create_task', {
          task: {
            title: data.title,
            description: data.description || null,
            priority: data.priority || 'medium',
            task_type: data.task_type || 'code',
            source: data.source || 'manual',
            status: 'queued',
            project: data.project || null,
            repo_url: data.repo_url || null,
            repo_path: data.repo_path || null,
            preview_url: data.preview_url || null,
            base_branch: data.base_branch || null,
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
        const result = await safeInvoke('supabase_update_task', {
          id,
          updates,
        });
        if (result === null) {
          throw new Error('Supabase update command did not return a result.');
        }
        tasks = tasks.map(t => t.id === id ? { ...t, ...updates, updated_at: new Date().toISOString() } : t);
        error = null;
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        console.warn('[tasks] update failed:', e);
        return false;
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
