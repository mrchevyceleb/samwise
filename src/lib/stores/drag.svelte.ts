/** Pointer-based drag store for Kanban - works in Tauri WebView2 where HTML5 drag fails */

import type { AeTask, TaskStatus } from '$lib/types';

let dragging = $state(false);
let draggedTask = $state<AeTask | null>(null);
let ghostX = $state(0);
let ghostY = $state(0);
let hoverColumn = $state<TaskStatus | null>(null);

export function getDragStore() {
  return {
    get dragging() { return dragging; },
    get draggedTask() { return draggedTask; },
    get ghostX() { return ghostX; },
    get ghostY() { return ghostY; },
    get hoverColumn() { return hoverColumn; },

    startDrag(task: AeTask, x: number, y: number) {
      draggedTask = task;
      ghostX = x;
      ghostY = y;
      dragging = true;
    },

    updatePosition(x: number, y: number) {
      ghostX = x;
      ghostY = y;
    },

    setHoverColumn(status: TaskStatus | null) {
      hoverColumn = status;
    },

    endDrag(): { task: AeTask; targetColumn: TaskStatus } | null {
      const result = (draggedTask && hoverColumn && hoverColumn !== draggedTask.status)
        ? { task: draggedTask, targetColumn: hoverColumn }
        : null;
      dragging = false;
      draggedTask = null;
      hoverColumn = null;
      return result;
    },

    cancelDrag() {
      dragging = false;
      draggedTask = null;
      hoverColumn = null;
    },
  };
}
