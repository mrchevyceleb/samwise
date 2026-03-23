<script lang="ts">
	import { onMount } from 'svelte';
	import type { AeTask, TaskStatus, TaskPriority } from '$lib/types';
	import { PRIORITY_COLORS, KANBAN_COLUMNS } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { formatTimeAgo } from '$lib/utils/relative-time';
	import { safeInvoke } from '$lib/utils/tauri';
	import CommentThread from './CommentThread.svelte';

	interface Artifact {
		id: string;
		task_id: string;
		title: string;
		content: string;
		artifact_type: string;
		created_at: string;
	}

	interface Props {
		task: AeTask;
		onClose: () => void;
	}

	let { task, onClose }: Props = $props();
	const taskStore = getTaskStore();
	const theme = getTheme();

	// Editing states
	let editingTitle = $state(false);
	let editTitle = $state(task.title);
	let editingDesc = $state(false);
	let editDesc = $state(task.description || '');
	let confirmDelete = $state(false);
	let deleteHovered = $state(false);
	let requeueHovered = $state(false);
	let prBtnHovered = $state(false);

	// Tabs: "details" or "report"
	let activeTab = $state<'details' | 'report'>('details');
	let artifacts = $state<Artifact[]>([]);
	let loadingArtifacts = $state(false);

	onMount(async () => {
		// Fetch artifacts for this task
		loadingArtifacts = true;
		try {
			const result = await safeInvoke<Artifact[]>('supabase_fetch_artifacts', { task_id: task.id });
			if (result && result.length > 0) {
				artifacts = result;
			}
		} catch (e) {
			console.warn('[task-detail] Failed to fetch artifacts:', e);
		} finally {
			loadingArtifacts = false;
		}
	});

	let hasReport = $derived(artifacts.some(a => a.artifact_type === 'report'));
	let reportArtifact = $derived(artifacts.find(a => a.artifact_type === 'report'));

	// Derived
	let elapsed = $derived(formatTimeAgo(new Date(task.created_at).getTime()));
	let priorityColor = $derived(PRIORITY_COLORS[task.priority]);
	let statusColumn = $derived(KANBAN_COLUMNS.find(c => c.status === task.status));
	let isFailed = $derived(task.status === 'failed');
	let isAgent = $derived(task.assignee === 'agent');
	let hasBefore = $derived(task.screenshots_before && task.screenshots_before.length > 0);
	let hasAfter = $derived(task.screenshots_after && task.screenshots_after.length > 0);
	let hasScreenshots = $derived(hasBefore || hasAfter);
	let hasVisualQA = $derived(task.visual_qa_result !== null);

	async function saveTitle() {
		if (editTitle.trim() && editTitle !== task.title) {
			await taskStore.updateTask(task.id, { title: editTitle.trim() });
		}
		editingTitle = false;
	}

	async function saveDescription() {
		if (editDesc !== (task.description || '')) {
			await taskStore.updateTask(task.id, { description: editDesc || null });
		}
		editingDesc = false;
	}

	async function changePriority(p: TaskPriority) {
		await taskStore.updateTask(task.id, { priority: p });
	}

	async function changeStatus(s: TaskStatus) {
		await taskStore.moveTask(task.id, s);
	}

	async function handleRequeue() {
		await taskStore.moveTask(task.id, 'queued');
	}

	async function handleDelete() {
		if (!confirmDelete) {
			confirmDelete = true;
			setTimeout(() => confirmDelete = false, 3000);
			return;
		}
		await taskStore.updateTask(task.id, { status: 'done' as TaskStatus });
		onClose();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}

	function formatDate(iso: string | null): string {
		if (!iso) return '-';
		return new Date(iso).toLocaleString();
	}

	const priorities: { value: TaskPriority; label: string; color: string }[] = [
		{ value: 'critical', label: 'Critical', color: '#f85149' },
		{ value: 'high', label: 'High', color: '#d29922' },
		{ value: 'medium', label: 'Medium', color: '#6366f1' },
		{ value: 'low', label: 'Low', color: '#6e7681' },
	];
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	style="
		position: fixed; inset: 0; z-index: 200;
		background: {theme.isDark ? 'rgba(0, 0, 0, 0.65)' : 'rgba(0, 0, 0, 0.3)'}; backdrop-filter: blur(6px);
		display: flex; align-items: flex-start; justify-content: center;
		padding-top: 5vh; overflow-y: auto;
		animation: fade-in 0.15s ease;
	"
	onclick={onClose}
