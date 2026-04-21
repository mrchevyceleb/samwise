<script lang="ts">
	import type { TaskPriority, TaskType, AeProject } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import { safeInvoke } from '$lib/utils/tauri';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();
	const taskStore = getTaskStore();
	const projectStore = getProjectStore();

	let problemInput = $state('');
	let project = $state('');
	let priority = $state<TaskPriority>('medium');
	let taskType = $state<TaskType>('code');
	let repoUrl = $state('');
	let repoPath = $state('');
	let previewUrl = $state('');
	let baseBranch = $state('');
	let saving = $state(false);
	let quickMode = $state(true);
	let priorityManuallySet = $state(false);
	let taskTypeManuallySet = $state(false);
	let aiError = $state(false);

	// AI-expanded fields (shown after expansion)
	let expandedTitle = $state('');
	let expandedDesc = $state('');
	let expanded = $state(false);
	let expanding = $state(false);

	let projects = $derived(projectStore.projects);

	// Group projects by client for optgroup rendering
	let groupedProjects = $derived(() => {
		const groups: Record<string, AeProject[]> = {};
		for (const p of projects) {
			const key = p.client || 'Uncategorized';
			if (!groups[key]) groups[key] = [];
			groups[key].push(p);
		}
		return groups;
	});

	function handleProjectSelect(e: Event) {
		const select = e.currentTarget as HTMLSelectElement;
		const selectedId = select.value;
		if (!selectedId) {
			project = '';
			repoUrl = '';
			repoPath = '';
			previewUrl = '';
			return;
		}
		const found = projects.find(p => p.id === selectedId);
		if (found) {
			project = found.name;
			repoUrl = found.repo_url || '';
			repoPath = found.repo_path || '';
			previewUrl = found.preview_url || '';
		}
	}

	interface ExpandedTask {
		title: string;
		description: string;
		priority: string;
		task_type: string;
	}

	const VALID_PRIORITIES: TaskPriority[] = ['critical', 'high', 'medium', 'low'];
	const VALID_TYPES: TaskType[] = ['code', 'research'];

	async function handleSubmit() {
		if (!problemInput.trim() || saving) return;

		// In review phase, validate expanded title
		if (expanded) {
			if (!expandedTitle.trim()) return;
		}

		saving = true;

		try {
			// If not yet expanded, run AI expansion first
			if (!expanded) {
				expanding = true;
				aiError = false;
				const result = await safeInvoke<ExpandedTask>('ai_expand_task', {
					rawInput: problemInput.trim(),
					project: project || '',
				});

				if (result) {
					expandedTitle = result.title;
					expandedDesc = result.description;
					// Only apply AI priority/taskType if user hasn't manually changed them
					if (!priorityManuallySet && result.priority && VALID_PRIORITIES.includes(result.priority as TaskPriority)) {
						priority = result.priority as TaskPriority;
					}
					if (!taskTypeManuallySet && result.task_type && VALID_TYPES.includes(result.task_type as TaskType)) {
						taskType = result.task_type as TaskType;
					}
					expanded = true;
					expanding = false;
					saving = false;
					return; // Show the expanded result for review before creating
				} else {
					// AI expansion failed, use raw input as title
					expandedTitle = problemInput.trim();
					expandedDesc = '';
					aiError = true;
					expanded = true;
					expanding = false;
					saving = false;
					return;
				}
			}

			// Create the task with expanded fields
			await taskStore.createTask({
				title: expandedTitle.trim(),
				description: expandedDesc.trim() || undefined,
				priority,
				task_type: taskType,
				project: project.trim() || undefined,
				repo_url: repoUrl.trim() || undefined,
				repo_path: repoPath.trim() || undefined,
				preview_url: previewUrl.trim() || undefined,
				base_branch: baseBranch.trim() || undefined,
			});
			onClose();
		} finally {
			saving = false;
			expanding = false;
		}
	}

	function handleBack() {
		expanded = false;
		expandedTitle = '';
		expandedDesc = '';
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			if (expanded) { handleBack(); } else { onClose(); }
			return;
		}
		if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			handleSubmit();
		}
	}

	const priorities: { value: TaskPriority; label: string; color: string }[] = [
		{ value: 'critical', label: 'Critical', color: '#f85149' },
		{ value: 'high', label: 'High', color: '#d29922' },
		{ value: 'medium', label: 'Medium', color: '#6366f1' },
		{ value: 'low', label: 'Low', color: '#6e7681' },
	];

	let expandHovered = $state(false);
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	style="
		position: fixed; inset: 0; z-index: 200;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(4px);
		display: flex; align-items: center; justify-content: center;
		animation: fade-in 0.15s ease;
	"
	onclick={onClose}
