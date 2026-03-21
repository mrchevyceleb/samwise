<script lang="ts">
	import type { TaskPriority, TaskType, AeProject } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
	import { onMount } from 'svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();
	const taskStore = getTaskStore();

	let title = $state('');
	let description = $state('');
	let project = $state('');
	let priority = $state<TaskPriority>('medium');
	let taskType = $state<TaskType>('code');
	let repoUrl = $state('');
	let repoPath = $state('');
	let previewUrl = $state('');
	let saving = $state(false);
	let quickMode = $state(true);

	let projects = $state<AeProject[]>([]);
	let projectsLoading = $state(true);

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

	onMount(async () => {
		const result = await safeInvoke<AeProject[]>('supabase_fetch_projects');
		if (result) {
			projects = result;
		}
		projectsLoading = false;
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

	let titleError = $derived(!title.trim() ? 'Title is required' : '');

	async function handleSubmit() {
		if (!title.trim() || saving) return;
		saving = true;
		try {
			await taskStore.createTask({
				title: title.trim(),
				description: description.trim() || undefined,
				priority,
				task_type: taskType,
				project: project.trim() || undefined,
				repo_url: repoUrl.trim() || undefined,
				repo_path: repoPath.trim() || undefined,
				preview_url: previewUrl.trim() || undefined,
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
			background: linear-gradient(180deg, #1c2333 0%, #161b22 100%);
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
				<div style="font-size: 11px; color: var(--text-muted);">Create a task for the AI agent</div>
			</div>
			<!-- Quick/Full toggle -->
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
		</div>

		<!-- Title -->
		<div style="margin-bottom: 14px;">
			<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
				Title <span style="color: var(--accent-red);">*</span>
			</label>
			<input
				type="text"
				bind:value={title}
				placeholder="What should the agent do?"
				style="
					width: 100%; padding: 10px 12px;
					background: var(--bg-primary); border: 1px solid {titleError && title ? 'var(--accent-red)' : 'var(--border-default)'};
					border-radius: 8px; color: var(--text-primary);
					font-family: var(--font-ui); font-size: 13px;
					outline: none; transition: border-color 0.15s;
				"
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
			/>
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
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
			>
				<option value="" style="color: var(--text-muted);">
					{projectsLoading ? 'Loading projects...' : 'Select a project'}
				</option>
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

		<!-- Priority -->
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
						onclick={() => priority = p.value}
					>
						{p.label}
					</button>
				{/each}
			</div>
		</div>

		<!-- Task Type -->
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
					onclick={() => taskType = 'code'}
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
					onclick={() => taskType = 'research'}
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" opacity="0.8">
						<path d="M11.742 10.344a6.5 6.5 0 10-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 001.415-1.414l-3.85-3.85a1.007 1.007 0 00-.115-.1zM12 6.5a5.5 5.5 0 11-11 0 5.5 5.5 0 0111 0z"/>
					</svg>
					Research / Analysis
				</button>
			</div>
			<div style="font-size: 10px; color: var(--text-muted); margin-top: 4px;">
				{taskType === 'code' ? 'Agent will make code changes and create a PR' : 'Agent will analyze and report back in comments (no code changes)'}
			</div>
		</div>

		<!-- Extended fields -->
		{#if !quickMode}
			<!-- Description -->
			<div style="margin-bottom: 14px; animation: slide-in-top 0.15s ease;">
				<label style="font-size: 11px; font-weight: 600; color: var(--text-secondary); display: block; margin-bottom: 4px;">
					Description
				</label>
				<textarea
					bind:value={description}
					placeholder="Detailed description (markdown supported)..."
					rows={4}
					style="
						width: 100%; padding: 10px 12px;
						background: var(--bg-primary); border: 1px solid var(--border-default);
						border-radius: 8px; color: var(--text-primary);
						font-family: var(--font-ui); font-size: 13px;
						outline: none; resize: vertical; min-height: 60px;
						transition: border-color 0.15s;
					"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				></textarea>
			</div>

			<!-- Repo Path (local) -->
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
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				/>
			</div>

			<!-- Repo URL -->
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
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				/>
			</div>

			<!-- Preview URL -->
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
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.4)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				/>
			</div>
		{/if}

		<!-- Actions -->
		<div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;">
			<button
				style="
					padding: 8px 16px; border-radius: 8px;
					background: none; border: 1px solid var(--border-default);
					color: var(--text-secondary); font-size: 12px; font-weight: 600;
					font-family: var(--font-ui); cursor: pointer;
					transition: all 0.15s ease;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-bright)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				onclick={onClose}
			>
				Cancel
			</button>
			<button
				style="
					padding: 8px 20px; border-radius: 8px;
					background: {!title.trim() || saving ? 'var(--text-muted)' : 'var(--accent-primary)'};
					border: none;
					color: white; font-size: 12px; font-weight: 700;
					font-family: var(--font-ui);
					cursor: {!title.trim() || saving ? 'not-allowed' : 'pointer'};
					transition: all 0.15s ease;
					box-shadow: {!title.trim() || saving ? 'none' : '0 4px 12px rgba(99, 102, 241, 0.3)'};
				"
				onmouseenter={(e) => { if (title.trim() && !saving) (e.currentTarget as HTMLElement).style.transform = 'scale(1.05)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.transform = 'scale(1)'; }}
				onclick={handleSubmit}
				disabled={!title.trim() || saving}
			>
				{saving ? 'Creating...' : 'Create Task'}
			</button>
		</div>

		<div style="text-align: center; margin-top: 10px; font-size: 10px; color: var(--text-muted);">
			Ctrl+Enter to create, Escape to cancel
		</div>
	</div>
</div>
