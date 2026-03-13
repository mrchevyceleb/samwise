/** Terminal store using Svelte 5 runes */

export interface TerminalInstance {
  id: string;
  title: string;
  cwd: string;
}

let instances = $state<TerminalInstance[]>([]);
let activeId = $state<string | null>(null);
let nextIndex = $state(1);

// Buffers for preserving terminal output across re-renders
const buffers: Map<string, string> = new Map();

export function getTerminals() {
  return {
    get instances() { return instances; },
    get activeId() { return activeId; },
    set activeId(v: string | null) { activeId = v; },

    add(cwd: string): string {
      const id = `term-${Date.now()}-${nextIndex}`;
      const title = `Terminal ${nextIndex}`;
      nextIndex++;
      instances = [...instances, { id, title, cwd }];
      activeId = id;
      return id;
    },

    remove(id: string) {
      instances = instances.filter(t => t.id !== id);
      buffers.delete(id);
      if (activeId === id) {
        activeId = instances.length > 0 ? instances[instances.length - 1].id : null;
      }
    },

    appendBuffer(id: string, data: string) {
      const existing = buffers.get(id) || '';
      // Cap buffer at 500KB to avoid memory bloat
      const combined = existing + data;
      buffers.set(id, combined.length > 500_000 ? combined.slice(-400_000) : combined);
    },

    getBuffer(id: string): string {
      return buffers.get(id) || '';
    },

    clearBuffer(id: string) {
      buffers.delete(id);
    },
  };
}
