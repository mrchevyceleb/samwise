<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { KANBAN_COLUMNS } from '$lib/types';
	import type { AeTask, TaskStatus } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getDragStore } from '$lib/stores/drag.svelte';
	import KanbanColumn from './KanbanColumn.svelte';
	import CardContextMenu from './CardContextMenu.svelte';
	import NewTaskModal from './NewTaskModal.svelte';
	import TaskDetailModal from './TaskDetailModal.svelte';

	const taskStore = getTaskStore();
	const commentStore = getCommentStore();
	const layout = getLayout();
	const drag = getDragStore();

	let showNewTask = $state(false);
	let selectedTask = $state<AeTask | null>(null);
	let selectedTaskId = $derived(selectedTask?.id ?? null);
	let liveSelectedTask = $derived(
		selectedTaskId ? taskStore.tasks.find(t => t.id === selectedTaskId) ?? selectedTask : null
	);
	let addBtnHovered = $state(false);
	let contextMenu = $state<{ task: AeTask; x: number; y: number } | null>(null);

	let columnsContainer = $state<HTMLDivElement | null>(null);

	/** Auto-poll comments for active/reviewed tasks so card summaries update live */
	let commentPollInterval: ReturnType<typeof setInterval> | null = null;

	function pollActiveComments() {
		const activeTasks = taskStore.tasks.filter(
			t => t.status === 'in_progress'
				|| t.status === 'testing'
				|| t.status === 'review'
				|| t.status === 'fixes_needed'
				|| t.status === 'approved'
		);
		for (const task of activeTasks) {
			commentStore.fetchComments(task.id);
		}
	}

	onMount(() => {
		taskStore.fetchTasks();
		// Poll comments for active tasks every 8 seconds
		commentPollInterval = setInterval(pollActiveComments, 8000);
		// Initial fetch
		setTimeout(pollActiveComments, 1000);
	});

	onDestroy(() => {
		if (commentPollInterval) clearInterval(commentPollInterval);
	});

	function handleMouseMove(e: MouseEvent) {
		if (!drag.dragging) return;
		drag.updatePosition(e.clientX, e.clientY);

		// Determine which column the pointer is over
		if (columnsContainer) {
			const columns = columnsContainer.querySelectorAll('[data-column-status]');
			let found = false;
			for (const col of columns) {
				const rect = col.getBoundingClientRect();
				if (e.clientX >= rect.left && e.clientX <= rect.right && e.clientY >= rect.top && e.clientY <= rect.bottom) {
					drag.setHoverColumn(col.getAttribute('data-column-status') as TaskStatus);
					found = true;
					break;
				}
			}
			if (!found) {
				drag.setHoverColumn(null);
			}
		}
	}

	function handleMouseUp() {
		if (!drag.dragging) return;
		const result = drag.endDrag();
		if (result) {
			taskStore.moveTask(result.task.id, result.targetColumn);
		}
	}

	function handleTaskClick(task: AeTask) {
		// Don't open detail if we just finished dragging
		if (!drag.dragging) {
			selectedTask = task;
		}
	}

	function handleTaskContextMenu(task: AeTask, x: number, y: number) {
		contextMenu = { task, x, y };
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'n' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			showNewTask = true;
		}
		if (e.key === 'Escape' && drag.dragging) {
			drag.cancelDrag();
		}
	}

	function isColumnCollapsed(status: TaskStatus): boolean {
		return status === 'done'
			? layout.doneColumnCollapsed
			: status === 'failed'
				? layout.failedColumnCollapsed
				: false;
	}

	function toggleColumnCollapse(status: TaskStatus) {
		if (status === 'done') layout.toggleDoneColumn();
		if (status === 'failed') layout.toggleFailedColumn();
	}
</script>

<svelte:window onkeydown={handleKeyDown} onmousemove={handleMouseMove} onmouseup={handleMouseUp} />

