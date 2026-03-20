<script lang="ts">
	import type { AeTask, TaskStatus } from '$lib/types';
	import KanbanCard from './KanbanCard.svelte';

	interface Props {
		status: TaskStatus;
		label: string;
		color: string;
		glowColor: string;
		icon: string;
		tasks: AeTask[];
		collapsed?: boolean;
		onToggleCollapse?: () => void;
		onDrop?: (taskId: string, status: string) => void;
		onTaskClick?: (task: AeTask) => void;
	}

	let { status, label, color, glowColor, icon, tasks, collapsed = false, onToggleCollapse, onDrop, onTaskClick }: Props = $props();

	let dragOver = $state(false);
	let headerHovered = $state(false);

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		e.dataTransfer!.dropEffect = 'move';
		dragOver = true;
	}

	function handleDragLeave(e: DragEvent) {
		// Only clear if we actually left the column (not just entered a child)
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const x = e.clientX;
		const y = e.clientY;
		if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) {
			dragOver = false;
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const taskId = e.dataTransfer?.getData('text/plain');
		if (taskId) {
			onDrop?.(taskId, status);
		}
	}

	let isInProgress = $derived(status === 'in_progress');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	style="
		display: flex; flex-direction: column;
		min-width: {collapsed ? '44px' : '200px'};
		flex: {collapsed ? '0 0 44px' : '1'};
		background: {dragOver ? glowColor : 'rgba(255, 255, 255, 0.015)'};
		border-radius: 10px;
		border-left: 2px solid {color}30;
		transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
		{isInProgress && !collapsed ? 'animation: working-glow 3s ease-in-out infinite;' : ''}
		{dragOver ? `outline: 2px dashed ${color}60; outline-offset: -2px;` : ''}
	"
	ondragover={handleDragOver}
	ondragleave={handleDragLeave}
	ondrop={handleDrop}
>
	<!-- Column header -->
	<button
		style="
			display: flex; align-items: center; gap: 6px;
			padding: {collapsed ? '8px 6px' : '8px 10px'};
			background: {headerHovered ? 'rgba(255, 255, 255, 0.02)' : 'none'};
			border: none; cursor: {onToggleCollapse ? 'pointer' : 'default'};
			width: 100%; text-align: left;
			transition: all 0.15s ease; border-radius: 8px;
			{collapsed ? 'writing-mode: vertical-rl; justify-content: center;' : ''}
		"
		onclick={() => onToggleCollapse?.()}
		onmouseenter={() => headerHovered = true}
		onmouseleave={() => headerHovered = false}
	>
		<!-- Status dot -->
		<span style="
			width: 9px; height: 9px; border-radius: 50%;
			background: {color};
			box-shadow: 0 0 8px {color}60;
			flex-shrink: 0;
			{isInProgress ? 'animation: pulse-ring 2s ease-out infinite;' : ''}
		"></span>

		{#if !collapsed}
			<!-- Icon -->
			<span style="
				font-size: 11px; font-weight: 800; color: {color};
				font-family: var(--font-mono); opacity: 0.6;
			">
				{icon}
			</span>

			<span style="
				font-size: 12px; font-weight: 700; color: var(--text-secondary);
				text-transform: uppercase; letter-spacing: 0.5px; flex: 1;
			">
				{label}
			</span>

			<!-- Task count badge -->
			<span style="
				font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 6px;
				background: {color}15; color: {color};
				font-family: var(--font-mono);
				min-width: 22px; text-align: center;
			">
				{tasks.length}
			</span>
		{:else}
			<span style="font-size: 11px; font-weight: 700; color: var(--text-muted); letter-spacing: 0.5px;">
				{label} ({tasks.length})
			</span>
		{/if}
	</button>

	<!-- Cards -->
	{#if !collapsed}
		<div style="
			flex: 1; overflow-y: auto; padding: 6px 8px 10px;
			display: flex; flex-direction: column; gap: 8px;
		">
			{#each tasks as task (task.id)}
				<KanbanCard {task} onClick={onTaskClick} />
			{/each}

			{#if tasks.length === 0}
				<div style="
					padding: 24px 8px; text-align: center;
					color: var(--text-muted); font-size: 12px;
					border: 1px dashed {dragOver ? color + '60' : 'var(--border-default)'};
					border-radius: 8px; opacity: {dragOver ? '0.8' : '0.4'};
					transition: all 0.2s ease;
				">
					{dragOver ? 'Drop here' : 'No tasks'}
				</div>
			{/if}
		</div>
	{/if}
</div>
