import type { AeCron, AeCronRun, AeProject } from '$lib/types';

type CronInput = {
  name: string;
  schedule: string;
  task_template: Record<string, unknown>;
  enabled?: boolean;
  next_run?: string | null;
};

async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init);
  const body = await res.json().catch(() => null) as { message?: string; error?: string } | T | null;

  if (!res.ok) {
    const message =
      body && typeof body === 'object' && 'message' in body && typeof body.message === 'string'
        ? body.message
        : body && typeof body === 'object' && 'error' in body && typeof body.error === 'string'
          ? body.error
          : `Request failed (${res.status})`;
    throw new Error(message);
  }

  return body as T;
}

class CronsStore {
  crons = $state<AeCron[]>([]);
  cronRuns = $state<AeCronRun[]>([]);
  projects = $state<AeProject[]>([]);
  loadingCrons = $state(false);
  loadingCronRuns = $state(false);
  loadingProjects = $state(false);
  checkingAdmin = $state(false);
  adminUnlocked = $state(false);
  saving = $state(false);
  error = $state<string | null>(null);

  async init() {
    if (!this.adminUnlocked) return;
    await Promise.all([this.fetchCrons(), this.fetchProjects(), this.fetchCronRuns()]);
  }

  async checkAdminSession() {
    this.checkingAdmin = true;
    this.error = null;
    try {
      const result = await requestJson<{ ok: boolean }>('/api/admin-session');
      this.adminUnlocked = result.ok;
      if (result.ok) await this.init();
    } catch (e) {
      this.adminUnlocked = false;
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.checkingAdmin = false;
    }
  }

  async unlockAdmin(key: string) {
    this.checkingAdmin = true;
    this.error = null;
    try {
      await requestJson<{ ok: true }>('/api/admin-session', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ key })
      });
      this.adminUnlocked = true;
      await this.init();
      return true;
    } catch (e) {
      this.adminUnlocked = false;
      this.error = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      this.checkingAdmin = false;
    }
  }

  async fetchCrons() {
    this.loadingCrons = true;
    this.error = null;
    try {
      this.crons = await requestJson<AeCron[]>('/api/crons');
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loadingCrons = false;
    }
  }

  async fetchCronRuns(cronId?: string) {
    this.loadingCronRuns = true;
    this.error = null;
    try {
      const qs = cronId ? `?cron_id=${encodeURIComponent(cronId)}` : '';
      const runs = await requestJson<AeCronRun[]>(`/api/cron-runs${qs}`);
      if (cronId) {
        const otherRuns = this.cronRuns.filter((run) => run.cron_id !== cronId);
        this.cronRuns = [...runs, ...otherRuns]
          .sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime())
          .slice(0, 200);
      } else {
        this.cronRuns = runs;
      }
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loadingCronRuns = false;
    }
  }

  async fetchProjects() {
    this.loadingProjects = true;
    this.error = null;
    try {
      this.projects = await requestJson<AeProject[]>('/api/projects');
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loadingProjects = false;
    }
  }

  async createCron(input: CronInput): Promise<AeCron | null> {
    this.saving = true;
    this.error = null;
    try {
      const cron = await requestJson<AeCron>('/api/crons', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input)
      });
      this.crons = [cron, ...this.crons];
      await this.fetchCronRuns();
      return cron;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      return null;
    } finally {
      this.saving = false;
    }
  }

  async updateCron(id: string, updates: Partial<CronInput>): Promise<AeCron | null> {
    this.saving = true;
    this.error = null;
    try {
      const cron = await requestJson<AeCron>(`/api/crons/${id}`, {
        method: 'PATCH',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(updates)
      });
      this.crons = this.crons.map((item) => (item.id === id ? cron : item));
      await this.fetchCronRuns(id);
      return cron;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      return null;
    } finally {
      this.saving = false;
    }
  }

  async toggleCron(id: string) {
    const cron = this.crons.find((item) => item.id === id);
    if (!cron) return null;
    return this.updateCron(id, { enabled: !cron.enabled });
  }

  async deleteCron(id: string): Promise<boolean> {
    this.saving = true;
    this.error = null;
    try {
      await requestJson<{ ok: true }>(`/api/crons/${id}`, { method: 'DELETE' });
      this.crons = this.crons.filter((item) => item.id !== id);
      this.cronRuns = this.cronRuns.filter((item) => item.cron_id !== id);
      return true;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      this.saving = false;
    }
  }
}

export const cronsStore = new CronsStore();
