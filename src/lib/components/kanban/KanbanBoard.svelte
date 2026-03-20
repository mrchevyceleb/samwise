<script lang="ts">
	import { onMount } from 'svelte';
	import { KANBAN_COLUMNS } from '$lib/types';
	import type { AeTask, TaskStatus } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getDragStore } from '$lib/stores/drag.svelte';
	import KanbanColumn from './KanbanColumn.svelte';
	import NewTaskModal from './NewTaskModal.svelte';
	import TaskDetailModal from './TaskDetailModal.svelte';

	const taskStore = getTaskStore();
	const layout = getLayout();
	const drag = getDragStore();

	let showNewTask = $state(false);
	let selectedTask = $state<AeTask | null>(null);
	let addBtnHovered = $state(false);

	/** Failed tasks shown in a collapsed row at the bottom */
	let failedTasks = $derived(taskStore.tasks.filter(t => t.status === 'failed'));
	let failedExpanded = $state(false);
	let failedHovered = $state(false);

	let columnsContainer = $state<HTMLDivElement | null>(null);

	onMount(() => {
		taskStore.fetchTasks();
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

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'n' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			showNewTask = true;
		}
		if (e.key === 'Escape' && drag.dragging) {
			drag.cancelDrag();
		}
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
				collapsed={column.status === 'done' && layout.doneColumnCollapsed}
				onToggleCollapse={column.status === 'done' ? () => layout.toggleDoneColumn() : undefined}
				onTaskClick={handleTaskClick}
				isDragTarget={drag.dragging && drag.hoverColumn === column.status && drag.draggedTask?.status !== column.status}
			/>
		{/each}
	</div>

	<!-- Failed tasks row (collapsed at bottom) -->
	{#if failedTasks.length > 0}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div style="
			flex-shrink: 0; border-top: 1px solid rgba(248, 81, 73, 0.15);
			background: rgba(248, 81, 73, 0.03);
		">
			<button
				style="
					width: 100%; display: flex; align-items: center; gap: 8px;
					padding: 8px 14px; border: none; background: none;
					cursor: pointer; transition: background 0.15s;
					{failedHovered ? 'background: rgba(248, 81, 73, 0.06);' : ''}
				"
				onmouseenter={() => failedHovered = true}
				onmouseleave={() => failedHovered = false}
				onclick={() => failedExpanded = !failedExpanded}
			>
				<span style="
					width: 8px; height: 8px; border-radius: 50%;
					background: var(--accent-red);
					box-shadow: 0 0 8px rgba(248, 81, 73, 0.4);
					animation: pulse-dot 2s ease-in-out infinite;
				"></span>
				<span style="font-size: 11px; font-weight: 700; color: var(--accent-red); text-transform: uppercase; letter-spacing: 0.5px;">
					Failed
				</span>
				<span style="
					font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 6px;
					background: rgba(248, 81, 73, 0.12); color: var(--accent-red);
					font-family: var(--font-mono);
				">
					{failedTasks.length}
				</span>
				<div style="flex: 1;"></div>
				<svg
					width="10" height="10" viewBox="0 0 10 10" fill="none"
					stroke="var(--accent-red)" stroke-width="1.5" stroke-linecap="round"
					style="transition: transform 0.2s; transform: rotate({failedExpanded ? '180deg' : '0'});"
				>
					<path d="M2 3.5l3 3 3-3"/>
				</svg>
			</button>

			{#if failedExpanded}
				<div style="
					display: flex; gap: 8px; padding: 4px 14px 10px;
					overflow-x: auto; animation: slide-in-top 0.15s ease;
				">
					{#each failedTasks as task (task.id)}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							style="
								flex-shrink: 0; padding: 8px 12px; border-radius: 8px;
								background: rgba(248, 81, 73, 0.06);
								border: 1px solid rgba(248, 81, 73, 0.15);
								cursor: pointer; transition: all 0.15s ease;
								max-width: 200px;
							"
							onclick={() => handleTaskClick(task)}
						>
							<div style="font-size: 12px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
								{task.title}
							</div>
							<div style="font-size: 10px; color: var(--accent-red); margin-top: 2px;">
								{task.priority} priority
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

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

{#if selectedTask}
	<TaskDetailModal task={selectedTask} onClose={() => selectedTask = null} />
{/if}