>
	<div
		style="
			width: 500px; max-height: 85vh;
			background: var(--gradient-modal);
			border: 1px solid var(--border-glow);
			border-radius: 16px;
			box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6), 0 0 40px rgba(99, 102, 241, 0.08);
			padding: 24px;
			animation: spring-in 0.25s ease;
			overflow-y: auto;
		"
		onclick={(e) => e.stopPropagation()}
	>
		<!-- Header -->
		<div style="display: flex; align-items: center; gap: 10px; margin-bottom: 20px;">
			<div style="
				width: 32px; height: 32px; border-radius: 8px;
				background: rgba(99, 102, 241, 0.15);
				display: flex; align-items: center; justify-content: center;
				font-size: 16px; color: var(--accent-indigo);
			">
				+
			</div>
			<div style="flex: 1;">
				<div style="font-size: 16px; font-weight: 700; color: var(--text-primary);">New Task</div>
				<div style="font-size: 11px; color: var(--text-muted);">
					{expanded ? 'Review and create' : 'Describe the problem or what you need'}
				</div>
			</div>
			{#if !expanded}
				<button
					style="
						padding: 4px 10px; border-radius: 6px; font-size: 10px; font-weight: 600;
						background: {expandHovered ? 'rgba(99, 102, 241, 0.1)' : 'transparent'};
						border: 1px solid var(--border-default);
						color: var(--text-muted); cursor: pointer;
						font-family: var(--font-ui); transition: all 0.15s;
					"
					onmouseenter={() => expandHovered = true}
					onmouseleave={() => expandHovered = false}
					onclick={() => quickMode = !quickMode}
				>
					{quickMode ? 'More fields' : 'Quick mode'}
				</button>
			{/if}
		</div>

		{#if !expanded}
			<!-- Input Phase: describe the problem -->
			<div style="margin-bottom: 14px;">
				<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
					What's the problem? <span style="color: var(--accent-red);">*</span>
				</label>
				<textarea
					bind:value={problemInput}
					placeholder="e.g. The slideshow and the funnel building AI chat are not working correctly."
					rows={3}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid var(--border-default);
						border-radius: 8px; color: var(--text-primary);
						font-family: var(--font-ui); font-size: 13px;
						outline: none; resize: vertical; min-height: 60px;
						transition: border-color 0.15s; line-height: 1.5;
					"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				></textarea>
				<div style="font-size: 10px; color: var(--text-muted); margin-top: 3px;">
					Just describe what's wrong or what you need. AI will generate the task details.
				</div>
			</div>

			<!-- Project -->
			<div style="margin-bottom: 14px;">
				<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
					Project
				</label>
				<select
					onchange={handleProjectSelect}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid var(--border-default);
						border-radius: 8px; color: var(--text-primary);
						font-family: var(--font-ui); font-size: 13px;
						outline: none; transition: border-color 0.15s;
						cursor: pointer; appearance: none;
						background-image: url('data:image/svg+xml;charset=UTF-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%2212%22%20height%3D%2212%22%20viewBox%3D%220%200%2012%2012%22%3E%3Cpath%20fill%3D%22%236e7681%22%20d%3D%22M2%204l4%204%204-4%22%2F%3E%3C%2Fsvg%3E');
						background-repeat: no-repeat;
						background-position: right 12px center;
						padding-right: 32px;
					"
				>
					<option value="">Select a project</option>
					{#each Object.entries(groupedProjects()) as [client, clientProjects]}
						<optgroup label={client} style="background: var(--bg-primary); color: var(--text-secondary);">
							{#each clientProjects as p}
								<option value={p.id} style="background: var(--bg-primary); color: var(--text-primary);">
									{p.name}
								</option>
							{/each}
						</optgroup>
					{/each}
				</select>
			</div>

			<!-- Extended fields (quick mode toggle) -->
			{#if !quickMode}
				<div style="margin-bottom: 14px; animation: slide-in-top 0.15s ease;">
					<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
						Repo Path (local)
					</label>
					<input
						type="text"
						bind:value={repoPath}
						placeholder="C:\PERSONAL-PROJECTS\my-repo"
						style="
							width: 100%; padding: 10px 12px;
							background: var(--bg-primary); border: 1px solid var(--border-default);
							border-radius: 8px; color: var(--text-primary);
							font-family: var(--font-mono); font-size: 12px;
							outline: none; transition: border-color 0.15s;
						"
					/>
				</div>
				<div style="margin-bottom: 14px; animation: slide-in-top 0.15s ease;">
					<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
						Repository URL
					</label>
					<input
						type="text"
						bind:value={repoUrl}
						placeholder="https://github.com/user/repo"
						style="
							width: 100%; padding: 10px 12px;
							background: var(--bg-primary); border: 1px solid var(--border-default);
							border-radius: 8px; color: var(--text-primary);
							font-family: var(--font-mono); font-size: 12px;
							outline: none; transition: border-color 0.15s;
						"
					/>
				</div>
				<div style="margin-bottom: 14px; animation: slide-in-top 0.15s ease;">
					<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
						Preview URL
					</label>
					<input
						type="text"
						bind:value={previewUrl}
						placeholder="https://my-app.vercel.app"
						style="
							width: 100%; padding: 10px 12px;
							background: var(--bg-primary); border: 1px solid var(--border-default);
							border-radius: 8px; color: var(--text-primary);
							font-family: var(--font-mono); font-size: 12px;
							outline: none; transition: border-color 0.15s;
						"
					/>
				</div>
				<div style="margin-bottom: 14px; animation: slide-in-top 0.15s ease;">
					<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
						Base branch <span style="font-weight: 400; color: var(--text-muted);">(leave blank for main/master)</span>
					</label>
					<input
						type="text"
						bind:value={baseBranch}
						placeholder="feature/payments"
						style="
							width: 100%; padding: 10px 12px;
							background: var(--bg-primary); border: 1px solid var(--border-default);
							border-radius: 8px; color: var(--text-primary);
							font-family: var(--font-mono); font-size: 12px;
							outline: none; transition: border-color 0.15s;
						"
					/>
				</div>
			{/if}

		{:else}
			<!-- Review Phase: show AI-generated title + description, let user edit before creating -->
			{#if aiError}
				<div style="
					margin-bottom: 14px; padding: 8px 12px; border-radius: 8px;
					background: rgba(210, 153, 34, 0.08); border: 1px solid rgba(210, 153, 34, 0.2);
					font-size: 11px; color: var(--accent-amber); line-height: 1.4;
				">
					AI generation wasn't available. Edit the title and description manually below.
				</div>
			{/if}
			<div style="margin-bottom: 14px;">
				<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
					Title
				</label>
				<input
					type="text"
					bind:value={expandedTitle}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid var(--border-default);
						border-radius: 8px; color: var(--text-primary);
						font-family: var(--font-ui); font-size: 13px;
						outline: none; transition: border-color 0.15s;
					"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				/>
			</div>

			<div style="margin-bottom: 14px;">
				<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
					Instructions for Agent
				</label>
				<textarea
					bind:value={expandedDesc}
					rows={5}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid var(--border-default);
						border-radius: 8px; color: var(--text-primary);
						font-family: var(--font-ui); font-size: 13px;
						outline: none; resize: vertical; min-height: 80px;
						transition: border-color 0.15s; line-height: 1.5;
					"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				></textarea>
			</div>

			<div style="margin-bottom: 14px; padding: 8px 10px; background: rgba(99, 102, 241, 0.05); border: 1px solid rgba(99, 102, 241, 0.1); border-radius: 8px;">
				<div style="font-size: 10px; color: var(--text-muted); margin-bottom: 4px;">Your original input:</div>
				<div style="font-size: 12px; color: var(--text-secondary); line-height: 1.4;">{problemInput}</div>
			</div>
		{/if}

		<!-- Priority (always visible) -->
		<div style="margin-bottom: 14px;">
			<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
				Priority
			</label>
			<div style="display: flex; gap: 6px;">
				{#each priorities as p}
					<button
						style="
							flex: 1; padding: 6px 10px; border-radius: 8px;
							border: 1px solid {priority === p.value ? p.color + '50' : 'var(--border-default)'};
							background: {priority === p.value ? p.color + '15' : 'var(--bg-primary)'};
							color: {priority === p.value ? p.color : 'var(--text-muted)'};
							font-size: 11px; font-weight: 600; font-family: var(--font-ui);
							cursor: pointer; transition: all 0.15s ease;
							transform: {priority === p.value ? 'scale(1.03)' : 'scale(1)'};
						"
						onclick={() => { priority = p.value; priorityManuallySet = true; }}
					>
						{p.label}
					</button>
				{/each}
			</div>
		</div>

		<!-- Task Type (always visible) -->
		<div style="margin-bottom: 14px;">
			<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
				Task Type
			</label>
			<div style="display: flex; gap: 6px;">
				<button
					style="
						flex: 1; padding: 8px 10px; border-radius: 8px;
						border: 1px solid {taskType === 'code' ? 'rgba(99, 102, 241, 0.5)' : 'var(--border-default)'};
						background: {taskType === 'code' ? 'rgba(99, 102, 241, 0.12)' : 'var(--bg-primary)'};
						color: {taskType === 'code' ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						font-size: 11px; font-weight: 600; font-family: var(--font-ui);
						cursor: pointer; transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px; justify-content: center;
					"
					onclick={() => { taskType = 'code'; taskTypeManuallySet = true; }}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M5.854 4.854a.5.5 0 10-.708-.708l-3.5 3.5a.5.5 0 000 .708l3.5 3.5a.5.5 0 00.708-.708L2.707 8l3.147-3.146zm4.292 0a.5.5 0 01.708-.708l3.5 3.5a.5.5 0 010 .708l-3.5 3.5a.5.5 0 01-.708-.708L13.293 8l-3.147-3.146z"/>
					</svg>
					Code Change
				</button>
				<button
					style="
						flex: 1; padding: 8px 10px; border-radius: 8px;
						border: 1px solid {taskType === 'research' ? 'rgba(245, 158, 11, 0.5)' : 'var(--border-default)'};
						background: {taskType === 'research' ? 'rgba(245, 158, 11, 0.12)' : 'var(--bg-primary)'};
						color: {taskType === 'research' ? '#f59e0b' : 'var(--text-muted)'};
						font-size: 11px; font-weight: 600; font-family: var(--font-ui);
						cursor: pointer; transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px; justify-content: center;
					"
					onclick={() => { taskType = 'research'; taskTypeManuallySet = true; }}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M11.742 10.344a6.5 6.5 0 10-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 001.415-1.414l-3.85-3.85a1.007 1.007 0 00-.115-.1zM12 6.5a5.5 5.5 0 11-11 0 5.5 5.5 0 0111 0z"/>
					</svg>
					Research / Analysis
				</button>
			</div>
			<div style="font-size: 10px; color: var(--text-muted); margin-top: 4px;">
				{taskType === 'code' ? 'Agent will make code changes and create a PR' : 'Agent will analyze and report back (no code changes)'}
			</div>
		</div>

		<!-- Actions -->
		<div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;">
			{#if expanded}
				<button
					style="
						padding: 8px 16px; border-radius: 8px;
						background: none; border: 1px solid var(--border-default);
						color: var(--text-secondary); font-size: 12px; font-weight: 600;
						font-family: var(--font-ui); cursor: pointer;
						transition: all 0.15s ease;
					"
					onclick={handleBack}
				>
					Back
				</button>
			{:else}
				<button
					style="
						padding: 8px 16px; border-radius: 8px;
						background: none; border: 1px solid var(--border-default);
						color: var(--text-secondary); font-size: 12px; font-weight: 600;
						font-family: var(--font-ui); cursor: pointer;
						transition: all 0.15s ease;
					"
					onclick={onClose}
				>
					Cancel
				</button>
			{/if}
			<button
				style="
					padding: 8px 20px; border-radius: 8px;
					background: {!problemInput.trim() || saving || expanding ? 'var(--text-muted)' : 'var(--accent-primary)'};
					border: none;
					color: white; font-size: 12px; font-weight: 700;
					font-family: var(--font-ui);
					cursor: {!problemInput.trim() || saving || expanding ? 'not-allowed' : 'pointer'};
					transition: all 0.15s ease;
					box-shadow: {!problemInput.trim() || saving || expanding ? 'none' : '0 4px 12px rgba(99, 102, 241, 0.3)'};
				"
				onmouseenter={(e) => { if (problemInput.trim() && !saving && !expanding) (e.currentTarget as HTMLElement).style.transform = 'scale(1.05)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.transform = 'scale(1)'; }}
				onclick={handleSubmit}
				disabled={!problemInput.trim() || saving || expanding || (expanded && !expandedTitle.trim())}
			>
				{#if expanding}
					Generating...
				{:else if expanded}
					{saving ? 'Creating...' : 'Create Task'}
				{:else}
					Generate Task
				{/if}
			</button>
		</div>

		<div style="text-align: center; margin-top: 10px; font-size: 10px; color: var(--text-muted);">
			{expanded ? 'Ctrl+Enter to create, Escape to cancel' : 'Ctrl+Enter to generate, Escape to cancel'}
		</div>
	</div>
</div>
