/** Worker store using Svelte 5 runes - manages AI worker state */

import type { AeTask, WorkerStatus } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';

type WorkerMode = 'master' | 'viewer' | 'unknown';

let status = $state<WorkerStatus>('offline');
let currentTask = $state<AeTask | null>(null);
let machineName = $state('agent-one');
let workerId = $state<string | null>(null);
let lastHeartbeat = $state<string | null>(null);
let mode = $state<WorkerMode>('unknown');

export function getWorkerStore() {
  return {
    get status() { return status; },
    get currentTask() { return currentTask; },
    get machineName() { return machineName; },
    set machineName(v: string) { machineName = v; },
    get workerId() { return workerId; },
    get lastHeartbeat() { return lastHeartbeat; },

    get mode() { return mode; },
    set mode(v: WorkerMode) { mode = v; },
    get isViewer() { return mode === 'viewer'; },
    get isMaster() { return mode === 'master'; },

    get isOnline() { return status === 'online' || status === 'busy'; },
    get isBusy() { return status === 'busy'; },

    get statusColor(): string {
      if (mode === 'viewer') return '#58a6ff';
      switch (status) {
        case 'online': return '#3fb950';
        case 'busy': return '#6366f1';
        case 'offline': default: return '#f85149';
      }
    },

    get statusLabel(): string {
      if (mode === 'viewer') return 'Viewer';
      switch (status) {
        case 'online': return 'Online';
        case 'busy': return 'Working';
        case 'offline': default: return 'Offline';
      }
    },

    async fetchStatus() {
      try {
        const result = await safeInvoke<{ running: boolean; machine_name: string | null; current_task_id: string | null }>('worker_status');
        if (result) {
          status = result.running ? 'online' : 'offline';
          machineName = result.machine_name || machineName;
          workerId = result.current_task_id || null;
        }
      } catch (e) {
        console.warn('[worker] fetch status failed:', e);
        status = 'offline';
      }
    },

    async startWorker() {
      try {
        await safeInvoke('worker_start', { machineName });
        status = 'online';
      } catch (e) {
        console.warn('[worker] start failed:', e);
      }
    },

    async stopWorker() {
      try {
        await safeInvoke('worker_stop');
        status = 'offline';
        currentTask = null;
        workerId = null;
      } catch (e) {
        console.warn('[worker] stop failed:', e);
      }
    },

    async heartbeat() {
      if (!machineName) return;
      try {
        await safeInvoke('supabase_worker_heartbeat', { machineName });
        lastHeartbeat = new Date().toISOString();
      } catch (e) {
        console.warn('[worker] heartbeat failed:', e);
      }
    },

    async checkActiveWorker(): Promise<{ active: boolean; machine_name: string | null; error?: boolean }> {
      try {
        const result = await safeInvoke<{ active: boolean; machine_name: string | null }>('supabase_check_active_worker');
        return result || { active: false, machine_name: null };
      } catch (e) {
        console.warn('[worker] checkActiveWorker failed:', e);
        return { active: false, machine_name: null, error: true };
      }
    },

    /** Apply realtime worker update */
    applyUpdate(data: { status: WorkerStatus; current_task?: AeTask | null }) {
      status = data.status;
      if (data.current_task !== undefined) currentTask = data.current_task;
    },
  };
}
