<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { AeTask } from '$lib/types';
	import { PRIORITY_COLORS } from '$lib/types';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import { getDragStore } from '$lib/stores/drag.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { formatTimeAgo } from '$lib/utils/relative-time';

	interface Props {
		task: AeTask;
		onClick?: (task: AeTask) => void;
	}

	let { task, onClick }: Props = $props();
	const commentStore = getCommentStore();
	const drag = getDragStore();
	const theme = getTheme();

	let hovered = $state(false);
	let mouseDownAt = $state<{ x: number; y: number } | null>(null);
	let nowTick = $state(Date.now());

	let elapsed = $derived(formatTimeAgo(new Date(task.created_at).getTime()));
	let priorityColor = $derived(PRIORITY_COLORS[task.priority]);
	let commentCount = $derived(commentStore.getCommentCount(task.id));
	let hasScreenshots = $derived(
		(task.screenshots_before && task.screenshots_before.length > 0) ||
		(task.screenshots_after && task.screenshots_after.length > 0)
	);
	let isAgent = $derived(task.assignee === 'agent');
	let isBeingDragged = $derived(drag.dragging && drag.draggedTask?.id === task.id);
	let isWorking = $derived(task.status === 'in_progress' || task.status === 'testing');
	let latestComment = $derived(commentStore.getLatestComment(task.id));
	let qaResult = $derived(task.visual_qa_result);

	/** Live elapsed timer for in-progress tasks */
	let workingElapsed = $derived(() => {
		if (!isWorking || !task.claimed_at) return '';
		const start = new Date(task.claimed_at).getTime();
		const diff = Math.max(0, nowTick - start);
		const secs = Math.floor(diff / 1000);
		if (secs < 60) return `${secs}s`;
		const mins = Math.floor(secs / 60);
		const remSecs = secs % 60;
		if (mins < 60) return `${mins}m ${remSecs}s`;
		const hrs = Math.floor(mins / 60);
		const remMins = mins % 60;
		return `${hrs}h ${remMins}m`;
	});

	let timerInterval: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		if (isWorking) {
			timerInterval = setInterval(() => { nowTick = Date.now(); }, 1000);
		}
	});

	onDestroy(() => {
		if (timerInterval) clearInterval(timerInterval);
	});

	// Start/stop timer when status changes
	$effect(() => {
		if (isWorking && !timerInterval) {
			timerInterval = setInterval(() => { nowTick = Date.now(); }, 1000);
		} else if (!isWorking && timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	});

	function handleMouseDown(e: MouseEvent) {
		// Only left click, not on links
		if (e.button !== 0) return;
		if ((e.target as HTMLElement).closest('a')) return;
		mouseDownAt = { x: e.clientX, y: e.clientY };
	}

	function handleMouseMove(e: MouseEvent) {
		if (!mouseDownAt) return;
		// Start drag after 5px movement threshold
		const dx = e.clientX - mouseDownAt.x;
		const dy = e.clientY - mouseDownAt.y;
		if (Math.abs(dx) + Math.abs(dy) > 5) {
			drag.startDrag(task, e.clientX, e.clientY);
			mouseDownAt = null;
		}
	}

	function handleMouseUp() {
		if (mouseDownAt && !drag.dragging) {
			// This was a click, not a drag
			onClick?.(task);
		}
		mouseDownAt = null;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	role="button"
	tabindex={0}
	style="
		padding: 12px 14px;
		border-radius: 10px;
		background: {hovered ? (theme.isDark ? 'rgba(99, 102, 241, 0.06)' : 'rgba(79, 70, 229, 0.04)') : theme.c.glassBg};
		backdrop-filter: blur(12px);
		border: {isWorking ? '1.5px solid rgba(99, 102, 241, 0.5)' : hovered ? '1px solid ' + theme.c.borderGlow : '1px solid ' + theme.c.glassBorder};
		{isWorking ? 'animation: working-card-pulse 2s ease-in-out infinite;' : ''}
		cursor: grab;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		transform: {isBeingDragged ? 'scale(0.95)' : hovered ? 'translateY(-2px)' : 'translateY(0)'};
		box-shadow: {hovered ? theme.c.shadowCardHover : theme.c.shadowCard};
		opacity: {isBeingDragged ? '0.4' : '1'};
		{!isWorking ? `animation: ${task.status !== 'done' && !isBeingDragged ? 'card-idle-bob 4s ease-in-out infinite' : 'none'}; animation-delay: ${Math.random() * 2}s;` : ''}
		user-select: none;
	"
	onmouseenter={() => hovered = true}
	onmouseleave={() => { hovered = false; mouseDownAt = null; }}
	onmousedown={handleMouseDown}
	onmousemove={handleMouseMove}
	onmouseup={handleMouseUp}
	onkeydown={(e) => { if (e.key === 'Enter') onClick?.(task); }}
>
	<!-- Title (truncated to 2 lines) -->
	<div style="
		font-size: 14px; font-weight: 600; color: var(--text-primary);
		margin-bottom: 6px; line-height: 1.4;
		display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
		overflow: hidden; text-overflow: ellipsis;
	">
		{task.title}
	</div>

	<!-- Priority + Project row -->
	<div style="display: flex; align-items: center; gap: 5px; flex-wrap: wrap; margin-bottom: 8px;">
		<!-- Priority pill -->
		<span style="
			font-size: 10px; font-weight: 700; padding: 2px 7px; border-radius: 5px;
			background: {priorityColor}18; color: {priorityColor};
			border: 1px solid {priorityColor}25;
			text-transform: uppercase; letter-spacing: 0.4px;
		">
			{task.priority}
		</span>

		<!-- Project label -->
		{#if task.project}
			<span style="
				font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: 5px;
				background: rgba(99, 102, 241, 0.08); color: var(--accent-indigo);
				max-width: 120px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
			">
				{task.project}
			</span>
		{/if}

		<div style="flex: 1;"></div>

		<!-- Time elapsed / working timer -->
		{#if isWorking}
			<span style="
				font-size: 10px; font-weight: 600; color: var(--accent-indigo);
				font-family: var(--font-mono);
				display: flex; align-items: center; gap: 3px;
			">
				<span style="
					width: 6px; height: 6px; border-radius: 50%;
					background: var(--accent-indigo);
					animation: pulse-dot 1.5s ease-in-out infinite;
				"></span>
				{workingElapsed()}
			</span>
		{:else}
			<span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">
				{elapsed}
			</span>
		{/if}
	</div>

	<!-- Latest agent comment preview (shows what the agent is doing) -->
	{#if latestComment && (isWorking || task.status === 'testing' || task.status === 'review')}
		<div style="
			font-size: 11px; color: var(--text-muted); line-height: 1.3;
			padding: 5px 8px; margin-bottom: 8px; border-radius: 6px;
			background: rgba(99, 102, 241, 0.04);
			border-left: 2px solid rgba(99, 102, 241, 0.3);
			white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
		">
			{#if isWorking}
				<span style="color: var(--accent-indigo); font-weight: 600; margin-right: 4px;">
					&#9679;
				</span>
			{/if}
			{latestComment}
		</div>
	{/if}

	<!-- Visual QA result badge -->
	{#if qaResult}
		<div style="
			font-size: 10px; font-weight: 600; padding: 3px 8px; margin-bottom: 8px;
			border-radius: 5px; display: inline-flex; align-items: center; gap: 4px;
			background: {qaResult.pass ? 'rgba(63, 185, 80, 0.08)' : 'rgba(248, 81, 73, 0.08)'};
			color: {qaResult.pass ? 'var(--accent-green)' : 'var(--accent-red)'};
			border: 1px solid {qaResult.pass ? 'rgba(63, 185, 80, 0.2)' : 'rgba(248, 81, 73, 0.2)'};
		">
			{qaResult.pass ? 'QA Passed' : 'QA Failed'}
		</div>
	{/if}

	<!-- Bottom row: indicators -->
	<div style="display: flex; align-items: center; gap: 8px;">
		<!-- Assignee indicator -->
		<span title="{isAgent ? 'Assigned to Agent' : 'Assigned to Matt'}" style="
			display: flex; align-items: center; justify-content: center;
			width: 22px; height: 22px; border-radius: 50%;
			background: {isAgent ? 'rgba(99, 102, 241, 0.12)' : 'rgba(63, 185, 80, 0.12)'};
			color: {isAgent ? 'var(--accent-indigo)' : 'var(--accent-green)'};
			font-size: 12px;
		">
			{#if isAgent}
				<!-- Robot icon -->
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
					<path d="M8 0a1 1 0 011 1v1.07A6.002 6.002 0 0114 8v3a2 2 0 01-2 2H4a2 2 0 01-2-2V8a6.002 6.002 0 015-5.93V1a1 1 0 011-1zM6 9a1 1 0 100 2 1 1 0 000-2zm4 0a1 1 0 100 2 1 1 0 000-2z"/>
				</svg>
			{:else}
				<!-- User icon -->
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
					<path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/>
				</svg>
			{/if}
		</span>

		<div style="flex: 1;"></div>

		<!-- Comment count -->
		{#if commentCount > 0}
			<span style="
				display: flex; align-items: center; gap: 3px;
				font-size: 11px; color: var(--text-muted);
			">
				<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
					<path d="M2.678 11.894a1 1 0 01.287.801 10.97 10.97 0 01-.398 2c1.395-.323 2.247-.697 2.634-.893a1 1 0 01.71-.074A8.06 8.06 0 008 14c3.996 0 7-2.807 7-6s-3.004-6-7-6-7 2.808-7 6c0 1.468.617 2.83 1.678 3.894z"/>
				</svg>
				{commentCount}
			</span>
		{/if}

		<!-- PR link icon -->
		{#if task.pr_url}
			<a
				href={task.pr_url}
				target="_blank"
				rel="noopener"
				title="View Pull Request"
				style="
					display: flex; align-items: center; justify-content: center;
					width: 22px; height: 22px; border-radius: 4px;
					background: rgba(63, 185, 80, 0.08);
					color: var(--accent-green);
					transition: all 0.15s ease;
				"
				onclick={(e) => e.stopPropagation()}
			>
				<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
					<path d="M7.177 3.073L9.573.677A.25.25 0 0110 .854v4.792a.25.25 0 01-.427.177L7.177 3.427a.25.25 0 010-.354zM3.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122v5.256a2.251 2.251 0 11-1.5 0V5.372A2.25 2.25 0 011.5 3.25zM11 2.5h-1V4h1a1 1 0 011 1v5.628a2.251 2.251 0 101.5 0V5A2.5 2.5 0 0011 2.5zm1 10.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0zM3.75 12a.75.75 0 100 1.5.75.75 0 000-1.5z"/>
				</svg>
			</a>
		{/if}

		<!-- Screenshot indicator -->
		{#if hasScreenshots}
			<span title="Has screenshots" style="
				display: flex; align-items: center; justify-content: center;
				width: 22px; height: 22px; border-radius: 4px;
				background: rgba(188, 140, 255, 0.08);
				color: var(--accent-purple);
			">
				<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
					<path d="M4.502 9a1.5 1.5 0 100-3 1.5 1.5 0 000 3z"/>
					<path d="M14.002 13a2 2 0 01-2 2h-10a2 2 0 01-2-2V5A2 2 0 012 3h2.5l.83-1.36A1 1 0 016.18 1h3.64a1 1 0 01.86.49L11.5 3h2.5a2 2 0 012 2v8z"/>
				</svg>
			</span>
		{/if}
	</div>
</div>
