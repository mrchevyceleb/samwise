/** Automation store using Svelte 5 runes - manages triggers and crons */

import type { AeTrigger, AeCron, TriggerSourceType } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

let triggers = $state<AeTrigger[]>([]);
let crons = $state<AeCron[]>([]);
let loadingTriggers = $state(false);
let loadingCrons = $state(false);

export function getAutomationStore() {
  return {
    get triggers() { return triggers; },
    get crons() { return crons; },
    get loadingTriggers() { return loadingTriggers; },
    get loadingCrons() { return loadingCrons; },

    get activeTriggers() { return triggers.filter(t => t.enabled); },
    get activeCrons() { return crons.filter(c => c.enabled); },

    // ── Triggers ──────────────────────────────────────────────
    async fetchTriggers() {
      loadingTriggers = true;
      try {
        const result = await safeInvoke<AeTrigger[]>('supabase_fetch_triggers');
        if (result) triggers = result;
      } catch (e) {
        console.warn('[automation] fetch triggers failed:', e);
      } finally {
        loadingTriggers = false;
      }
    },

    async createTrigger(data: {
      name: string;
      source_type: TriggerSourceType;
      source_config: Record<string, unknown>;
      task_template: Record<string, unknown>;
      enabled?: boolean;
    }) {
      try {
        const result = await safeInvoke<AeTrigger[]>('supabase_create_trigger', {
          trigger: { ...data, enabled: data.enabled ?? true },
        });
        if (result && Array.isArray(result) && result.length > 0) {
          const t = result[0] as AeTrigger;
          triggers = [t, ...triggers];
          return t;
        }
      } catch (e) {
        console.warn('[automation] create trigger failed:', e);
      }
      return null;
    },

    async updateTrigger(id: string, updates: Partial<AeTrigger>) {
      try {
        await safeInvoke('supabase_update_trigger', {
          id,
          updates,
        });
        triggers = triggers.map(t => t.id === id ? { ...t, ...updates } : t);
      } catch (e) {
        console.warn('[automation] update trigger failed:', e);
      }
    },

    async toggleTrigger(id: string) {
      const trigger = triggers.find(t => t.id === id);
      if (trigger) {
        await this.updateTrigger(id, { enabled: !trigger.enabled });
      }
    },

    // ── Crons ─────────────────────────────────────────────────
    async fetchCrons() {
      loadingCrons = true;
      try {
        const result = await safeInvoke<AeCron[]>('supabase_fetch_crons');
        if (result) crons = result;
      } catch (e) {
        console.warn('[automation] fetch crons failed:', e);
      } finally {
        loadingCrons = false;
      }
    },

    async createCron(data: {
      name: string;
      schedule: string;
      task_template: Record<string, unknown>;
      enabled?: boolean;
    }) {
      try {
        const result = await safeInvoke<AeCron[]>('supabase_create_cron', {
          cron: { ...data, enabled: data.enabled ?? true },
        });
        if (result && Array.isArray(result) && result.length > 0) {
          const c = result[0] as AeCron;
          crons = [c, ...crons];
          return c;
        }
      } catch (e) {
        console.warn('[automation] create cron failed:', e);
      }
      return null;
    },

    async updateCron(id: string, updates: Partial<AeCron>) {
      try {
        await safeInvoke('supabase_update_cron', {
          id,
          updates,
        });
        crons = crons.map(c => c.id === id ? { ...c, ...updates } : c);
      } catch (e) {
        console.warn('[automation] update cron failed:', e);
      }
    },

    async toggleCron(id: string) {
      const cron = crons.find(c => c.id === id);
      if (cron) {
        await this.updateCron(id, { enabled: !cron.enabled });
      }
    },
  };
}
