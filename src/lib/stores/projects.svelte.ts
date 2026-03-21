/** Projects store - manages the project registry */

import type { AeProject } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

let projects = $state<AeProject[]>([]);
let loading = $state(false);

export function getProjectStore() {
  return {
    get projects() { return projects; },
    get loading() { return loading; },

    async fetchProjects() {
      loading = true;
      try {
        const result = await safeInvoke<AeProject[]>('supabase_fetch_projects');
        if (result) {
          projects = result;
        }
      } catch (e) {
        console.warn('[projects] fetch failed:', e);
      } finally {
        loading = false;
      }
    },

    async createProject(data: Partial<AeProject>) {
      try {
        const result = await safeInvoke<AeProject[]>('supabase_create_project', { project: data });
        if (result && result.length > 0) {
          projects = [...projects, result[0]];
          return result[0];
        }
      } catch (e) {
        console.warn('[projects] create failed:', e);
      }
      return null;
    },

    async updateProject(id: string, updates: Partial<AeProject>) {
      try {
        await safeInvoke('supabase_update_project', { id, updates });
        projects = projects.map(p => p.id === id ? { ...p, ...updates } : p);
      } catch (e) {
        console.warn('[projects] update failed:', e);
      }
    },

    async deleteProject(id: string) {
      try {
        await safeInvoke('supabase_delete_project', { id });
        projects = projects.filter(p => p.id !== id);
      } catch (e) {
        console.warn('[projects] delete failed:', e);
      }
    },
  };
}