<div style="display: flex; flex-direction: column; height: 100%; overflow: hidden; position: relative;">
	<!-- Header -->
	<div style="
		display: flex; align-items: center; gap: 8px;
		padding: 10px 14px; flex-shrink: 0;
		border-bottom: 1px solid var(--border-subtle);
	">
		<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--accent-indigo)" stroke-width="2" stroke-linecap="round">
			<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
		</svg>
		<span style="font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.3px;">Tasks</span>

		<div style="flex: 1;"></div>

		<!-- Task counts -->
		{#if taskStore.loading}
			<span style="font-size: 12px; color: var(--text-muted); font-family: var(--font-mono);">loading...</span>
		{:else}
			<span style="font-size: 12px; color: var(--text-muted); font-family: var(--font-mono);">
				{taskStore.taskCounts.total} tasks
			</span>
		{/if}

		<!-- Add task button -->
		<button
			title="New Task (Ctrl+N)"
			style="
				width: 32px; height: 32px; display: flex; align-items: center; justify-content: center;
				border: 1px solid {addBtnHovered ? 'rgba(99, 102, 241, 0.4)' : 'var(--border-default)'};
				border-radius: 8px; cursor: pointer; transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
				background: {addBtnHovered ? 'rgba(99, 102, 241, 0.12)' : 'transparent'};
				color: {addBtnHovered ? 'var(--accent-indigo)' : 'var(--text-muted)'};
				transform: {addBtnHovered ? 'scale(1.1) rotate(90deg)' : 'scale(1) rotate(0)'};
			"
			onmouseenter={() => addBtnHovered = true}
			onmouseleave={() => addBtnHovered = false}
			onclick={() => showNewTask = true}
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
				<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
			</svg>
		</button>
	</div>

	<!-- Columns (horizontal scroll) -->
	<div
		bind:this={columnsContainer}
		style="
			flex: 1; display: flex; gap: 4px; padding: 8px;
			overflow-x: auto; overflow-y: hidden;
			{drag.dragging ? 'cursor: grabbing; user-select: none;' : ''}
		"
	>
		{#each KANBAN_COLUMNS as column (column.status)}
			<KanbanColumn
				status={column.status}
				label={column.label}
				color={column.color}
				glowColor={column.glowColor}
				icon={column.icon}
				tasks={taskStore.tasksByColumn[column.status] || []}
				collapsed={isColumnCollapsed(column.status)}
				onToggleCollapse={column.status === 'done' || column.status === 'failed' ? () => toggleColumnCollapse(column.status) : undefined}
				onTaskClick={handleTaskClick}
				onTaskContextMenu={handleTaskContextMenu}
				isDragTarget={drag.dragging && drag.hoverColumn === column.status && drag.draggedTask?.status !== column.status}
			/>
		{/each}
	</div>

	<!-- Drag ghost (floating card that follows the cursor) -->
	{#if drag.dragging && drag.draggedTask}
		<div style="
			position: fixed; left: {drag.ghostX + 12}px; top: {drag.ghostY - 16}px;
			z-index: 9999; pointer-events: none;
			padding: 10px 14px; border-radius: 10px;
			background: rgba(99, 102, 241, 0.15);
			border: 2px solid rgba(99, 102, 241, 0.5);
			backdrop-filter: blur(12px);
			box-shadow: 0 8px 32px rgba(99, 102, 241, 0.3);
			max-width: 220px; transform: rotate(2deg) scale(1.02);
			animation: card-idle-bob 2s ease-in-out infinite;
		">
			<div style="font-size: 13px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
				{drag.draggedTask.title}
			</div>
			<div style="font-size: 10px; color: var(--accent-indigo); margin-top: 4px;">
				{drag.hoverColumn ? `Drop in ${drag.hoverColumn.replace('_', ' ')}` : 'Drag to a column'}
			</div>
		</div>
	{/if}
</div>

<!-- Modals -->
{#if showNewTask}
	<NewTaskModal onClose={() => showNewTask = false} />
{/if}

{#if liveSelectedTask}
	<TaskDetailModal task={liveSelectedTask} onClose={() => selectedTask = null} />
{/if}

{#if contextMenu}
	<CardContextMenu
		task={contextMenu.task}
		x={contextMenu.x}
		y={contextMenu.y}
		onClose={() => contextMenu = null}
		onOpenDetail={(task) => { selectedTask = task; }}
	/>
{/if}
