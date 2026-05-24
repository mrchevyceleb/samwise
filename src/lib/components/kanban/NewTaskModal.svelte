<script lang="ts">
	import type { TaskPriority, TaskType, AeProject } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';

	interface Props {
		onClose: () => void;
	}

	type RepoMode = 'project' | 'none' | 'multiple';

	let { onClose }: Props = $props();
	const taskStore = getTaskStore();
	const projectStore = getProjectStore();

	let prompt = $state('');
	let repoMode = $state<RepoMode>('project');
	let selectedProjectId = $state('');
	let project = $state('');
	let priority = $state<TaskPriority>('medium');
	let taskType = $state<TaskType>('code');
	let qaEnvironment = $state<'staging' | 'production'>('staging');
	let repoUrl = $state('');
	let repoPath = $state('');
	let previewUrl = $state('');
	let baseBranch = $state('');
	let saving = $state(false);

	let projects = $derived(projectStore.projects);
	let repoRequired = $derived(repoMode === 'project' && !project.trim());
	let canSubmit = $derived(prompt.trim().length > 0 && !repoRequired && !saving);

	let groupedProjects = $derived(() => {
		const groups: Record<string, AeProject[]> = {};
		for (const p of projects) {
			const key = p.client || 'Uncategorized';
			if (!groups[key]) groups[key] = [];
			groups[key].push(p);
		}
		return groups;
	});

	function clearProject() {
		selectedProjectId = '';
		project = '';
		repoUrl = '';
		repoPath = '';
		previewUrl = '';
		baseBranch = '';
	}

	function setRepoMode(mode: RepoMode) {
		repoMode = mode;
		if (mode !== 'project') clearProject();
	}

	function handleProjectSelect(e: Event) {
		const selectedId = (e.currentTarget as HTMLSelectElement).value;
		selectedProjectId = selectedId;
		// Branch names are repo-specific, so switching projects must wipe any
		// stale branch the user typed for the previous repo.
		baseBranch = '';
		if (!selectedId) {
			clearProject();
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

	function titleFromPrompt(value: string) {
		const first = value
			.trim()
			.split(/\n+/)
			.map(line => line.trim())
			.find(Boolean) || 'New Samwise task';
		return first.length > 90 ? `${first.slice(0, 87).trimEnd()}...` : first;
	}

	async function handleSubmit() {
		if (!canSubmit) return;
		saving = true;

		try {
			const context: Record<string, unknown> = {
				repo_mode: repoMode,
				original_prompt: prompt.trim(),
			};

			if (repoMode === 'none') {
				context.repo_label = 'No repo';
			} else if (repoMode === 'multiple') {
				context.repo_label = 'Multiple repos';
			} else if (project) {
				context.project_id = selectedProjectId;
			}

			const isQa = taskType === 'qa-verify';
			if (isQa) {
				// Worker resolves the QA target from the project + this env at
				// run time (production -> production_url, else staging).
				context.qa_environment = qaEnvironment;
			}

			await taskStore.createTask({
				title: titleFromPrompt(prompt),
				description: prompt.trim(),
				priority,
				task_type: taskType,
				project: repoMode === 'project' ? project.trim() || undefined : undefined,
				repo_url: repoMode === 'project' ? repoUrl.trim() || undefined : undefined,
				repo_path: repoMode === 'project' ? repoPath.trim() || undefined : undefined,
				// For qa-verify leave preview_url unset unless the user typed an
				// explicit override, so the worker picks staging/production.
				preview_url: isQa
					? (previewUrl.trim() || undefined)
					: (repoMode === 'project' ? previewUrl.trim() || undefined : undefined),
				// qa-verify checks staging/production URLs — base branch is ignored
				// by the worker for that path, so don't send a stale value.
				base_branch: (repoMode === 'project' && !isQa) ? baseBranch.trim() || undefined : undefined,
				context,
			});
			onClose();
		} finally {
			saving = false;
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
			return;
		}
		if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			handleSubmit();
		}
	}

	const repoModes: { value: RepoMode; label: string }[] = [
		{ value: 'project', label: 'Single repo' },
		{ value: 'none', label: 'No repo' },
		{ value: 'multiple', label: 'Multiple repos' },
	];
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
			width: 540px; max-height: 85vh;
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
			</div>
		</div>

		<div style="margin-bottom: 16px;">
			<div style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
				Repo
			</div>
			<div style="display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px;">
				{#each repoModes as mode}
					<button
						type="button"
						style="
							min-height: 40px; padding: 8px 10px; border-radius: 8px;
							border: 1px solid {repoMode === mode.value ? 'rgba(99, 102, 241, 0.5)' : 'var(--border-default)'};
							background: {repoMode === mode.value ? 'rgba(99, 102, 241, 0.12)' : 'var(--bg-primary)'};
							color: {repoMode === mode.value ? 'var(--accent-indigo)' : 'var(--text-secondary)'};
							font-family: var(--font-ui); cursor: pointer; text-align: left;
							transition: all 0.15s ease;
						"
						onclick={() => setRepoMode(mode.value)}
					>
						<div style="font-size: 12px; font-weight: 700;">{mode.label}</div>
					</button>
				{/each}
			</div>
		</div>

		{#if repoMode === 'project'}
			<div style="margin-bottom: 16px;">
				<select
					bind:value={selectedProjectId}
					onchange={handleProjectSelect}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid {repoRequired ? 'rgba(248, 81, 73, 0.35)' : 'var(--border-default)'};
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
					<option value="">{projects.length === 0 ? 'No projects configured' : 'Select a repo...'}</option>
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

			{#if taskType !== 'qa-verify'}
				<div style="margin-bottom: 16px;">
					<label for="new-task-base-branch" style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
						Base branch (optional)
					</label>
					<input
						id="new-task-base-branch"
						type="text"
						bind:value={baseBranch}
						placeholder="Leave blank for default branch"
						autocomplete="off"
						spellcheck="false"
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
			{/if}
		{/if}

		<div style="margin-bottom: 16px;">
			<label for="new-task-prompt" style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
				Prompt
			</label>
			<textarea
				id="new-task-prompt"
				bind:value={prompt}
				placeholder={repoMode === 'multiple'
					? 'Name the repos and describe exactly what Sam should do across them.'
					: 'Tell Sam what to build, fix, investigate, or change.'}
				rows={6}
				style="
					width: 100%; padding: 10px 12px;
					background: var(--bg-primary); border: 1px solid var(--border-default);
					border-radius: 8px; color: var(--text-primary);
					font-family: var(--font-ui); font-size: 13px;
					outline: none; resize: vertical; min-height: 120px;
					transition: border-color 0.15s; line-height: 1.5;
				"
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
			></textarea>
		</div>

		<div style="margin-bottom: 14px;">
			<div style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
				Mode
			</div>
			<div style="display: flex; gap: 6px;">
				<button
					type="button"
					style="
						flex: 1; padding: 8px 10px; border-radius: 8px;
						border: 1px solid {taskType === 'code' ? 'rgba(99, 102, 241, 0.5)' : 'var(--border-default)'};
						background: {taskType === 'code' ? 'rgba(99, 102, 241, 0.12)' : 'var(--bg-primary)'};
						color: {taskType === 'code' ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						font-size: 11px; font-weight: 700; font-family: var(--font-ui);
						cursor: pointer; transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px; justify-content: center;
					"
					onclick={() => { taskType = 'code'; }}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M5.854 4.854a.5.5 0 10-.708-.708l-3.5 3.5a.5.5 0 000 .708l3.5 3.5a.5.5 0 00.708-.708L2.707 8l3.147-3.146zm4.292 0a.5.5 0 01.708-.708l3.5 3.5a.5.5 0 010 .708l-3.5 3.5a.5.5 0 01-.708-.708L13.293 8l-3.147-3.146z"/>
					</svg>
					Coding
				</button>
				<button
					type="button"
					style="
						flex: 1; padding: 8px 10px; border-radius: 8px;
						border: 1px solid {taskType === 'research' ? 'rgba(245, 158, 11, 0.5)' : 'var(--border-default)'};
						background: {taskType === 'research' ? 'rgba(245, 158, 11, 0.12)' : 'var(--bg-primary)'};
						color: {taskType === 'research' ? '#f59e0b' : 'var(--text-muted)'};
						font-size: 11px; font-weight: 700; font-family: var(--font-ui);
						cursor: pointer; transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px; justify-content: center;
					"
					onclick={() => { taskType = 'research'; }}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M11.742 10.344a6.5 6.5 0 10-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 001.415-1.414l-3.85-3.85a1.007 1.007 0 00-.115-.1zM12 6.5a5.5 5.5 0 11-11 0 5.5 5.5 0 0111 0z"/>
					</svg>
					Research
				</button>
				<button
					type="button"
					style="
						flex: 1; padding: 8px 10px; border-radius: 8px;
						border: 1px solid {taskType === 'qa-verify' ? 'rgba(16, 185, 129, 0.5)' : 'var(--border-default)'};
						background: {taskType === 'qa-verify' ? 'rgba(16, 185, 129, 0.12)' : 'var(--bg-primary)'};
						color: {taskType === 'qa-verify' ? '#10b981' : 'var(--text-muted)'};
						font-size: 11px; font-weight: 700; font-family: var(--font-ui);
						cursor: pointer; transition: all 0.15s ease;
						display: flex; align-items: center; gap: 6px; justify-content: center;
					"
					onclick={() => { taskType = 'qa-verify'; }}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M13.854 3.646a.5.5 0 010 .708l-7 7a.5.5 0 01-.708 0l-3.5-3.5a.5.5 0 11.708-.708L6.5 10.293l6.646-6.647a.5.5 0 01.708 0z"/>
					</svg>
					QA Verify
				</button>
			</div>

			{#if taskType === 'qa-verify'}
				<div style="margin-top: 10px;">
					<div style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 6px;">
						QA Environment
					</div>
					<div style="display: flex; gap: 6px;">
						<button
							type="button"
							style="
								flex: 1; padding: 8px 10px; border-radius: 8px;
								border: 1px solid {qaEnvironment === 'staging' ? 'rgba(16, 185, 129, 0.5)' : 'var(--border-default)'};
								background: {qaEnvironment === 'staging' ? 'rgba(16, 185, 129, 0.12)' : 'var(--bg-primary)'};
								color: {qaEnvironment === 'staging' ? '#10b981' : 'var(--text-muted)'};
								font-size: 11px; font-weight: 700; font-family: var(--font-ui);
								cursor: pointer; transition: all 0.15s ease;
							"
							onclick={() => { qaEnvironment = 'staging'; }}
						>
							Staging
						</button>
						<button
							type="button"
							style="
								flex: 1; padding: 8px 10px; border-radius: 8px;
								border: 1px solid {qaEnvironment === 'production' ? 'rgba(239, 68, 68, 0.5)' : 'var(--border-default)'};
								background: {qaEnvironment === 'production' ? 'rgba(239, 68, 68, 0.12)' : 'var(--bg-primary)'};
								color: {qaEnvironment === 'production' ? '#ef4444' : 'var(--text-muted)'};
								font-size: 11px; font-weight: 700; font-family: var(--font-ui);
								cursor: pointer; transition: all 0.15s ease;
							"
							onclick={() => { qaEnvironment = 'production'; }}
						>
							Production
						</button>
					</div>
					<div style="font-size: 10px; color: var(--text-muted); margin-top: 5px;">
						Resolves the project's staging or production URL automatically. Pick a project above.
					</div>
				</div>
			{/if}
		</div>

		<div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px;">
			<button
				type="button"
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
			<button
				type="button"
				style="
					padding: 8px 20px; border-radius: 8px;
					background: {!canSubmit ? 'var(--text-muted)' : 'var(--accent-primary)'};
					border: none;
					color: white; font-size: 12px; font-weight: 700;
					font-family: var(--font-ui);
					cursor: {!canSubmit ? 'not-allowed' : 'pointer'};
					transition: all 0.15s ease;
					box-shadow: {!canSubmit ? 'none' : '0 4px 12px rgba(99, 102, 241, 0.3)'};
				"
				onmouseenter={(e) => { if (canSubmit) (e.currentTarget as HTMLElement).style.transform = 'scale(1.05)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.transform = 'scale(1)'; }}
				onclick={handleSubmit}
				disabled={!canSubmit}
			>
				{saving ? 'Queuing...' : 'Queue Task'}
			</button>
		</div>
	</div>
</div>
