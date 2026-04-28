<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { AeTask, Subtask } from '$lib/types';
	import { PRIORITY_COLORS, ORIGIN_BADGES } from '$lib/types';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import { getDragStore } from '$lib/stores/drag.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import {
		extractReviewActionPanel,
		getUiStamp,
		getMergeDeployState,
		getMergeConflictFixState,
		isMergeConflictError,
		isMergeConflictFixBusy,
		isReviewActionStatus,
		isMergeDeployBusy,
		mergeConflictFixButtonLabel,
		mergeDeployButtonLabel,
		nextManualInProgressStampContext,
		requestMergeConflictFixContext,
		requestMergeDeployContext,
	} from '$lib/utils/review-actions';
	import { formatTimeAgo } from '$lib/utils/relative-time';
	import { openExternal } from '$lib/utils/tauri';

	interface Props {
		task: AeTask;
		onClick?: (task: AeTask) => void;
		onContextMenu?: (task: AeTask, x: number, y: number) => void;
	}

	let { task, onClick, onContextMenu }: Props = $props();
	const commentStore = getCommentStore();
	const drag = getDragStore();
	const theme = getTheme();
	const taskStore = getTaskStore();

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
	let comments = $derived(commentStore.getComments(task.id));
	let reviewPanel = $derived(extractReviewActionPanel(task, comments));
	let uiStamp = $derived(getUiStamp(task));
	let mergeDeployState = $derived(getMergeDeployState(task));
	let mergeConflictFixState = $derived(getMergeConflictFixState(task));
	let mergeDeployRequestError = $state<string | null>(null);
	let mergeConflictFixRequestError = $state<string | null>(null);
	let showReviewActions = $derived(isReviewActionStatus(task.status) && !!(reviewPanel || task.pr_url));
	let canMergeDeploy = $derived(!!task.pr_url && (task.status === 'approved' || mergeDeployState.status === 'failed'));
	let canRequestMergeConflictFix = $derived(
		!!task.pr_url &&
		mergeDeployState.status === 'failed' &&
		isMergeConflictError(mergeDeployState.error) &&
		!isMergeConflictFixBusy(mergeConflictFixState)
	);
	let qaResult = $derived(task.visual_qa_result);
	let originBadge = $derived(
		task.origin_system && task.origin_system !== 'manual'
			? ORIGIN_BADGES[task.origin_system]
			: null
	);
	let subtasks = $derived(task.subtasks || []);
	let subtaskTotal = $derived(subtasks.length);
	let subtaskDone = $derived(subtasks.filter((s: Subtask) => s.done).length);
	let hasSubtasks = $derived(subtaskTotal > 0);
	let subtaskAllDone = $derived(subtaskTotal > 0 && subtaskDone === subtaskTotal);

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
		if ((e.target as HTMLElement).closest('a,button')) return;
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

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		onContextMenu?.(task, e.clientX, e.clientY);
	}

	async function markDone(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		await taskStore.moveTask(task.id, 'done');
	}

	async function toggleManualInProgressStamp(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		await taskStore.updateTask(task.id, { context: nextManualInProgressStampContext(task) });
	}

	async function requestMergeDeploy(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		if (!canMergeDeploy || isMergeDeployBusy(mergeDeployState)) return;
		mergeDeployRequestError = null;
		const ok = await taskStore.updateTask(task.id, { context: requestMergeDeployContext(task) });
		if (!ok) {
			mergeDeployRequestError = taskStore.error || 'Could not queue Merge + Deploy.';
		}
	}

	async function requestMergeConflictFix(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		if (!canRequestMergeConflictFix) return;
		mergeConflictFixRequestError = null;
		const ok = await taskStore.updateTask(task.id, { context: requestMergeConflictFixContext(task) });
		if (!ok) {
			mergeConflictFixRequestError = taskStore.error || 'Could not queue Sam conflict recovery.';
		}
	}

	function openPr(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		if (task.pr_url) openExternal(task.pr_url);
	}

	function verdictColor(verdict: string | undefined): string {
		if (verdict === 'merge') return '#3fb950';
		if (verdict === 'fix' || verdict === 'blocked') return '#f0883e';
		if (verdict === 'errored') return '#f85149';
		return '#58a6ff';
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
	oncontextmenu={handleContextMenu}
>
	{#if uiStamp}
		<div style="
			display: flex; align-items: center; justify-content: space-between; gap: 8px;
			margin: -2px -2px 10px -2px; padding: 8px 10px; border-radius: 12px;
			background: linear-gradient(135deg, rgba(249, 115, 22, 0.38), rgba(245, 158, 11, 0.20));
			border: 1px solid rgba(253, 186, 116, 0.72);
			box-shadow: 0 0 24px rgba(249, 115, 22, 0.24), inset 0 1px 0 rgba(255,255,255,0.18);
		">
			<span style="
				color: #fff7ed; font-size: 11px; font-weight: 950;
				text-transform: uppercase; letter-spacing: 0.55px;
			">
				Manual In Progress
			</span>
			<span style="
				padding: 2px 6px; border-radius: 999px;
				background: rgba(2, 6, 23, 0.36); color: #fdba74;
				border: 1px solid rgba(253, 186, 116, 0.45);
				font-size: 9px; font-weight: 950; letter-spacing: 0.45px;
			">
				MINE
			</span>
		</div>
	{/if}

	{#if task.on_hold}
		<div style="
			display: flex; align-items: center; justify-content: space-between; gap: 8px;
			margin: -2px -2px 10px -2px; padding: 8px 10px; border-radius: 12px;
			background: linear-gradient(135deg, rgba(148, 163, 184, 0.30), rgba(100, 116, 139, 0.16));
			border: 1px solid rgba(203, 213, 225, 0.55);
			box-shadow: 0 0 18px rgba(100, 116, 139, 0.20), inset 0 1px 0 rgba(255,255,255,0.18);
		">
			<span style="display: inline-flex; align-items: center; gap: 6px;">
				<svg width="11" height="11" viewBox="0 0 16 16" fill="#e2e8f0" aria-hidden="true">
					<path d="M11.5 1.75C11.5 .784 12.284 0 13.25 0a1.75 1.75 0 011.75 1.75v12.5A1.75 1.75 0 0113.25 16a1.75 1.75 0 01-1.75-1.75V1.75zm-7 0C4.5.784 5.284 0 6.25 0A1.75 1.75 0 018 1.75v12.5A1.75 1.75 0 016.25 16 1.75 1.75 0 014.5 14.25V1.75z"/>
				</svg>
				<span style="color: #f1f5f9; font-size: 11px; font-weight: 950; text-transform: uppercase; letter-spacing: 0.55px;">
					On Hold
				</span>
			</span>
			<span style="
				padding: 2px 6px; border-radius: 999px;
				background: rgba(2, 6, 23, 0.36); color: #cbd5e1;
				border: 1px solid rgba(203, 213, 225, 0.45);
				font-size: 9px; font-weight: 950; letter-spacing: 0.45px;
			">
				SAM SKIPS
			</span>
		</div>
	{/if}

	<!-- Title (truncated to 2 lines) -->
	<div style="
		font-size: 14px; font-weight: 600; color: var(--text-primary);
		margin-bottom: 6px; line-height: 1.4;
		display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
		overflow: hidden; text-overflow: ellipsis;
	">
		{task.title}
	</div>

	{#if reviewPanel && isReviewActionStatus(task.status)}
		<div style="
			margin-bottom: 9px; padding: 9px; border-radius: 10px;
			background: linear-gradient(135deg, {verdictColor(reviewPanel.verdict)}22, rgba(99, 102, 241, 0.08));
			border: 1px solid {verdictColor(reviewPanel.verdict)}55;
			box-shadow: inset 0 1px 0 rgba(255,255,255,0.08);
		">
			<div style="display: flex; align-items: center; gap: 6px; margin-bottom: 6px;">
				<span style="
					font-size: 10px; font-weight: 900; text-transform: uppercase; letter-spacing: 0.45px;
					color: {verdictColor(reviewPanel.verdict)};
				">
					{reviewPanel.label}
				</span>
				<div style="flex: 1;"></div>
				{#if task.pr_url}
					<button
						type="button"
						style="
							padding: 3px 7px; border-radius: 6px;
							border: 1px solid rgba(255,255,255,0.12);
							background: rgba(0,0,0,0.18);
							color: var(--text-primary);
							font-size: 10px; font-weight: 800;
							cursor: pointer;
						"
						onmousedown={(e) => e.stopPropagation()}
						onclick={openPr}
					>
						PR
					</button>
				{/if}
			</div>
			<div style="
				font-size: 11px; color: var(--text-secondary); line-height: 1.35;
				display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical;
				overflow: hidden;
			">
				{reviewPanel.why}
			</div>
			<div style="
				margin-top: 6px; font-size: 10px; line-height: 1.3;
				color: {reviewPanel.hasDeploymentCallout ? 'var(--accent-orange)' : 'var(--text-muted)'};
				font-weight: 700;
				display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
				overflow: hidden;
			">
				Deploy: {reviewPanel.deployment}
			</div>
		</div>
	{/if}

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

		<!-- Origin badge (Operly / Banana / Sentry) -->
		{#if originBadge}
			{#if task.origin_url}
				<a
					href={task.origin_url}
					title="Open source ticket in {originBadge.label}"
					onclick={(e) => { e.stopPropagation(); e.preventDefault(); openExternal(task.origin_url!); }}
					onmousedown={(e) => e.stopPropagation()}
					style="
						font-size: 10px; font-weight: 700; padding: 2px 7px; border-radius: 5px;
						background: {originBadge.bg}; color: {originBadge.color};
						border: 1px solid {originBadge.border};
						text-decoration: none; cursor: pointer;
						display: inline-flex; align-items: center; gap: 4px;
					"
				>
					<span style="width: 5px; height: 5px; border-radius: 50%; background: {originBadge.color};"></span>
					{originBadge.label}
				</a>
			{:else}
				<span
					title="From {originBadge.label}"
					style="
						font-size: 10px; font-weight: 700; padding: 2px 7px; border-radius: 5px;
						background: {originBadge.bg}; color: {originBadge.color};
						border: 1px solid {originBadge.border};
						display: inline-flex; align-items: center; gap: 4px;
					"
				>
					<span style="width: 5px; height: 5px; border-radius: 50%; background: {originBadge.color};"></span>
					{originBadge.label}
				</span>
			{/if}
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

	<!-- Sam's commit message (Root Cause / Fixes Made / CS Message). Shown on
	     any post-commit status so Matt can scan what's about to merge without
	     opening the PR. Constrained height + scroll so a long body doesn't
	     dominate the card. -->
	{#if task.commit_message && task.commit_message.trim()}
		<div style="
			margin-bottom: 8px; padding: 8px 10px; border-radius: 6px;
			background: rgba(99, 102, 241, 0.05);
			border-left: 2px solid rgba(99, 102, 241, 0.45);
			font-family: var(--font-mono); font-size: 10px; line-height: 1.4;
			color: var(--text-secondary); white-space: pre-wrap; word-wrap: break-word;
			max-height: 200px; overflow-y: auto;
		"
			onmousedown={(e) => e.stopPropagation()}
			onclick={(e) => e.stopPropagation()}
		>{task.commit_message}</div>
	{/if}

	{#if showReviewActions && task.status !== 'done'}
		<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 6px; margin-bottom: 8px;">
			<button
				type="button"
				style="
					display: flex; align-items: center; justify-content: center; gap: 5px;
					padding: 7px 8px; border-radius: 8px;
					background: {uiStamp ? 'rgba(249, 115, 22, 0.22)' : 'rgba(249, 115, 22, 0.08)'};
					border: 1px solid {uiStamp ? 'rgba(253, 186, 116, 0.54)' : 'rgba(253, 186, 116, 0.24)'};
					color: #fdba74;
					font-size: 10px; font-weight: 900; font-family: var(--font-ui);
					cursor: pointer; transition: all 0.15s ease;
				"
				onmousedown={(e) => e.stopPropagation()}
				onclick={toggleManualInProgressStamp}
			>
				{uiStamp ? 'Clear Stamp' : 'In Progress'}
			</button>
			<button
				type="button"
				style="
					display: flex; align-items: center; justify-content: center; gap: 5px;
					padding: 7px 8px; border-radius: 8px;
					background: {canMergeDeploy ? 'rgba(34, 211, 238, 0.10)' : 'rgba(63, 185, 80, 0.08)'};
					border: 1px solid {canMergeDeploy ? 'rgba(103, 232, 249, 0.32)' : 'rgba(63, 185, 80, 0.24)'};
					color: {canMergeDeploy ? '#67e8f9' : 'var(--accent-green)'};
					font-size: 10px; font-weight: 900; font-family: var(--font-ui);
					cursor: {isMergeDeployBusy(mergeDeployState) ? 'wait' : 'pointer'}; transition: all 0.15s ease;
					opacity: {isMergeDeployBusy(mergeDeployState) ? '0.72' : '1'};
				"
				onmousedown={(e) => e.stopPropagation()}
				onclick={canMergeDeploy ? requestMergeDeploy : markDone}
				disabled={isMergeDeployBusy(mergeDeployState)}
			>
				{canMergeDeploy ? mergeDeployButtonLabel(mergeDeployState) : 'Mark Done'}
			</button>
		</div>
		{#if mergeDeployRequestError || mergeDeployState.error}
			<div style="
				margin-top: 7px; padding: 7px 8px; border-radius: 8px;
				background: rgba(248, 81, 73, 0.12);
				border: 1px solid rgba(248, 81, 73, 0.34);
				color: #ffb4ae; font-size: 10px; font-weight: 750; line-height: 1.35;
				display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical;
				overflow: hidden;
			">
				Merge + Deploy failed: {mergeDeployRequestError || mergeDeployState.error}
			</div>
		{/if}
		{#if canRequestMergeConflictFix || isMergeConflictFixBusy(mergeConflictFixState) || mergeConflictFixRequestError || mergeConflictFixState.error}
			<button
				type="button"
				style="
					width: 100%; margin-top: 7px; padding: 8px 9px; border-radius: 9px;
					background: linear-gradient(135deg, rgba(251, 146, 60, 0.22), rgba(20, 184, 166, 0.14));
					border: 1px solid rgba(251, 191, 36, 0.42);
					color: #fed7aa;
					font-size: 10px; font-weight: 950; font-family: var(--font-ui);
					cursor: {isMergeConflictFixBusy(mergeConflictFixState) ? 'wait' : 'pointer'};
					opacity: {isMergeConflictFixBusy(mergeConflictFixState) ? '0.72' : '1'};
				"
				onmousedown={(e) => e.stopPropagation()}
				onclick={requestMergeConflictFix}
				disabled={isMergeConflictFixBusy(mergeConflictFixState)}
			>
				{mergeConflictFixButtonLabel(mergeConflictFixState)}
			</button>
		{/if}
		{#if mergeConflictFixRequestError || mergeConflictFixState.error}
			<div style="
				margin-top: 7px; padding: 7px 8px; border-radius: 8px;
				background: rgba(251, 146, 60, 0.12);
				border: 1px solid rgba(251, 191, 36, 0.34);
				color: #fed7aa; font-size: 10px; font-weight: 750; line-height: 1.35;
				display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical;
				overflow: hidden;
			">
				Sam conflict recovery failed: {mergeConflictFixRequestError || mergeConflictFixState.error}
			</div>
		{/if}
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

		{#if hasSubtasks}
			<span style="
				display: flex; align-items: center; gap: 4px;
				font-size: 10px; font-weight: 600;
				color: {subtaskAllDone ? 'var(--accent-green)' : subtaskDone > 0 ? 'var(--accent-indigo)' : 'var(--text-muted)'};
				padding: 2px 6px; border-radius: 4px;
				background: {subtaskAllDone ? 'rgba(63, 185, 80, 0.08)' : subtaskDone > 0 ? 'rgba(99, 102, 241, 0.08)' : 'var(--bg-primary)'};
			">
				<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
					<path d="M2.5 1.75a.25.25 0 01.25-.25h10.5a.25.25 0 01.25.25v12.5a.25.25 0 01-.25.25H2.75a.25.25 0 01-.25-.25V1.75zM2.75 0A1.75 1.75 0 001 1.75v12.5c0 .966.784 1.75 1.75 1.75h10.5A1.75 1.75 0 0015 14.25V1.75A1.75 1.75 0 0013.25 0H2.75zM5 6a1 1 0 100-2 1 1 0 000 2zm0 4a1 1 0 100-2 1 1 0 000 2zm0 4a1 1 0 100-2 1 1 0 000 2zm3-11a.75.75 0 000 1.5h3a.75.75 0 000-1.5H8zM7.25 7a.75.75 0 01.75-.75h3a.75.75 0 010 1.5H8A.75.75 0 017.25 7zM8 10.25a.75.75 0 000 1.5h3a.75.75 0 000-1.5H8z"/>
				</svg>
				{subtaskDone}/{subtaskTotal}
			</span>
			<div style="width: 36px; height: 3px; border-radius: 2px; background: var(--bg-primary); overflow: hidden;">
				<div style="
					width: {subtaskTotal > 0 ? (subtaskDone / subtaskTotal) * 100 : 0}%; height: 100%;
					background: {subtaskAllDone ? 'var(--accent-green)' : 'var(--accent-indigo)'};
					transition: width 0.3s ease;
				"></div>
			</div>
		{/if}

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
			<button
				type="button"
				title="View Pull Request"
				style="
					display: flex; align-items: center; justify-content: center;
					width: 22px; height: 22px; border-radius: 4px;
					background: rgba(63, 185, 80, 0.08);
					color: var(--accent-green);
					border: none; cursor: pointer;
					transition: all 0.15s ease;
				"
				onclick={(e) => { e.stopPropagation(); openExternal(task.pr_url!); }}
			>
				<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
					<path d="M7.177 3.073L9.573.677A.25.25 0 0110 .854v4.792a.25.25 0 01-.427.177L7.177 3.427a.25.25 0 010-.354zM3.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122v5.256a2.251 2.251 0 11-1.5 0V5.372A2.25 2.25 0 011.5 3.25zM11 2.5h-1V4h1a1 1 0 011 1v5.628a2.251 2.251 0 101.5 0V5A2.5 2.5 0 0011 2.5zm1 10.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0zM3.75 12a.75.75 0 100 1.5.75.75 0 000-1.5z"/>
				</svg>
			</button>
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