>
	<div
		style="
			width: 780px; max-width: 95vw;
			background: {theme.c.gradientModal};
			border: 1px solid {theme.c.borderGlow};
			border-radius: 16px;
			box-shadow: {theme.isDark ? '0 24px 80px rgba(0,0,0,0.6), 0 0 40px rgba(99,102,241,0.08)' : '0 24px 80px rgba(0,0,0,0.15), 0 0 0 1px rgba(0,0,0,0.05)'};
			animation: spring-in 0.25s ease;
			margin-bottom: 5vh;
		"
		onclick={(e) => e.stopPropagation()}
	>
		<!-- Top bar: status + priority + close -->
		<div style="
			display: flex; align-items: center; gap: 8px;
			padding: 16px 20px; border-bottom: 1px solid var(--border-subtle);
		">
			{#if statusColumn}
				<span style="
					display: flex; align-items: center; gap: 5px;
					padding: 4px 10px; border-radius: 6px;
					background: {statusColumn.color}18; color: {statusColumn.color};
					font-size: 11px; font-weight: 700; text-transform: uppercase;
				">
					<span style="width: 7px; height: 7px; border-radius: 50%; background: {statusColumn.color}; box-shadow: 0 0 6px {statusColumn.color}50;"></span>
					{statusColumn.label}
				</span>
			{/if}

			<span style="
				padding: 4px 10px; border-radius: 6px;
				background: {priorityColor}15; color: {priorityColor};
				font-size: 11px; font-weight: 700; text-transform: uppercase;
				border: 1px solid {priorityColor}25;
			">
				{task.priority}
			</span>

			{#if task.source}
				<span style="
					padding: 4px 8px; border-radius: 6px;
					background: rgba(99, 102, 241, 0.08); color: var(--accent-indigo);
					font-size: 10px; font-weight: 600; font-family: var(--font-mono);
				">
					{task.source}
				</span>
			{/if}

			<div style="flex: 1;"></div>

			<span style="font-size: 11px; color: var(--text-muted);">{elapsed} ago</span>

			<!-- Close -->
			<button
				style="width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: 1px solid var(--border-default); background: none; cursor: pointer; color: var(--text-muted); border-radius: 7px; transition: all 0.12s;"
				onmouseenter={(e) => { const el = e.currentTarget as HTMLElement; el.style.background = 'var(--bg-elevated)'; el.style.color = 'var(--text-primary)'; }}
				onmouseleave={(e) => { const el = e.currentTarget as HTMLElement; el.style.background = 'none'; el.style.color = 'var(--text-muted)'; }}
				onclick={onClose}
			>
				<svg width="12" height="12" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
			</button>
		</div>

		<!-- Tab bar -->
		{#if hasReport}
			<div style="
				display: flex; gap: 0; padding: 0 20px;
				border-bottom: 1px solid var(--border-subtle);
				background: var(--bg-surface);
			">
				<button
					style="
						padding: 10px 16px; font-size: 12px; font-weight: 600;
						font-family: var(--font-ui); cursor: pointer;
						background: none; border: none;
						color: {activeTab === 'details' ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						border-bottom: 2px solid {activeTab === 'details' ? 'var(--accent-indigo)' : 'transparent'};
						transition: all 0.15s ease;
					"
					onclick={() => activeTab = 'details'}
				>
					Details
				</button>
				<button
					style="
						padding: 10px 16px; font-size: 12px; font-weight: 600;
						font-family: var(--font-ui); cursor: pointer;
						background: none; border: none;
						color: {activeTab === 'report' ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						border-bottom: 2px solid {activeTab === 'report' ? 'var(--accent-indigo)' : 'transparent'};
						transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px;
					"
					onclick={() => activeTab = 'report'}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 0A2.5 2.5 0 002 2.5v11A2.5 2.5 0 004.5 16h7a2.5 2.5 0 002.5-2.5v-8a.5.5 0 00-.146-.354l-4.5-4.5A.5.5 0 009 .5H4.5zM10 4V1l4 4h-3a1 1 0 01-1-1zM5 7.5a.5.5 0 01.5-.5h5a.5.5 0 010 1h-5a.5.5 0 01-.5-.5zm.5 2.5a.5.5 0 000 1h5a.5.5 0 000-1h-5z"/></svg>
					Report
				</button>
			</div>
		{/if}

		{#if activeTab === 'report' && hasReport && reportArtifact}
			<!-- Report view -->
			<div style="padding: 24px; max-height: 60vh; overflow-y: auto;">
				<div style="
					font-size: 13px; line-height: 1.8; color: var(--text-secondary);
					white-space: pre-wrap; font-family: var(--font-ui);
				">
					{reportArtifact.content}
				</div>
			</div>
		{:else}

		<!-- Main body: left content + right sidebar -->
		<div style="display: flex; min-height: 300px;">
			<!-- Left content (~65%) -->
			<div style="flex: 1; padding: 20px; overflow-y: auto; border-right: 1px solid var(--border-subtle);">
				<!-- Title (editable inline) -->
				{#if editingTitle}
					<input
						type="text"
						bind:value={editTitle}
						style="
							width: 100%; padding: 6px 10px; margin-bottom: 16px;
							background: var(--bg-primary); border: 1px solid rgba(99, 102, 241, 0.3);
							border-radius: 8px; color: var(--text-primary);
							font-size: 20px; font-weight: 700; font-family: var(--font-ui); outline: none;
						"
						onblur={saveTitle}
						onkeydown={(e) => { if (e.key === 'Enter') saveTitle(); if (e.key === 'Escape') editingTitle = false; }}
					/>
				{:else}
					<div
						style="
							font-size: 20px; font-weight: 700; color: var(--text-primary);
							margin-bottom: 16px; cursor: pointer; padding: 4px 0;
							border-radius: 6px; transition: background 0.1s; line-height: 1.3;
						"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
						onclick={() => { editingTitle = true; editTitle = task.title; }}
					>
						{task.title}
						<span style="font-size: 11px; color: var(--text-muted); font-weight: 400; margin-left: 8px;">click to edit</span>
					</div>
				{/if}

				<!-- Description (editable, markdown) -->
				<div style="margin-bottom: 20px;">
					<div style="font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 6px; text-transform: uppercase; letter-spacing: 0.5px;">
						Description
					</div>
					{#if editingDesc}
						<textarea
							bind:value={editDesc}
							rows={6}
							style="
								width: 100%; padding: 10px 12px;
								background: var(--bg-primary); border: 1px solid rgba(99, 102, 241, 0.3);
								border-radius: 8px; color: var(--text-primary);
								font-size: 13px; font-family: var(--font-ui); outline: none; resize: vertical;
								line-height: 1.6;
							"
							onblur={saveDescription}
							onkeydown={(e) => { if (e.key === 'Escape') editingDesc = false; }}
						></textarea>
						<div style="font-size: 10px; color: var(--text-muted); margin-top: 4px;">Click outside or Escape to save</div>
					{:else}
						<div
							style="
								padding: 10px 12px; border-radius: 8px; background: var(--bg-primary);
								min-height: 50px; font-size: 13px; line-height: 1.6;
								color: {task.description ? 'var(--text-secondary)' : 'var(--text-muted)'};
								cursor: pointer; white-space: pre-wrap; transition: background 0.1s;
							"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-primary)'; }}
							onclick={() => { editingDesc = true; editDesc = task.description || ''; }}
						>
							{task.description || 'Click to add a description...'}
						</div>
					{/if}
				</div>

				<!-- Before/After Screenshots (side-by-side) -->
				{#if hasScreenshots}
					<div style="margin-bottom: 20px;">
						<div style="font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.5px;">
							Screenshots
						</div>
						<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
							{#if hasBefore}
								<div>
									<div style="font-size: 10px; font-weight: 600; color: var(--accent-orange); margin-bottom: 4px; text-transform: uppercase;">Before</div>
									<div style="display: flex; flex-direction: column; gap: 6px;">
										{#each task.screenshots_before || [] as src}
											<div style="
												border-radius: 8px; overflow: hidden;
												border: 1px solid var(--border-default);
												background: var(--bg-primary);
											">
												<img {src} alt="Before screenshot" style="width: 100%; height: auto; display: block;" />
											</div>
										{/each}
									</div>
								</div>
							{/if}
							{#if hasAfter}
								<div>
									<div style="font-size: 10px; font-weight: 600; color: var(--accent-green); margin-bottom: 4px; text-transform: uppercase;">After</div>
									<div style="display: flex; flex-direction: column; gap: 6px;">
										{#each task.screenshots_after || [] as src}
											<div style="
												border-radius: 8px; overflow: hidden;
												border: 1px solid var(--border-default);
												background: var(--bg-primary);
											">
												<img {src} alt="After screenshot" style="width: 100%; height: auto; display: block;" />
											</div>
										{/each}
									</div>
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Visual QA Result -->
				{#if hasVisualQA && task.visual_qa_result}
					<div style="margin-bottom: 20px;">
						<div style="font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.5px;">
							Visual QA
						</div>
						<div style="
							display: flex; align-items: flex-start; gap: 10px;
							padding: 12px; border-radius: 10px;
							background: {task.visual_qa_result.pass ? 'rgba(63, 185, 80, 0.06)' : 'rgba(248, 81, 73, 0.06)'};
							border: 1px solid {task.visual_qa_result.pass ? 'rgba(63, 185, 80, 0.15)' : 'rgba(248, 81, 73, 0.15)'};
						">
							<span style="
								padding: 3px 10px; border-radius: 6px; font-size: 11px; font-weight: 700;
								background: {task.visual_qa_result.pass ? 'rgba(63, 185, 80, 0.15)' : 'rgba(248, 81, 73, 0.15)'};
								color: {task.visual_qa_result.pass ? 'var(--accent-green)' : 'var(--accent-red)'};
								text-transform: uppercase; flex-shrink: 0;
							">
								{task.visual_qa_result.pass ? 'PASS' : 'FAIL'}
							</span>
							<div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
								{task.visual_qa_result.explanation}
							</div>
						</div>
					</div>
				{/if}

				<!-- PR Link -->
				{#if task.pr_url}
					<div style="margin-bottom: 20px;">
						<a
							href={task.pr_url}
							target="_blank"
							rel="noopener"
							style="
								display: inline-flex; align-items: center; gap: 8px;
								padding: 8px 16px; border-radius: 8px;
								background: {prBtnHovered ? 'rgba(63, 185, 80, 0.12)' : 'rgba(63, 185, 80, 0.06)'};
								border: 1px solid rgba(63, 185, 80, 0.2);
								color: var(--accent-green); text-decoration: none;
								font-size: 12px; font-weight: 600;
								transition: all 0.15s ease;
								transform: {prBtnHovered ? 'translateY(-1px)' : 'none'};
								box-shadow: {prBtnHovered ? '0 4px 12px rgba(63, 185, 80, 0.15)' : 'none'};
							"
							onmouseenter={() => prBtnHovered = true}
							onmouseleave={() => prBtnHovered = false}
						>
							<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
								<path d="M7.177 3.073L9.573.677A.25.25 0 0110 .854v4.792a.25.25 0 01-.427.177L7.177 3.427a.25.25 0 010-.354zM3.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122v5.256a2.251 2.251 0 11-1.5 0V5.372A2.25 2.25 0 011.5 3.25zM11 2.5h-1V4h1a1 1 0 011 1v5.628a2.251 2.251 0 101.5 0V5A2.5 2.5 0 0011 2.5zm1 10.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0zM3.75 12a.75.75 0 100 1.5.75.75 0 000-1.5z"/>
							</svg>
							View Pull Request
						</a>
					</div>
				{/if}
			</div>

			<!-- Right sidebar (~35%) -->
			<div style="width: 260px; flex-shrink: 0; padding: 20px; display: flex; flex-direction: column; gap: 16px;">
				<!-- Status dropdown -->
				<div>
					<div style="font-size: 10px; font-weight: 600; color: var(--text-muted); margin-bottom: 6px; text-transform: uppercase; letter-spacing: 0.5px;">Status</div>
					<div style="display: flex; flex-direction: column; gap: 3px;">
						{#each KANBAN_COLUMNS as col}
							<button
								style="
									display: flex; align-items: center; gap: 6px;
									padding: 5px 8px; border-radius: 6px;
									border: 1px solid {task.status === col.status ? col.color + '40' : 'transparent'};
									background: {task.status === col.status ? col.color + '12' : 'transparent'};
									color: {task.status === col.status ? col.color : 'var(--text-muted)'};
									font-size: 11px; font-weight: 600; cursor: pointer;
									font-family: var(--font-ui); transition: all 0.12s;
									text-align: left; width: 100%;
								"
								onmouseenter={(e) => { if (task.status !== col.status) (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
								onmouseleave={(e) => { if (task.status !== col.status) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
								onclick={() => changeStatus(col.status)}
							>
								<span style="width: 6px; height: 6px; border-radius: 50%; background: {col.color}; flex-shrink: 0;"></span>
								{col.label}
							</button>
						{/each}
					</div>
				</div>

				<!-- Priority -->
				<div>
					<div style="font-size: 10px; font-weight: 600; color: var(--text-muted); margin-bottom: 6px; text-transform: uppercase; letter-spacing: 0.5px;">Priority</div>
					<div style="display: flex; flex-direction: column; gap: 3px;">
						{#each priorities as p}
							<button
								style="
									display: flex; align-items: center; gap: 6px;
									padding: 5px 8px; border-radius: 6px;
									border: 1px solid {task.priority === p.value ? p.color + '40' : 'transparent'};
									background: {task.priority === p.value ? p.color + '12' : 'transparent'};
									color: {task.priority === p.value ? p.color : 'var(--text-muted)'};
									font-size: 11px; font-weight: 600; cursor: pointer;
									font-family: var(--font-ui); transition: all 0.12s;
									text-align: left; width: 100%;
								"
								onmouseenter={(e) => { if (task.priority !== p.value) (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
								onmouseleave={(e) => { if (task.priority !== p.value) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
								onclick={() => changePriority(p.value)}
							>
								<span style="width: 6px; height: 6px; border-radius: 50%; background: {p.color}; flex-shrink: 0;"></span>
								{p.label}
							</button>
						{/each}
					</div>
				</div>

				<!-- Metadata fields -->
				<div style="display: flex; flex-direction: column; gap: 8px; font-size: 11px;">
					<!-- Assignee -->
					<div style="display: flex; align-items: center; gap: 6px;">
						<span style="color: var(--text-muted); width: 60px;">Assignee</span>
						<span style="display: flex; align-items: center; gap: 4px; color: var(--text-secondary);">
							{#if isAgent}
								<svg width="10" height="10" viewBox="0 0 16 16" fill="var(--accent-indigo)"><path d="M8 0a1 1 0 011 1v1.07A6.002 6.002 0 0114 8v3a2 2 0 01-2 2H4a2 2 0 01-2-2V8a6.002 6.002 0 015-5.93V1a1 1 0 011-1zM6 9a1 1 0 100 2 1 1 0 000-2zm4 0a1 1 0 100 2 1 1 0 000-2z"/></svg>
								Agent
							{:else}
								<svg width="10" height="10" viewBox="0 0 16 16" fill="var(--accent-green)"><path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/></svg>
								Matt
							{/if}
						</span>
					</div>

					<!-- Project -->
					{#if task.project}
						<div style="display: flex; align-items: center; gap: 6px;">
							<span style="color: var(--text-muted); width: 60px;">Project</span>
							<span style="color: var(--accent-indigo); font-weight: 600;">{task.project}</span>
						</div>
					{/if}

					<!-- Source -->
					<div style="display: flex; align-items: center; gap: 6px;">
						<span style="color: var(--text-muted); width: 60px;">Source</span>
						<span style="color: var(--text-secondary);">{task.source}</span>
					</div>

					<!-- Repo URL -->
					{#if task.repo_url}
						<div style="display: flex; align-items: flex-start; gap: 6px;">
							<span style="color: var(--text-muted); width: 60px; flex-shrink: 0;">Repo</span>
							<span style="color: var(--accent-blue); word-break: break-all; font-family: var(--font-mono); font-size: 10px;">{task.repo_url}</span>
						</div>
					{/if}

					<!-- Branch -->
					{#if task.branch}
						<div style="display: flex; align-items: center; gap: 6px;">
							<span style="color: var(--text-muted); width: 60px;">Branch</span>
							<span style="color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px;">{task.branch}</span>
						</div>
					{/if}

					<!-- Dates -->
					<div style="border-top: 1px solid var(--border-subtle); padding-top: 8px; margin-top: 4px; display: flex; flex-direction: column; gap: 4px;">
						<div style="display: flex; align-items: center; gap: 6px;">
							<span style="color: var(--text-muted); width: 60px;">Created</span>
							<span style="color: var(--text-secondary); font-size: 10px;">{formatDate(task.created_at)}</span>
						</div>
						{#if task.claimed_at}
							<div style="display: flex; align-items: center; gap: 6px;">
								<span style="color: var(--text-muted); width: 60px;">Claimed</span>
								<span style="color: var(--text-secondary); font-size: 10px;">{formatDate(task.claimed_at)}</span>
							</div>
						{/if}
						{#if task.completed_at}
							<div style="display: flex; align-items: center; gap: 6px;">
								<span style="color: var(--text-muted); width: 60px;">Done</span>
								<span style="color: var(--text-secondary); font-size: 10px;">{formatDate(task.completed_at)}</span>
							</div>
						{/if}
					</div>
				</div>

				<!-- Actions -->
				<div style="border-top: 1px solid var(--border-subtle); padding-top: 12px; display: flex; flex-direction: column; gap: 6px;">
					{#if isFailed}
						<button
							style="
								width: 100%; padding: 7px 12px; border-radius: 8px;
								background: {requeueHovered ? 'rgba(99, 102, 241, 0.12)' : 'rgba(99, 102, 241, 0.06)'};
								border: 1px solid rgba(99, 102, 241, 0.2);
								color: var(--accent-indigo); font-size: 11px; font-weight: 700;
								font-family: var(--font-ui); cursor: pointer;
								transition: all 0.15s ease;
								transform: {requeueHovered ? 'translateY(-1px)' : 'none'};
							"
							onmouseenter={() => requeueHovered = true}
							onmouseleave={() => requeueHovered = false}
							onclick={handleRequeue}
						>
							Re-queue Task
						</button>
					{/if}

					<button
						style="
							width: 100%; padding: 7px 12px; border-radius: 8px;
							background: {deleteHovered ? 'rgba(248, 81, 73, 0.12)' : 'rgba(248, 81, 73, 0.04)'};
							border: 1px solid {confirmDelete ? 'rgba(248, 81, 73, 0.4)' : 'rgba(248, 81, 73, 0.1)'};
							color: var(--accent-red); font-size: 11px; font-weight: 700;
							font-family: var(--font-ui); cursor: pointer;
							transition: all 0.15s ease;
						"
						onmouseenter={() => deleteHovered = true}
						onmouseleave={() => deleteHovered = false}
						onclick={handleDelete}
					>
						{confirmDelete ? 'Click again to confirm' : 'Delete Task'}
					</button>
				</div>
			</div>
		</div>

		<!-- Bottom section: Comment Thread (full width) -->
		<div style="border-top: 1px solid var(--border-subtle); padding: 16px 20px 20px;">
			<CommentThread taskId={task.id} />
		</div>

		{/if}
	</div>
</div>
